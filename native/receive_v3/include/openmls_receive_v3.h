#ifndef OPENMLS_RECEIVE_V3_H
#define OPENMLS_RECEIVE_V3_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenMlsReceiveV3Buffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} OpenMlsReceiveV3Buffer;

// The caller supplies readable bytes for the duration of this call and owns
// wiping its input. The adapter retains no pointer or mutable group state.
// Results contain a complete caller-owned batch, never an applied mutation.
OpenMlsReceiveV3Buffer openmls_receive_v3_execute(
    const uint8_t *request_data, size_t request_len);

// Zeroizes and frees one unchanged buffer returned by v3_execute, exactly once.
// Do not free the data pointer separately or use the buffer after this call.
void openmls_receive_v3_free(OpenMlsReceiveV3Buffer buffer);

// Additive contract: returns 3; existing v2 entry points still report 2.
uint16_t openmls_receive_v3_version(void);

#ifdef __cplusplus
}
#endif

#endif
