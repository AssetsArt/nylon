package sdk

import (
	"encoding/binary"
	"encoding/json"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_plugin"
	flatbuffers "github.com/google/flatbuffers/go"
)

// Import types from types.go

// Get retrieves the value of a specific header by key.
// Returns an empty string if the header is not found.
func (h *Headers) Get(key string) string {
	return h.headers[key]
}

// GetAll returns all headers as a map[string]string.
// This provides direct access to the underlying headers map.
func (h *Headers) GetAll() map[string]string {
	return h.headers
}

// SetHeader sets a response header with the specified key and value.
// This method uses FlatBuffers to serialize the header data before sending it to Rust.
func (r *Response) SetHeader(key, value string) {
	builder := flatbuffers.NewBuilder(0)
	headerKey := builder.CreateString(key)
	headerValue := builder.CreateString(value)
	nylon_plugin.HeaderKeyValueStart(builder)
	nylon_plugin.HeaderKeyValueAddKey(builder, headerKey)
	nylon_plugin.HeaderKeyValueAddValue(builder, headerValue)
	builder.Finish(nylon_plugin.HeaderKeyValueEnd(builder))

	RequestMethod(r.ctx.sessionID, NylonMethodSetResponseHeader, builder.FinishedBytes())
}

// RemoveHeader removes a response header with the specified key.
func (r *Response) RemoveHeader(key string) {
	RequestMethod(r.ctx.sessionID, NylonMethodRemoveResponseHeader, []byte(key))
}

// SetStatus sets the HTTP status code for the response.
// The status code is serialized as a big-endian uint16 before being sent to Rust.
func (r *Response) SetStatus(status uint16) {
	buf := make([]byte, 2)
	binary.BigEndian.PutUint16(buf, status)
	RequestMethod(r.ctx.sessionID, NylonMethodSetResponseStatus, buf)
}

// BodyRaw sets the response body with raw bytes.
// This method sends the body data directly to Rust without any content-type headers.
func (r *Response) BodyRaw(body []byte) {
	RequestMethod(r.ctx.sessionID, NylonMethodSetResponseFullBody, body)
}

// BodyJSON sets the response body with JSON data.
// It automatically sets the Content-Type header to application/json and
// marshals the provided value to JSON format.
// Returns the response for method chaining.
func (r *Response) BodyJSON(v any) *Response {
	r.SetHeader(HeaderContentType, ContentTypeJSON)
	b, _ := json.Marshal(v)
	r.BodyRaw(b)
	return r
}

// BodyText sets the response body with plain text data.
// It automatically sets the Content-Type header to text/plain; charset=utf-8.
// Returns the response for method chaining.
func (r *Response) BodyText(s string) *Response {
	r.SetHeader(HeaderContentType, ContentTypeText)
	r.BodyRaw([]byte(s))
	return r
}

// BodyHTML sets the response body with HTML data.
// It automatically sets the Content-Type header to text/html; charset=utf-8.
// Returns the response for method chaining.
func (r *Response) BodyHTML(s string) *Response {
	r.SetHeader(HeaderContentType, ContentTypeHTML)
	r.BodyRaw([]byte(s))
	return r
}

// Redirect creates a redirect response to the specified URL.
// By default, it uses status code 302 (Found), but you can specify a different
// status code as an optional parameter.
// Returns the response for method chaining.
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

// Stream creates a streaming response with chunked transfer encoding.
// It sets the Transfer-Encoding header to "chunked" and removes the Content-Length header.
// Returns a ResponseStream that can be used to write data incrementally.
func (r *Response) Stream() (*ResponseStream, error) {
	r.SetHeader(HeaderTransferEncoding, "chunked")
	r.RemoveHeader(HeaderContentLength)

	// Send headers to the client
	err := RequestMethod(r.ctx.sessionID, NylonMethodSetResponseStreamHeader, nil)
	if err != nil {
		return nil, err
	}
	return &ResponseStream{
		response: r,
	}, nil
}

// Write implements the io.Writer interface for streaming response data.
// It sends the provided bytes to the client as a chunk of the response.
// Returns the number of bytes written and any error that occurred.
func (s *ResponseStream) Write(p []byte) (n int, err error) {
	return len(p), RequestMethod(s.response.ctx.sessionID, NylonMethodSetResponseStreamData, p)
}

// End signals the end of the streaming response.
// This method should be called when all data has been written to the stream.
func (s *ResponseStream) End() error {
	return RequestMethod(s.response.ctx.sessionID, NylonMethodSetResponseStreamEnd, nil)
}

// ReadBody reads the full response body from the upstream service.
// This method blocks until the body is received from Rust.
// Returns the response body as a byte slice.
func (r *Response) ReadBody() []byte {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseFullBody]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Ask Rust to read body
	RequestMethod(ctx.sessionID, NylonMethodReadResponseFullBody, nil)

	// Wait for response
	ctx.cond.Wait()
	return ctx.dataMap[methodID]
}

// RawBody reads the full request body.
// This method blocks until the body is received from Rust.
// Returns the request body as a byte slice.
func (r *Request) RawBody() []byte {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestFullBody]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Ask Rust to read body
	RequestMethod(ctx.sessionID, NylonMethodReadRequestFullBody, nil)

	// Wait for response
	ctx.cond.Wait()
	return ctx.dataMap[methodID]
}

// Header retrieves the value of a specific request header by key.
// This method blocks until the header value is received from Rust.
// Returns an empty string if the header is not found.
func (r *Request) Header(key string) string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestHeader]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	RequestMethod(ctx.sessionID, NylonMethodReadRequestHeader, []byte(key))

	// Wait for response
	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

// Headers retrieves all request headers.
// This method blocks until all headers are received from Rust.
// The headers are parsed from FlatBuffers format and returned as a Headers object.
func (r *Request) Headers() *Headers {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestHeaders]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	RequestMethod(ctx.sessionID, NylonMethodReadRequestHeaders, nil)

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
