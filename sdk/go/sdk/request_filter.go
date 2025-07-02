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

func (p *PhaseRequestFilter) GetPayload() map[string]any {
	return p._ctx.GetPayload()
}

func (p *PhaseRequestFilter) Next() {
	p._ctx.Next()
}

func (p *PhaseRequestFilter) End() {
	p._ctx.End()
}
