# NATS Plugin Quick Start

## Overview

This guide shows how to create a NATS-based plugin using the Go SDK. NATS plugins enable horizontal scaling of plugin workers using queue groups.

## Prerequisites

- Go 1.21+
- NATS server running (local or remote)
- Nylon configured with `messaging:` block

## Step 1: Start NATS Server

```bash
# Using Docker
docker run -d --name nats -p 4222:4222 nats:latest

# Or install locally
brew install nats-server
nats-server
```

## Step 2: Configure Nylon

Add to your `nylon.yaml`:

```yaml
messaging:
  - name: default
    servers:
      - nats://localhost:4222

plugins:
  - name: my-plugin
    messaging: default
    entry_point: authz
```

**That's it!** No `per_phase` config needed - smart defaults applied automatically.

## Step 3: Create NATS Plugin Worker (Go)

### Install Dependencies

```bash
go get github.com/AssetsArt/nylon/sdk/go
```

### Create Plugin

```go
package main

import (
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/AssetsArt/nylon/sdk/go/sdk"
)

func main() {
	// Create NATS plugin config
	config := &sdk.NatsPluginConfig{
		Name:       "my-plugin",
		Servers:    []string{"nats://localhost:4222"},
		QueueGroup: "my-workers",
		MaxWorkers: 10,
	}

	// Create NATS plugin
	plugin, err := sdk.NewNylonNatsPlugin(config)
	if err != nil {
		log.Fatalf("Failed to create plugin: %v", err)
	}

	// Register phase handler
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		// Request filter (modify request)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			res := ctx.Response()

			fmt.Printf("Processing: %s %s\n", req.Method(), req.Path())

			// Add custom header
			res.SetHeader("X-Plugin-Transport", "NATS")

			ctx.Next()
		})

		// Response filter (modify response headers)
		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			ctx.SetResponseHeader("X-Processed-By", "NATS-Worker")
			ctx.Next()
		})

		// Logging (log after response)
		phase.Logging(func(ctx *sdk.PhaseLogging) {
			req := ctx.Request()
			res := ctx.Response()

			fmt.Printf("Completed: %s %s | Status: %d | Duration: %dms\n",
				req.Method(),
				req.Path(),
				res.Status(),
				res.Duration(),
			)

			ctx.Next()
		})
	})

	// Graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-sigChan
		fmt.Println("\nShutting down...")
		plugin.Close()
		os.Exit(0)
	}()

	// Start plugin (blocks)
	fmt.Printf("Starting NATS plugin: %s\n", config.Name)
	if err := plugin.Start(); err != nil {
		log.Fatalf("Failed to start: %v", err)
	}
}
```

### Build and Run

```bash
go build -o worker main.go
./worker
```

## Step 4: Scale Workers

Run multiple workers - NATS automatically load balances:

```bash
# Terminal 1
./worker

# Terminal 2
./worker

# Terminal 3
./worker
```

All workers share the queue group and receive requests round-robin!

## Comparison: FFI vs NATS

### FFI Plugin (Traditional)

```go
// In-process, compiled as .so
plugin := sdk.NewNylonPlugin()
// ... handlers ...
```

- ✅ Low latency (~100μs)
- ❌ No horizontal scaling
- ❌ Requires C compilation
- ❌ Runs in Nylon process

### NATS Plugin (New)

```go
// Separate process, NATS-based
plugin, _ := sdk.NewNylonNatsPlugin(&sdk.NatsPluginConfig{
    Name:    "my-plugin",
    Servers: []string{"nats://localhost:4222"},
})
// ... same handlers ...
```

- ✅ Horizontal scaling (queue groups)
- ✅ Independent processes
- ✅ Language-agnostic (any NATS client)
- ✅ Zero-downtime deployments
- ⚡ ~0.5-1ms latency

## Configuration Options

### Full Config Example

```go
config := &sdk.NatsPluginConfig{
    Name:       "my-plugin",
    Servers:    []string{"nats://nats-1:4222", "nats://nats-2:4222"},
    QueueGroup: "production-workers",
    SubjectPrefix: "nylon.plugin",
    MaxWorkers: 20,
    NatsOptions: []nats.Option{
        nats.Token("my-secret-token"),
        nats.MaxReconnects(-1),
    },
}
```

### Nylon Config (Optional Overrides)

```yaml
messaging:
  - name: default
    servers:
      - nats://nats-1:4222
      - nats://nats-2:4222
    request_timeout_ms: 10000
    max_inflight: 2048

plugins:
  - name: my-plugin
    messaging: default
    entry_point: authz
    group: production-workers
    
    # Optional: Override defaults
    per_phase:
      request_filter:
        timeout_ms: 8000
        on_error: retry
        retry:
          max: 5
```

## NATS Subject Layout

Workers automatically subscribe to:

```
nylon.plugin.my-plugin.request_filter  (queue: production-workers)
nylon.plugin.my-plugin.response_filter (queue: production-workers)
nylon.plugin.my-plugin.response_body_filter (queue: production-workers)
nylon.plugin.my-plugin.logging         (queue: production-workers)
```

NATS picks ONE worker per request based on queue group.

## Monitoring

### Worker Logs

```
[NatsPlugin] Connected to NATS: nats://localhost:4222
[NatsPlugin] Subscribed to nylon.plugin.my-plugin.request_filter with queue group my-workers
[NatsPlugin] Plugin my-plugin started successfully
[NatsPlugin] Received request: session=1 phase=1 method=
Processing: GET /api/users
Completed: GET /api/users | Status: 200 | Duration: 45ms
```

### NATS CLI

```bash
# Monitor subjects
nats sub "nylon.plugin.>"

# Check queue groups
nats sub --queue my-workers "nylon.plugin.my-plugin.>"
```

## Troubleshooting

### Worker doesn't receive requests

1. Check NATS connection: `nats server check`
2. Verify plugin name matches config
3. Check queue group name
4. Ensure Nylon has `messaging:` config

### Multiple workers receive same request

- Queue group name must be identical across workers
- Check `QueueGroup` in NatsPluginConfig

### Timeout errors

- Increase `request_timeout_ms` in messaging config
- Check worker processing time
- Verify NATS connectivity

## Next Steps

- **Load Testing**: Benchmark NATS vs FFI performance
- **Monitoring**: Add metrics and observability
- **WebSocket Support**: See `docs/WEBSOCKET_NATS_DESIGN.md`
- **Production Deployment**: Multi-node NATS cluster

## Resources

- [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough)
- [NATS Plugin Configuration](./NATS_PLUGIN_CONFIG.md)
- [NATS Plugin Roadmap](./NATS_PLUGIN_ROADMAP.md)

