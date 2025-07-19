package sdk

func (ctx *PhaseResponseFilter) Request() *Request {
	return &Request{
		ctx: ctx.ctx,
	}
}

func (ctx *PhaseResponseFilter) Response() *Response {
	return &Response{
		ctx: ctx.ctx,
	}
}

func (p *PhaseResponseFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload(1)
}

func (p *PhaseResponseFilter) Next() {
	p.ctx.Next(1)
}

func (p *PhaseResponseFilter) End() {
	p.ctx.End(1)
}
