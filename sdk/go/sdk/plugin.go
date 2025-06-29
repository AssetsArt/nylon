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

// ====================
// Nylon Method Types
// ====================
type NylonMethods string

const (
	NylonMethodNext       NylonMethods = "next"
	NylonMethodEnd        NylonMethods = "end"
	NylonMethodGetPayload NylonMethods = "get_payload"

	// response
	NylonMethodSetResponseHeader    NylonMethods = "set_response_header"
	NylonMethodRemoveResponseHeader NylonMethods = "remove_response_header"
	NylonMethodSetResponseStatus    NylonMethods = "set_response_status"
)

// Mapping of NylonMethods to IDs used in FFI
var mapMethod = map[NylonMethods]uint32{
	NylonMethodNext:       1,
	NylonMethodEnd:        2,
	NylonMethodGetPayload: 3,

	// response
	NylonMethodSetResponseHeader:    100,
	NylonMethodRemoveResponseHeader: 101,
	NylonMethodSetResponseStatus:    102,
}

// ====================
// NylonPlugin Core Types
// ====================

// User-defined request handler
type HttpPluginFunc func(ctx *NylonHttpPluginCtx)

// NylonPlugin represents the plugin itself
type NylonPlugin struct{}

// NylonPluginCtx represents a per-session context
type NylonHttpPluginCtx struct {
	sessionID uint32

	mu      sync.Mutex
	cond    *sync.Cond
	dataMap map[uint32][]byte
}

// ====================
// Session State
// ====================

var (
	// sessionID -> FFI callback
	sessionCallbacks = make(map[uint32]C.data_event_fn)

	// sessionID -> session context
	streamSessions = make(map[uint32]*NylonHttpPluginCtx)

	// sessionID -> true if open
	sessionIsOpen = make(map[uint32]bool)

	// name -> Go-side handler
	handlerMap = make(map[string]HttpPluginFunc)

	// Global mutexes
	sessionMu sync.Mutex
	handlerMu sync.Mutex
)

// ====================
// NylonPlugin API
// ====================

// NewNylonPlugin creates a new NylonPlugin
func NewNylonPlugin() *NylonPlugin {
	return &NylonPlugin{}
}

// Register a Go handler for an entry name
func (plugin *NylonPlugin) HttpPlugin(entry string, handler HttpPluginFunc) {
	handlerMu.Lock()
	defer handlerMu.Unlock()
	// handlerMap[entry] = handler
	_, exists := handlerMap[entry]
	if exists {
		fmt.Printf("[NylonPlugin] HttpPlugin already registered for entry=%s\n", entry)
		return
	}
	handlerMap[entry] = handler
}

// ====================
// FFI Exported Functions
// ====================

//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

//export close_session_stream
func close_session_stream(sessionID C.uint32_t) {
	sessionMu.Lock()
	delete(sessionCallbacks, uint32(sessionID))
	delete(streamSessions, uint32(sessionID))
	delete(sessionIsOpen, uint32(sessionID))
	sessionMu.Unlock()

	fmt.Printf("[NylonPlugin] Closed session %d\n", sessionID)
}

//export register_session_stream
func register_session_stream(sessionID C.uint32_t, entry *C.char, length C.int32_t, cb C.data_event_fn) bool {
	entryName := C.GoStringN(entry, length)

	// Lookup Go handler
	handlerMu.Lock()
	handler, exists := handlerMap[entryName]
	handlerMu.Unlock()

	if !exists {
		fmt.Printf("[NylonPlugin] No handler registered for entry=%s\n", entryName)
		return false
	}

	// Store FFI callback
	sessionMu.Lock()
	sessionCallbacks[uint32(sessionID)] = cb

	// Create context if new
	ctx, exists := streamSessions[uint32(sessionID)]
	if !exists {
		ctx = &NylonHttpPluginCtx{
			sessionID: uint32(sessionID),
			dataMap:   make(map[uint32][]byte),
		}
		ctx.cond = sync.NewCond(&ctx.mu)
		streamSessions[uint32(sessionID)] = ctx
	}
	sessionIsOpen[uint32(sessionID)] = true
	sessionMu.Unlock()

	// Invoke Go handler
	go handler(ctx)
	return true
}

//export event_stream
func event_stream(sessionID C.uint32_t, method C.uint32_t, data *C.char, length C.int32_t) {
	sessionMu.Lock()
	ctx, exists := streamSessions[uint32(sessionID)]
	sessionMu.Unlock()

	if !exists {
		return
	}

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Special case: notify without data
	if length == 0 {
		ctx.cond.Broadcast()
		return
	}

	// Store payload
	dataBytes := C.GoBytes(unsafe.Pointer(data), length)
	ctx.dataMap[uint32(method)] = dataBytes
	ctx.cond.Broadcast()
}

// ====================
// NylonPluginCtx Methods
// ====================

// RequestMethod calls into Rust using the FFI callback
func RequestMethod(sessionID uint32, method NylonMethods, data []byte) error {
	sessionMu.Lock()
	cb := sessionCallbacks[sessionID]
	sessionMu.Unlock()

	if cb == nil {
		return fmt.Errorf("session %d not open", sessionID)
	}

	var dataPtr *C.char
	dataLen := len(data)
	if dataLen > 0 {
		dataPtr = (*C.char)(unsafe.Pointer(&data[0]))
	}

	methodID := mapMethod[method]
	C.call_event_method(
		cb,
		C.uint32_t(sessionID),
		C.uint32_t(methodID),
		dataPtr,
		C.int32_t(dataLen),
	)
	return nil
}

// Next sends a 'next' request to Rust
func (ctx *NylonHttpPluginCtx) Next() {
	RequestMethod(ctx.sessionID, NylonMethodNext, nil)
}

// End sends a 'end' request to Rust
func (ctx *NylonHttpPluginCtx) End() {
	RequestMethod(ctx.sessionID, NylonMethodEnd, nil)
}

// GetPayload requests and waits for payload from Rust
func (ctx *NylonHttpPluginCtx) GetPayload() map[string]any {
	methodID := mapMethod[NylonMethodGetPayload]

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	// Check if data is already available
	payload, exists := ctx.dataMap[methodID]
	if !exists {
		// Ask Rust to send payload
		RequestMethod(ctx.sessionID, NylonMethodGetPayload, nil)

		// Wait for response
		ctx.cond.Wait()
		payload, exists = ctx.dataMap[methodID]
		if !exists {
			return nil
		}
	}

	// Decode JSON
	var payloadMap map[string]any
	if err := json.Unmarshal(payload, &payloadMap); err != nil {
		fmt.Println("[NylonPlugin] JSON unmarshal error:", err)
		return nil
	}

	return payloadMap
}

func (ctx *NylonHttpPluginCtx) Response() *Response {
	return &Response{
		_ctx: ctx,
	}
}
