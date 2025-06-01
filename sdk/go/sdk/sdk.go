package sdk

import (
	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_dispatcher"
	"github.com/AssetsArt/easy-proxy/sdk/go/fbs/nylon_http_context"
)

type SwitchHttpContextStruct struct {
	End      bool
	Request  Request
	Response *nylon_http_context.NylonHttpResponse
}

func ParseContext(input []byte) *nylon_dispatcher.NylonDispatcher {
	return nylon_dispatcher.GetRootAsNylonDispatcher(input, 0)
}

func SwitchHttpContext(input []byte) *SwitchHttpContextStruct {
	ctx := nylon_http_context.GetRootAsNylonHttpContext(input, 0)
	res := ctx.Response(nil)

	return &SwitchHttpContextStruct{
		End:      ctx.End(),
		Request:  *WrapRequest(ctx),
		Response: res,
	}
}
