package sdk

import (
	"sync"
)

type HttpPluginFunc func(ctx *NylonHttpPluginCtx)

type NylonPlugin struct{}

type NylonHttpPluginCtx struct {
	sessionID int32

	mu      sync.Mutex
	cond    *sync.Cond
	dataMap map[uint32][]byte

	// WebSocket state
	wsCallbacks *WebSocketCallbacks
	wsUpgraded  bool
}

type Headers struct {
	headers map[string]string
}

type Response struct {
	ctx *NylonHttpPluginCtx
}

type Request struct {
	ctx *NylonHttpPluginCtx
}

type ResponseStream struct {
	response *Response
}

type PhaseRequestFilter struct {
	ctx *NylonHttpPluginCtx
}

type PhaseResponseFilter struct {
	ctx *NylonHttpPluginCtx
}

type PhaseResponseBodyFilter struct {
	ctx *NylonHttpPluginCtx
}

type PhaseLogging struct {
	ctx *NylonHttpPluginCtx
}

// WebSocket types

type WebSocketConn struct {
	ctx *NylonHttpPluginCtx
}

type WebSocketCallbacks struct {
	OnOpen          func(ws *WebSocketConn)
	OnMessageText   func(ws *WebSocketConn, msg string)
	OnMessageBinary func(ws *WebSocketConn, data []byte)
	OnClose         func(ws *WebSocketConn)
	OnError         func(ws *WebSocketConn, err string)
}
