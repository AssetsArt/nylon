//go:build cgo

package main

import "C"
import (
	"encoding/json"
	"fmt"
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

func main() {}

//export initialize
func initialize(config *C.char, length C.int) {
	configBytes := C.GoBytes(unsafe.Pointer(config), C.int(length))
	configData := struct {
		Debug bool `json:"debug"`
	}{
		Debug: false,
	}
	err := json.Unmarshal(configBytes, &configData)
	if err != nil {
		fmt.Println("[NylonPlugin] Error unmarshalling config", err)
		return
	}

	// Print the config data
	fmt.Println("[NylonPlugin] Plugin initialized", string(configBytes))

	// Create a new plugin
	plugin := sdk.NewNylonPlugin()

	// Register shutdown handler
	plugin.Shutdown(func() {
		fmt.Println("[NylonPlugin] Plugin shutdown")
	})

	// Register middleware
	plugin.AddRequestFilter("authz", func(ctx *sdk.PhaseRequestFilter) {
		// payload := ctx.GetPayload()
		// fmt.Println("Payload", payload)

		// read request body
		// body := ctx.Request().ReadBody()
		// fmt.Println("Request body", string(body))

		// // set headers
		ctx.Response().
			SetHeader("x-authz", "true")

		// next middleware
		ctx.Next()
	})

	// example of streaming response
	plugin.AddRequestFilter("stream_body", func(ctx *sdk.PhaseRequestFilter) {
		res := ctx.Response()

		// set status and headers
		res.SetStatus(200)
		res.SetHeader("Content-Type", "text/plain")

		// Start streaming response
		stream, err := res.Stream()
		if err != nil {
			fmt.Println("[NylonPlugin] Error streaming response", err)
			ctx.Next()
			return
		}
		stream.Write([]byte("Hello"))
		w := ", World"
		for i := 0; i < len(w); i++ {
			stream.Write([]byte(w[i : i+1]))
		}

		// End streaming response
		stream.End()
	})

}
