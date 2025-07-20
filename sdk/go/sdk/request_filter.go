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
