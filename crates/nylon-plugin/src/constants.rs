//! Constants used throughout the plugin system

// Plugin method constants
pub mod methods {
    // Control methods
    pub const NEXT: u32 = 1;
    pub const END: u32 = 2;
    pub const GET_PAYLOAD: u32 = 3;

    // Response methods
    pub const SET_RESPONSE_HEADER: u32 = 100;
    pub const REMOVE_RESPONSE_HEADER: u32 = 101;
    pub const SET_RESPONSE_STATUS: u32 = 102;
    pub const SET_RESPONSE_FULL_BODY: u32 = 103;
    pub const SET_RESPONSE_STREAM_DATA: u32 = 104;
    pub const SET_RESPONSE_STREAM_END: u32 = 105;
    pub const SET_RESPONSE_STREAM_HEADER: u32 = 106;
    pub const READ_RESPONSE_FULL_BODY: u32 = 107;
    pub const READ_RESPONSE_STATUS: u32 = 108;
    pub const READ_RESPONSE_BYTES: u32 = 109;

    // Request methods
    pub const READ_REQUEST_FULL_BODY: u32 = 200;
    pub const READ_REQUEST_HEADER: u32 = 201;
    pub const READ_REQUEST_HEADERS: u32 = 202;
    pub const READ_REQUEST_URL: u32 = 203;
    pub const READ_REQUEST_PATH: u32 = 204;
    pub const READ_REQUEST_QUERY: u32 = 205;
    pub const READ_REQUEST_PARAMS: u32 = 206;
    pub const READ_REQUEST_HOST: u32 = 207;
    pub const READ_REQUEST_CLIENT_IP: u32 = 208;
    pub const READ_REQUEST_METHOD: u32 = 209;
    pub const READ_REQUEST_BYTES: u32 = 210;

    // WebSocket methods (Plugin -> Rust)
    pub const WEBSOCKET_UPGRADE: u32 = 300;
    pub const WEBSOCKET_SEND_TEXT: u32 = 301;
    pub const WEBSOCKET_SEND_BINARY: u32 = 302;
    pub const WEBSOCKET_CLOSE: u32 = 303;

    // WebSocket room methods (Plugin -> Rust)
    pub const WEBSOCKET_JOIN_ROOM: u32 = 310;
    pub const WEBSOCKET_LEAVE_ROOM: u32 = 311;
    pub const WEBSOCKET_BROADCAST_ROOM_TEXT: u32 = 312;
    pub const WEBSOCKET_BROADCAST_ROOM_BINARY: u32 = 313;

    // WebSocket events (Rust -> Plugin)
    pub const WEBSOCKET_ON_OPEN: u32 = 350;
    pub const WEBSOCKET_ON_MESSAGE_TEXT: u32 = 351;
    pub const WEBSOCKET_ON_MESSAGE_BINARY: u32 = 352;
    pub const WEBSOCKET_ON_CLOSE: u32 = 353;
    pub const WEBSOCKET_ON_ERROR: u32 = 354;
}

// FFI symbol names
pub mod ffi_symbols {
    pub const INITIALIZE: &str = "initialize";
    pub const PLUGIN_FREE: &str = "plugin_free";
    pub const REGISTER_SESSION: &str = "register_session_stream";
    pub const EVENT_STREAM: &str = "event_stream";
    pub const CLOSE_SESSION: &str = "close_session_stream";
    pub const SHUTDOWN: &str = "shutdown";
}

// Builtin plugin names
pub mod builtin_plugins {
    pub const REQUEST_HEADER_MODIFIER: &str = "RequestHeaderModifier";
    pub const RESPONSE_HEADER_MODIFIER: &str = "ResponseHeaderModifier";
}
