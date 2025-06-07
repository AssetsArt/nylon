package sdk

import "C"
import (
	"net/url"

	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_dispatcher"
	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_http_context"
	flatbuffers "github.com/google/flatbuffers/go"
)

type Dispatcher struct {
	HttpEnd    bool
	RequestId  string
	PluginName string
	Entry      string
	Data       []byte
}

// new dispatcher
func NewDispatcher() *Dispatcher {
	return &Dispatcher{
		HttpEnd:    false,
		RequestId:  "",
		PluginName: "",
		Entry:      "",
		Data:       nil,
	}
}

// new http context
func NewHttpContext() *HttpContext {
	return &HttpContext{
		Request: Request{
			Method:  "",
			Path:    "",
			Query:   url.Values{},
			Headers: make(map[string]string),
			Body:    nil,
			Params:  make(map[string]string),
		},
		Response: Response{
			Status:  200,
			Headers: make(map[string]string),
			Body:    nil,
		},
	}
}

func WrapDispatcher(input []byte) *Dispatcher {
	raw := nylon_dispatcher.GetRootAsNylonDispatcher(input, 0)
	return &Dispatcher{
		HttpEnd:    raw.HttpEnd(),
		RequestId:  string(raw.RequestId()),
		PluginName: string(raw.Name()),
		Entry:      string(raw.Entry()),
		Data:       raw.DataBytes(),
	}
}

func (d *Dispatcher) ToBytes() []byte {
	bufSize := len(d.Data) + len(d.RequestId) + len(d.PluginName) + len(d.Entry) + 256
	builder := flatbuffers.NewBuilder(bufSize)

	// Build args
	requestIDOffset := builder.CreateString(d.RequestId)
	pluginNameOffset := builder.CreateString(d.PluginName)
	entryOffset := builder.CreateString(d.Entry)
	dataOffset := builder.CreateByteVector(d.Data)

	// Build dispatcher
	nylon_dispatcher.NylonDispatcherStart(builder)
	nylon_dispatcher.NylonDispatcherAddHttpEnd(builder, d.HttpEnd)
	nylon_dispatcher.NylonDispatcherAddRequestId(builder, requestIDOffset)
	nylon_dispatcher.NylonDispatcherAddName(builder, pluginNameOffset)
	nylon_dispatcher.NylonDispatcherAddEntry(builder, entryOffset)
	nylon_dispatcher.NylonDispatcherAddData(builder, dataOffset)
	dispatcher := nylon_dispatcher.NylonDispatcherEnd(builder)
	builder.Finish(dispatcher)
	return builder.FinishedBytes()
}

func (d *Dispatcher) SwitchDataToHttpContext() *HttpContext {
	ctx := nylon_http_context.GetRootAsNylonHttpContext(d.Data, 0)

	return &HttpContext{
		Request:  *WrapRequest(ctx),
		Response: *WrapResponse(ctx),
	}
}

func (d *Dispatcher) SwitchDataToResponseFilter() *ResponseFilter {
	return &ResponseFilter{
		http_ctx: d.SwitchDataToHttpContext(),
	}
}

// func (d *Dispatcher) SwitchDataToRequestFilter() *RequestFilter {}

