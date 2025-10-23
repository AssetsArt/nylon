package sdk

import (
	"encoding/json"
	"fmt"
	"sync"
	"sync/atomic"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/vmihailenco/msgpack/v5"
)

var natsSessions sync.Map

// NatsPlugin is a NATS-based plugin instance
type NatsPlugin struct {
	config          *NatsPluginConfig
	conn            *nats.Conn
	subscriptions   []*nats.Subscription
	phaseHandlers   sync.Map
	initHandler     atomic.Value
	shutdownHandler atomic.Value
	mu              sync.RWMutex
	started         bool
}

// NatsPluginConfig holds configuration for NATS plugin
type NatsPluginConfig struct {
	// Plugin name (required)
	Name string

	// NATS servers (required)
	Servers []string

	// Queue group name for load balancing (optional, default: "default")
	QueueGroup string

	// Subject prefix (optional, default: "nylon.plugin")
	SubjectPrefix string

	// NATS connection options
	NatsOptions []nats.Option

	// Worker concurrency (optional, default: 10)
	MaxWorkers int
}

// PluginRequest represents an incoming request from Nylon
type PluginRequest struct {
	Version   uint16            `msgpack:"version"`
	RequestID interface{}       `msgpack:"request_id"` // Can be string or u128
	SessionID uint32            `msgpack:"session_id"`
	Phase     uint8             `msgpack:"phase"`
	Method    uint32            `msgpack:"method"`
	Data      []byte            `msgpack:"data"`
	Timestamp uint64            `msgpack:"timestamp"`
	Headers   map[string]string `msgpack:"headers,omitempty"`
}

// PluginResponse represents a response to Nylon
type ResponseAction string

const (
	ResponseActionNext  ResponseAction = "next"
	ResponseActionEnd   ResponseAction = "end"
	ResponseActionError ResponseAction = "error"
)

type PluginResponse struct {
	Version   uint16            `msgpack:"version"`
	RequestID interface{}       `msgpack:"request_id"`
	SessionID uint32            `msgpack:"session_id"`
	Method    *uint32           `msgpack:"method,omitempty"`
	Action    ResponseAction    `msgpack:"action"`
	Data      []byte            `msgpack:"data"`
	Error     *string           `msgpack:"error,omitempty"`
	Headers   map[string]string `msgpack:"headers,omitempty"`
}

// NatsPhaseContext holds context for phase execution
type NatsPhaseContext struct {
	SessionID uint32
	Phase     uint8
	RequestID string
	conn      *nats.Conn
	natsCtx   *NylonHttpPluginCtx
}

const (
	ProtocolVersion = 1
)

// NewNylonNatsPlugin creates a new NATS-based plugin
func NewNylonNatsPlugin(config *NatsPluginConfig) (*NatsPlugin, error) {
	if config == nil {
		return nil, fmt.Errorf("config is required")
	}
	if config.Name == "" {
		return nil, fmt.Errorf("plugin name is required")
	}
	if len(config.Servers) == 0 {
		return nil, fmt.Errorf("at least one NATS server is required")
	}

	// Set defaults
	if config.QueueGroup == "" {
		config.QueueGroup = "default"
	}
	if config.SubjectPrefix == "" {
		config.SubjectPrefix = "nylon.plugin"
	}
	if config.MaxWorkers <= 0 {
		config.MaxWorkers = 10
	}

	return &NatsPlugin{
		config: config,
	}, nil
}

// Connect establishes connection to NATS server
func (p *NatsPlugin) Connect() error {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.conn != nil {
		return nil
	}

	opts := []nats.Option{
		nats.Name(p.config.Name),
		nats.MaxReconnects(-1), // Unlimited reconnects
		nats.ReconnectWait(1 * time.Second),
		nats.ReconnectBufSize(10 * 1024 * 1024), // 10MB buffer
	}

	// Append user-provided options
	opts = append(opts, p.config.NatsOptions...)

	// Connect to NATS
	conn, err := nats.Connect(
		p.config.Servers[0], // TODO: Support multiple servers
		opts...,
	)
	if err != nil {
		return fmt.Errorf("failed to connect to NATS: %w", err)
	}

	p.conn = conn
	fmt.Printf("[NatsPlugin] Connected to NATS: %s\n", p.config.Servers[0])

	return nil
}

