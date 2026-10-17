#include "openmls_receive_v2.h"
#include "openmls_receive_v3.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(sizeof(OpenMlsReceiveV2Buffer) == sizeof(OpenMlsReceiveV3Buffer), "buffer size");
_Static_assert(offsetof(OpenMlsReceiveV2Buffer, capacity) == offsetof(OpenMlsReceiveV3Buffer, capacity), "buffer layout");

static unsigned char *read_file(const char *path, size_t *length) {
  FILE *file = fopen(path, "rb");
  if (!file || fseek(file, 0, SEEK_END)) exit(2);
  long size = ftell(file);
  if (size <= 0 || fseek(file, 0, SEEK_SET)) exit(2);
  unsigned char *bytes = malloc((size_t)size);
  if (!bytes || fread(bytes, 1, (size_t)size, file) != (size_t)size) exit(2);
  fclose(file);
  *length = (size_t)size;
  return bytes;
}
static void wipe(void *pointer, size_t length) {
  volatile unsigned char *bytes = pointer;
  while (length-- > 0) *bytes++ = 0;
}
int main(int argc, char **argv) {
  if (argc != 2 || openmls_receive_v2_version() != 2 || openmls_receive_v3_version() != 3) return 3;
  const char *ids[] = {"external_success", "external_wrong_base", "inspect_none", "inspect_pending", "inspect_wrong_base"};
  for (size_t i = 0; i < sizeof(ids) / sizeof(ids[0]); i++) {
    char path[2048];
    size_t request_len, expected_len;
    snprintf(path, sizeof(path), "%s/%s.request.bin", argv[1], ids[i]);
    unsigned char *request = read_file(path, &request_len);
    snprintf(path, sizeof(path), "%s/%s.response.bin", argv[1], ids[i]);
    unsigned char *expected = read_file(path, &expected_len);
    OpenMlsReceiveV3Buffer actual = openmls_receive_v3_execute(request, request_len);
    if (!actual.data || actual.capacity < actual.len || actual.len != expected_len ||
        memcmp(actual.data, expected, expected_len)) return 4;
    printf("PASS %s exact_core_response_bytes=%zu stateApplied=false\n", ids[i], actual.len);
    wipe(request, request_len); wipe(expected, expected_len);
    free(request); free(expected); openmls_receive_v3_free(actual);
  }
  OpenMlsReceiveV3Buffer empty = openmls_receive_v3_execute(NULL, 0);
  if (!empty.data || empty.len < 12 || memcmp(empty.data, "KMLS\0\3", 6)) return 5;
  openmls_receive_v3_free(empty);
  puts("PASS v2_version=2 v3_version=3 C_ABI_layout null_failure_and_free_once");
  return 0;
}
