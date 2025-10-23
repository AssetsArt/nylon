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

	plugin.AddPhaseHandler("myapp", func(phase *sdk.PhaseHandler) {
		// fmt.Println("Start MyApp[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			// fmt.Println("MyApp[Go] RequestFilter sessionID", phase.SessionId)

			req := ctx.Request()

			// Test new methods
			// fmt.Println("MyApp[Go] URL:", req.URL())
			// fmt.Println("MyApp[Go] Path:", req.Path())
			// fmt.Println("MyApp[Go] Query:", req.Query())
			// fmt.Println("MyApp[Go] Params:", req.Params())
			// fmt.Println("MyApp[Go] Host:", req.Host())
			// fmt.Println("MyApp[Go] ClientIP:", req.ClientIP())
			// fmt.Println("MyApp[Go] Headers:", req.Headers())

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

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// go func() {
	// 	<-sigChan
	// 	fmt.Println("\n[NatsPlugin] Shutting down gracefully...")
	// 	plugin.Close()
	// 	os.Exit(0)
	// }()

	// Start the plugin (blocks forever)
	fmt.Printf("[NatsPlugin] Starting plugin: %s\n", config.Name)
	fmt.Printf("[NatsPlugin] NATS servers: %v\n", config.Servers)
	fmt.Printf("[NatsPlugin] Queue group: %s\n", config.QueueGroup)

	if err := plugin.Start(); err != nil {
		log.Fatalf("Failed to start plugin: %v", err)
	}
}
