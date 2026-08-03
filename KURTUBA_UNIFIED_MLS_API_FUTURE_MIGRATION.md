# Kurtuba Unified MLS API Future Migration

**Date:** 2026-08-03

**Status:** Implemented on the issue branch; pull-request and release evidence pending

**Scope:** `openmls_dart` public API and consumer migration boundary

## Decision

Kurtuba should eventually use one strict caller-owned-storage MLS API for both
direct and group E2EE.

A direct conversation is not a different cryptographic construction. It is an
MLS group whose product authority permits exactly two active installation
leaves once the conversation is send-usable. Owner-only initialization remains
pre-activation state, and an installation replacement remains one atomic
remove-plus-add Commit. Chat and Flutter enforce those product rules; the
OpenMLS package must not encode a special two-member mode.

The strict variable-roster surface released in `openmls_frb-2.1.0` is the
technical base for this convergence. The earlier direct-compatible surface was
kept unchanged in 2.1.0 to avoid breaking the existing consumer while the
group contract was still being designed.

Kurtuba is the fork's only consumer, and neither direct nor group MLS is a
production feature with a production dataset. The planned major release
therefore does not need a deprecation window, compatibility shim, aliases,
dual dispatch, dual Chat routes, or old/new database authority. The weak and
`V2` surfaces are being replaced together now that the shared Chat/Flutter
package-facing contract is frozen.