// Initialize registers the initialize handler
func (p *NatsPlugin) Initialize(fn func(map[string]interface{})) {
	p.initHandler.Store(fn)
}

// Shutdown registers the shutdown handler
func (p *NatsPlugin) Shutdown(fn func()) {
	p.shutdownHandler.Store(fn)
}

// AddPhaseHandler registers a phase handler
func (p *NatsPlugin) AddPhaseHandler(phaseName string, handler func(phase *PhaseHandler)) {
	p.phaseHandlers.Store(phaseName, handler)
}

// Start begins listening for NATS messages
func (p *NatsPlugin) Start() error {
	// Check if already started
	p.mu.Lock()
	if p.started {
		p.mu.Unlock()
		return fmt.Errorf("plugin already started")
	}
	p.mu.Unlock()

	// Connect without holding the lock (Connect has its own lock)
	if p.conn == nil {
		if err := p.Connect(); err != nil {
			fmt.Printf("[NatsPlugin] Failed to connect to NATS: %v\n", err)
			return err
		}
	}
	fmt.Printf("[NatsPlugin] Connected to NATS: %s\n", p.config.Servers[0])

	// Lock again for subscription setup
	p.mu.Lock()

	// Subscribe to all phases with queue group
	phases := []string{"request_filter", "response_filter", "response_body_filter", "logging"}

	for _, phase := range phases {
		subject := fmt.Sprintf("%s.%s.%s", p.config.SubjectPrefix, p.config.Name, phase)

		sub, err := p.conn.QueueSubscribe(subject, p.config.QueueGroup, p.handleMessage)

		if err != nil {
			p.mu.Unlock()
			return fmt.Errorf("failed to subscribe to %s: %w", subject, err)
		}

		p.subscriptions = append(p.subscriptions, sub)
		fmt.Printf("[NatsPlugin] Subscribed to %s with queue group %s\n", subject, p.config.QueueGroup)
	}

	// Subscribe to lifecycle subject WITHOUT queue group so all workers receive it
	lifecycleSubject := fmt.Sprintf("%s.%s.lifecycle", p.config.SubjectPrefix, p.config.Name)
	lifecycleSub, err := p.conn.Subscribe(lifecycleSubject, p.handleMessage)
	if err != nil {
		p.mu.Unlock()
		return fmt.Errorf("failed to subscribe to %s: %w", lifecycleSubject, err)
	}
	p.subscriptions = append(p.subscriptions, lifecycleSub)
	fmt.Printf("[NatsPlugin] Subscribed to %s (broadcast)\n", lifecycleSubject)

	p.started = true
	p.mu.Unlock()

	fmt.Printf("[NatsPlugin] Plugin %s started successfully\n", p.config.Name)

	// Block forever (NATS runs in background)
	select {}
}

// handleMessage processes incoming NATS messages
func (p *NatsPlugin) handleMessage(msg *nats.Msg) {
	// Decode request
	var req PluginRequest
	if err := msgpack.Unmarshal(msg.Data, &req); err != nil {
		fmt.Printf("[NatsPlugin] Failed to decode request: %v\n", err)
		// Try to respond with error even if decode failed
		errStr := fmt.Sprintf("decode error: %v", err)
		errorResp := PluginResponse{
			Version: ProtocolVersion,
			Error:   &errStr,
		}
		if data, err := msgpack.Marshal(errorResp); err == nil {
			msg.Respond(data)
		}
		return
	}

	methodName := ""
	if name, ok := methodNameFromID(req.Method); ok {
		methodName = string(name)
	}

	fmt.Printf("[NatsPlugin] Received request: session=%d phase=%d method=%s headers=%+v\n",
		req.SessionID, req.Phase, methodName, req.Headers)

	// Handle special methods from headers
	if req.Headers != nil {
		if method, ok := req.Headers["method"]; ok {
			switch method {
			case "initialize":
				p.handleInitialize(msg, &req)
				return
			case "shutdown":
				p.handleShutdown(msg, &req)
				return
			}
		}
	}

	// Handle phase event
	switch req.Phase {
	case 0:
		if handled := p.handleDataEvent(msg, &req); !handled {
			p.respondError(msg, &req, nil, fmt.Sprintf("no active session for %d", req.SessionID))
		}

	case 1: // RequestFilter
		p.handleRequestFilterPhase(msg, &req)

	case 2: // ResponseFilter
		p.handleResponseFilterPhase(msg, &req)

	case 3: // ResponseBodyFilter
		p.handleResponseBodyFilterPhase(msg, &req)

	case 4: // Logging
		p.handleLoggingPhase(msg, &req)

	default:
		p.respondError(msg, &req, nil, fmt.Sprintf("unknown phase: %d", req.Phase))
	}
}