func (h *HttpContext) ToBytes() []byte {
	bufSize := len(h.Request.Body) + len(h.Response.Body) + 256
	bufSize += len(h.Request.Headers) + len(h.Response.Headers)
	bufSize += len(h.Request.Query) + len(h.Request.Params)
	bufSize += len(h.Request.Method) + len(h.Request.Path)

	builder := flatbuffers.NewBuilder(bufSize)

	// Build request
	req := h.Request
	reqMethodOffset := builder.CreateString(req.Method)
	reqPathOffset := builder.CreateString(req.Path)
	reqQueryOffset := builder.CreateString(req.Query.Encode())
	reqBodyOffset := builder.CreateByteString(req.Body)

	// Params
	paramsOffsets := make([]flatbuffers.UOffsetT, 0, len(req.Params))
	for k, v := range req.Params {
		kStr := builder.CreateString(k)
		vStr := builder.CreateString(v)
		nylon_http_context.KeyValueStart(builder)
		nylon_http_context.KeyValueAddKey(builder, kStr)
		nylon_http_context.KeyValueAddValue(builder, vStr)
		paramsOffsets = append(paramsOffsets, nylon_http_context.KeyValueEnd(builder))
	}
	nylon_http_context.NylonHttpRequestStartParamsVector(builder, len(paramsOffsets))
	for i := len(paramsOffsets) - 1; i >= 0; i-- {
		builder.PrependUOffsetT(paramsOffsets[i])
	}
	paramsVec := builder.EndVector(len(paramsOffsets))

	// Headers
	reqHeadersOffsets := make([]flatbuffers.UOffsetT, 0, len(req.Headers))
	for k, v := range req.Headers {
		kStr := builder.CreateString(k)
		vStr := builder.CreateString(v)
		nylon_http_context.KeyValueStart(builder)
		nylon_http_context.KeyValueAddKey(builder, kStr)
		nylon_http_context.KeyValueAddValue(builder, vStr)
		reqHeadersOffsets = append(reqHeadersOffsets, nylon_http_context.KeyValueEnd(builder))
	}
	nylon_http_context.NylonHttpRequestStartHeadersVector(builder, len(reqHeadersOffsets))
	for i := len(reqHeadersOffsets) - 1; i >= 0; i-- {
		builder.PrependUOffsetT(reqHeadersOffsets[i])
	}
	reqHeadersVec := builder.EndVector(len(reqHeadersOffsets))

	nylon_http_context.NylonHttpRequestStart(builder)
	nylon_http_context.NylonHttpRequestAddMethod(builder, reqMethodOffset)
	nylon_http_context.NylonHttpRequestAddPath(builder, reqPathOffset)
	nylon_http_context.NylonHttpRequestAddQuery(builder, reqQueryOffset)
	nylon_http_context.NylonHttpRequestAddParams(builder, paramsVec)
	nylon_http_context.NylonHttpRequestAddHeaders(builder, reqHeadersVec)
	nylon_http_context.NylonHttpRequestAddBody(builder, reqBodyOffset)
	request := nylon_http_context.NylonHttpRequestEnd(builder)

	// Build response
	res := h.Response
	resHeadersOffsets := make([]flatbuffers.UOffsetT, 0, len(res.Headers))
	for k, v := range res.Headers {
		kStr := builder.CreateString(k)
		vStr := builder.CreateString(v)
		nylon_http_context.KeyValueStart(builder)
		nylon_http_context.KeyValueAddKey(builder, kStr)
		nylon_http_context.KeyValueAddValue(builder, vStr)
		resHeadersOffsets = append(resHeadersOffsets, nylon_http_context.KeyValueEnd(builder))
	}

	nylon_http_context.NylonHttpResponseStartHeadersVector(builder, len(resHeadersOffsets))
	for i := len(resHeadersOffsets) - 1; i >= 0; i-- {
		builder.PrependUOffsetT(resHeadersOffsets[i])
	}
	resHeadersVec := builder.EndVector(len(resHeadersOffsets))

	body := builder.CreateByteString(res.Body)

	nylon_http_context.NylonHttpResponseStart(builder)
	nylon_http_context.NylonHttpResponseAddStatus(builder, int32(res.Status))
	nylon_http_context.NylonHttpResponseAddHeaders(builder, resHeadersVec)
	nylon_http_context.NylonHttpResponseAddBody(builder, body)
	response := nylon_http_context.NylonHttpResponseEnd(builder)

	// Build HttpContext
	nylon_http_context.NylonHttpContextStart(builder)
	nylon_http_context.NylonHttpContextAddRequest(builder, request)
	nylon_http_context.NylonHttpContextAddResponse(builder, response)
	httpCtx := nylon_http_context.NylonHttpContextEnd(builder)
	builder.Finish(httpCtx)

	return builder.FinishedBytes()
}

func (d *Dispatcher) SetPluginName(name string) {
	d.PluginName = name
}

func (d *Dispatcher) SetRequestId(requestId string) {
	d.RequestId = requestId
}

func (d *Dispatcher) SetEntry(entry string) {
	d.Entry = entry
}

func (d *Dispatcher) SetData(data []byte) {
	d.Data = data
}

func (d *Dispatcher) SetHttpEnd(httpEnd bool) {
	d.HttpEnd = httpEnd
}
