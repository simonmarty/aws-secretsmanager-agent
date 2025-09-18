use crate::config::Config;
use crate::error::HttpError;
use aws_secretsmanager_caching::SecretsManagerCachingClient;

/// Wrapper around the caching library
///
/// Used to cache and retrieve secrets.
#[derive(Debug)]
pub struct CacheManager(SecretsManagerCachingClient);

// Use either the real Secrets Manager client or the stub for testing
#[doc(hidden)]
#[cfg(not(test))]
use crate::utils::validate_and_create_asm_client as asm_client;
#[cfg(test)]
use tests::init_client as asm_client;

/// Wrapper around the caching library
///
/// Used to cache and retrieve secrets.
impl CacheManager {
    /// Create a new CacheManager. For simplicity I'm propagating the errors back up for now.
    pub async fn new(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(SecretsManagerCachingClient::new(
            asm_client(cfg).await?,
            cfg.cache_size(),
            cfg.ttl(),
            cfg.ignore_transient_errors(),
        )))
    }

    /// Fetch a secret from the cache.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the secret to fetch.
    /// * `version` - The version of the secret to fetch.
    /// * `label` - The label of the secret to fetch.
    /// * `refresh_now` - Whether to serve from the cache or fetch from ASM.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The value of the secret.
    /// * `Err((u16, String))` - The error code and message.
    ///
    /// # Errors
    ///
    /// * `SerializationError` - The error returned from the serde_json::to_string method.
    ///
    /// # Example
    ///
    /// ```
    /// let cache_manager = CacheManager::new().await.unwrap();
    /// let value = cache_manager.fetch("my-secret", None, None).unwrap();
    /// ```
    pub async fn fetch(
        &self,
        secret_id: &str,
        version: Option<&str>,
        label: Option<&str>,
        refresh_now: bool,
    ) -> Result<String, HttpError> {
        // Read the secret from the cache or fetch it over the network.
        let found = self
            .0
            .get_secret_value(secret_id, version, label, refresh_now)
            .await?;
        // Serialize and return the value
        Ok(serde_json::to_string(&found)?)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::constants::APPNAME;
    use crate::utils::AgentModifierInterceptor;
    use aws_config::BehaviorVersion;
    use aws_sdk_secretsmanager as secretsmanager;
    use aws_smithy_runtime::client::http::test_util::{infallible_client_fn, NeverClient};
    use aws_smithy_types::body::SdkBody;
    use http::{Request, Response};
    use serde_json::Value;
    use std::thread::sleep;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use std::cell::RefCell;
    use std::thread_local;

    pub const FAKE_ARN: &str =
        "arn:aws:secretsmanager:us-west-2:123456789012:secret:{{name}}-NhBWsc";
    pub const DEFAULT_VERSION: &str = "5767290c-d089-49ed-b97c-17086f8c9d79";
    pub const DEFAULT_LABEL: &str = "AWSCURRENT";
    pub const DEFAULT_SECRET_STRING: &str = "hunter2";

    // Template GetSecretValue responses for testing
    const GSV_BODY: &str = r###"{
        "ARN": "{{arn}}",
        "Name": "{{name}}",
        "VersionId": "{{version}}",
        "SecretString": "{{secret}}",
        "VersionStages": [
            "{{label}}"
        ],
        "CreatedDate": 1569534789.046
    }"###;

    // Template DescribeSecret responses for testing
    const DESC_BODY: &str = r###"{
          "ARN": "{{arn}}",
          "Name": "{{name}}",
          "Description": "My test secret",
          "KmsKeyId": "arn:aws:kms:us-west-2:123456789012:key/exampled-90ab-cdef-fedc-bbd6-7e6f303ac933",
          "LastChangedDate": 1523477145.729,
          "LastAccessedDate": 1524572133.25,
          "VersionIdsToStages": {
              "{{version}}": [
                  "{{label}}"
              ]
          },
          "CreatedDate": 1569534789.046
      }"###;

    // Template for access denied testing
    const KMS_ACCESS_DENIED_BODY: &str = r###"{
        "__type":"AccessDeniedException",
        "Message":"Access to KMS is not allowed"
    }"###;

    // Template for testing other errors (bad creds in this case)
    const OTHER_EXCEPTION_BODY: &str = r###"{
        "__type":"InvalidSignatureException",
        "message":"The request signature we calculated does not match ..."
    }"###;

    // Template for testing resource not found with DescribeSecret
    const NOT_FOUND_EXCEPTION_BODY: &str = r###"{
        "__type":"ResourceNotFoundException",
        "message":"Secrets Manager can't find the specified secret."
    }"###;

    // Used to inject a test client to stub off Secrets Manager network calls.
    thread_local! {
        static CLIENT: RefCell<secretsmanager::Client> = RefCell::new(def_fake_client());
    }

    // Test interface to override the default client used.
    pub fn set_client(client: secretsmanager::Client) {
        CLIENT.set(client);
    }

    // Used to replace the real client with the stub client.
    pub async fn init_client(
        _cfg: &Config,
    ) -> Result<secretsmanager::Client, Box<dyn std::error::Error>> {
        Ok(CLIENT.with_borrow(|v| v.clone()))
    }

    // Private helper to look at the request and provide the correct reponse.
    fn format_rsp(req: Request<SdkBody>) -> (u16, String) {
        let (parts, body) = req.into_parts();
        assert!(parts.headers["user-agent"]
            .to_str()
            .unwrap()
            .contains(APPNAME)); // validate user-agent

        let req_map: serde_json::Map<String, Value> =
            serde_json::from_slice(body.bytes().unwrap()).unwrap();
        let version = req_map
            .get("VersionId")
            .map_or(DEFAULT_VERSION, |x| x.as_str().unwrap());
        let label = req_map
            .get("VersionStage")
            .map_or(DEFAULT_LABEL, |x| x.as_str().unwrap());
        let name = req_map.get("SecretId").unwrap().as_str().unwrap(); // Does not handle full ARN case.

        let secret_string = match name {
            secret if secret.starts_with("REFRESHNOW") => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
            _ => DEFAULT_SECRET_STRING.to_string(),
        };

        let (code, template) = match parts.headers["x-amz-target"].to_str().unwrap() {
            "secretsmanager.GetSecretValue" if name.starts_with("KMSACCESSDENIED") => {
                (400, KMS_ACCESS_DENIED_BODY)
            }
            "secretsmanager.GetSecretValue" if name.starts_with("OTHERERROR") => {
                (400, OTHER_EXCEPTION_BODY)
            }
            "secretsmanager.DescribeSecret" if name.starts_with("NOTFOUND") => {
                (400, NOT_FOUND_EXCEPTION_BODY)
            }
            "secretsmanager.GetSecretValue" => (200, GSV_BODY),
            "secretsmanager.DescribeSecret" => (200, DESC_BODY),
            _ => panic!("Unknown operation"),
        };

        // Implement a sleep for testing. We can not do an async sleep here so
        // timeout tests should use the timeout_client instead.
        if let Some(sleep_val) = name.strip_prefix("SleepyTest_") {
            if let Ok(sleep_num) = sleep_val.parse::<u64>() {
                sleep(Duration::from_secs(sleep_num));
            }
        }

        // Fill in the template and return the response.
        let rsp = template
            .replace("{{arn}}", FAKE_ARN)
            .replace("{{name}}", name)
            .replace("{{version}}", version)
            .replace("{{secret}}", &secret_string)
            .replace("{{label}}", label);
        (code, rsp)
    }

    // Test client that stubs off network call and provides a canned response.
    fn def_fake_client() -> secretsmanager::Client {
        let fake_creds = secretsmanager::config::Credentials::new(
            "AKIDTESTKEY",
            "astestsecretkey",
            Some("atestsessiontoken".to_string()),
            None,
            "",
        );
        let http_client = infallible_client_fn(|_req| {
            let (code, rsp) = format_rsp(_req);
            Response::builder()
                .status(code)
                .body(SdkBody::from(rsp))
                .unwrap()
        });

        secretsmanager::Client::from_conf(
            secretsmanager::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(fake_creds)
                .interceptor(AgentModifierInterceptor)
                .region(secretsmanager::config::Region::new("us-west-2"))
                .http_client(http_client)
                .build(),
        )
    }

    // Test client that makes all Secrets Manager calls time out.
    pub fn timeout_client() -> secretsmanager::Client {
        let fake_creds = secretsmanager::config::Credentials::new(
            "AKIDTESTKEY",
            "astestsecretkey",
            Some("atestsessiontoken".to_string()),
            None,
            "",
        );

        secretsmanager::Client::from_conf(
            secretsmanager::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(fake_creds)
                .region(secretsmanager::config::Region::new("us-west-2"))
                .http_client(NeverClient::new())
                .build(),
        )
    }
}
