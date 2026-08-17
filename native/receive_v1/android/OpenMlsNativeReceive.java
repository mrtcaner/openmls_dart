package app.kurtuba.openmls;

/** Mechanical app-owned transport for package-owned JNI symbols. */
public final class OpenMlsNativeReceive {
  private OpenMlsNativeReceive() {}

  public static native byte[] nativeExecuteReceiveV1(byte[] request);
  public static native void nativeZeroize(byte[] bytes);
  public static native int nativeContractVersion();
}
