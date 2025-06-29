//go:build cgo

package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"fmt"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

func main() {}

func init() {
	fmt.Println("Plugin loaded")
	plugin := sdk.NewNylonPlugin()

	// Register middleware
	plugin.HttpPlugin("authz", func(ctx *sdk.NylonHttpPluginCtx) {
		// fmt.Println("Ctx", ctx)
		// payload := ctx.GetPayload()
		// fmt.Println("Payload", payload)

		ctx.SetResponseHeader("X-Test", "test")

		// next middleware
		ctx.Next()
	})
}
