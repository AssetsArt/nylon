//go:build cgo

package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"fmt"
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

func main() {}

func init() {
	fmt.Println("Plugin loaded")
	plugin := sdk.NewNylonPlugin()

	// Register middleware
	plugin.HttpPlugin("authz", func(ctx *sdk.NylonHttpPluginCtx) {
		// fmt.Println("Ctx", ctx)
		_ = ctx.GetPayload()
		// fmt.Println("Payload", payload)

		ctx.SetResponseHeader("X-Test", "test")

		// next middleware
		ctx.Next()
	})
}
