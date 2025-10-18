package sdk

import (
	"encoding/binary"
	"encoding/json"
	"strconv"

	"github.com/AssetsArt/nylon/sdk/go/fbs/nylon_plugin"
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

func (r *Request) URL() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestURL]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestURL, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Path() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestPath]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestPath, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Query() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestQuery]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestQuery, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Params() map[string]string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestParams]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestParams, nil)
	}()

	ctx.cond.Wait()

	// Parse JSON response
	var params map[string]string
	json.Unmarshal(ctx.dataMap[methodID], &params)
	return params
}

func (r *Request) Host() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestHost]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestHost, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) ClientIP() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestClientIP]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestClientIP, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Method() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestMethod]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestMethod, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

func (r *Request) Bytes() int64 {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestBytes]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestBytes, nil)
	}()

	ctx.cond.Wait()
	bytesStr := string(ctx.dataMap[methodID])
	bytes := int64(0)
	if len(bytesStr) > 0 {
		bytes, _ = strconv.ParseInt(bytesStr, 10, 64)
	}
	return bytes
}

func (r *Request) Timestamp() int64 {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadRequestTimestamp]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadRequestTimestamp, nil)
	}()

	ctx.cond.Wait()
	timestampStr := string(ctx.dataMap[methodID])
	timestamp := int64(0)
	if len(timestampStr) > 0 {
		timestamp, _ = strconv.ParseInt(timestampStr, 10, 64)
	}
	return timestamp
}

func (r *Response) Status() int {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseStatus]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseStatus, nil)
	}()

	ctx.cond.Wait()
	statusStr := string(ctx.dataMap[methodID])
	status := 0
	if len(statusStr) > 0 {
		status, _ = strconv.Atoi(statusStr)
	}
	return status
}

func (r *Response) Bytes() int64 {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseBytes]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseBytes, nil)
	}()

	ctx.cond.Wait()
	bytesStr := string(ctx.dataMap[methodID])
	bytes := int64(0)
	if len(bytesStr) > 0 {
		bytes, _ = strconv.ParseInt(bytesStr, 10, 64)
	}
	return bytes
}

func (r *Response) Headers() map[string]string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseHeaders]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseHeaders, nil)
	}()

	ctx.cond.Wait()
	data := ctx.dataMap[methodID]

	headers := make(map[string]string)
	if len(data) == 0 {
		return headers
	}

	fb := nylon_plugin.GetRootAsNylonHttpHeaders(data, 0)
	for i := 0; i < fb.HeadersLength(); i++ {
		header := new(nylon_plugin.HeaderKeyValue)
		if fb.Headers(header, i) {
			key := string(header.Key())
			value := string(header.Value())
			headers[key] = value
		}
	}
	return headers
}

func (r *Response) Duration() int64 {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseDuration]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseDuration, nil)
	}()

	ctx.cond.Wait()
	durationStr := string(ctx.dataMap[methodID])
	duration := int64(0)
	if len(durationStr) > 0 {
		duration, _ = strconv.ParseInt(durationStr, 10, 64)
	}
	return duration
}

func (r *Response) Error() string {
	ctx := r.ctx
	methodID := MethodIDMapping[NylonMethodReadResponseError]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	go func() {
		RequestMethod(ctx.sessionID, 0, NylonMethodReadResponseError, nil)
	}()

	ctx.cond.Wait()
	return string(ctx.dataMap[methodID])
}

// WebSocket send helpers
func (ws *WebSocketConn) SendText(msg string) error {
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketSendText, []byte(msg))
}

func (ws *WebSocketConn) SendBinary(data []byte) error {
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketSendBinary, data)
}

func (ws *WebSocketConn) Close() error {
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketClose, nil)
}

// Room helpers
func (ws *WebSocketConn) JoinRoom(room string) error {
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketJoinRoom, []byte(room))
}

func (ws *WebSocketConn) LeaveRoom(room string) error {
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketLeaveRoom, []byte(room))
}

// Broadcast helpers (room + NUL + payload)
func (ws *WebSocketConn) BroadcastText(room string, message string) error {
	data := make([]byte, 0, len(room)+1+len(message))
	data = append(data, []byte(room)...)
	data = append(data, 0)
	data = append(data, []byte(message)...)
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketBroadcastRoomText, data)
}

func (ws *WebSocketConn) BroadcastBinary(room string, payload []byte) error {
	data := make([]byte, 0, len(room)+1+len(payload))
	data = append(data, []byte(room)...)
	data = append(data, 0)
	data = append(data, payload...)
	return RequestMethod(ws.ctx.sessionID, 0, NylonMethodWebSocketBroadcastRoomBinary, data)
}
