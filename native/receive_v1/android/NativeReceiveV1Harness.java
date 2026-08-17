package app.kurtuba.openmls;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.List;

/** Disposable device harness for committed package-owned binary vectors. */
public final class NativeReceiveV1Harness {
  private static final String[] VECTOR_IDS = {
    "welcome_success",
    "welcome_wrong_key_package",
    "welcome_wrong_local_leaf",
    "application_success",
    "application_wrong_base",
    "application_wrong_aad",
    "application_wrong_sender",
    "application_wrong_roster",
    "application_wrong_kind",
    "commit_success",
    "welcome_256_leaves",
    "application_256_leaves"
  };

  private NativeReceiveV1Harness() {}

  public static void main(String[] args) throws Exception {
    if (args.length != 2) {
      throw new IllegalArgumentException("library path and fixture directory required");
    }
    System.load(args[0]);
    if (OpenMlsNativeReceive.nativeContractVersion() != 1) {
      throw new AssertionError("native receive contract version mismatch");
    }
    Path fixtures = Paths.get(args[1]);
    for (String id : VECTOR_IDS) {
      byte[] request = Files.readAllBytes(fixtures.resolve(id + ".request.bin"));
      byte[] expected = Files.readAllBytes(fixtures.resolve(id + ".response.bin"));
      long startedNanos = System.nanoTime();
      byte[] actual = OpenMlsNativeReceive.nativeExecuteReceiveV1(request);
      long elapsedMicros = (System.nanoTime() - startedNanos) / 1000L;
      if (actual == null || !Arrays.equals(actual, expected)) {
        throw new AssertionError(id + " response mismatch");
      }
      if (id.endsWith("_256_leaves")) {
        System.out.println(
            "native_receive_v1_android_limit id=" + id
                + " request_bytes=" + request.length
                + " response_bytes=" + actual.length
                + " elapsed_us=" + elapsedMicros
                + " vm_rss_kib=" + procStatusKib("VmRSS:")
                + " vm_hwm_kib=" + procStatusKib("VmHWM:"));
      }
      OpenMlsNativeReceive.nativeZeroize(request);
      OpenMlsNativeReceive.nativeZeroize(actual);
      assertZero(id + " request", request);
      assertZero(id + " response", actual);
      Arrays.fill(expected, (byte) 0);
    }
    System.out.println("native_receive_v1_android_vectors=" + VECTOR_IDS.length + " passed=true");
  }

  private static void assertZero(String label, byte[] bytes) {
    byte[] expected = new byte[bytes.length];
    if (!Arrays.equals(bytes, expected)) {
      throw new AssertionError(label + " was not zeroized");
    }
  }

  private static long procStatusKib(String field) {
    try {
      List<String> lines = Files.readAllLines(Paths.get("/proc/self/status"));
      for (String line : lines) {
        if (line.startsWith(field)) {
          String[] parts = line.trim().split("\\s+");
          return Long.parseLong(parts[1]);
        }
      }
    } catch (Exception ignored) {
      // Measurement is evidence only; vector correctness remains authoritative.
    }
    return -1L;
  }
}
