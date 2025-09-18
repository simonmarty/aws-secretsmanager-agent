use aws_sdk_secretsmanager::error::ProvideErrorMetadata;
use aws_sdk_secretsmanager::error::SdkError;
use aws_secretsmanager_caching::error::SecretsManagerCachingClientError;
use aws_smithy_runtime_api::http::Response as HttpResponse;

#[derive(Debug)]
pub(crate) struct HttpError(pub u16, pub String);

impl From<url::ParseError> for HttpError {
    fn from(e: url::ParseError) -> Self {
        HttpError(400, e.to_string())
    }
}

impl<E: ProvideErrorMetadata + std::error::Error + 'static> From<SdkError<E, HttpResponse>>
    for HttpError
{
    fn from(e: SdkError<E, HttpResponse>) -> Self {
        match e {
            SdkError::DispatchFailure(derr) if derr.is_timeout() => {
                HttpError(504, err_response("TimeoutError", "Timeout"))
            }
            SdkError::TimeoutError(_) => HttpError(504, err_response("TimeoutError", "Timeout")),
            SdkError::DispatchFailure(derr) if derr.is_io() => {
                HttpError(502, err_response("ConnectionError", "Read Error"))
            }
            SdkError::ResponseError(_) => {
                HttpError(502, err_response("ConnectionError", "Response Error"))
            }
            SdkError::ServiceError(e) => {
                let meta = e.err().meta();
                let code = meta.code().unwrap_or("InternalError");

                let http_code = match code.contains("Internal") {
                    true => 500,
                    false => match code.contains("NotFound") {
                        true => 404,
                        false => 400,
                    },
                };

                HttpError(
                    http_code,
                    err_response(code, meta.message().unwrap_or_default()),
                )
            }
            _ => int_err(),
        }
    }
}

impl From<SecretsManagerCachingClientError> for HttpError {
    fn from(e: SecretsManagerCachingClientError) -> Self {
        match e {
            SecretsManagerCachingClientError::SecretStoreError(_) => int_err(),
            SecretsManagerCachingClientError::GetSecretValueError(e) => e.into(),
            SecretsManagerCachingClientError::DescribeSecretError(e) => e.into(),
        }
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(e: serde_json::Error) -> Self {
        log::error!("JSON serialization error: {e}");
        HttpError(500, err_response("InternalFailure", ""))
    }
}

fn int_err() -> HttpError {
    log::error!("Internal failure");
    HttpError(500, err_response("InternalFailure", ""))
}

/// Helper to format error response body in Coral JSON 1.1 format.
///
/// Callers need to pass in the error code (e.g.  InternalFailure,
/// InvalidParameterException, ect.) and the error message. This function will
/// then format a response body in JSON 1.1 format.
///
/// # Arguments
///
/// * `err_code` - The modeled exception name or InternalFailure for 500s.
/// * `msg` - The optional error message or "" for InternalFailure.
///
/// # Returns
///
/// * `String` - The JSON 1.1 response body.
///
/// # Example
///
/// ```
/// assert_eq!(err_response("InternalFailure", ""), "{\"__type\":\"InternalFailure\"}");
/// assert_eq!(
///     err_response("ResourceNotFoundException", "Secrets Manager can't find the specified secret."),
///     "{\"__type\":\"ResourceNotFoundException\",\"message\":\"Secrets Manager can't find the specified secret.\"}"
/// );
/// ```
#[doc(hidden)]
fn err_response(err_code: &str, msg: &str) -> String {
    if msg.is_empty() || err_code == "InternalFailure" {
        return String::from("{\"__type\":\"InternalFailure\"}");
    }
    format!("{{\"__type\":\"{err_code}\", \"message\":\"{msg}\"}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error() {
        let error = HttpError::from(url::ParseError::Overflow);
        assert_eq!(400, error.0);
        assert_eq!("URLs more than 4 GB are not supported", error.1);
    }
}
