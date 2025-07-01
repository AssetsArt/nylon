package sdk

import (
	"encoding/binary"
	"encoding/json"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_plugin"
	flatbuffers "github.com/google/flatbuffers/go"
)

type ResponseStream struct {
	_r *Response
}
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
	RequestMethod(r._ctx.sessionID, NylonMethodRemoveResponseHeader, []byte(key))
}

func (r *Response) SetStatus(status uint16) {
	buf := make([]byte, 2)
	binary.BigEndian.PutUint16(buf, status)
	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseStatus, buf)
}

func (r *Response) BodyRaw(body []byte) {
	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseFullBody, body)
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

func (r *Response) Stream() (*ResponseStream, error) {
	r.SetHeader("Transfer-Encoding", "chunked")
	r.RemoveHeader("Content-Length")

	// send headers to the client
	err := RequestMethod(r._ctx.sessionID, NylonMethodSetResponseStreamHeader, nil)
	if err != nil {
		return nil, err
	}
	return &ResponseStream{
		_r: r,
	}, nil
}

// StreamHttpBody
func (s *ResponseStream) Write(p []byte) (n int, err error) {
	return len(p), RequestMethod(s._r._ctx.sessionID, NylonMethodSetResponseStreamData, p)
}

func (s *ResponseStream) End() error {
	return RequestMethod(s._r._ctx.sessionID, NylonMethodSetResponseStreamEnd, nil)
}
