package sdk

type ResponseFilter struct {
	http_ctx *HttpContext
}

func (r *ResponseFilter) ToBytes() []byte {
	return r.http_ctx.ToBytes()
}

func (r *ResponseFilter) ReadRequest() Request {
	return r.http_ctx.Request
}

func (r *ResponseFilter) SetStatus(status int) {
	r.http_ctx.Response.SetStatus(status)
}

func (r *ResponseFilter) Headers() map[string]string {
	return r.http_ctx.Response.Headers
}

func (r *ResponseFilter) SetHeader(key, value string) {
	r.http_ctx.Response.SetHeader(key, value)
}

func (r *ResponseFilter) RemoveHeader(key string) {
	r.http_ctx.Response.RemoveHeader(key)
}

func (r *ResponseFilter) GetHeader(key string) string {
	return r.http_ctx.Response.Headers[key]
}
