# NATS Plugin Configuration

## Overview

Nylon's NATS plugin transport comes with **sensible defaults** for all phases. You don't need to configure anything unless you want to override the defaults.

## Minimal Configuration

```yaml
# Minimal config - just specify NATS servers
messaging:
  - name: default
    servers:
      - nats://localhost:4222

plugins:
  - name: my-plugin
    messaging: default
    # That's it! No per_phase config needed
```

## Default Phase Policies

The system automatically applies these defaults:

### Request Filter Phase
```yaml
# Automatically applied (no config needed)
request_filter:
  timeout_ms: 5000
  on_error: retry
  retry:
    max: 3
    backoff_ms_initial: 100
    backoff_ms_max: 1000
```

### Response Filter Phase
```yaml
# Automatically applied (no config needed)
response_filter:
  timeout_ms: 3000
  on_error: continue  # Don't block response on failure
  retry:
    max: 2
    backoff_ms_initial: 50
    backoff_ms_max: 500
```

### Logging Phase
```yaml
# Automatically applied (no config needed)
logging:
  timeout_ms: 200     # Fast, non-blocking
  on_error: continue  # Never block response
  retry:
    max: 1            # No retries for logging
```

### Response Body Filter Phase
```yaml
# Automatically applied (no config needed)
response_body_filter:
  timeout_ms: 3000
  on_error: continue
  retry:
    max: 2
    backoff_ms_initial: 50
    backoff_ms_max: 500
```

## Optional: Override Defaults

You can override specific values while keeping others as defaults:

### Override Only Timeout

```yaml
plugins:
  - name: my-plugin
    messaging: default
    per_phase:
      request_filter:
        timeout_ms: 10000  # Just increase timeout
        # on_error and retry use defaults
```

### Override Only Error Handling

```yaml
plugins:
  - name: my-plugin
    messaging: default
    per_phase:
      request_filter:
        on_error: end  # Fail fast instead of retry
        # timeout_ms and retry use defaults
```

### Override Retry Policy

```yaml
plugins:
  - name: my-plugin
    messaging: default
    per_phase:
      request_filter:
        retry:
          max: 5
          backoff_ms_initial: 200
          backoff_ms_max: 2000
        # timeout_ms and on_error use defaults
```

### Full Custom Configuration

```yaml
plugins:
  - name: critical-plugin
    messaging: default
    per_phase:
      request_filter:
        timeout_ms: 10000
        on_error: retry
        retry:
          max: 5
          backoff_ms_initial: 200
          backoff_ms_max: 5000
      
      response_filter:
        timeout_ms: 5000
        on_error: continue
        retry:
          max: 3
      
      logging:
        timeout_ms: 500
        on_error: continue
        retry:
          max: 1
```

## Error Handling Modes

### `retry` (Default for critical phases)
- Automatically retries on failure
- Uses exponential backoff
- Good for: request_filter, zero phase

### `continue` (Default for non-critical phases)
- Logs error but continues execution
- Doesn't block response
- Good for: logging, response_filter

### `end`
- Fails immediately
- Returns error to caller
- Good for: strict validation requirements

## Best Practices

### 1. Start with Defaults
```yaml
# Just this works!
plugins:
  - name: my-plugin
    messaging: default
```

### 2. Override Only What's Needed
```yaml
# Only change timeout for slow plugins
plugins:
  - name: slow-plugin
    messaging: default
    per_phase:
      request_filter:
        timeout_ms: 30000
```

### 3. Phase-Specific Tuning
```yaml
plugins:
  - name: my-plugin
    messaging: default
    per_phase:
      # Critical: strict with retries
      request_filter:
        timeout_ms: 5000
        on_error: retry
        retry:
          max: 3
      
      # Observability: fast and non-blocking
      logging:
        timeout_ms: 100
        on_error: continue
```

## Complete Example

### Development (defaults are fine)
```yaml
messaging:
  - name: dev
    servers:
      - nats://localhost:4222

plugins:
  - name: auth-check
    messaging: dev
  
  - name: logger
    messaging: dev
```

### Production (with custom tuning)
```yaml
messaging:
  - name: prod
    servers:
      - nats://nats-1.prod:4222
      - nats://nats-2.prod:4222
      - nats://nats-3.prod:4222
    max_inflight: 2048
    request_timeout_ms: 10000

plugins:
  - name: auth-check
    messaging: prod
    per_phase:
      request_filter:
        timeout_ms: 8000
        retry:
          max: 5
  
  - name: rate-limiter
    messaging: prod
    per_phase:
      request_filter:
        timeout_ms: 3000
        on_error: end  # Fail fast on rate limit errors
  
  - name: logger
    messaging: prod
    per_phase:
      logging:
        timeout_ms: 500
        # on_error: continue (default - never block)
```

## Summary

| Phase | Default Timeout | Default on_error | Default Retries | Rationale |
|-------|----------------|------------------|-----------------|-----------|
| **request_filter** | 5000ms | retry | 3 attempts | Critical path, needs reliability |
| **response_filter** | 3000ms | continue | 2 attempts | Important but shouldn't block |
| **response_body_filter** | 3000ms | continue | 2 attempts | Transform responses, non-critical |
| **logging** | 200ms | continue | 1 attempt | Observability, must be fast |

**Key Point:** All defaults are production-ready. Only override when you have specific requirements!

## FAQ

### Do I need to configure `per_phase`?
**No!** The system has smart defaults for all phases. Only configure if you need custom behavior.

### What happens if I don't specify a phase?
The default policy for that phase is automatically applied.

### Can I partially override a phase?
**Yes!** You can override just timeout, just on_error, or just retry. Unspecified values use defaults.

### Are these defaults production-ready?
**Yes!** They're designed for production use:
- Critical phases (request_filter) retry automatically
- Non-critical phases (logging) never block responses
- Timeouts are reasonable for most use cases

