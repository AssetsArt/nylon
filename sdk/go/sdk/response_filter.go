package sdk

func (p *PhaseResponseFilter) SetResponseHeader(key, value string) {
	httpCtx := Response{
		ctx: p.ctx,
	}
	httpCtx.SetHeader(key, value)
}

func (p *PhaseResponseFilter) RemoveResponseHeader(key string) {
	httpCtx := Response{
		ctx: p.ctx,
	}
	httpCtx.RemoveHeader(key)
}

func (p *PhaseResponseFilter) SetResponseStatus(status uint16) {
	httpCtx := Response{
		ctx: p.ctx,
	}
	httpCtx.SetStatus(status)
}

func (p *PhaseResponseFilter) GetRequestHeader(key string) string {
	httpCtx := Request{
		ctx: p.ctx,
	}
	return httpCtx.Headers().Get(key)
}

func (p *PhaseResponseFilter) GetRequestHeaders() map[string]string {
	httpCtx := Request{
		ctx: p.ctx,
	}
	return httpCtx.Headers().GetAll()
}

func (p *PhaseResponseFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload()
}

func (p *PhaseResponseFilter) Next() {
	p.ctx.Next()
}
