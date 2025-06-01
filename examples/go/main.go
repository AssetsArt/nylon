//go:build cgo

package main

/*
#include <stdlib.h>
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
	ctx := sdk.ParseContext(input)
	http_ctx := sdk.SwitchHttpContext(ctx.DataBytes())

	// create response
	resp := sdk.NewHttpResponse(http_ctx).JSON(map[string]any{
		"ok": true,
		"ts": time.Now().Unix(),
	})

	out := resp.Send(ctx)
	return send_response(out)
}
