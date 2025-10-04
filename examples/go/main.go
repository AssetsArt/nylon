package main

import "C"
import (
	"fmt"

	"github.com/AssetsArt/nylon/sdk/go/sdk"
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
		fmt.Println("Start Authz[Go] sessionID", phase.SessionId)
		// Initialize phase state per request
		myPhaseState := map[string]bool{
			"authz": false,
		}

		// Phase request filter
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("Authz[Go] RequestFilter sessionID", phase.SessionId)
			myPhaseState["authz"] = true

			payload := ctx.GetPayload()
			fmt.Println("[Authz][NylonPlugin] Payload", payload)

			response := ctx.Response()
			response.SetHeader("X-RequestFilter", "authz-1")
			// sleep 2 seconds
			// time.Sleep(2 * time.Second)
			// next phase
			ctx.Next()
		})

		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			fmt.Println("Authz[Go] ResponseFilter sessionID", phase.SessionId)
			ctx.SetResponseHeader("X-ResponseFilter", "authz-2")

			ctx.Next()
		})

	})

	plugin.AddPhaseHandler("stream", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start Stream[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("Stream[Go] RequestFilter sessionID", phase.SessionId)
			res := ctx.Response()
			// set status and headers
			res.SetStatus(200)
			res.SetHeader("Content-Type", "text/plain")

			// Start streaming response
			stream, err := res.Stream()
			if err != nil {
				fmt.Println("[Stream][NylonPlugin] Error streaming response", err)
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

	// WebSocket example
	plugin.AddPhaseHandler("ws", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start WS[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("WS[Go] RequestFilter sessionID", phase.SessionId)
			err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
				OnOpen: func(ws *sdk.WebSocketConn) {
					fmt.Println("[WS][Go] onOpen")
					ws.SendText("hello from plugin")
					// Join default room and broadcast welcome
					_ = ws.JoinRoom("lobby")
					_ = ws.BroadcastText("lobby", "user joined")
				},
				OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
					fmt.Println("[WS][Go] onMessageText:", msg)
					ws.SendText("echo: " + msg)
					// Broadcast to room
					_ = ws.BroadcastText("lobby", msg)
				},
				OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
					fmt.Println("[WS][Go] onMessageBinary", len(data))
					ws.SendBinary(data)
				},
				OnClose: func(ws *sdk.WebSocketConn) {
					fmt.Println("[WS][Go] onClose")
					_ = ws.LeaveRoom("lobby")
				},
				OnError: func(ws *sdk.WebSocketConn, err string) {
					fmt.Println("[WS][Go] onError:", err)
				},
			})
			if err != nil {
				fmt.Println("[WS][Go] upgrade error:", err)
				// On error fallback to HTTP
				ctx.Next()
			}
		})
	})
}
