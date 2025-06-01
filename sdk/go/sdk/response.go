package sdk

import "C"
import (
	"encoding/json"
	"net/http"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_dispatcher"
	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_http_context"
	flatbuffers "github.com/google/flatbuffers/go"
)

type HttpResponseBuilder struct {
	request Request
	status  int
	headers map[string]string
	body    []byte
	end     bool
}

func NewHttpResponse(http_ctx *SwitchHttpContextStruct) *HttpResponseBuilder {
	return &HttpResponseBuilder{
		request: http_ctx.Request,
		status:  200,
		headers: map[string]string{},
		end:     true,
	}
}

func (r *HttpResponseBuilder) estimateSize(add int) int {
	size := len(r.body) + add
	for k, v := range r.headers {
		size += len(k) + len(v) + 16
	}
	return size
}

func (r *HttpResponseBuilder) Status(code int) *HttpResponseBuilder {
	r.status = code
	return r
}

func (r *HttpResponseBuilder) Header(key, value string) *HttpResponseBuilder {
	r.headers[key] = value
	return r
}

func (r *HttpResponseBuilder) JSON(v any) *HttpResponseBuilder {
	r.Header("Content-Type", "application/json")
	b, _ := json.Marshal(v)
	r.body = b
	return r
}

func (r *HttpResponseBuilder) Text(s string) *HttpResponseBuilder {
	r.Header("Content-Type", "text/plain; charset=utf-8")
	r.body = []byte(s)
	return r
}

func (r *HttpResponseBuilder) HTML(s string) *HttpResponseBuilder {
	r.Header("Content-Type", "text/html; charset=utf-8")
	r.body = []byte(s)
	return r
}

func (r *HttpResponseBuilder) Redirect(url string, code ...int) *HttpResponseBuilder {
	status := 302 // default
	if len(code) > 0 {
		status = code[0]
	}
	r.status = status
	r.Header("Location", url)
	r.body = []byte{}
	return r
}

func (r *HttpResponseBuilder) Error(status int, message string) *HttpResponseBuilder {
	r.status = status
	r.Header("Content-Type", "application/json")
	body, _ := json.Marshal(map[string]string{
		"error":   http.StatusText(status),
		"message": message,
	})
	r.body = body
	return r
}

func (r *HttpResponseBuilder) Raw(b []byte, contentType string) *HttpResponseBuilder {
	if contentType != "" {
		r.Header("Content-Type", contentType)
	}
	r.body = b
	return r
}

func (r *HttpResponseBuilder) End(val bool) *HttpResponseBuilder {
	r.end = val
	return r
}

func (r *HttpResponseBuilder) Build(builder *flatbuffers.Builder) flatbuffers.UOffsetT {
	headerOffsets := make([]flatbuffers.UOffsetT, 0, len(r.headers))
	for k, v := range r.headers {
		kStr := builder.CreateString(k)
		vStr := builder.CreateString(v)
		nylon_http_context.HeaderStart(builder)
		nylon_http_context.HeaderAddKey(builder, kStr)
		nylon_http_context.HeaderAddValue(builder, vStr)
		headerOffsets = append(headerOffsets, nylon_http_context.HeaderEnd(builder))
	}

	nylon_http_context.NylonHttpResponseStartHeadersVector(builder, len(headerOffsets))
	for i := len(headerOffsets) - 1; i >= 0; i-- {
		builder.PrependUOffsetT(headerOffsets[i])
	}
	headersVec := builder.EndVector(len(headerOffsets))

	body := builder.CreateByteString(r.body)

	nylon_http_context.NylonHttpResponseStart(builder)
	nylon_http_context.NylonHttpResponseAddStatus(builder, int32(r.status))
	nylon_http_context.NylonHttpResponseAddHeaders(builder, headersVec)
	nylon_http_context.NylonHttpResponseAddBody(builder, body)
	return nylon_http_context.NylonHttpResponseEnd(builder)
}

func (r *HttpResponseBuilder) Send(dispatcher *nylon_dispatcher.NylonDispatcher) []byte {
	requestID := string(dispatcher.RequestId())
	pluginName := string(dispatcher.Name())
	innerBuilder := flatbuffers.NewBuilder(r.estimateSize(256))

	// Build request
	reqMethodOffset := innerBuilder.CreateString(r.request.Method)
	reqPathOffset := innerBuilder.CreateString(r.request.Path)
	reqQueryOffset := innerBuilder.CreateString(r.request.Query.Encode())

	// Build response
	res := r.Build(innerBuilder)

	// Build request
	nylon_http_context.NylonHttpRequestStart(innerBuilder)
	nylon_http_context.NylonHttpRequestAddMethod(innerBuilder, reqMethodOffset)
	nylon_http_context.NylonHttpRequestAddPath(innerBuilder, reqPathOffset)
	nylon_http_context.NylonHttpRequestAddQuery(innerBuilder, reqQueryOffset)
	request := nylon_http_context.NylonHttpRequestEnd(innerBuilder)

	// Build HttpContext
	nylon_http_context.NylonHttpContextStart(innerBuilder)
	nylon_http_context.NylonHttpContextAddRequest(innerBuilder, request)
	nylon_http_context.NylonHttpContextAddResponse(innerBuilder, res)
	nylon_http_context.NylonHttpContextAddEnd(innerBuilder, r.end)
	ctx := nylon_http_context.NylonHttpContextEnd(innerBuilder)
	innerBuilder.Finish(ctx)
	httpCtxBytes := innerBuilder.FinishedBytes()

	// Build dispatcher
	outerBuilder := flatbuffers.NewBuilder(len(httpCtxBytes) + len(requestID) + len(pluginName) + 256)
	requestIDOffset := outerBuilder.CreateString(requestID)
	pluginNameOffset := outerBuilder.CreateString(pluginName)
	dataOffset := outerBuilder.CreateByteVector(httpCtxBytes)

	nylon_dispatcher.NylonDispatcherStart(outerBuilder)
	nylon_dispatcher.NylonDispatcherAddRequestId(outerBuilder, requestIDOffset)
	nylon_dispatcher.NylonDispatcherAddName(outerBuilder, pluginNameOffset)
	nylon_dispatcher.NylonDispatcherAddData(outerBuilder, dataOffset)
	dispatcherData := nylon_dispatcher.NylonDispatcherEnd(outerBuilder)
	outerBuilder.Finish(dispatcherData)

	return outerBuilder.FinishedBytes()
}
