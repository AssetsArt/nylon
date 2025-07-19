// nylon.h - FFI Interface
#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>
#include <stdint.h>

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
