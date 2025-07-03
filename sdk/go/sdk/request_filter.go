package sdk

// Import types from types.go

// Response returns a Response object that can be used to manipulate the HTTP response.
// This provides access to methods for setting headers, status codes, and body content.
func (ctx *PhaseRequestFilter) Response() *Response {
	return &Response{
		ctx: ctx.ctx,
	}
}

// Request returns a Request object that can be used to read HTTP request data.
// This provides access to methods for reading headers and body content.
func (ctx *PhaseRequestFilter) Request() *Request {
	return &Request{
		ctx: ctx.ctx,
	}
}

// GetPayload retrieves the plugin payload data from the session context.
// The payload contains additional data that may have been passed to the plugin.
func (p *PhaseRequestFilter) GetPayload() map[string]any {
	return p.ctx.GetPayload()
}

// Next continues processing the request pipeline.
// This should be called when the plugin wants to pass control to the next handler.
func (p *PhaseRequestFilter) Next() {
	p.ctx.Next()
}

// End terminates the request processing.
// This should be called when the plugin wants to stop processing and return a response.
func (p *PhaseRequestFilter) End() {
	p.ctx.End()
}
