package sdk

import (
	"encoding/json"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_plugin"
	flatbuffers "github.com/google/flatbuffers/go"
)

type HttpContext struct {
	// Request  Request
	Response Response
}

// Response
type Response struct {
	_ctx *NylonHttpPluginCtx
}

// Builder
func (r *Response) SetHeader(key, value string) {
	builder := flatbuffers.NewBuilder(0)
	headerKey := builder.CreateString(key)
	headerValue := builder.CreateString(value)
	nylon_plugin.HeaderKeyValueStart(builder)
	nylon_plugin.HeaderKeyValueAddKey(builder, headerKey)
	nylon_plugin.HeaderKeyValueAddValue(builder, headerValue)
	builder.Finish(nylon_plugin.HeaderKeyValueEnd(builder))

	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseHeader, builder.FinishedBytes())
}

func (r *Response) RemoveHeader(key string) {
	builder := flatbuffers.NewBuilder(0)
	headerKey := builder.CreateString(key)
	nylon_plugin.RemoveResponseHeaderStart(builder)
	nylon_plugin.RemoveResponseHeaderAddKey(builder, headerKey)
	builder.Finish(nylon_plugin.RemoveResponseHeaderEnd(builder))
	RequestMethod(r._ctx.sessionID, NylonMethodRemoveResponseHeader, builder.FinishedBytes())
}

func (r *Response) SetStatus(status uint16) {
	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseStatus, []byte{byte(status >> 8), byte(status)})
}

func (r *Response) BodyRaw(body []byte) {
	panic("not implemented")
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

func (r *Response) Redirect(url string, code ...uint16) *Response {
	status := uint16(302) // default
	if len(code) > 0 {
		status = code[0]
	}
	r.SetStatus(status)
	r.SetHeader("Location", url)
	r.BodyRaw([]byte{})
	return r
}
