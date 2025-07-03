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

// Memory pool for buffer reuse
typedef struct {
    FfiBuffer* buffers;
    uint32_t count;
    uint32_t capacity;
} FfiBufferPool;

// Initialize buffer pool
static inline FfiBufferPool* ffi_buffer_pool_init(uint32_t initial_capacity) {
    FfiBufferPool* pool = malloc(sizeof(FfiBufferPool));
    if (pool) {
        pool->buffers = malloc(sizeof(FfiBuffer) * initial_capacity);
        pool->count = 0;
        pool->capacity = initial_capacity;
    }
    return pool;
}

// Get buffer from pool (zero allocation if available)
static inline FfiBuffer* ffi_buffer_pool_get(FfiBufferPool* pool) {
    if (pool->count > 0) {
        return &pool->buffers[--pool->count];
    }
    return NULL;
}

// Return buffer to pool
static inline void ffi_buffer_pool_return(FfiBufferPool* pool, FfiBuffer* buffer) {
    if (pool->count < pool->capacity) {
        pool->buffers[pool->count++] = *buffer;
    }
}

// Cleanup buffer pool
static inline void ffi_buffer_pool_free(FfiBufferPool* pool) {
    if (pool) {
        free(pool->buffers);
        free(pool);
    }
}

#endif // NYLON_H
