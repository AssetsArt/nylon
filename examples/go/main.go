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

			// for modify response body
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.Next()
		})

		phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
			fmt.Println("Authz[Go] ResponseBodyFilter sessionID", phase.SessionId)

			// Read response body
			res := ctx.Response()
			body := res.ReadBody()
			fmt.Println("Authz[Go] ResponseBody length:", len(body))

			// Modify response body (example: append text)
			modifiedBody := append(body, []byte("\n<!-- Modified by Authz plugin -->")...)
			res.BodyRaw(modifiedBody)

			ctx.Next()
		})

		phase.Logging(func(ctx *sdk.PhaseLogging) {
			fmt.Println("Authz[Go] Logging sessionID", phase.SessionId)

			// Access request info for logging
			req := ctx.Request()
			res := ctx.Response()

			fmt.Printf("Authz[Go] Log: %s %s | Status: %d | ReqBytes: %d | ResBytes: %d | Host: %s | Client: %s\n",
				req.Method(),
				req.Path(),
				res.Status(),
				req.Bytes(),
				res.Bytes(),
				req.Host(),
				req.ClientIP(),
			)

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

	plugin.AddPhaseHandler("myapp", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start MyApp[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("MyApp[Go] RequestFilter sessionID", phase.SessionId)

			req := ctx.Request()

			// Test new methods
			fmt.Println("MyApp[Go] URL:", req.URL())
			fmt.Println("MyApp[Go] Path:", req.Path())
			fmt.Println("MyApp[Go] Query:", req.Query())
			fmt.Println("MyApp[Go] Params:", req.Params())
			fmt.Println("MyApp[Go] Host:", req.Host())
			fmt.Println("MyApp[Go] ClientIP:", req.ClientIP())
			fmt.Println("MyApp[Go] Headers:", req.Headers())

			res := ctx.Response()
			// set status and headers
			res.SetStatus(200)
			res.SetHeader("Content-Type", "application/json")
			res.SetHeader("Transfer-Encoding", "chunked")

			// Return info as JSON
			info := map[string]interface{}{
				"url":       req.URL(),
				"path":      req.Path(),
				"query":     req.Query(),
				"params":    req.Params(),
				"host":      req.Host(),
				"client_ip": req.ClientIP(),
			}
			res.BodyJSON(info)

			ctx.End()
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
