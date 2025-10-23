pub mod error;
mod nats_client;
pub mod protocol;
pub mod transport;

pub use error::MessagingError;
pub use nats_client::{NatsClient, NatsClientOptions, QueueSubscription, RetryPolicy};
pub use nylon_types::plugins::OverflowPolicy;
pub use protocol::{
    MessageHeaders, PROTOCOL_VERSION, PluginRequest, PluginResponse, ProtocolError, ResponseAction,
    decode_request, decode_response, encode_request, encode_response, new_request_id,
};
pub use transport::MessagingTransport;
