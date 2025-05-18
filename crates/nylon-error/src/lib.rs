use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NylonError {
    #[error("Failed to parse config: {0}")]
    ConfigError(String),

    #[error("Failed to start pingora: {0}")]
    PingoraError(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Failed to start proxy: {0}")]
    ProxyError(String),

    #[error("Failed to generate ACME key pair: {0}")]
    AcmeKeyPairError(String),

    #[error("Failed to send ACME HTTP request: {0}")]
    AcmeHttpClientError(String),

    #[error("Failed to sign ACME JWS: {0}")]
    AcmeJWSError(String),

    #[error("Failed to send ACME client request: {0}")]
    AcmeClientError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}
