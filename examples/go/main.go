package main

import "C"
import (
	"fmt"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

type PluginConfig struct {
	Debug bool `json:"debug"`
}

func main() {}
func init() {

	// Create a new plugin
	plugin := sdk.NewNylonPlugin()

	// Register initialize handler
	plugin.Initialize(sdk.NewInitializer(func(config PluginConfig) {
		fmt.Println("[NylonPlugin] Plugin initialized")
		fmt.Println("[NylonPlugin] Config: Debug", config.Debug)
	}))

	// Register shutdown handler
	plugin.Shutdown(func() {
		fmt.Println("[NylonPlugin] Plugin shutdown")
	})

	// phase
	// - RequestFilter // Can return a full response
	//   |
	//   V
	// - ResponseFilter // Can modify the response headers
	//   |
	//   V
	// - ResponseBodyFilter // Can modify the response body
	//   |
	//   V
	// - Logging // Can log the request and response

	// Register middleware
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		fmt.Println("[Go] sessionID", phase.SessionId)
		// Initialize phase state per request
		myPhaseState := map[string]bool{
			"authz": false,
		}

		// Phase request filter
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("[NylonPlugin] Authz phase")
			myPhaseState["authz"] = true

			payload := ctx.GetPayload()
			fmt.Println("[NylonPlugin] Payload", payload)

			response := ctx.Response()
			response.SetHeader("X-RequestFilter", "authz-1")

			// next phase
			ctx.Next()
		})

		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			fmt.Println("[NylonPlugin] Response filter")
			ctx.SetResponseHeader("X-ResponseFilter", "authz-2")

			ctx.Next()
		})

	})

	plugin.AddPhaseHandler("stream", func(phase *sdk.PhaseHandler) {
		fmt.Println("[NylonPlugin] Stream phase")
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("[NylonPlugin] Stream request filter")
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
	})
}
