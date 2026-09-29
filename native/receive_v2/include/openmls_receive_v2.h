#ifndef OPENMLS_RECEIVE_V2_H
#define OPENMLS_RECEIVE_V2_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenMlsReceiveV2Buffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} OpenMlsReceiveV2Buffer;

OpenMlsReceiveV2Buffer openmls_receive_v2_execute(
    const uint8_t *request_data,
    size_t request_len);

// Zeroizes and frees one buffer returned by openmls_receive_v2_execute.
// Passing a buffer more than once or changing its fields is invalid.
void openmls_receive_v2_free(OpenMlsReceiveV2Buffer buffer);

uint16_t openmls_receive_v2_version(void);

#ifdef __cplusplus
}
#endif

#endif
