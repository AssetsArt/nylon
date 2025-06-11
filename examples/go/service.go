package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"fmt"
	"time"
)

//export sdk_go_service
func sdk_go_service(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// payload
	payload := dispatcher.SwitchPayloadToJson()
	fmt.Println("payload", payload)

	// create response
	http_ctx.Response.BodyJSON(map[string]any{
		"request_id":  dispatcher.RequestId,
		"plugin_name": dispatcher.PluginName,
		"entry":       dispatcher.Entry,
		"ok":          true,
		"ts":          time.Now().Unix(),
		"headers":     http_ctx.Request.Headers,
	})
	// http_ctx.Response.SetStatus(400)
	http_ctx.Response.SetHeader("x-service", "true")

	// set http end and data
	dispatcher.SetHttpEnd(true)            // set http end to true
	dispatcher.SetData(http_ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}
