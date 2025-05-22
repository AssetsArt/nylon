use serde_json::{Value, json};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NylonError {
    #[error("Failed to parse configuration: {0}")]
    ConfigError(String),

    #[error("Could not start the Pingora server: {0}")]
    PingoraError(String),

    #[error("{{ \"status\": \"{0}\", \"error\": \"{1}\", \"message\": \"{2}\" }}")]
    HttpException(u16, &'static str, &'static str),

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

impl NylonError {
    pub fn http_status(&self) -> u16 {
        match self {
            NylonError::HttpException(status, _, _) => *status,
            _ => 500,
        }
    }

    pub fn error_code(&self) -> String {
        match self {
            NylonError::HttpException(_, error, _) => error.to_string(),
            NylonError::ConfigError(_) => "CONFIG_ERROR".to_string(),
            NylonError::PingoraError(_) => "PINGORA_ERROR".to_string(),
            NylonError::ServiceNotFound(_) => "SERVICE_NOT_FOUND".to_string(),
            NylonError::RouteNotFound(_) => "ROUTE_NOT_FOUND".to_string(),
            NylonError::AcmeKeyPairError(_) => "ACME_KEY_PAIR_ERROR".to_string(),
            NylonError::AcmeHttpClientError(_) => "ACME_HTTP_CLIENT_ERROR".to_string(),
            NylonError::AcmeJWSError(_) => "ACME_JWS_ERROR".to_string(),
            NylonError::AcmeClientError(_) => "ACME_CLIENT_ERROR".to_string(),
            NylonError::InternalServerError(_) => "INTERNAL_SERVER_ERROR".to_string(),
            NylonError::ShouldNeverHappen(_) => "SHOULD_NEVER_HAPPEN".to_string(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            NylonError::HttpException(_, _, message) => message.to_string(),
            NylonError::ConfigError(message) => message.to_string(),
            NylonError::PingoraError(message) => message.to_string(),
            NylonError::ServiceNotFound(message) => message.to_string(),
            NylonError::RouteNotFound(message) => message.to_string(),
            NylonError::AcmeKeyPairError(message) => message.to_string(),
            NylonError::AcmeHttpClientError(message) => message.to_string(),
            NylonError::AcmeJWSError(message) => message.to_string(),
            NylonError::AcmeClientError(message) => message.to_string(),
            NylonError::InternalServerError(message) => message.to_string(),
            NylonError::ShouldNeverHappen(message) => format!(
                "[BUG] This should never happen. Please report it at https://github.com/AssetsArt/nylon: {}",
                message
            ),
        }
    }

    pub fn exception_json(&self) -> Value {
        json!({
            "status": self.http_status(),
            "error": self.error_code(),
            "message": self.message(),
        })
    }
}
