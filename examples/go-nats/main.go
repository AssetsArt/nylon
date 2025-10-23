package main

import (
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/AssetsArt/nylon/sdk/go/sdk"
)

type PluginConfig struct {
	Debug bool `json:"debug"`
}

func main() {
	// Create NATS plugin config
	config := &sdk.NatsPluginConfig{
		Name:       "my-nats-plugin",
		Servers:    []string{"nats://localhost:4222"},
		QueueGroup: "my-workers",
		MaxWorkers: 10,
	}

	// Create NATS plugin
	plugin, err := sdk.NewNylonNatsPlugin(config)
	if err != nil {
		log.Fatalf("Failed to create plugin: %v", err)
	}

	// Register initialize handler
	plugin.Initialize(sdk.NewInitializer(func(config PluginConfig) {
		fmt.Println("[NatsPlugin] Plugin initialized")
		fmt.Println("[NatsPlugin] Config: Debug", config.Debug)
	}))

	// Register shutdown handler
	plugin.Shutdown(func() {
		fmt.Println("[NatsPlugin] Plugin shutdown")
	})

	// Register authz phase handler
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start Authz[Go-NATS] sessionID", phase.SessionId)

		// Phase request filter
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("Authz[Go-NATS] RequestFilter sessionID", phase.SessionId)

			payload := ctx.GetPayload()
			fmt.Println("[Authz][NatsPlugin] Payload", payload)

			response := ctx.Response()
			response.SetHeader("X-RequestFilter", "authz-nats-1")
			response.SetHeader("X-Transport", "NATS")

			ctx.Next()
		})

		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			fmt.Println("Authz[Go-NATS] ResponseFilter sessionID", phase.SessionId)
			ctx.SetResponseHeader("X-ResponseFilter", "authz-nats-2")

			// for modify response body
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.Next()
		})

		phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
			fmt.Println("Authz[Go-NATS] ResponseBodyFilter sessionID", phase.SessionId)

			// Read response body
			res := ctx.Response()
			body := res.ReadBody()
			fmt.Println("Authz[Go-NATS] ResponseBody length:", len(body))

			// Modify response body
			modifiedBody := append(body, []byte("\n<!-- Modified by NATS plugin -->")...)
			res.BodyRaw(modifiedBody)

			ctx.Next()
		})

		phase.Logging(func(ctx *sdk.PhaseLogging) {
			fmt.Println("Authz[Go-NATS] Logging sessionID", phase.SessionId)

			req := ctx.Request()
			res := ctx.Response()

			fmt.Printf("Authz[Go-NATS] Log: %s %s | Status: %d | ReqBytes: %d | ResBytes: %d | Duration: %dms\n",
				req.Method(),
				req.Path(),
				res.Status(),
				req.Bytes(),
				res.Bytes(),
				res.Duration(),
			)

			ctx.Next()
		})
	})

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-sigChan
		fmt.Println("\n[NatsPlugin] Shutting down gracefully...")
		plugin.Close()
		os.Exit(0)
	}()

	// Start the plugin (blocks forever)
	fmt.Printf("[NatsPlugin] Starting plugin: %s\n", config.Name)
	fmt.Printf("[NatsPlugin] NATS servers: %v\n", config.Servers)
	fmt.Printf("[NatsPlugin] Queue group: %s\n", config.QueueGroup)

	if err := plugin.Start(); err != nil {
		log.Fatalf("Failed to start plugin: %v", err)
	}
}
