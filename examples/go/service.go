package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"time"
)

//export sdk_go_service
func sdk_go_service(ptr *C.uchar, input_len C.int) C.FfiOutput {
	dispatcher := InputToDispatcher(ptr, input_len)
	http_ctx := dispatcher.SwitchDataToHttpContext()

	// create response
	http_ctx.Response.BodyJSON(map[string]any{
		"request_id":  dispatcher.RequestId,
		"plugin_name": dispatcher.PluginName,
		"entry":       dispatcher.Entry,
		"ok":          true,
		"ts":          time.Now().Unix(),
		"headers":     http_ctx.Request.Headers,
	})

	// set http end and data
	dispatcher.SetHttpEnd(true)            // set http end to true
	dispatcher.SetData(http_ctx.ToBytes()) // set data to http context

	return SendResponse(dispatcher)
}
