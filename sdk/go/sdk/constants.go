package sdk

// NylonMethods represents the available method types for communication with the Rust backend
type NylonMethods string

// Method constants for plugin lifecycle
const (
	NylonMethodNext       NylonMethods = "next"
	NylonMethodEnd        NylonMethods = "end"
	NylonMethodGetPayload NylonMethods = "get_payload"
)

// Method constants for response manipulation
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

// Method constants for request reading
const (
	NylonMethodReadRequestFullBody NylonMethods = "read_request_full_body"
	NylonMethodReadRequestHeader   NylonMethods = "read_request_header"
	NylonMethodReadRequestHeaders  NylonMethods = "read_request_headers"
)

// MethodIDMapping maps NylonMethods to their corresponding IDs used in FFI communication
var MethodIDMapping = map[NylonMethods]uint32{
	// Plugin lifecycle methods
	NylonMethodNext:       1,
	NylonMethodEnd:        2,
	NylonMethodGetPayload: 3,

	// Response manipulation methods
	NylonMethodSetResponseHeader:       100,
	NylonMethodRemoveResponseHeader:    101,
	NylonMethodSetResponseStatus:       102,
	NylonMethodSetResponseFullBody:     103,
	NylonMethodSetResponseStreamData:   104,
	NylonMethodSetResponseStreamEnd:    105,
	NylonMethodSetResponseStreamHeader: 106,
	NylonMethodReadResponseFullBody:    107,

	// Request reading methods
	NylonMethodReadRequestFullBody: 200,
	NylonMethodReadRequestHeader:   201,
	NylonMethodReadRequestHeaders:  202,
}

// HTTP status codes commonly used
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

// Content-Type constants
const (
	ContentTypeJSON = "application/json"
	ContentTypeText = "text/plain; charset=utf-8"
	ContentTypeHTML = "text/html; charset=utf-8"
)

// HTTP header constants
const (
	HeaderContentType      = "Content-Type"
	HeaderContentLength    = "Content-Length"
	HeaderLocation         = "Location"
	HeaderTransferEncoding = "Transfer-Encoding"
)
