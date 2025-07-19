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

func (p *PhaseResponseFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload(2)
}

func (p *PhaseResponseFilter) Next() {
	p.ctx.Next(2)
}
