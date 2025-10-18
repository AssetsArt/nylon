package sdk

func (p *PhaseResponseBodyFilter) Response() *Response {
	return &Response{
		ctx: p.ctx,
	}
}

func (p *PhaseResponseBodyFilter) Request() *Request {
	return &Request{
		ctx: p.ctx,
	}
}

func (p *PhaseResponseBodyFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload()
}

func (p *PhaseResponseBodyFilter) Next() {
	p.ctx.Next()
}
