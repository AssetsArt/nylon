package sdk

/*
#include "../../../c/nylon.h"
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"sync"
	"unsafe"
)

// Import constants from constants.go

// Import types from types.go

// ====================
// Session State Management
// ====================

var (
	// sessionCallbacks maps sessionID to FFI callback functions
	sessionCallbacks = make(map[int]C.data_event_fn)

	// streamSessions maps sessionID to session context
	streamSessions = make(map[int]*NylonHttpPluginCtx)

	// sessionIsOpen tracks whether a session is currently open
	sessionIsOpen = make(map[int]bool)

	// handlerMap maps entry names to Go-side handlers
	handlerMap = make(map[string]HttpPluginFunc)

	// Global mutexes for thread safety
	sessionMu sync.Mutex
	handlerMu sync.Mutex

	// shutdownHandler stores the function to be called during shutdown
	shutdownHandler func() = nil
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
	handlerMu.Lock()
	defer handlerMu.Unlock()
	shutdownHandler = fn
}

// AddRequestFilter registers a Go handler function for a specific entry point.
// The handler will be called whenever a request matches the given entry name.
// If a handler is already registered for the entry, a warning is logged and
// the registration is skipped.
func (plugin *NylonPlugin) AddRequestFilter(entry string, handler func(ctx *PhaseRequestFilter)) {
	handlerMu.Lock()
	defer handlerMu.Unlock()

	_, exists := handlerMap[entry]
	if exists {
		fmt.Printf("[NylonPlugin] HttpPlugin already registered for entry=%s\n", entry)
		return
	}

	handlerMap[entry] = func(ctx *NylonHttpPluginCtx) {
		handler(ctx.RequestFilter())
	}
}

// ====================
// FFI Exported Functions
// ====================

// shutdown is called by the Rust backend when the plugin is being shut down.
// It invokes the registered shutdown handler if one exists.
//
//export shutdown
func shutdown() {
	if shutdownHandler != nil {
		shutdownHandler()
	}
}

// plugin_free is called by the Rust backend to free memory allocated by the plugin.
//
//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

// close_session_stream is called by the Rust backend when a session is being closed.
// It cleans up all session-related data structures.
//
//export close_session_stream
func close_session_stream(sessionID C.int) {
	sessionMu.Lock()
	delete(sessionCallbacks, int(sessionID))
	delete(streamSessions, int(sessionID))
	delete(sessionIsOpen, int(sessionID))
	sessionMu.Unlock()

	fmt.Printf("[NylonPlugin] Closed session %d\n", sessionID)
}

// register_session_stream is called by the Rust backend when a new session is being created.
// It looks up the appropriate handler for the entry point and creates a new session context.
// Returns true if the session was successfully registered, false otherwise.
//
//export register_session_stream
func register_session_stream(sessionID C.int, entry *C.char, length C.int, cb C.data_event_fn) bool {
	entryName := C.GoStringN(entry, length)

	// Lookup Go handler for the entry point
	handlerMu.Lock()
	handler, exists := handlerMap[entryName]
	handlerMu.Unlock()

	if !exists {
		fmt.Printf("[NylonPlugin] No handler registered for entry=%s\n", entryName)
		return false
	}

	// Store FFI callback for this session
	sessionMu.Lock()
	sessionCallbacks[int(sessionID)] = cb

	// Create or retrieve session context
	ctx, exists := streamSessions[int(sessionID)]
	if !exists {
		ctx = &NylonHttpPluginCtx{
			sessionID: int(sessionID),
			dataMap:   make(map[uint32][]byte),
		}
		ctx.cond = sync.NewCond(&ctx.mu)
		streamSessions[int(sessionID)] = ctx
	}
	sessionIsOpen[int(sessionID)] = true
	sessionMu.Unlock()

	// Invoke Go handler in a new goroutine
	go handler(ctx)
	return true
}

// event_stream is called by the Rust backend to send data to a specific session.
// It stores the received data in the session's data map and notifies waiting goroutines.
//
//export event_stream
func event_stream(sessionID C.int, method C.uint32_t, data *C.char, length C.int) {
	sessionMu.Lock()
	ctx, exists := streamSessions[int(sessionID)]
	sessionMu.Unlock()

	if !exists {
		return
	}

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Special case: notify without data (used for signaling)
	if length == 0 {
		ctx.cond.Broadcast()
		return
	}

	// Store payload in session's data map
	dataBytes := C.GoBytes(unsafe.Pointer(data), length)
	ctx.dataMap[uint32(method)] = dataBytes

	// Notify waiting goroutines that data is available
	ctx.cond.Broadcast()
}

// ====================
// NylonPluginCtx Methods
// ====================

// RequestMethod calls into Rust using the FFI callback to execute a specific method.
// It handles the conversion of Go data to C-compatible format and manages the FFI call.
func RequestMethod(sessionID int, method NylonMethods, data []byte) error {
	sessionMu.Lock()
	cb := sessionCallbacks[int(sessionID)]
	sessionMu.Unlock()

	if cb == nil {
		return fmt.Errorf("session %d not open", sessionID)
	}

	var dataPtr *C.char
	dataLen := len(data)
	if dataLen > 0 {
		dataPtr = (*C.char)(unsafe.Pointer(&data[0]))
	}

	methodID := MethodIDMapping[method]
	C.call_event_method(
		cb,
		C.size_t(sessionID),
		C.uint32_t(methodID),
		dataPtr,
		C.size_t(dataLen),
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

	return payloadMap
}

// RequestFilter creates and returns a PhaseRequestFilter instance that provides
// access to both request and response objects for the current session.
func (ctx *NylonHttpPluginCtx) RequestFilter() *PhaseRequestFilter {
	return &PhaseRequestFilter{
		ctx: ctx,
	}
}
