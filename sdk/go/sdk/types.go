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