The owner approved implementation on 2026-08-03. Work is tracked in
[`mrtcaner/openmls_dart#28`](https://github.com/mrtcaner/openmls_dart/issues/28)
on branch `codex/28-unified-mls-api`; it still requires pull-request review and
a complete all-platform release before either consumer pins it.

## Reconciliation evidence

Chat and Flutter completed repository-owned migration assessments on
2026-08-03:

- [Chat migration assessment](https://github.com/mrtcaner/kurtuba_app_chat/blob/main/docs/issues/open/features/2026-08-03_unified_mls_api_migration_assessment.md)
- [Flutter migration assessment](https://github.com/mrtcaner/kurtuba_app_flutter/blob/main/docs/issues/features/open/2026-08-03_unified_mls_api_migration_assessment.md)

Both approve one breaking cut without a compatibility layer. Neither found a
missing OpenMLS capability. They agree that the package rename/removal is
mechanically small and that the substantive work is shared consumer authority:

- Chat must replace pair-shaped cryptographic authority with canonical
  installation leaves, rosters, transitions, readiness, signature-key
  bindings and versioned AAD;
- Flutter must replace immediate direct bootstrap application with inactive
  owner/Commit candidates, canonical roster validation and atomic promotion;
- routine installation replacement changes from destructive rebootstrap to
  one same-incarnation atomic remove-plus-add Commit; and
- OpenMLS storage format `1` remains usable without a package storage
  migration.

## Implementation record — 2026-08-03

Issue #28 is implemented on `codex/28-unified-mls-api` as the 3.0.0 API cut:

- strict create/add/Welcome-join/process functions and result types now own the
  unsuffixed names;
- weak implementations, legacy-only results and every public `V2` symbol are
  removed together;
- remove, swap, self-update, KeyPackage, application-message, delete, roster
  and base-digest operations are unchanged;
- generated Rust, Dart IO and Dart Web bindings are regenerated and committed;
- direct-sized and variable-roster tests use the same strict API; and
- package/native versions advance to 3.0.0 while MLS storage format remains
  version `1` and the locked dependency/notice hash remains unchanged.

Local evidence is green for `make analyze`, `make rust-check`,
`make rust-test` (12 tests), `make build`, `make test` (108 tests),
`make rust-clippy`, `make format-check`, and
`make verify-third-party-notices`. This is not release evidence for every
target. Pull-request CI and the signed Android, iOS, macOS, Linux, Windows and
Web release matrix must still pass before Chat or Flutter pins 3.0.0.

## 2.1.0 surfaces before the cut

The earlier compatibility operations are:

- `createGroupWithStorage`
- `addMembersWithStorage`
- `joinGroupFromWelcomeWithStorage`
- `processMessageWithStorage`

The strict general operations are:

- `createGroupWithStorageV2`
- `addMembersWithStorageV2`
- `removeMembersWithStorage`
- `swapMembersWithStorage`
- `selfUpdateWithStorage`
- `joinGroupFromWelcomeWithStorageV2`
- `processMessageWithStorageV2`

These operations are already common to both models and remain common:

- `createKeyPackageWithStorage`
- `createMessageWithStorage`
- `deleteGroupWithStorage`
- `mlsStorageFormatVersion`

There is intentionally no separate group-message encryption method. MLS
application-message creation is independent of whether the consumer presents
the conversation as direct or group chat.

## Frozen target surface for the major release

The strict semantics take the unsuffixed base names:

- `createGroupWithStorage` returns the strict
  `CreateGroupWithStorageResult` containing group ID, resulting roster and
  caller-owned batch;
- `addMembersWithStorage` returns `PreparedCommitWithStorageResult` and
  requires the exact authorized additions, AAD and canonical previous roster;
- `joinGroupFromWelcomeWithStorage` returns the strict
  `JoinGroupWithStorageResult` and requires the canonical expected resulting
  roster; and
- `processMessageWithStorage` returns the strict
  `ProcessMessageWithStorageResult` and requires expected AAD plus canonical
  previous and resulting rosters.

The major release deletes:

- the four weak implementations and their legacy-only result definitions;
- `createGroupWithStorageV2`, `addMembersWithStorageV2`,
  `joinGroupFromWelcomeWithStorageV2` and
  `processMessageWithStorageV2`; and
- `CreateGroupWithStorageV2Result`, `JoinGroupWithStorageV2Result` and
  `ProcessMessageWithStorageV2Result` after their strict fields move to the
  unsuffixed result names.

`PreparedCommitWithStorageResult` remains the common result for add, remove,
swap and self-update. No legacy or `V2` alias remains after the cut.

## Why the strict surface is the base

The strict API supplies guarantees that the compatibility API cannot safely
provide for a variable roster:

- an explicit server-issued MLS group ID and exact owner installation
  authority at creation;
- deterministic group, epoch, leaf-index, Basic Credential identity and
  signature-key roster evidence;
- authenticated previous-roster and exact addition/removal authority;
- OpenMLS-selected resulting roster evidence for canonical server acceptance;
- strict canonical resulting-roster validation during Welcome join and
  received processing;
- removal, atomic installation replacement and local self-update;
- deferred Commit candidate batches bound to the exact base group-state
  digest; and
- previous/resulting epoch reporting and exact AAD validation for received
  application, Proposal and Commit messages.

Using the weaker surface as the common base would either lose these checks or
require consumer code to reproduce MLS tree and credential validation. The
migration must therefore move direct E2EE to the strict contract, not extend
the compatibility methods with another set of optional arguments.

## Permanent package boundary

The unified API remains a cryptographic and caller-owned-transaction boundary.
It does not gain knowledge of:

- direct versus group product type;
- minimum or maximum product roster size;
- user membership, roles, invitations or blocking;
- transition leases, server acceptance or mailbox ordering;
- network requests, SQLite transactions or UI state; or
- Chat incarnation identifiers, which remain authenticated outside the MLS
  roster digest.

The package continues to receive exact authenticated authority and returns a
complete batch for the caller to retain, apply atomically or discard. It must
not regain a database, retained mutable `MlsGroup`, or server workflow.

## Direct-chat consumer rule

After migration, Chat and Flutter enforce the direct-chat invariant rather
than relying on a direct-only package method:

1. Chat allocates the explicit MLS group identity and authenticates the owner
   installation.
2. Flutter prepares owner-only initialization with the strict create
   operation and does not make it send-usable before canonical acceptance.
3. The peer is admitted through one authorized add Commit and Welcome.
4. The canonical send-usable direct roster contains exactly the two authorized
   installation leaves.
5. Installation replacement uses one atomic `swapMembersWithStorage` Commit so
   the accepted result still contains exactly two leaves.
6. A direct product flow does not accept an ordinary add that would create a
   third leaf or a removal that would leave a send-usable one-leaf group.

Those are application invariants. The same package operations remain available
to group chat for larger authorized rosters.

## Coordinated migration sequence

There is no backward-compatible runtime phase. The security-sensitive
package-facing semantics are frozen, so the mechanical fork cut may be
implemented and reviewed before Chat/Flutter finish their own migrations.
Consumers must not pin the major release until their coordinated contract work
is ready.

### 1. Freeze one Chat/Flutter authority contract

Chat and Flutter froze the shared installation-leaf, owner initialization,
roster transition, candidate acceptance, Welcome, mailbox dependency,
readiness, recovery and AAD contract. Direct and group use the same vocabulary;
direct adds only the exact two-leaf send-admission rule.

The remaining consumer work is implementation, not an unsettled package
contract: Chat must expose the agreed signature-key, initialization, roster,
Welcome/mailbox and AAD authority, and Flutter must consume that authority.

### 2. Replace Chat's pair-shaped MLS core

Chat implements canonical roster/transition authority as the only model. It
does not retain dual routes, dual writes or compatibility columns for the
undeployed direct model. Normal versioned schema migrations keep local and CI
environments reproducible; the agreed development reset may clear incompatible
unreleased data.

The direct product becomes the two-leaf specialization. Routine installation
replacement becomes a transition using one atomic swap Commit rather than
tombstoning the incarnation.

### 3. Prepare and verify the fork major cut — in progress

On issue #28 and its dedicated branch, the fork promotes the strict functions and result
types to the frozen unsuffixed names, deletes both weak and `V2` symbols,
regenerates FRB bindings, and verifies that storage format remains `1`.

The rename/removal must not change the already-reviewed 2.1.0 cryptographic
semantics or error behavior. Chat suggested optionally echoing credential
identity and signature public key from KeyPackage creation, but both values are
already authenticated inputs and the strict add/join checks remain authority;
the planned cut therefore does not add that redundant result data.

### 4. Migrate Flutter once to the strict base API

Flutter implements one product-neutral MLS adapter and the shared inactive
initialization/Commit candidate, canonical roster, readiness and atomic
promotion authority. Its four production legacy call sites move directly to
the major unsuffixed API. No runtime legacy/strict branch or package shim is
retained.

Development may use 2.1.0's already-released strict `V2` operations to prove
semantics before the final name-only patch, but that is temporary branch
sequencing, not a shipped compatibility contract.

### 5. Release and pin atomically

After cross-repository fixtures and direct/group evidence pass, publish one
signed and attested major native release for Android, iOS, macOS, Linux,
Windows and Web. Flutter updates the exact package revision, lockfile, native
notice guard and adapter names together. Chat raises its client-contract
revision so an old development build fails explicitly rather than speaking the
removed wire contract.

## Compatibility and release consequences

The consumer migration does not change OpenMLS storage format version `1`;
both released surfaces operate on the same opaque rows. Flutter needs normal
forward migrations for its new candidate/roster/readiness projections, and
Chat needs versioned schema changes for its canonical roster/transition
authority. Neither is a package storage migration.

Removing or renaming bridge functions changes the public Dart API and native
symbol set even if storage bytes remain compatible. The removal release must:

- use a major package/native version;
- regenerate and commit all bridge bindings;
- retain direct and variable-roster interop/regression coverage;
- publish one complete signed and attested all-platform release; and
- require consumers to pin the exact reviewed merge revision and matching
  native release.

No existing immutable release is rewritten.

## Guardrails

- Do not put the two-installation rule in Rust or infer conversation type from
  roster size.
- Do not preserve a weaker validation path for direct chat after migration.
- Do not predict OpenMLS leaf indexes in Chat or Flutter.
- Do not apply owner initialization or a prepared Commit batch before its
  application authority is canonical.
- Do not regenerate a Commit for an exact retry.
- Do not treat epoch equality as a replacement for the candidate base-state
  digest.
- Do not add Chat incarnation to roster-summary v1; authenticate it in the
  application contract.
- Do not publish the major cut until the only consumer passes the coordinated
  strict direct/group evidence.

## Completion criteria

This future migration is complete only when:

1. direct and group E2EE use one strict cryptographic adapter;
2. two-party behavior is enforced entirely by consumer authority;
3. the only supported consumer calls no weak or `V2` function/result symbol;
4. the weak and `V2` surfaces are removed in the major release;
5. direct, group, native, Web and all-platform release evidence is green; and
6. README, CHANGELOG and consumer handoffs describe one MLS API with product
   roster policies layered above it.

## Reasoning record

MLS is natively a group protocol. A two-member conversation does not require a
second cryptographic abstraction. Keeping separate permanent surfaces would
duplicate validation and encourage security behavior to diverge as one side
evolves.

The 2.1.0 additive split reduced migration risk while direct E2EE already used
the earlier API. Now that the sole-consumer scope is explicit, retaining a
deprecation or compatibility layer would add work without protecting a real
user. The coordinated major cut keeps release semantics honest while removing
the temporary distinction completely.
