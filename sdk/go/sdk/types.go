package sdk

import (
	"sync"
)

// HttpPluginFunc represents a user-defined request handler function
type HttpPluginFunc func(ctx *NylonHttpPluginCtx)

// NylonPlugin represents the main plugin instance
type NylonPlugin struct{}

// NylonHttpPluginCtx represents a per-session context for HTTP plugin operations
type NylonHttpPluginCtx struct {
	sessionID int

	mu      sync.Mutex
	cond    *sync.Cond
	dataMap map[uint32][]byte
}

// Headers represents HTTP headers with a map-based implementation
type Headers struct {
	headers map[string]string
}

// Response represents an HTTP response that can be manipulated by plugins
type Response struct {
	ctx *NylonHttpPluginCtx
}

// Request represents an HTTP request that can be read by plugins
type Request struct {
	ctx *NylonHttpPluginCtx
}

// ResponseStream represents a streaming response for chunked transfer encoding
type ResponseStream struct {
	response *Response
}

// PhaseRequestFilter represents a request filter phase that provides access to both request and response
type PhaseRequestFilter struct {
	ctx *NylonHttpPluginCtx
}
