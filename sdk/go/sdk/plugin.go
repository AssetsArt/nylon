//go:build cgo

package sdk

/*
#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    uint32_t sid;
    uint8_t phase;
    uint32_t method;
    const unsigned char *ptr;
    uint64_t len;
} FfiBuffer;

typedef void (*data_event_fn)(const FfiBuffer* ffiBuffer);

static inline void call_event_method(data_event_fn cb, const FfiBuffer* ffiBuffer) {
    cb(ffiBuffer);
}

#endif // NYLON_H
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"sync"
	"sync/atomic"
	"unsafe"
)

var (
	streamSessions    = sync.Map{}
	shutdownHandler   atomic.Value
	initializeHandler atomic.Value
	phaseHandlerMap   = sync.Map{}
	pluginInstance    *NylonPlugin
)

func NewInitializer[T any](fn func(config T)) func(map[string]interface{}) {
	return func(raw map[string]interface{}) {
		var cfg T
		data, _ := json.Marshal(raw)
		json.Unmarshal(data, &cfg)
		fn(cfg)
	}
}

// NewNylonPlugin creates a new NylonPlugin instance
func NewNylonPlugin() *NylonPlugin {
	if pluginInstance != nil {
		fmt.Println("[NylonPlugin] Plugin instance already exists")
		return pluginInstance
	}
	pluginInstance = &NylonPlugin{}
	return pluginInstance
}

func (plugin *NylonPlugin) Initialize(fn func(map[string]interface{})) {
	initializeHandler.Store(fn)
}

func (plugin *NylonPlugin) Shutdown(fn func()) {
	shutdownHandler.Store(fn)
}

//export shutdown
func shutdown() {
	if handler := shutdownHandler.Load(); handler != nil {
		if fn, ok := handler.(func()); ok {
			fn()
		}
	}
}

//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

//export close_session_stream
func close_session_stream(sessionID C.uint32_t) {
	sid := int(sessionID)
	streamSessions.Delete(sid)
}

//export initialize
func initialize(config *C.char, length C.int) {
	if pluginInstance != nil {
		configBytes := C.GoBytes(unsafe.Pointer(config), C.int(length))
		if fn, ok := initializeHandler.Load().(func(map[string]interface{})); ok {
			var configMap map[string]interface{}
			json.Unmarshal(configBytes, &configMap)
			fn(configMap)
		}

		phaseHandlerMap.Range(func(key, _ interface{}) bool {
			fmt.Println("[NylonPlugin] Added phase handler:", key)
			return true
		})
	} else {
		fmt.Println("[NylonPlugin] Plugin instance not found")
	}
}

//export register_session_stream
func register_session_stream(sessionID C.uint32_t, entry *C.char, length C.uint32_t, cb C.data_event_fn) bool {
	entryName := C.GoStringN(entry, C.int(length))

	handlerValue, exists := phaseHandlerMap.Load(entryName)
	if !exists {
		fmt.Printf("[NylonPlugin] No handler registered for entry=%s\n", entryName)
		return false
	}

	handler, ok := handlerValue.(func(phase *PhaseHandler))
	if !ok {
		fmt.Printf("[NylonPlugin] Invalid handler type for entry=%s\n", entryName)
		return false
	}

	sid := int32(sessionID)
	http_ctx := &NylonHttpPluginCtx{
		sessionID: sid,
		dataMap:   make(map[uint32][]byte, 8),
	}
	http_ctx.cond = sync.NewCond(&http_ctx.mu)
	phase := &PhaseHandler{
		SessionId: sid,
		cb:        cb,
		http_ctx:  http_ctx,
		requestFilter: func(ctx *PhaseRequestFilter) {
			ctx.Next()
		},
		responseFilter: func(ctx *PhaseResponseFilter) {
			ctx.Next()
		},
		responseBodyFilter: func(ctx *PhaseResponseBodyFilter) {
			ctx.Next()
		},
		logging: func(ctx *PhaseLogging) {
			ctx.Next()
		},
	}
	handler(phase)
	streamSessions.Store(sid, phase)
	return true
}

//export event_stream
func event_stream(ffiBuffer *C.FfiBuffer) {
	sid := int32(ffiBuffer.sid)
	phase, exists := streamSessions.Load(sid)
	if !exists {
		return
	}
	phaseHandler, ok := phase.(*PhaseHandler)
	if !ok {
		return
	}
	switch ffiBuffer.phase {
	case 1:
		// Use worker pool to reduce goroutine spawning overhead
		_ = GetDefaultWorkerPool().Submit(func() {
			phaseHandler.requestFilter(&PhaseRequestFilter{
				ctx: phaseHandler.http_ctx,
			})
		})
	case 2:
		_ = GetDefaultWorkerPool().Submit(func() {
			phaseHandler.responseFilter(&PhaseResponseFilter{
				ctx: phaseHandler.http_ctx,
			})
		})
	case 3:
		_ = GetDefaultWorkerPool().Submit(func() {
			phaseHandler.responseBodyFilter(&PhaseResponseBodyFilter{
				ctx: phaseHandler.http_ctx,
			})
		})
	case 4:
		_ = GetDefaultWorkerPool().Submit(func() {
			phaseHandler.logging(&PhaseLogging{
				ctx: phaseHandler.http_ctx,
			})
		})
	default:
		ctx := phaseHandler.http_ctx
		ctx.mu.Lock()
		defer ctx.mu.Unlock()
		length := int(ffiBuffer.len)
		method := uint32(ffiBuffer.method)
		data := ffiBuffer.ptr

		// WebSocket event dispatch (handle these even with length == 0)
		switch method {
		case MethodIDMapping[NylonMethodWebSocketOnOpen]:
			ctx.wsUpgraded = true
			if ctx.wsCallbacks != nil && ctx.wsCallbacks.OnOpen != nil {
				_ = GetDefaultWorkerPool().Submit(func() {
					ctx.wsCallbacks.OnOpen(&WebSocketConn{ctx: ctx})
				})
			}
			return
		case MethodIDMapping[NylonMethodWebSocketOnClose]:
			if ctx.wsCallbacks != nil && ctx.wsCallbacks.OnClose != nil {
				_ = GetDefaultWorkerPool().Submit(func() {
					ctx.wsCallbacks.OnClose(&WebSocketConn{ctx: ctx})
				})
			}
			return
		case MethodIDMapping[NylonMethodWebSocketOnError]:
			msg := C.GoStringN((*C.char)(unsafe.Pointer(data)), C.int(length))
			if ctx.wsCallbacks != nil && ctx.wsCallbacks.OnError != nil {
				msgCopy := msg // Capture for closure
				_ = GetDefaultWorkerPool().Submit(func() {
					ctx.wsCallbacks.OnError(&WebSocketConn{ctx: ctx}, msgCopy)
				})
			}
			return
		case MethodIDMapping[NylonMethodWebSocketOnMessageText]:
			msg := C.GoStringN((*C.char)(unsafe.Pointer(data)), C.int(length))
			if ctx.wsCallbacks != nil && ctx.wsCallbacks.OnMessageText != nil {
				msgCopy := msg // Capture for closure
				_ = GetDefaultWorkerPool().Submit(func() {
					ctx.wsCallbacks.OnMessageText(&WebSocketConn{ctx: ctx}, msgCopy)
				})
			}
			return
		case MethodIDMapping[NylonMethodWebSocketOnMessageBinary]:
			buf := ctx.dataMap[method]
			if cap(buf) < length {
				buf = make([]byte, length)
			} else {
				buf = buf[:length]
			}
			copy(buf, (*[1 << 30]byte)(unsafe.Pointer(data))[:length:length])
			if ctx.wsCallbacks != nil && ctx.wsCallbacks.OnMessageBinary != nil {
				dataCopy := make([]byte, length)
				copy(dataCopy, buf)
				_ = GetDefaultWorkerPool().Submit(func() {
					ctx.wsCallbacks.OnMessageBinary(&WebSocketConn{ctx: ctx}, dataCopy)
				})
			}
			return
		default:
			// For non-WebSocket events, check length and handle normally
			if length == 0 {
				ctx.dataMap[method] = []byte{}
				ctx.cond.Broadcast()
				return
			}
		}

		// default behavior: store data and wake waiter
		var dataBytes []byte
		if length > 0 {
			dataBytes = make([]byte, length)
			copy(dataBytes, (*[1 << 30]byte)(unsafe.Pointer(data))[:length:length])
		}
		ctx.dataMap[method] = dataBytes
		ctx.cond.Broadcast()
	}
}

func RequestMethod(sessionID int32, phase int8, method NylonMethods, data []byte) error {
	phaseSession, exists := streamSessions.Load(sessionID)
	if !exists {
		return fmt.Errorf("session %d not open", sessionID)
	}
	phaseHandler, ok := phaseSession.(*PhaseHandler)
	if !ok {
		return fmt.Errorf("invalid callback type for session %d", sessionID)
	}
	cb := phaseHandler.cb
	var dataPtr *C.uchar
	var poolSize int
	dataLen := len(data)
	if dataLen > 0 {
		// Use memory pool to reduce allocation overhead
		dataPtr, poolSize = GetBuffer(data)
		if dataPtr == nil {
			return fmt.Errorf("failed to allocate memory for data")
		}
	}
	methodID := MethodIDMapping[method]
	C.call_event_method(
		cb,
		&C.FfiBuffer{
			sid:    C.uint32_t(sessionID),
			phase:  C.uint8_t(phase),
			method: C.uint32_t(methodID),
			ptr:    dataPtr,
			len:    C.uint64_t(dataLen),
		},
	)
	// Note: Buffer will be freed by Rust side via plugin_free, which we'll update
	_ = poolSize // Kept for future pool management
	return nil
}

func (ctx *NylonHttpPluginCtx) GetPayload() map[string]any {
	data := ctx.requestAndWait(NylonMethodGetPayload, nil)
	if len(data) == 0 {
		return nil
	}
	var payloadMap map[string]any
	json.Unmarshal(data, &payloadMap)
	return payloadMap
}

func (ctx *NylonHttpPluginCtx) Next() {
	go RequestMethod(ctx.sessionID, 0, NylonMethodNext, nil)
}

func (ctx *NylonHttpPluginCtx) End() {
	go RequestMethod(ctx.sessionID, 0, NylonMethodEnd, nil)
}

type PhaseHandler struct {
	SessionId          int32
	cb                 C.data_event_fn
	http_ctx           *NylonHttpPluginCtx
	requestFilter      func(ctx *PhaseRequestFilter)
	responseFilter     func(ctx *PhaseResponseFilter)
	responseBodyFilter func(ctx *PhaseResponseBodyFilter)
	logging            func(ctx *PhaseLogging)
}

func (p *NylonPlugin) AddPhaseHandler(phaseName string, phaseHandler func(phase *PhaseHandler)) {
	phaseHandlerMap.Store(phaseName, phaseHandler)
}

func (p *PhaseHandler) RequestFilter(phaseRequestFilter func(requestFilter *PhaseRequestFilter)) {
	p.requestFilter = phaseRequestFilter
}

func (p *PhaseHandler) ResponseFilter(phaseResponseFilter func(responseFilter *PhaseResponseFilter)) {
	p.responseFilter = phaseResponseFilter
}

func (p *PhaseHandler) ResponseBodyFilter(phaseResponseBodyFilter func(responseBodyFilter *PhaseResponseBodyFilter)) {
	p.responseBodyFilter = phaseResponseBodyFilter
}

func (p *PhaseHandler) Logging(phaseLogging func(logging *PhaseLogging)) {
	p.logging = phaseLogging
}
