//go:build cgo

package sdk

/*
#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

// Zero-copy data structure
typedef struct {
    uint32_t sid;
    uint8_t phase;
    uint32_t method;
    const unsigned char *ptr;
    uint64_t len;
    uint64_t capacity;
} FfiBuffer;

// Event callback with optimized signature
typedef void (*data_event_fn)(const FfiBuffer* ffiBuffer);

// Inline wrapper for minimal overhead
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
	// Lock-free session management using atomic operations
	sessionCallbacks = sync.Map{}
	streamSessions   = sync.Map{}
	sessionIsOpen    = sync.Map{}

	// Lock-free handler map
	handlerMap = sync.Map{}

	// Atomic shutdown handler
	shutdownHandler atomic.Value
	// Atomic initialize handler
	initializeHandler atomic.Value

	// Pre-allocated buffer pool for zero-copy operations
	bufferPool = sync.Pool{
		New: func() interface{} {
			return make([]byte, 0, 1024) // Pre-allocate with reasonable capacity
		},
	}

	// phase handler map
	phaseHandlerMap = sync.Map{}

	// plugin instance
	pluginInstance *NylonPlugin
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
	sessionCallbacks.Delete(sid)
	streamSessions.Delete(sid)
	sessionIsOpen.Delete(sid)
}

//export initialize
func initialize(config *C.char, length C.int) {
	// fmt.Println("[NylonPlugin] Plugin initialized")
	if pluginInstance != nil {
		configBytes := C.GoBytes(unsafe.Pointer(config), C.int(length))
		if fn, ok := initializeHandler.Load().(func(map[string]interface{})); ok {
			var configMap map[string]interface{}
			json.Unmarshal(configBytes, &configMap)
			fn(configMap)
		}

		phaseHandlerMap.Range(func(key, _ interface{}) bool {
			fmt.Println("[NylonPlugin] Phase handler", key)
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

	phase := &PhaseHandler{
		SessionId: int32(sessionID),
		cb:        cb,
	}

	handler(phase)
	streamSessions.Store(int32(sessionID), phase)
	return true
}

//export event_stream
func event_stream(ffiBuffer *C.FfiBuffer) {
	sid := int32(ffiBuffer.sid)
	// fmt.Println("[NylonPlugin] Event stream", sid)
	// Lock-free session lookup
	phase, exists := streamSessions.Load(sid)
	if !exists {
		// fmt.Println("[NylonPlugin] Phase not found")
		return
	}
	// fmt.Println("[NylonPlugin] Phase", phase)
	// cast phase to *PhaseHandler
	phaseHandler, ok := phase.(*PhaseHandler)
	if !ok {
		// fmt.Println("[NylonPlugin] Phase handler not found")
		return
	}
	// fmt.Println("[NylonPlugin] Phase handler", phaseHandler)
	if ffiBuffer.phase == 1 {
		phaseHandler.requestFilter(&PhaseRequestFilter{})
	}
}

func RequestMethod(sessionID int, phase int8, method NylonMethods, data []byte) error {
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
		&C.FfiBuffer{
			sid:      C.uint32_t(sessionID),
			phase:    C.uint8_t(phase),
			method:   C.uint32_t(methodID),
			ptr:      dataPtr,
			len:      C.uint64_t(dataLen),
			capacity: C.uint64_t(dataLen),
		},
	)
	return nil
}

func (ctx *NylonHttpPluginCtx) GetPayload(phase int8) map[string]any {
	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	RequestMethod(ctx.sessionID, phase, NylonMethodGetPayload, nil)

	// Wait for response
	ctx.cond.Wait()
	payload, exists := ctx.dataMap[MethodIDMapping[NylonMethodGetPayload]]
	if !exists {
		return nil
	}
	fmt.Println("[NylonPlugin] Payload", string(payload))
	return nil
}

type PhaseHandler struct {
	SessionId     int32
	cb            C.data_event_fn
	requestFilter func(ctx *PhaseRequestFilter)
}

func (p *NylonPlugin) AddPhaseHandler(phaseName string, phaseHandler func(phase *PhaseHandler)) {
	phaseHandlerMap.Store(phaseName, phaseHandler)
}

// PhaseHandler
func (p *PhaseHandler) RequestFilter(phaseRequestFilter func(requestFilter *PhaseRequestFilter)) {
	p.requestFilter = phaseRequestFilter
}
