# Go SDK

Complete Go SDK API reference.

## Installation

```bash
go get github.com/AssetsArt/nylon/sdk/go/sdk
```

## Basic Usage

```go
package main

import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

//export NewNylonPlugin
func NewNylonPlugin() *sdk.NylonPlugin {
	plugin := sdk.NylonPlugin{}
	
	plugin.AddPhaseHandler("handler", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			// Your code here
			ctx.Next()
		})
	})
	
	return &plugin
}
```

## API Reference

### Request Methods

- `req.Method()` - HTTP method
- `req.Path()` - Request path
- `req.URL()` - Full URL
- `req.Query()` - Query string
- `req.Header(key)` - Get header
- `req.Headers()` - All headers
- `req.Params()` - URL parameters
- `req.ReadBody()` - Request body
- `req.Host()` - Host header
- `req.ClientIP()` - Client IP
- `req.Timestamp()` - Request timestamp
- `req.Bytes()` - Request size

### Response Methods

- `res.SetStatus(code)` - Set status code
- `res.SetHeader(key, value)` - Set header
- `res.SetResponseHeader(key, value)` - Set response header
- `res.RemoveResponseHeader(key)` - Remove header
- `res.BodyRaw([]byte)` - Set body
- `res.BodyText(string)` - Set text body
- `res.ReadBody()` - Read response body
- `res.Status()` - Get status code
- `res.Headers()` - Get response headers
- `res.Bytes()` - Response size
- `res.Duration()` - Request duration
- `res.Error()` - Error message

More details coming soon...

