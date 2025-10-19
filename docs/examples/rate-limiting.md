# Rate Limiting

Implement rate limiting to protect your APIs from abuse.

## Simple IP-Based Rate Limiting

Limit requests per IP address:

```go
package main

import "C"
import (
	"fmt"
	"sync"
	"time"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

type RateLimiter struct {
	requests map[string][]int64
	mu       sync.Mutex
	limit    int
	window   int64 // in seconds
}

func NewRateLimiter(limit int, window int64) *RateLimiter {
	return &RateLimiter{
		requests: make(map[string][]int64),
		limit:    limit,
		window:   window,
	}
}

func (rl *RateLimiter) Allow(ip string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()
	
	now := time.Now().Unix()
	cutoff := now - rl.window
	
	// Get requests for this IP
	requests := rl.requests[ip]
	
	// Filter out old requests
	var recent []int64
	for _, t := range requests {
		if t > cutoff {
			recent = append(recent, t)
		}
	}
	
	// Check if under limit
	if len(recent) >= rl.limit {
		rl.requests[ip] = recent
		return false
	}
	
	// Add new request
	recent = append(recent, now)
	rl.requests[ip] = recent
	
	return true
}

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	// 100 requests per minute
	limiter := NewRateLimiter(100, 60)
	
	plugin.AddPhaseHandler("rate-limit", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			clientIP := req.ClientIP()
			
		if !limiter.Allow(clientIP) {
			res := ctx.Response()
			res.SetStatus(429)
			res.SetHeader("Retry-After", "60")
			res.BodyJSON(map[string]interface{}{
				"error": "Too Many Requests",
				"message": "Rate limit exceeded. Try again later.",
			})
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			ctx.Next()
		})
	})
}
```

## Token Bucket Algorithm

More sophisticated rate limiting:

```go
package main

import "C"
import (
	"sync"
	"time"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

type TokenBucket struct {
	tokens    float64
	capacity  float64
	rate      float64 // tokens per second
	lastCheck time.Time
	mu        sync.Mutex
}

func NewTokenBucket(capacity, rate float64) *TokenBucket {
	return &TokenBucket{
		tokens:    capacity,
		capacity:  capacity,
		rate:      rate,
		lastCheck: time.Now(),
	}
}

func (tb *TokenBucket) Allow() bool {
	tb.mu.Lock()
	defer tb.mu.Unlock()
	
	now := time.Now()
	elapsed := now.Sub(tb.lastCheck).Seconds()
	tb.lastCheck = now
	
	// Add tokens based on elapsed time
	tb.tokens += elapsed * tb.rate
	if tb.tokens > tb.capacity {
		tb.tokens = tb.capacity
	}
	
	// Check if we have a token
	if tb.tokens >= 1 {
		tb.tokens--
		return true
	}
	
	return false
}

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	buckets := make(map[string]*TokenBucket)
	var mu sync.Mutex
	
	plugin.AddPhaseHandler("token-bucket", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			clientIP := req.ClientIP()
			
			mu.Lock()
			bucket, exists := buckets[clientIP]
			if !exists {
				// 10 requests burst, 2 per second refill
				bucket = NewTokenBucket(10, 2)
				buckets[clientIP] = bucket
			}
			mu.Unlock()
			
		if !bucket.Allow() {
			res := ctx.Response()
			res.SetStatus(429)
			res.BodyJSON(map[string]interface{}{
				"error": "Rate limit exceeded",
			})
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			ctx.Next()
		})
	})
}
```

## Per-User Rate Limiting

Rate limit by user ID instead of IP:

```go
plugin.AddPhaseHandler("user-rate-limit", func(phase *sdk.PhaseHandler) {
	limiters := make(map[string]*RateLimiter)
	var mu sync.Mutex
	
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		
		// Get user ID from token
		token := req.Header("Authorization")
		userID := validateAndGetUserID(token)
		
		if userID == "" {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Unauthorized")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		// Get or create limiter for this user
		mu.Lock()
		limiter, exists := limiters[userID]
		if !exists {
			// 1000 requests per hour per user
			limiter = NewRateLimiter(1000, 3600)
			limiters[userID] = limiter
		}
		mu.Unlock()
		
	if !limiter.Allow(userID) {
		res := ctx.Response()
		res.SetStatus(429)
		res.SetHeader("X-RateLimit-Limit", "1000")
		res.SetHeader("X-RateLimit-Remaining", "0")
		res.BodyText("Rate limit exceeded")
		ctx.RemoveResponseHeader("Content-Length")
		ctx.SetResponseHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
		
		ctx.Next()
	})
})
```

## Path-Based Rate Limiting

Different limits for different endpoints:

