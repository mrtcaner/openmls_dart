# Pinned OpenMLS source patch

Baseline: https://github.com/openmls/openmls at
`47dbedecad0c1fd8eb5368d582250ebfcc1e1ce6` (`openmls-v0.8.1`).

Only the `openmls` crate source and manifest are vendored. LICENSE and the parent
README are copied from the same baseline. The packaging-only workspace manifest
resolves inherited dependencies to the existing pinned source; it introduces no
cryptographic upgrade or optional production features. Test vectors, benchmarks
and other workspace crates are not copied. Upstream source test modules remain
unchanged. Package-owned focused tests exercise the patch.

Behavioral patch: explicit existing leaf selection on the external builder before
insertion-index allocation. The caller MUST independently authenticate the exact
old-to-new participant binding. Omission preserves upstream same-key behavior.
Accessor patch: expose signer index on a **verified** GroupInfo, avoiding private
field/Serde observations in production validation. Both are recorded in
`openmls-external-resync.patch` for review/backport. No signature/tree/lifetime
validation is disabled.

Tracking: https://github.com/mrtcaner/openmls_dart/issues/59 . This source has not
been independently released; no upstream patch acceptance is implied.

Patch SHA-256:
`283789ca1159f4b111451c54aacff44d1d6c3d7eb4fb56b97e8f5e97ce1f426f`.
Patch paths are relative to the upstream `openmls/` crate directory.
The package notice generator explicitly retains `rust/vendor` dependencies as
third-party code. The inventory remains 226 crates; only this crate's license
location changes from `../LICENSE` to the identical copied `LICENSE`.
