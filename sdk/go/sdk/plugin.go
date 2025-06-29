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

type NylonMethods string

const (
	NylonMethodNext       NylonMethods = "next"
	NylonMethodGetPayload NylonMethods = "get_payload"
)

var (
	// mutex for session
	sessionMu sync.Mutex
	// mutex for handler
	handlerMu sync.Mutex
	// name -> handler
	handlerMap = make(map[string]HandlerFunc)

	// sessionID -> callback
	sessionCallbacks = make(map[uint32]C.data_event_fn)
	// sessionID -> ctx
	streamSession = make(map[uint32]NylonPluginCtx)
	// sessionID -> true or false
	sessionIsOpen = make(map[uint32]bool)

	// mapMethod
	mapMethod = map[NylonMethods]uint32{
		NylonMethodNext:       1,
		NylonMethodGetPayload: 2,
	}
)

type HandlerFunc func(ctx *NylonPluginCtx)
type NylonPlugin struct{}
type NylonPluginCtx struct {
	sessionID uint32
	mu        sync.Mutex
	cond      *sync.Cond
	dataMap   map[uint32][]byte
}

//export close_session_stream
func close_session_stream(sessionID C.uint32_t) {
	sessionMu.Lock()
	delete(sessionCallbacks, uint32(sessionID))
	sessionMu.Unlock()

	sessionMu.Lock()
	delete(streamSession, uint32(sessionID))
	sessionMu.Unlock()

	fmt.Printf("[NylonPlugin] Closed session %d\n", sessionID)
}

//export register_session_stream
func register_session_stream(sessionID C.uint32_t, entry *C.char, length C.int32_t, cb C.data_event_fn) bool {
	// call handler
	entryName := C.GoStringN(entry, length)
	handlerMu.Lock()
	handler, handlerOk := handlerMap[entryName]
	handlerMu.Unlock()

	if !handlerOk {
		return false
	}

	// register callback
	sessionMu.Lock()
	sessionCallbacks[uint32(sessionID)] = cb
	sessionMu.Unlock()
	sessionMu.Lock()
	ctx, ok := streamSession[uint32(sessionID)]
	if !ok {
		ctx = NylonPluginCtx{
			sessionID: uint32(sessionID),
			// dataReady: make(map[uint32]bool),
			dataMap: make(map[uint32][]byte),
			mu:      sync.Mutex{},
			cond:    sync.NewCond(&ctx.mu),
		}
		streamSession[uint32(sessionID)] = ctx
	}
	sessionIsOpen[uint32(sessionID)] = true
	sessionMu.Unlock()

	go handler(&ctx)
	return true
}

//export event_stream
func event_stream(sessionID C.uint32_t, method C.uint32_t, data *C.char, length C.int32_t) {
	sessionMu.Lock()
	ctx, ok := streamSession[uint32(sessionID)]
	sessionMu.Unlock()
	if !ok {
		return
	}

	ctx.mu.Lock()
	defer ctx.mu.Unlock()

	dataBytes := C.GoBytes(unsafe.Pointer(data), length)
	ctx.dataMap[uint32(method)] = dataBytes
	ctx.cond.Broadcast() // notify waiters
}

// NewNylonPlugin
func NewNylonPlugin() *NylonPlugin {
	return &NylonPlugin{}
}

func (plugin *NylonPlugin) HandleRequest(entry string, handler HandlerFunc) {
	handlerMu.Lock()
	handlerMap[entry] = handler
	handlerMu.Unlock()
}

// NylonPluginCtx
func RequestMethod(sessionID uint32, method NylonMethods) (string, error) {
	sessionMu.Lock()
	cb := sessionCallbacks[sessionID]
	sessionMu.Unlock()
	if cb == nil {
		return "", fmt.Errorf("session %d not open", sessionID)
	}
	methodID := mapMethod[method]
	C.call_event_method(
		cb,
		C.uint32_t(sessionID),
		C.uint32_t(methodID),
		nil,
		0,
	)
	return "", nil
}

func (ctx *NylonPluginCtx) Next() {
	RequestMethod(ctx.sessionID, NylonMethodNext)
}

func (ctx *NylonPluginCtx) GetPayload() map[string]any {
	methodID := mapMethod[NylonMethodGetPayload]
	ctx.mu.Lock()
	payload, ok := ctx.dataMap[methodID]
	if !ok {
		RequestMethod(ctx.sessionID, NylonMethodGetPayload)
		ctx.cond.Wait()
		payload, ok = ctx.dataMap[methodID]
		if !ok {
			return nil
		}
	}
	ctx.mu.Unlock()
	var payloadMap map[string]any
	err := json.Unmarshal(payload, &payloadMap)
	if err != nil {
		fmt.Println("[NylonPlugin] JSON unmarshal error:", err)
		return nil
	}
	return payloadMap
}