func (p *NatsPlugin) handleDataEvent(msg *nats.Msg, req *PluginRequest) bool {
	ctxValue, ok := natsSessions.Load(req.SessionID)
	if !ok {
		return false
	}

	natsCtx, ok := ctxValue.(*NylonHttpPluginCtx)
	if !ok {
		return false
	}

	methodID := req.Method

	natsCtx.mu.Lock()
	if natsCtx.dataMap == nil {
		natsCtx.dataMap = make(map[uint32][]byte)
	}
	natsCtx.lastRequestID = req.RequestID
	payload := make([]byte, len(req.Data))
	copy(payload, req.Data)
	natsCtx.dataMap[methodID] = payload
	natsCtx.cond.Broadcast()
	natsCtx.mu.Unlock()

	p.respondOK(msg, req, natsCtx)
	return true
}

// handleInitialize processes initialize request from Nylon
func (p *NatsPlugin) handleInitialize(msg *nats.Msg, req *PluginRequest) {
	fmt.Println("[NatsPlugin] Received Initialize request")

	// Call initialize handler if registered
	if handler := p.initHandler.Load(); handler != nil {
		if fn, ok := handler.(func(map[string]interface{})); ok {
			// Decode config from request data (sent as JSON bytes)
			var config map[string]interface{}
			if len(req.Data) > 0 {
				// Try JSON first (sent by Rust)
				if err := json.Unmarshal(req.Data, &config); err != nil {
					fmt.Printf("[NatsPlugin] Failed to decode config as JSON: %v\n", err)
					// Try MessagePack as fallback
					if err := msgpack.Unmarshal(req.Data, &config); err != nil {
						fmt.Printf("[NatsPlugin] Failed to decode config as MessagePack: %v\n", err)
						config = make(map[string]interface{})
					}
				}
			} else {
				config = make(map[string]interface{})
			}

			fmt.Println("[NatsPlugin] Calling initialize handler")
			fmt.Printf("[NatsPlugin] Config: %+v\n", config)
			fn(config)
		}
	}

	fmt.Println("[NatsPlugin] Sending OK response")
	p.respondOK(msg, req, nil)
}

// handleShutdown processes shutdown request from Nylon
func (p *NatsPlugin) handleShutdown(msg *nats.Msg, req *PluginRequest) {
	fmt.Println("[NatsPlugin] Received Shutdown request")

	// Call shutdown handler if registered
	if handler := p.shutdownHandler.Load(); handler != nil {
		if fn, ok := handler.(func()); ok {
			fmt.Println("[NatsPlugin] Calling shutdown handler")
			fn()
		}
	}

	p.respondOK(msg, req, nil)
}

// handleRequestFilterPhase handles RequestFilter phase
func (p *NatsPlugin) handleRequestFilterPhase(msg *nats.Msg, req *PluginRequest) {
	natsCtx, phaseHandler, entryName := p.setupPhaseHandler(msg, req)

	handlerFn, exists := p.phaseHandlers.Load(entryName)
	if !exists {
		p.respondError(msg, req, natsCtx, fmt.Sprintf("no handler for entry: %s", entryName))
		return
	}

	if fn, ok := handlerFn.(func(*PhaseHandler)); ok {
		fn(phaseHandler)
	}

	phaseHandler.requestFilter(&PhaseRequestFilter{ctx: natsCtx})

	p.respondOK(msg, req, natsCtx)
}

