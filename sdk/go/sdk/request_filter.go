package sdk

type PhaseRequestFilter struct {
	_ctx *NylonHttpPluginCtx
}

// Methods
func (ctx *PhaseRequestFilter) Response() *Response {
	return &Response{
		_ctx: ctx._ctx,
	}
}
