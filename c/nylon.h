#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    uint32_t sid;
    uint8_t phase;
    uint32_t method;
    const unsigned char *ptr;
    uint64_t len;
} FfiBuffer;

typedef void (*data_event_fn)(const FfiBuffer* ffiBuffer);

static inline void call_event_method(data_event_fn cb, const FfiBuffer* ffiBuffer) {
    cb(ffiBuffer);
}

#endif // NYLON_H