use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NylonError {
    #[error("Failed to parse configuration: {0}")]
    ConfigError(String),

    #[error("Could not start the Pingora server: {0}")]
    PingoraError(String),

    #[error("Requested service is unavailable: {0}")]
    ServiceNotFound(String),

    #[error("No route matched the request: {0}")]
    RouteNotFound(String),

    #[error("Unable to generate ACME key pair: {0}")]
    AcmeKeyPairError(String),

    #[error("ACME HTTP request encountered an error: {0}")]
    AcmeHttpClientError(String),

    #[error("ACME JWS signing failed: {0}")]
    AcmeJWSError(String),

    #[error("ACME client encountered an error: {0}")]
    AcmeClientError(String),

    #[error("An unexpected internal server error occurred: {0}")]
    InternalServerError(String),

    #[error(
        "[BUG] This should never happen. Please report it at https://github.com/AssetsArt/nylon: {0}"
    )]
    ShouldNeverHappen(String),
}
