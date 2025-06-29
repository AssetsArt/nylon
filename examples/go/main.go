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
		// fmt.Println("authz")
		// fmt.Println("Ctx", ctx)
		// payload := ctx.GetPayload()
		// fmt.Println("Payload", payload)

		// set headers
		ctx.Response().SetHeader("x-test", "test")
		ctx.Response().SetHeader("Transfer-Encoding", "chunked")
		// set Basic Auth
		// ctx.Response().SetHeader("WWW-Authenticate", "Basic realm=\"Restricted\"")

		// remove  headers
		ctx.Response().RemoveHeader("Content-Type")
		ctx.Response().RemoveHeader("Content-Length")

		// set status
		// ctx.Response().SetStatus(401)

		// sleep 3 second
		// time.Sleep(3 * time.Second)

		// next middleware
		ctx.Next()
	})
}