```go
type PathLimits struct {
	paths map[string]*RateLimiter
	mu    sync.Mutex
}

func NewPathLimits() *PathLimits {
	return &PathLimits{
		paths: make(map[string]*RateLimiter),
	}
}

func (pl *PathLimits) GetLimiter(path string) *RateLimiter {
	pl.mu.Lock()
	defer pl.mu.Unlock()
	
	limiter, exists := pl.paths[path]
	if !exists {
		// Default: 100 requests per minute
		limiter = NewRateLimiter(100, 60)
		pl.paths[path] = limiter
	}
	return limiter
}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	limits := NewPathLimits()
	
	// Configure specific paths
	limits.paths["/api/expensive"] = NewRateLimiter(10, 60)  // 10 req/min
	limits.paths["/api/search"] = NewRateLimiter(30, 60)      // 30 req/min
	limits.paths["/api/users"] = NewRateLimiter(100, 60)      // 100 req/min
	
	plugin.AddPhaseHandler("path-rate-limit", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			path := req.Path()
			clientIP := req.ClientIP()
			
			limiter := limits.GetLimiter(path)
			key := clientIP + ":" + path
			
			if !limiter.Allow(key) {
				res := ctx.Response()
				res.SetStatus(429)
				res.BodyText("Rate limit exceeded")
				ctx.RemoveResponseHeader("Content-Length")
				ctx.SetResponseHeader("Transfer-Encoding", "chunked")
				ctx.End()
				return
			}
			
			ctx.Next()
		})
	})
}
```

## Rate Limit Headers

Include rate limit information in response headers:

```go
plugin.AddPhaseHandler("rate-limit-headers", func(phase *sdk.PhaseHandler) {
	limiter := NewRateLimiter(100, 60)
	
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		clientIP := req.ClientIP()
		
		allowed := limiter.Allow(clientIP)
		
		// Get current count
		limiter.mu.Lock()
		requests := limiter.requests[clientIP]
		remaining := limiter.limit - len(requests)
		limiter.mu.Unlock()
		
		// Set headers
		res := ctx.Response()
		res.SetHeader("X-RateLimit-Limit", fmt.Sprintf("%d", limiter.limit))
		res.SetHeader("X-RateLimit-Remaining", fmt.Sprintf("%d", remaining))
		res.SetHeader("X-RateLimit-Reset", fmt.Sprintf("%d", time.Now().Unix()+limiter.window))
		
	if !allowed {
		res.SetStatus(429)
		res.SetHeader("Retry-After", fmt.Sprintf("%d", limiter.window))
		res.BodyJSON(map[string]interface{}{
			"error": "Rate limit exceeded",
			"limit": limiter.limit,
			"window": limiter.window,
		})
		ctx.RemoveResponseHeader("Content-Length")
		ctx.SetResponseHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
		
		ctx.Next()
	})
})
```

## Configuration

```yaml
plugins:
  - name: rate-limit
    type: ffi
    file: ./rate-limit.so
    config:
      limit: 100
      window: 60

middleware_groups:
  api:
    - plugin: rate-limit
      entry: "rate-limit"

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    middleware:
      - group: api
    paths:
      - path: /api/*
        service:
          name: api-service
```

## Best Practices

### 1. Choose Appropriate Limits

```go
// Public endpoints - restrictive
limiter := NewRateLimiter(10, 60)  // 10 req/min

// Authenticated users - generous
limiter := NewRateLimiter(1000, 60)  // 1000 req/min

// Internal services - unlimited
// Don't apply rate limiting
```

### 2. Include Rate Limit Headers

```go
res.SetHeader("X-RateLimit-Limit", "100")
res.SetHeader("X-RateLimit-Remaining", "95")
res.SetHeader("X-RateLimit-Reset", "1234567890")
res.SetHeader("Retry-After", "60")
```

### 3. Clean Up Old Entries

```go
// Periodically clean up old entries
go func() {
	ticker := time.NewTicker(5 * time.Minute)
	for range ticker.C {
		limiter.mu.Lock()
		for ip, requests := range limiter.requests {
			if len(requests) == 0 {
				delete(limiter.requests, ip)
			}
		}
		limiter.mu.Unlock()
	}
}()
```

### 4. Use Redis for Distributed Systems

For multi-server setups, use Redis for shared state:

```go
import "github.com/go-redis/redis/v8"

func NewRedisRateLimiter(client *redis.Client, limit int, window int64) *RedisRateLimiter {
	// Implement using Redis sorted sets
	// ...
}
```

### 5. Return Helpful Error Messages

```go
res := ctx.Response()
res.SetStatus(429)
res.BodyJSON(map[string]interface{}{
	"error": "Rate limit exceeded",
	"message": "You have exceeded the rate limit of 100 requests per minute",
	"retry_after": 42,
})
ctx.RemoveResponseHeader("Content-Length")
ctx.SetResponseHeader("Transfer-Encoding", "chunked")
ctx.End()
return
```

## See Also

- [Authentication](/examples/authentication) - Combine with rate limiting
- [Request Handling](/plugins/request) - Access request information
- [Configuration](/core/configuration) - Configure plugins

