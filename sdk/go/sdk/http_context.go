package sdk

import (
	"encoding/json"
	"net/url"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_http_context"
)

// Request
type Request struct {
	Method  string
	Path    string
	Query   url.Values
	Headers map[string]string
	Body    []byte
}

func WrapRequest(ctx *nylon_http_context.NylonHttpContext) *Request {
	raw := ctx.Request(nil)

	// Headers
	headers := map[string]string{}
	for i := range raw.HeadersLength() {
		var h nylon_http_context.Header
		if raw.Headers(&h, i) {
			headers[string(h.Key())] = string(h.Value())
		}
	}

	// Query
	q, _ := url.ParseQuery(string(raw.Query()))

	return &Request{
		Method:  string(raw.Method()),
		Path:    string(raw.Path()),
		Query:   q,
		Headers: headers,
		Body:    raw.BodyBytes(),
	}
}

func (r *Request) QueryGet(key string) string {
	return r.Query.Get(key)
}

func (r *Request) QueryRaw() string {
	return r.Query.Encode()
}

func (r *Request) Header(key string) string {
	return r.Headers[key]
}

func (r *Request) HeadersAll() map[string]string {
	return r.Headers
}

func (r *Request) BodyRaw() []byte {
	return r.Body
}

func (r *Request) BodyJSON(v any) error {
	return json.Unmarshal(r.Body, v)
}

// Headers modify
func (r *Request) SetHeader(key, value string) {
	r.Headers[key] = value
}

func (r *Request) RemoveHeader(key string) {
	delete(r.Headers, key)
}

// Response
type Response struct {
	Status  int
	Headers map[string]string
	Body    []byte
}

func WrapResponse(ctx *nylon_http_context.NylonHttpContext) *Response {
	raw := ctx.Response(nil)

	// Headers
	headers := map[string]string{}
	for i := range raw.HeadersLength() {
		var h nylon_http_context.Header
		if raw.Headers(&h, i) {
			headers[string(h.Key())] = string(h.Value())
		}
	}

	return &Response{
		Status:  int(raw.Status()),
		Headers: headers,
		Body:    raw.BodyBytes(),
	}
}

// Builder
func (r *Response) SetHeader(key, value string) {
	r.Headers[key] = value
}

func (r *Response) RemoveHeader(key string) {
	delete(r.Headers, key)
}

func (r *Response) SetStatus(status int) {
	r.Status = status
}

func (r *Response) BodyRaw(body []byte) {
	r.Body = body
}

func (r *Response) BodyJSON(v any) *Response {
	r.SetHeader("Content-Type", "application/json")
	b, _ := json.Marshal(v)
	r.BodyRaw(b)
	return r
}

func (r *Response) BodyText(s string) *Response {
	r.SetHeader("Content-Type", "text/plain; charset=utf-8")
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) BodyHTML(s string) *Response {
	r.SetHeader("Content-Type", "text/html; charset=utf-8")
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) Redirect(url string, code ...int) *Response {
	status := 302 // default
	if len(code) > 0 {
		status = code[0]
	}
	r.SetStatus(status)
	r.SetHeader("Location", url)
	r.BodyRaw([]byte{})
	return r
}
