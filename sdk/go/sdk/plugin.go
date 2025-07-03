package sdk

/*
#include "../../../c/nylon.h"
#include <string.h>
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"sync"
	"sync/atomic"
	"unsafe"
)

// Import constants from constants.go

// Import types from types.go

// ====================
// Ultra Low Latency Session State Management
// ====================

var (
	// Lock-free session management using atomic operations
	sessionCallbacks = sync.Map{}
	streamSessions   = sync.Map{}
	sessionIsOpen    = sync.Map{}

	// Lock-free handler map
	handlerMap = sync.Map{}

	// Atomic shutdown handler
	shutdownHandler atomic.Value

	// Pre-allocated buffer pool for zero-copy operations
	bufferPool = sync.Pool{
		New: func() interface{} {
			return make([]byte, 0, 1024) // Pre-allocate with reasonable capacity
		},
	}
)

// ====================
// NylonPlugin API
// ====================

// NewNylonPlugin creates a new NylonPlugin instance
func NewNylonPlugin() *NylonPlugin {
	return &NylonPlugin{}
}

// Shutdown registers a function to be called when the plugin is shutting down.
// This is useful for cleanup operations like closing database connections or
// saving state before the plugin terminates.
func (plugin *NylonPlugin) Shutdown(fn func()) {
	shutdownHandler.Store(fn)
}

// AddRequestFilter registers a Go handler function for a specific entry point.
// The handler will be called whenever a request matches the given entry name.
// If a handler is already registered for the entry, a warning is logged and
// the registration is skipped.
func (plugin *NylonPlugin) AddRequestFilter(entry string, phasehandler func(ctx *PhaseRequestFilter)) {
	handler := func(ctx *NylonHttpPluginCtx) {
		phasehandler(ctx.RequestFilter())
	}
	if _, loaded := handlerMap.LoadOrStore(entry, handler); loaded {
		fmt.Printf("[NylonPlugin] HttpPlugin already registered for entry=%s\n", entry)
	}
}

// ====================
// Ultra Low Latency FFI Exported Functions
// ====================

// shutdown is called by the Rust backend when the plugin is being shut down.
// It invokes the registered shutdown handler if one exists.
//
//export shutdown
func shutdown() {
	if handler := shutdownHandler.Load(); handler != nil {
		if fn, ok := handler.(func()); ok {
			fn()
		}
	}
}

// plugin_free is called by the Rust backend to free memory allocated by the plugin.
//
//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

// close_session_stream is called by the Rust backend when a session is being closed.
// It cleans up all session-related data structures using lock-free operations.
//
//export close_session_stream
func close_session_stream(sessionID C.uint32_t) {
	sid := int(sessionID)
	sessionCallbacks.Delete(sid)
	streamSessions.Delete(sid)
	sessionIsOpen.Delete(sid)

	fmt.Printf("[NylonPlugin] Closed session %d\n", sessionID)
}

// register_session_stream is called by the Rust backend when a new session is being created.
// It looks up the appropriate handler for the entry point and creates a new session context.
// Returns true if the session was successfully registered, false otherwise.
//
//export register_session_stream
func register_session_stream(sessionID C.int, entry *C.char, length C.int, cb C.data_event_fn) bool {
	entryName := C.GoStringN(entry, length)

	// Lock-free handler lookup
	handlerValue, exists := handlerMap.Load(entryName)
	if !exists {
		fmt.Printf("[NylonPlugin] No handler registered for entry=%s\n", entryName)
		return false
	}

	handler, ok := handlerValue.(func(ctx *NylonHttpPluginCtx))
	if !ok {
		fmt.Printf("[NylonPlugin] Invalid handler type for entry=%s\n", entryName)
		return false
	}

	sid := int(sessionID)

	// Lock-free session registration
	sessionCallbacks.Store(sid, cb)

	// Create session context with pre-allocated structures
	ctx := &NylonHttpPluginCtx{
		sessionID: sid,
		dataMap:   make(map[uint32][]byte, 8), // Pre-allocate with expected capacity
	}
	ctx.cond = sync.NewCond(&ctx.mu)
	streamSessions.Store(sid, ctx)
	sessionIsOpen.Store(sid, true)

	// Invoke Go handler in a new goroutine
	go handler(ctx)
	return true
}

// event_stream is called by the Rust backend to send data to a specific session.
// It stores the received data in the session's data map and notifies waiting goroutines.
// Optimized for minimal latency with zero-copy operations where possible.
//
//export event_stream
func event_stream(sessionID C.int, method C.uint32_t, data *C.char, length C.int) {
	sid := int(sessionID)

	// Lock-free session lookup
	ctxValue, exists := streamSessions.Load(sid)
	if !exists {
		return
	}

	ctx, ok := ctxValue.(*NylonHttpPluginCtx)
	if !ok {
		return
	}

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Special case: notify without data (used for signaling)
	if length == 0 {
		ctx.cond.Broadcast()
		return
	}

	// Use buffer pool for zero-allocation data handling
	var dataBytes []byte
	if length > 0 {
		// Get buffer from pool or create new one
		if pooled := bufferPool.Get(); pooled != nil {
			dataBytes = pooled.([]byte)
			if cap(dataBytes) < int(length) {
				dataBytes = make([]byte, length)
			} else {
				dataBytes = dataBytes[:length]
			}
		} else {
			dataBytes = make([]byte, length)
		}

		// Copy data efficiently using Go's copy function
		copy(dataBytes, (*[1 << 30]byte)(unsafe.Pointer(data))[:length:length])
	}

	// Store payload in session's data map
	ctx.dataMap[uint32(method)] = dataBytes

	// Notify waiting goroutines that data is available
	ctx.cond.Broadcast()
}

// ====================
// Ultra Low Latency NylonPluginCtx Methods
// ====================

// RequestMethod calls into Rust using the FFI callback to execute a specific method.
// It handles the conversion of Go data to C-compatible format and manages the FFI call.
// Optimized for minimal latency with zero-copy operations where possible.
func RequestMethod(sessionID int, method NylonMethods, data []byte) error {
	// Lock-free callback lookup
	cbValue, exists := sessionCallbacks.Load(sessionID)
	if !exists {
		return fmt.Errorf("session %d not open", sessionID)
	}

	cb, ok := cbValue.(C.data_event_fn)
	if !ok {
		return fmt.Errorf("invalid callback type for session %d", sessionID)
	}

	var dataPtr *C.uchar
	dataLen := len(data)
	if dataLen > 0 {
		dataPtr = (*C.uchar)(C.malloc(C.size_t(dataLen)))
		if dataPtr == nil {
			return fmt.Errorf("failed to allocate memory for data")
		}
		C.memcpy(unsafe.Pointer(dataPtr), unsafe.Pointer(&data[0]), C.size_t(dataLen))
		defer C.free(unsafe.Pointer(dataPtr))
	}

	methodID := MethodIDMapping[method]
	C.call_event_method(
		cb,
		C.uint32_t(sessionID),
		C.uint32_t(methodID),
		&C.FfiBuffer{
			ptr:      dataPtr,
			len:      C.uint32_t(dataLen),
			capacity: C.uint32_t(dataLen),
		},
	)
	return nil
}

// Next sends a 'next' request to Rust, indicating that the plugin should continue
// processing the request pipeline.
func (ctx *NylonHttpPluginCtx) Next() {
	RequestMethod(ctx.sessionID, NylonMethodNext, nil)
}

// End sends an 'end' request to Rust, indicating that the plugin has finished
// processing and the request should be terminated.
func (ctx *NylonHttpPluginCtx) End() {
	RequestMethod(ctx.sessionID, NylonMethodEnd, nil)
}

// GetPayload requests and waits for payload data from Rust.
// It blocks until the payload is received and then decodes it as JSON.
// Returns nil if no payload is available or if JSON decoding fails.
// Optimized for minimal latency with buffer pooling.
func (ctx *NylonHttpPluginCtx) GetPayload() map[string]any {
	methodID := MethodIDMapping[NylonMethodGetPayload]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Request payload from Rust
	RequestMethod(ctx.sessionID, NylonMethodGetPayload, nil)

	// Wait for response
	ctx.cond.Wait()
	payload, exists := ctx.dataMap[methodID]
	if !exists {
		return nil
	}

	// Decode JSON payload
	var payloadMap map[string]any
	if err := json.Unmarshal(payload, &payloadMap); err != nil {
		fmt.Println("[NylonPlugin] JSON unmarshal error:", err)
		return nil
	}

	// Return buffer to pool for reuse
	if len(payload) > 0 {
		// Reset slice but keep capacity for reuse
		payload = payload[:0]
		bufferPool.Put(payload)
	}

	return payloadMap
}

// RequestFilter creates and returns a PhaseRequestFilter instance that provides
// access to both request and response objects for the current session.
func (ctx *NylonHttpPluginCtx) RequestFilter() *PhaseRequestFilter {
	return &PhaseRequestFilter{
		ctx: ctx,
	}
}
