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

    // Request methods
    pub const READ_REQUEST_FULL_BODY: u32 = 200;
    pub const READ_REQUEST_HEADER: u32 = 201;
    pub const READ_REQUEST_HEADERS: u32 = 202;
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
