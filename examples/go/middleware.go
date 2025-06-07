package main

/*
#include "../../c/nylon.h"
*/
import "C"

//export sdk_go_middleware
func sdk_go_middleware(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// set request header
	http_ctx.Request.SetHeader("x-middleware", "true")

	// set http end and data
	dispatcher.SetHttpEnd(false)           // set http end to false
	dispatcher.SetData(http_ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}
