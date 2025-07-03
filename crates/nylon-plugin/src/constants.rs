//! Constants used throughout the plugin system

// Plugin method constants
pub mod methods {
    // Control methods
    pub const NEXT: usize = 1;
    pub const END: usize = 2;
    pub const GET_PAYLOAD: usize = 3;

    // Response methods
    pub const SET_RESPONSE_HEADER: usize = 100;
    pub const REMOVE_RESPONSE_HEADER: usize = 101;
    pub const SET_RESPONSE_STATUS: usize = 102;
    pub const SET_RESPONSE_FULL_BODY: usize = 103;
    pub const SET_RESPONSE_STREAM_DATA: usize = 104;
    pub const SET_RESPONSE_STREAM_END: usize = 105;
    pub const SET_RESPONSE_STREAM_HEADER: usize = 106;
    pub const READ_RESPONSE_FULL_BODY: usize = 107;

    // Request methods
    pub const READ_REQUEST_FULL_BODY: usize = 200;
    pub const READ_REQUEST_HEADER: usize = 201;
    pub const READ_REQUEST_HEADERS: usize = 202;
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
