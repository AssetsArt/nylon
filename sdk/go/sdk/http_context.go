package sdk

import (
	"encoding/binary"
	"encoding/json"
	"strconv"

	"github.com/AssetsArt/nylon/sdk/go/fbs/nylon_plugin"
	flatbuffers "github.com/google/flatbuffers/go"
)

func (ctx *NylonHttpPluginCtx) requestAndWait(method NylonMethods, payload []byte) []byte {
	methodID := MethodIDMapping[method]

	ctx.mu.Lock()
	delete(ctx.dataMap, methodID)
	ctx.mu.Unlock()

	if err := RequestMethod(ctx.sessionID, 0, method, payload); err != nil {
		ctx.mu.Lock()
		ctx.dataMap[methodID] = nil
		ctx.cond.Broadcast()
		ctx.mu.Unlock()
	}

	ctx.mu.Lock()
	defer ctx.mu.Unlock()
	for {
		if data, ok := ctx.dataMap[methodID]; ok {
			delete(ctx.dataMap, methodID)
			return data
		}
		ctx.cond.Wait()
	}
}

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
	return r.ctx.requestAndWait(NylonMethodReadResponseFullBody, nil)
}

func (r *Request) RawBody() []byte {
	return r.ctx.requestAndWait(NylonMethodReadRequestFullBody, nil)
}

func (r *Request) Header(key string) string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestHeader, []byte(key)))
}

func (r *Request) Headers() *Headers {
	data := r.ctx.requestAndWait(NylonMethodReadRequestHeaders, nil)
	headersMap := make(map[string]string)

	if len(data) == 0 {
		return &Headers{headers: headersMap}
	}

	fb := nylon_plugin.GetRootAsNylonHttpHeaders(data, 0)
	for i := 0; i < fb.HeadersLength(); i++ {
		header := &nylon_plugin.HeaderKeyValue{}
		if fb.Headers(header, i) {
			headersMap[string(header.Key())] = string(header.Value())
		}
	}

	return &Headers{headers: headersMap}
}

func (r *Request) URL() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestURL, nil))
}

func (r *Request) Path() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestPath, nil))
}

func (r *Request) Query() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestQuery, nil))
}

func (r *Request) Params() map[string]string {
	data := r.ctx.requestAndWait(NylonMethodReadRequestParams, nil)
	var params map[string]string
	if len(data) > 0 {
		json.Unmarshal(data, &params)
	}
	return params
}

func (r *Request) Host() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestHost, nil))
}

func (r *Request) ClientIP() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestClientIP, nil))
}

func (r *Request) Method() string {
	return string(r.ctx.requestAndWait(NylonMethodReadRequestMethod, nil))
}

func (r *Request) Bytes() int64 {
	bytesStr := string(r.ctx.requestAndWait(NylonMethodReadRequestBytes, nil))
	bytes := int64(0)
	if len(bytesStr) > 0 {
		bytes, _ = strconv.ParseInt(bytesStr, 10, 64)
	}
	return bytes
}

func (r *Request) Timestamp() int64 {
	timestampStr := string(r.ctx.requestAndWait(NylonMethodReadRequestTimestamp, nil))
	timestamp := int64(0)
	if len(timestampStr) > 0 {
		timestamp, _ = strconv.ParseInt(timestampStr, 10, 64)
	}
	return timestamp
}

func (r *Response) Status() int {
	statusStr := string(r.ctx.requestAndWait(NylonMethodReadResponseStatus, nil))
	status := 0
	if len(statusStr) > 0 {
		status, _ = strconv.Atoi(statusStr)
	}
	return status
}

func (r *Response) Bytes() int64 {
	bytesStr := string(r.ctx.requestAndWait(NylonMethodReadResponseBytes, nil))
	bytes := int64(0)
	if len(bytesStr) > 0 {
		bytes, _ = strconv.ParseInt(bytesStr, 10, 64)
	}
	return bytes
}

func (r *Response) Headers() map[string]string {
	data := r.ctx.requestAndWait(NylonMethodReadResponseHeaders, nil)
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
	durationStr := string(r.ctx.requestAndWait(NylonMethodReadResponseDuration, nil))
	duration := int64(0)
	if len(durationStr) > 0 {
		duration, _ = strconv.ParseInt(durationStr, 10, 64)
	}
	return duration
}

func (r *Response) Error() string {
	return string(r.ctx.requestAndWait(NylonMethodReadResponseError, nil))
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
