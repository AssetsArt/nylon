// nylon.h - FFI Interface
#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>
#include <stdint.h>

// Zero-copy data structure
typedef struct {
    const unsigned char *ptr;
    uint32_t len;
    uint32_t capacity;  // For potential reuse
} FfiBuffer;

// Optimized output structure
typedef struct {
    FfiBuffer buffer;
    uint32_t flags;  // For metadata without additional allocations
} FfiOutput;

// Event callback with optimized signature
typedef void (*data_event_fn)(uint32_t session_id, uint32_t method, const FfiBuffer* data);

// Inline wrapper for minimal overhead
static inline void call_event_method(data_event_fn cb, uint32_t session_id, uint32_t method, const FfiBuffer* data) {
    cb(session_id, method, data);
}

#endif // NYLON_H
