package sdk

func (p *PhaseLogging) Response() *Response {
	return &Response{
		ctx: p.ctx,
	}
}

func (p *PhaseLogging) Request() *Request {
	return &Request{
		ctx: p.ctx,
	}
}

func (p *PhaseLogging) GetPayload() map[string]any {
	return p.ctx.GetPayload()
}

func (p *PhaseLogging) Next() {
	p.ctx.Next()
}
