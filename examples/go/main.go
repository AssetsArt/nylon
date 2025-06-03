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

	/*
		bodyJson := map[string]any{}
		err := http_ctx.Request.BodyJSON(&bodyJson)
		if err != nil {
			http_ctx.Response.BodyHTML(`
			<html>
				<body>
					<h1>Error</h1>
					<p>
						` + err.Error() + `
					</p>
				</body>
			</html>
			`)
			http_ctx.Response.SetStatus(400)
			dispatcher.SetHttpEnd(true)
			dispatcher.SetData(http_ctx.ToBytes())
			return send_response(dispatcher.ToBytes())
		}
	*/
	// create response
	http_ctx.Response.BodyJSON(map[string]any{
		"ok": true,
		"ts": time.Now().Unix(),
		// "body": bodyJson,
	})

	// switch http context to bytes
	dispatcher.SetHttpEnd(true)
	dispatcher.SetData(http_ctx.ToBytes())

	return send_response(dispatcher.ToBytes())
}

//export sdk_handle_middleware
func sdk_handle_middleware(ptr *C.uchar, input_len C.int) C.FfiOutput {
	input := C.GoBytes(unsafe.Pointer(ptr), C.int(input_len))
	dispatcher := sdk.WrapDispatcher(input)
	http_ctx := dispatcher.SwitchDataToHttpContext()
	// create response
	http_ctx.Response.BodyJSON(map[string]any{
		"ok":   true,
		"ts":   time.Now().Unix(),
		"from": "middleware",
	})

	pass := http_ctx.Request.Query.Get("pass")
	if pass == "true" {
		// switch http context to bytes
		dispatcher.SetHttpEnd(true)
	} else {
		// switch http context to bytes
		dispatcher.SetHttpEnd(false)
	}
	dispatcher.SetData(http_ctx.ToBytes())

	return send_response(dispatcher.ToBytes())
}
