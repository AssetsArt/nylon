//go:build cgo

package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"time"
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

func main() {}

func send_response(output []byte) C.FfiOutput {
	return C.FfiOutput{
		ptr: (*C.uchar)(C.CBytes(output)),
		len: C.ulong(len(output)),
	}
}

//export sdk_handle_request
func sdk_handle_request(ptr *C.uchar, input_len C.int) C.FfiOutput {
	input := C.GoBytes(unsafe.Pointer(ptr), C.int(input_len))
	dispatcher := sdk.WrapDispatcher(input)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// create response
	http_ctx.Response.BodyJSON(map[string]any{
		"ok": true,
		"ts": time.Now().Unix(),
	})

	// switch http context to bytes
	dispatcher.SetHttpEnd(true)
	dispatcher.SetData(http_ctx.SwitchHttpContextToBytes())

	return send_response(dispatcher.ToBytes())
}