// handleResponseFilterPhase handles ResponseFilter phase
func (p *NatsPlugin) handleResponseFilterPhase(msg *nats.Msg, req *PluginRequest) {
	natsCtx, phaseHandler, entryName := p.setupPhaseHandler(msg, req)

	handlerFn, exists := p.phaseHandlers.Load(entryName)
	if !exists {
		p.respondError(msg, req, natsCtx, fmt.Sprintf("no handler for entry: %s", entryName))
		return
	}

	if fn, ok := handlerFn.(func(*PhaseHandler)); ok {
		fn(phaseHandler)
	}

	phaseHandler.responseFilter(&PhaseResponseFilter{ctx: natsCtx})

	p.respondOK(msg, req, natsCtx)
}

// handleResponseBodyFilterPhase handles ResponseBodyFilter phase
func (p *NatsPlugin) handleResponseBodyFilterPhase(msg *nats.Msg, req *PluginRequest) {
	natsCtx, phaseHandler, entryName := p.setupPhaseHandler(msg, req)

	handlerFn, exists := p.phaseHandlers.Load(entryName)
	if !exists {
		p.respondError(msg, req, natsCtx, fmt.Sprintf("no handler for entry: %s", entryName))
		return
	}

	if fn, ok := handlerFn.(func(*PhaseHandler)); ok {
		fn(phaseHandler)
	}

	phaseHandler.responseBodyFilter(&PhaseResponseBodyFilter{ctx: natsCtx})

	p.respondOK(msg, req, natsCtx)
}

// handleLoggingPhase handles Logging phase
func (p *NatsPlugin) handleLoggingPhase(msg *nats.Msg, req *PluginRequest) {
	natsCtx, phaseHandler, entryName := p.setupPhaseHandler(msg, req)

	handlerFn, exists := p.phaseHandlers.Load(entryName)
	if !exists {
		p.respondError(msg, req, natsCtx, fmt.Sprintf("no handler for entry: %s", entryName))
		return
	}

	if fn, ok := handlerFn.(func(*PhaseHandler)); ok {
		fn(phaseHandler)
	}

	phaseHandler.logging(&PhaseLogging{ctx: natsCtx})

	p.respondOK(msg, req, natsCtx)
}

