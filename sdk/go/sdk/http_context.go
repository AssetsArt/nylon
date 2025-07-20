package sdk

import (
	"encoding/binary"
	"encoding/json"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_plugin"
	flatbuffers "github.com/google/flatbuffers/go"
)

func (h *Headers) Get(key string) string {
	return h.headers[key]
}

func (h *Headers) GetAll() map[string]string {
	return h.headers
}

func (r *Response) SetHeader(key, value string) {
	builder := flatbuffers.NewBuilder(0)
	headerKey := builder.CreateString(key)
	headerValue := builder.CreateString(value)
	nylon_plugin.HeaderKeyValueStart(builder)
	nylon_plugin.HeaderKeyValueAddKey(builder, headerKey)
	nylon_plugin.HeaderKeyValueAddValue(builder, headerValue)
	builder.Finish(nylon_plugin.HeaderKeyValueEnd(builder))

	RequestMethod(r.ctx.sessionID, 0, NylonMethodSetResponseHeader, builder.FinishedBytes())
}

func (r *Response) RemoveHeader(key string) {
	RequestMethod(r.ctx.sessionID, 0, NylonMethodRemoveResponseHeader, []byte(key))
}

func (r *Response) SetStatus(status uint16) {
	buf := make([]byte, 2)
	binary.BigEndian.PutUint16(buf, status)
	RequestMethod(r.ctx.sessionID, 0, NylonMethodSetResponseStatus, buf)
}

func (r *Response) BodyRaw(body []byte) {
	RequestMethod(r.ctx.sessionID, 0, NylonMethodSetResponseFullBody, body)
}

func (r *Response) BodyJSON(v any) *Response {
	r.SetHeader(HeaderContentType, ContentTypeJSON)
	b, _ := json.Marshal(v)
	r.BodyRaw(b)
	return r
}

func (r *Response) BodyText(s string) *Response {
	r.SetHeader(HeaderContentType, ContentTypeText)
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) BodyHTML(s string) *Response {
	r.SetHeader(HeaderContentType, ContentTypeHTML)
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) Redirect(url string, code ...uint16) *Response {
	status := uint16(StatusFound) // default
	if len(code) > 0 {
		status = code[0]
	}
	r.SetStatus(status)
	r.SetHeader(HeaderLocation, url)
	r.BodyRaw([]byte{})
	return r
}

func (r *Response) Stream() (*ResponseStream, error) {
	r.SetHeader(HeaderTransferEncoding, "chunked")
	r.RemoveHeader(HeaderContentLength)

	// Send headers to the client
	err := RequestMethod(r.ctx.sessionID, 0, NylonMethodSetResponseStreamHeader, nil)
	if err != nil {
		return nil, err
	}
	return &ResponseStream{
		response: r,
	}, nil
}

func (s *ResponseStream) Write(p []byte) (n int, err error) {
	return len(p), RequestMethod(s.response.ctx.sessionID, 0, NylonMethodSetResponseStreamData, p)
}

func (s *ResponseStream) End() error {
	return RequestMethod(s.response.ctx.sessionID, 0, NylonMethodSetResponseStreamEnd, nil)
}

func (r *Response) ReadBody() []byte {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseFullBody]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Ask Rust to read body
	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseFullBody, nil)
	}()

	// Wait for response
	ctx.cond.Wait()
	return ctx.dataMap[methodID]
}

func (r *Request) RawBody() []byte {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestFullBody]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Ask Rust to read body
	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestFullBody, nil)
	}()

	// Wait for response
	ctx.cond.Wait()
	return ctx.dataMap[methodID]
}

func (r *Request) Header(key string) string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestHeader]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestHeader, []byte(key))
	}()

	// Wait for response
	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Headers() *Headers {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestHeaders]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestHeaders, nil)
	}()

	// Wait for response
	ctx.cond.Wait()

	// parse flatbuffers
	headersBytes := ctx.dataMap[methodID]
	headers := nylon_plugin.GetRootAsNylonHttpHeaders(headersBytes, 0)

	headersMap := make(map[string]string)
	for i := 0; i < headers.HeadersLength(); i++ {
		header := &nylon_plugin.HeaderKeyValue{}
		headers.Headers(header, i)
		headersMap[string(header.Key())] = string(header.Value())
	}

	return &Headers{
		headers: headersMap,
	}
}
