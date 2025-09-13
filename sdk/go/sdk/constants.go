package sdk

type NylonMethods string

const (
	NylonMethodNext       NylonMethods = "next"
	NylonMethodEnd        NylonMethods = "end"
	NylonMethodGetPayload NylonMethods = "get_payload"
)

const (
	NylonMethodSetResponseHeader       NylonMethods = "set_response_header"
	NylonMethodRemoveResponseHeader    NylonMethods = "remove_response_header"
	NylonMethodSetResponseStatus       NylonMethods = "set_response_status"
	NylonMethodSetResponseFullBody     NylonMethods = "set_response_full_body"
	NylonMethodSetResponseStreamData   NylonMethods = "set_response_stream_data"
	NylonMethodSetResponseStreamEnd    NylonMethods = "set_response_stream_end"
	NylonMethodSetResponseStreamHeader NylonMethods = "set_response_stream_header"
	NylonMethodReadResponseFullBody    NylonMethods = "read_response_full_body"
)

const (
	NylonMethodReadRequestFullBody NylonMethods = "read_request_full_body"
	NylonMethodReadRequestHeader   NylonMethods = "read_request_header"
	NylonMethodReadRequestHeaders  NylonMethods = "read_request_headers"
)

// WebSocket methods
const (
	// Plugin -> Rust
	NylonMethodWebSocketUpgrade    NylonMethods = "websocket_upgrade"
	NylonMethodWebSocketSendText   NylonMethods = "websocket_send_text"
	NylonMethodWebSocketSendBinary NylonMethods = "websocket_send_binary"
	NylonMethodWebSocketClose      NylonMethods = "websocket_close"

	// Rust -> Plugin
	NylonMethodWebSocketOnOpen          NylonMethods = "websocket_on_open"
	NylonMethodWebSocketOnMessageText   NylonMethods = "websocket_on_message_text"
	NylonMethodWebSocketOnMessageBinary NylonMethods = "websocket_on_message_binary"
	NylonMethodWebSocketOnClose         NylonMethods = "websocket_on_close"
	NylonMethodWebSocketOnError         NylonMethods = "websocket_on_error"
)

var MethodIDMapping = map[NylonMethods]uint32{
	NylonMethodNext:       1,
	NylonMethodEnd:        2,
	NylonMethodGetPayload: 3,

	// Response methods
	NylonMethodSetResponseHeader:       100,
	NylonMethodRemoveResponseHeader:    101,
	NylonMethodSetResponseStatus:       102,
	NylonMethodSetResponseFullBody:     103,
	NylonMethodSetResponseStreamData:   104,
	NylonMethodSetResponseStreamEnd:    105,
	NylonMethodSetResponseStreamHeader: 106,
	NylonMethodReadResponseFullBody:    107,

	// Request methods
	NylonMethodReadRequestFullBody: 200,
	NylonMethodReadRequestHeader:   201,
	NylonMethodReadRequestHeaders:  202,

	// WebSocket methods
	NylonMethodWebSocketUpgrade:         300,
	NylonMethodWebSocketSendText:        301,
	NylonMethodWebSocketSendBinary:      302,
	NylonMethodWebSocketClose:           303,
	NylonMethodWebSocketOnOpen:          350,
	NylonMethodWebSocketOnMessageText:   351,
	NylonMethodWebSocketOnMessageBinary: 352,
	NylonMethodWebSocketOnClose:         353,
	NylonMethodWebSocketOnError:         354,
}

const (
	StatusOK                  = 200
	StatusFound               = 302
	StatusBadRequest          = 400
	StatusUnauthorized        = 401
	StatusForbidden           = 403
	StatusNotFound            = 404
	StatusTooManyRequests     = 429
	StatusInternalServerError = 500
)

const (
	ContentTypeJSON = "application/json"
	ContentTypeText = "text/plain; charset=utf-8"
	ContentTypeHTML = "text/html; charset=utf-8"
)

const (
	HeaderContentType      = "Content-Type"
	HeaderContentLength    = "Content-Length"
	HeaderLocation         = "Location"
	HeaderTransferEncoding = "Transfer-Encoding"
)
