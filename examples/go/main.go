//go:build cgo

package main

import "C"
import (
	"fmt"
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

func main() {}

//export initialize
func initialize(config *C.char, length C.int) {
	configData := C.GoBytes(unsafe.Pointer(config), C.int(length))
	fmt.Println("[NylonPlugin] Plugin initialized", string(configData))
	plugin := sdk.NewNylonPlugin()

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

		// next middleware
		ctx.Next()
	})
}