// setupPhaseHandler creates phase handler context and structure
func (p *NatsPlugin) setupPhaseHandler(msg *nats.Msg, req *PluginRequest) (*NylonHttpPluginCtx, *PhaseHandler, string) {
	var natsCtx *NylonHttpPluginCtx
	if ctxValue, ok := natsSessions.Load(req.SessionID); ok {
		if existing, ok := ctxValue.(*NylonHttpPluginCtx); ok {
			natsCtx = existing
		}
	}

	if natsCtx == nil {
		natsCtx = &NylonHttpPluginCtx{
			sessionID: int32(req.SessionID),
			dataMap:   make(map[uint32][]byte),
			natsMode:  true,
		}
		natsCtx.cond = sync.NewCond(&natsCtx.mu)
		natsSessions.Store(req.SessionID, natsCtx)
	}

	natsCtx.mu.Lock()
	natsCtx.natsMode = true
	natsCtx.lastRequestID = req.RequestID
	natsCtx.natsConn = p.conn
	if req.Headers != nil {
		if reply, ok := req.Headers["reply"]; ok {
			natsCtx.replySubject = reply
		}
	}
	if natsCtx.replySubject == "" && msg.Reply != "" {
		natsCtx.replySubject = msg.Reply
	}
	natsCtx.mu.Unlock()

	phaseHandler := &PhaseHandler{
		SessionId: int32(req.SessionID),
		http_ctx:  natsCtx,
		natsMode:  true,
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

	entryName := "default"
	if req.Headers != nil {
		if entry, ok := req.Headers["entry"]; ok {
			entryName = entry
		}
	}

	streamSessions.Store(int32(req.SessionID), phaseHandler)

	return natsCtx, phaseHandler, entryName
}

// respondOK sends a success response
func (p *NatsPlugin) respondOK(msg *nats.Msg, req *PluginRequest, ctx *NylonHttpPluginCtx) {
	resp := PluginResponse{
		Version:   ProtocolVersion,
		RequestID: req.RequestID,
		SessionID: req.SessionID,
		Action:    ResponseActionNext,
	}
	p.sendResponse(msg, req, ctx, &resp)
}

// respondError sends an error response
func (p *NatsPlugin) respondError(msg *nats.Msg, req *PluginRequest, ctx *NylonHttpPluginCtx, errMsg string) {
	resp := PluginResponse{
		Version:   ProtocolVersion,
		RequestID: req.RequestID,
		SessionID: req.SessionID,
		Action:    ResponseActionError,
		Error:     &errMsg,
	}
	p.sendResponse(msg, req, ctx, &resp)
}

// sendResponse sends a response back via NATS
func (p *NatsPlugin) sendResponse(msg *nats.Msg, req *PluginRequest, ctx *NylonHttpPluginCtx, resp *PluginResponse) {
	reply := ""
	if req != nil && req.Headers != nil {
		if value, ok := req.Headers["reply"]; ok && value != "" {
			reply = value
		}
	}
	if reply == "" && ctx != nil {
		reply = ctx.replySubject
	}
	if reply == "" && msg.Reply != "" {
		reply = msg.Reply
	}

	if reply == "" {
		fmt.Printf("[NatsPlugin] No reply subject for response action=%s session=%d\n", resp.Action, resp.SessionID)
		return
	}

	data, err := msgpack.Marshal(resp)
	if err != nil {
		fmt.Printf("[NatsPlugin] Failed to encode response: %v\n", err)
		return
	}

	if err := p.conn.Publish(reply, data); err != nil {
		fmt.Printf("[NatsPlugin] Failed to send response: %v\n", err)
	}
}

// Close closes the NATS connection
func (p *NatsPlugin) Close() error {
	p.mu.Lock()
	defer p.mu.Unlock()

	fmt.Println("[NatsPlugin] Shutting down...")

	// Call shutdown handler
	if handler := p.shutdownHandler.Load(); handler != nil {
		if fn, ok := handler.(func()); ok {
			fmt.Println("[NatsPlugin] Calling shutdown handler")
			fn()
		}
	}

	// Unsubscribe from all subjects
	for _, sub := range p.subscriptions {
		sub.Unsubscribe()
	}

	if p.conn != nil {
		p.conn.Close()
		p.conn = nil
	}

	p.started = false
	fmt.Printf("[NatsPlugin] Plugin %s stopped\n", p.config.Name)

	return nil
}

// Helper to send NATS request from context (used by request methods)
func (ctx *NylonHttpPluginCtx) natsRequest(method NylonMethods, data []byte) error {
	if !ctx.natsMode {
		return fmt.Errorf("nats mode disabled")
	}

	ctx.mu.Lock()
	conn := ctx.natsConn
	reply := ctx.replySubject
	requestID := ctx.lastRequestID
	sessionID := uint32(ctx.sessionID)
	ctx.mu.Unlock()

	if conn == nil || reply == "" {
		return fmt.Errorf("nats context not initialized")
	}

	methodID, ok := MethodIDMapping[method]
	if !ok {
		return fmt.Errorf("unknown method: %s", method)
	}

	action := ResponseActionNext
	switch method {
	case NylonMethodEnd:
		action = ResponseActionEnd
	case NylonMethodNext:
		action = ResponseActionNext
	}

	resp := PluginResponse{
		Version:   ProtocolVersion,
		RequestID: requestID,
		SessionID: sessionID,
		Method:    &methodID,
		Action:    action,
		Data:      data,
	}

	payload, err := msgpack.Marshal(&resp)
	if err != nil {
		return err
	}

	return conn.Publish(reply, payload)
}
