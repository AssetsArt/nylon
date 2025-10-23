use crate::protocol::ProtocolError;
use nylon_types::plugins::OverflowPolicy;
use std::{error::Error, fmt, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("failed to connect to NATS: {0}")]
    Connect(String),
    #[error("NATS request failed: {0}")]
    Request(String),
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("subscription failed: {0}")]
    Subscription(String),
    #[error("timed out after {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("request overflow (policy {policy:?})")]
    Overflow { policy: OverflowPolicy },
    #[error("retry attempts exhausted after {attempts} tries")]
    RetryExhausted {
        attempts: usize,
        #[source]
        last_error: Box<MessagingError>,
    },
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("invalid header: {0}")]
    Header(String),
    #[error("client is closed")]
    Closed,
}

impl MessagingError {
    pub fn from_error<E>(kind: ErrorKind, err: E) -> Self
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        let message = err.into();
        match kind {
            ErrorKind::Connect => Self::Connect(message.to_string()),
            ErrorKind::Request => Self::Request(message.to_string()),
            ErrorKind::Publish => Self::Publish(message.to_string()),
            ErrorKind::Subscription => Self::Subscription(message.to_string()),
            ErrorKind::Header => Self::Header(message.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Connect,
    Request,
    Publish,
    Subscription,
    Header,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
