package main

/*
#include "../../c/nylon.h"
*/
import "C"

//export sdk_go_mid_request_filter
func sdk_go_mid_request_filter(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// set request header
	http_ctx.Request.SetHeader("x-middleware", "true")

	// set http end and data
	dispatcher.SetHttpEnd(false)           // set http end to false
	dispatcher.SetData(http_ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}

//export sdk_go_mid_response_filter
func sdk_go_mid_response_filter(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	res := dispatcher.SwitchDataToResponseFilter()

	// set response header
	res.SetHeader("x-response-filter", "true")

	// if modify body, set transfer-encoding to chunked
	res.SetHeader("transfer-encoding", "chunked")
	res.RemoveHeader("content-length")

	// set data to dispatcher
	dispatcher.SetData(res.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}
