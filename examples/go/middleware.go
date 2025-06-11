package main

/*
#include "../../c/nylon.h"
*/
import "C"
import "fmt"

//export sdk_go_mid_request_filter
func sdk_go_mid_request_filter(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// set request header
	http_ctx.Request.SetHeader("x-middleware", "true")

	// set response header
	http_ctx.Response.SetHeader("x-request-filter", "true")

	// set http end and data
	dispatcher.SetHttpEnd(false)           // set http end to false
	dispatcher.SetData(http_ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}

//export sdk_go_mid_response_filter
func sdk_go_mid_response_filter(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	ctx := dispatcher.SwitchDataToResponseFilter()

	// set response header
	ctx.SetHeader("x-response-filter", "true")

	// if modify body, set transfer-encoding to chunked
	ctx.SetHeader("transfer-encoding", "chunked")
	ctx.RemoveHeader("content-length")

	// set response status
	ctx.SetStatus(201)

	// set data to dispatcher
	dispatcher.SetData(ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}

//export sdk_go_mid_response_body_filter
func sdk_go_mid_response_body_filter(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	ctx := dispatcher.SwitchDataToResponseBodyFilter()

	oldBody, err := ctx.BodyJSON()
	if err != nil {
		fmt.Println("[sdk_go_mid_response_body_filter] error", err)
		return SendResponse(dispatcher)
	}
	oldBody["x-response-body-filter"] = "true"
	ctx.SetBodyJSON(oldBody)

	// set data to dispatcher
	dispatcher.SetData(ctx.ToBytes())

	return SendResponse(dispatcher)
}
