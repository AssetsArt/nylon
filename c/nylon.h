// nylon.h
#ifndef NYLON_H
#define NYLON_H

#include <stdlib.h>

typedef struct {
    const unsigned char *ptr;
    unsigned long len;
} FfiOutput;

typedef void (*data_event_fn)(uint32_t session_id, uint32_t method, const char* data, int32_t len);

// C wrapper function to actually call the function pointer
static inline void call_event_method(data_event_fn cb, uint32_t session_id, uint32_t method, const char* data, int32_t len) {
    cb(session_id, method, data, len);
}

#endif // NYLON_H
