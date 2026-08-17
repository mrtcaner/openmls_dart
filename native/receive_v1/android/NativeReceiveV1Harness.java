package app.kurtuba.openmls;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;

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
    "commit_success"
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
      byte[] actual = OpenMlsNativeReceive.nativeExecuteReceiveV1(request);
      if (actual == null || !Arrays.equals(actual, expected)) {
        throw new AssertionError(id + " response mismatch");
      }
      OpenMlsNativeReceive.nativeZeroize(request);
      OpenMlsNativeReceive.nativeZeroize(actual);
      assertZero(id + " request", request);
      assertZero(id + " response", actual);
      Arrays.fill(expected, (byte) 0);
    }
    System.out.println("native_receive_v1_android_vectors=10 passed=true");
  }

  private static void assertZero(String label, byte[] bytes) {
    byte[] expected = new byte[bytes.length];
    if (!Arrays.equals(bytes, expected)) {
      throw new AssertionError(label + " was not zeroized");
    }
  }
}
