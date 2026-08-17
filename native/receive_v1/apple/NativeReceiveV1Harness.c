#include "openmls_receive_v1.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

static const char *vector_ids[] = {
    "welcome_success",
    "welcome_wrong_key_package",
    "welcome_wrong_local_leaf",
    "application_success",
    "application_wrong_base",
    "application_wrong_aad",
    "application_empty_aad",
    "application_wrong_sender",
    "application_wrong_roster",
    "application_wrong_kind",
    "commit_success",
    "welcome_256_leaves",
    "application_256_leaves",
};

static long long elapsed_microseconds(struct timespec start, struct timespec end) {
  return (long long)(end.tv_sec - start.tv_sec) * 1000000LL +
         (long long)(end.tv_nsec - start.tv_nsec) / 1000LL;
}

static unsigned char *read_file(const char *path, size_t *length) {
  FILE *file = fopen(path, "rb");
  if (file == NULL) {
    return NULL;
  }
  if (fseek(file, 0, SEEK_END) != 0) {
    fclose(file);
    return NULL;
  }
  long size = ftell(file);
  if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
    fclose(file);
    return NULL;
  }
  unsigned char *bytes = malloc((size_t)size);
  if (bytes == NULL) {
    fclose(file);
    return NULL;
  }
  if (fread(bytes, 1, (size_t)size, file) != (size_t)size) {
    free(bytes);
    fclose(file);
    return NULL;
  }
  fclose(file);
  *length = (size_t)size;
  return bytes;
}

static void wipe(void *pointer, size_t length) {
  volatile unsigned char *bytes = pointer;
  while (length-- > 0) {
    *bytes++ = 0;
  }
}

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "fixture directory required\n");
    return 2;
  }
  if (openmls_receive_v1_version() != 1) {
    fprintf(stderr, "contract version mismatch\n");
    return 3;
  }
  for (size_t index = 0; index < sizeof(vector_ids) / sizeof(vector_ids[0]); index++) {
    char request_path[1024];
    char response_path[1024];
    snprintf(request_path, sizeof(request_path), "%s/%s.request.bin", argv[1], vector_ids[index]);
    snprintf(response_path, sizeof(response_path), "%s/%s.response.bin", argv[1], vector_ids[index]);
    size_t request_length = 0;
    size_t expected_length = 0;
    unsigned char *request = read_file(request_path, &request_length);
    unsigned char *expected = read_file(response_path, &expected_length);
    if (request == NULL || expected == NULL) {
      fprintf(stderr, "%s fixture read failed\n", vector_ids[index]);
      return 4;
    }
    struct timespec started;
    struct timespec finished;
    clock_gettime(CLOCK_MONOTONIC, &started);
    OpenMlsReceiveV1Buffer actual =
        openmls_receive_v1_execute(request, request_length);
    clock_gettime(CLOCK_MONOTONIC, &finished);
    if (actual.data == NULL || actual.len != expected_length ||
        memcmp(actual.data, expected, expected_length) != 0) {
      fprintf(stderr, "%s response mismatch\n", vector_ids[index]);
      return 5;
    }
    if (strstr(vector_ids[index], "_256_leaves") != NULL) {
      struct rusage usage;
      memset(&usage, 0, sizeof(usage));
      getrusage(RUSAGE_SELF, &usage);
      printf("native_receive_v1_apple_limit id=%s request_bytes=%zu "
             "response_bytes=%zu elapsed_us=%lld max_rss_bytes=%ld\n",
             vector_ids[index], request_length, actual.len,
             elapsed_microseconds(started, finished), usage.ru_maxrss);
    }
    wipe(request, request_length);
    wipe(expected, expected_length);
    free(request);
    free(expected);
    openmls_receive_v1_free(actual);
  }
  printf("native_receive_v1_apple_vectors=%zu passed=true\n",
         sizeof(vector_ids) / sizeof(vector_ids[0]));
  return 0;
}
