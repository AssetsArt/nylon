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
	plugin.HandleRequest("authz", func(ctx *sdk.NylonPluginCtx) {
		// fmt.Println("Ctx", ctx)
		payload := ctx.GetPayload()
		fmt.Println("Payload", payload)

		// next middleware
		ctx.Next()
	})
}
