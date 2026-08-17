#ifndef OPENMLS_RECEIVE_V1_H
#define OPENMLS_RECEIVE_V1_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenMlsReceiveV1Buffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} OpenMlsReceiveV1Buffer;

OpenMlsReceiveV1Buffer openmls_receive_v1_execute(
    const uint8_t *request_data,
    size_t request_len);

// Zeroizes and frees one buffer returned by openmls_receive_v1_execute.
// Passing a buffer more than once or changing its fields is invalid.
void openmls_receive_v1_free(OpenMlsReceiveV1Buffer buffer);

uint16_t openmls_receive_v1_version(void);

#ifdef __cplusplus
}
#endif

#endif
