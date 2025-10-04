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
	// Store callbacks in context for dispatch before requesting upgrade
	// This ensures callbacks are available when events arrive
	p.ctx.mu.Lock()
	p.ctx.wsCallbacks = &cbs
	p.ctx.wsUpgraded = false // Reset state before upgrade
	p.ctx.mu.Unlock()

	// Ask Rust to upgrade - this will trigger OnOpen event after handshake
	return RequestMethod(p.ctx.sessionID, 0, NylonMethodWebSocketUpgrade, nil)
}
