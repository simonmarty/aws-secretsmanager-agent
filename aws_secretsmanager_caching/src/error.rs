use aws_sdk_secretsmanager::operation::{
    describe_secret::DescribeSecretError, get_secret_value::GetSecretValueError,
};
use aws_smithy_runtime_api::client::{orchestrator::HttpResponse, result::SdkError};

use crate::secret_store::SecretStoreError;

/// Helper function to determine transient errors. Transient errors include any timeout error,
/// unparseable response error, dispatch error due to timeout or IO, and 5xx server-side error.
///
/// # Arguments
/// * `e` - An SDK error
///
/// # Returns
/// * true if transient error, false if not
pub fn is_transient_error<S>(e: &SdkError<S, HttpResponse>) -> bool
where
    S: std::error::Error + 'static,
{
    match e {
        SdkError::TimeoutError(_) => true,
        SdkError::ResponseError(_) => true,
        SdkError::DispatchFailure(derr) if derr.is_timeout() || derr.is_io() => true,
        SdkError::ServiceError(serr) if serr.raw().status().is_server_error() => true,
        _ => false,
    }
}

/// Secrets Manager Caching Client Error
#[derive(thiserror::Error, Debug)]
pub enum SecretsManagerCachingClientError {
    /// Secret store error
    #[error("Secret store error {0:?}")]
    SecretStoreError(#[from] SecretStoreError),

    /// DescribeSecret error
    #[error("Describe Secret Error {0:?}")]
    DescribeSecretError(#[from] SdkError<DescribeSecretError, HttpResponse>),

    /// GetSecretValue error
    #[error("Get Secret Value Error {0:?}")]
    GetSecretValueError(#[from] SdkError<GetSecretValueError, HttpResponse>),
}
