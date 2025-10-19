# Go SDK

Complete Go SDK API reference. Pair this document with the [plugin overview](/plugins/overview) for end-to-end examples.

## Installation

```bash
go get github.com/AssetsArt/nylon/sdk/go/sdk
```

## Basic Usage

```go
package main

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()

	plugin.AddPhaseHandler("handler", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			// Gate, mutate, or short-circuit the request.
			ctx.Next()
		})
	})
}
```

## API Reference

### Request helpers

| Method | Description |
|--------|-------------|
| `req.Method()` | HTTP method (`GET`, `POST`, …). |
| `req.Path()` | Request path. |
| `req.URL()` | Full URL (scheme + host + path + query). |
| `req.Query()` | Raw query string. |
| `req.Params()` | Route parameters (`map[string]string`). |
| `req.Header(name)` | Single header value. |
| `req.Headers()` | Iterator with `.Get`/`.GetAll()` helpers. |
| `req.RawBody()` | Request body (lazy-loaded). |
| `req.Host()` | Host header. |
| `req.ClientIP()` | Client IP address. |
| `req.Timestamp()` | Request timestamp (milliseconds). |
| `req.Bytes()` | Request body size. |

### Response helpers

| Method | Description |
|--------|-------------|
| `res.SetStatus(code)` | Set status code. |
| `res.Status()` | Retrieve status. |
| `res.SetHeader(name, value)` | Set/overwrite header. |
| `res.RemoveHeader(name)` | Remove header. |
| `res.Headers()` | Map of response headers. |
| `res.BodyRaw([]byte)` | Replace body with bytes. |
| `res.BodyText(string)` | Convenience for UTF-8 text. |
| `res.BodyJSON(any)` | Marshal and send JSON. |
| `res.ReadBody()` | Read upstream body (response body filter / logging). |
| `res.Redirect(url, code...)` | Issue redirect (default 302). |
| `res.Bytes()` | Response size. |
| `res.Duration()` | Elapsed time in ms. |
| `res.Error()` | Captured upstream errors. |
| `res.Stream()` | Start streaming response. |

> `ctx.GetPayload()` is available on every phase context and returns the static payload configured in your YAML middleware entry.

## WebSocket helper APIs

Upgrade an HTTP connection to WebSocket and register callbacks:

```go
callbacks := sdk.WebSocketCallbacks{
	OnOpen: func(ws *sdk.WebSocketConn) {
		ws.JoinRoom("lobby")
		ws.SendText("Welcome!")
	},
	OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
		ws.BroadcastText("lobby", msg)
	},
	OnClose: func(ws *sdk.WebSocketConn) {
		ws.LeaveRoom("lobby")
	},
}

if err := ctx.WebSocketUpgrade(callbacks); err != nil {
	res := ctx.Response()
	res.RemoveHeader("Content-Length")
	res.SetHeader("Transfer-Encoding", "chunked")
	res.SetStatus(400)
	res.BodyText("Upgrade failed")
	ctx.End()
	return
}

ctx.Next()
```

Connection methods:

| Method | Description |
|--------|-------------|
| `SendText(message)` | Send text frame. |
| `SendBinary(data)` | Send binary frame. |
| `Close()` | Close connection. |
| `JoinRoom(room)` / `LeaveRoom(room)` | Convenience helpers for broadcast rooms. |
| `BroadcastText(room, message)` | Broadcast text to room. |
| `BroadcastBinary(room, data)` | Broadcast binary payload to room. |

## Streaming responses

```go
stream, err := ctx.Response().Stream()
if err != nil {
	log.Printf("stream error: %v", err)
	ctx.End()
	return
}

defer stream.End()
stream.Write([]byte("chunk 1"))
stream.Write([]byte("chunk 2"))
```

## Error handling patterns

```go
if err := ctx.WebSocketUpgrade(callbacks); err != nil {
	res := ctx.Response()
	res.RemoveHeader("Content-Length")
	res.SetHeader("Transfer-Encoding", "chunked")
	res.SetStatus(400)
	res.BodyText("Error")
	ctx.End()
	return
}

ctx.Next()
```

## Best practices

1. **Always call `ctx.Next()`** unless you explicitly terminate the request with `ctx.End()`.
2. **Handle errors** – return proper status codes and chunked responses to avoid hangs.
3. **Avoid long blocking work** inside phase handlers; offload to goroutines if necessary.
4. **Use middleware payloads** for configuration; they are exposed through `ctx.GetPayload()` in every phase.
