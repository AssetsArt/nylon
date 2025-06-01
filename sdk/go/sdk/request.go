package sdk

import (
	"encoding/json"
	"net/url"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_http_context"
)

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
