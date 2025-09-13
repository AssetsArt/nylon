package sdk

func (ctx *PhaseRequestFilter) Request() *Request {
	return &Request{
		ctx: ctx.ctx,
	}
}

func (ctx *PhaseRequestFilter) Response() *Response {
	return &Response{
		ctx: ctx.ctx,
	}
}

func (p *PhaseRequestFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload()
}

func (p *PhaseRequestFilter) Next() {
	p.ctx.Next()
}

func (p *PhaseRequestFilter) End() {
	p.ctx.End()
}

// WebSocket helpers
func (p *PhaseRequestFilter) WebSocketUpgrade(cbs WebSocketCallbacks) error {
	// store callbacks in context for dispatch
	p.ctx.mu.Lock()
	p.ctx.wsCallbacks = &cbs
	p.ctx.mu.Unlock()
	// ask Rust to upgrade
	return RequestMethod(p.ctx.sessionID, 0, NylonMethodWebSocketUpgrade, nil)
}
