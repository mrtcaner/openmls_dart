# Local Linux public-validator artifacts

This is a build/verification recipe, not a release or service deployment.
The executable uses the existing pinned Rust/OpenMLS public validator core and
KMPV/1 stdin/stdout protocol. It has no database, network listener or private MLS
state. Do not replace the consumer's authenticated product admission rules with
the larger package transport limits.

From the fork checkout root:

```sh
make build-public-validator-linux
```

The default builds linux/amd64 and linux/arm64 sequentially (compatible with the
Docker driver), one Cargo build job per architecture. To build one architecture:

```sh
make build-public-validator-linux PUBLIC_VALIDATOR_PLATFORMS=linux/arm64
```

The pinned Rust1.97.0/Alpine3.23 image index selects the corresponding architecture
manifest. Cargo uses the lockfile. No host Cargo credentials/caches, private
consumer docs or target build products are copied into the Docker context.
Build dependencies are downloaded; this is not an offline build. APK package
versions appear in retained build output; this recipe is not a promise of
bit-for-bit reproduction under future APK repository changes.

Default output is the ignored directory
`.buildlog/external-resync-r0-20261016/n3a/linux/{amd64,arm64}/`, containing:

- `openmls_public_validator` and `artifact.sha256`;
- `source-base-revision.txt` (base Git revision, NOT a clean-tree assertion);
- `source-files.sha256` and `source-manifest.sha256` (actual copied source bytes,
  including the local vendored OpenMLS patch and Cargo lockfile);
- `toolchain.txt` and `binary-format.txt`.

Bind a consuming image to the reviewed artifact hash and its source manifest,
not merely the base revision of a dirty development tree. This local output is
not an all-platform package release or finalized license/notice distribution.

After generating the N3a size fixtures with `make measure-external-resync`, check
the executable inside the intended existing Alpine runtime image:

```sh
make -f native/public_validator_v1/linux/Makefile verify \
  ARCH=amd64 RUNTIME_IMAGE='<exact-local-image-id-or-pinned-digest>'
```

The verification replaces the service entry point with a one-shot shell, disables
networking, drops capabilities, uses UID10001, read-only mounts/rootfs, a bounded
tmpfs and128MiB/oneCPU/32PID limits. It executes eleven existing binary vectors
and six N3a public size fixtures, requiring exact output, exit0 and empty stderr
with an empty environment for every validator call. Repeat with ARCH=arm64 and
the intended arm64 Alpine runtime image. Do not launch the service or deploy
these development artifacts merely to validate linkage.

The test does not exercise Go's process wrapper/admission, service concurrency,
production rollout, mobile frame3 or Android/Apple packaging. Those remain their
separate owner gates. Existing v2 exports and profile2/storage1 are unchanged.
