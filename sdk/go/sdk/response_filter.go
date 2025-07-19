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
	return p.ctx.GetPayload(2)
}

func (p *PhaseResponseFilter) Next() {
	p.ctx.Next(2)
}
