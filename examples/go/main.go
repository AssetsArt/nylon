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
	plugin.HttpPlugin("authz", func(ctx *sdk.NylonHttpPluginCtx) {
		payload := ctx.GetPayload()
		fmt.Println("Payload", payload)

		// set headers
		ctx.Response().SetHeader("x-test", "test")
		ctx.Response().SetHeader("Transfer-Encoding", "chunked")

		// remove  headers
		ctx.Response().RemoveHeader("Content-Type")
		ctx.Response().RemoveHeader("Content-Length")

		ctx.Response().SetStatus(201)

		// next middleware
		ctx.Next()
	})
}
