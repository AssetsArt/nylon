package sdk

import (
	"context"
	"time"
)

// OptimizedHttpPluginCtx uses channels instead of mutex+cond for better performance
type OptimizedHttpPluginCtx struct {
	sessionID   int32
	responseMap map[uint32]chan []byte // method -> response channel
	wsUpgraded  bool
	wsCallbacks *WebSocketCallbacks
}

// NewOptimizedHttpPluginCtx creates a new optimized context
func NewOptimizedHttpPluginCtx(sessionID int32) *OptimizedHttpPluginCtx {
	return &OptimizedHttpPluginCtx{
		sessionID:   sessionID,
		responseMap: make(map[uint32]chan []byte),
	}
}

// requestAndWaitOptimized uses channels for better performance
func (ctx *OptimizedHttpPluginCtx) requestAndWaitOptimized(method NylonMethods, payload []byte, timeout time.Duration) ([]byte, error) {
	methodID := MethodIDMapping[method]

	// Create response channel for this request
	respCh := make(chan []byte, 1)
	ctx.responseMap[methodID] = respCh

	// Send request
	if err := RequestMethod(ctx.sessionID, 0, method, payload); err != nil {
		delete(ctx.responseMap, methodID)
		close(respCh)
		return nil, err
	}

	// Wait for response with timeout
	if timeout > 0 {
		timeoutCtx, cancel := context.WithTimeout(context.Background(), timeout)
		defer cancel()

		select {
		case data := <-respCh:
			delete(ctx.responseMap, methodID)
			return data, nil
		case <-timeoutCtx.Done():
			delete(ctx.responseMap, methodID)
			return nil, context.DeadlineExceeded
		}
	} else {
		// No timeout
		data := <-respCh
		delete(ctx.responseMap, methodID)
		return data, nil
	}
}

// HandleResponse handles incoming response (called from event_stream)
func (ctx *OptimizedHttpPluginCtx) HandleResponse(methodID uint32, data []byte) {
	if ch, ok := ctx.responseMap[methodID]; ok {
		select {
		case ch <- data:
		default:
			// Channel full or closed, drop data
		}
	}
}
