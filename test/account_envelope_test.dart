import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:openmls/openmls.dart';
import 'package:test/test.dart';

final _senderAccountId = _uuid(0x12);
final _senderRootId = _uuid(0x22);
final _recipientAccountId = _uuid(0x32);
final _recipientRootId = _uuid(0x42);
final _envelopeId = _uuid(0x52);
final _inviteId = _uuid(0x62);

void main() {
  setUpAll(Openmls.init);
  tearDownAll(Openmls.cleanup);

  test('generated high-level API seals and opens a normalized preview', () {
    final sender = _generateInitial(_senderAccountId, _senderRootId);
    final recipient = _generateInitial(_recipientAccountId, _recipientRootId);
    final authority = _invitationAuthority();

    final envelope = AccountEnvelopeCrypto.sealContextInvitationPreviewV1(
      authority: authority,
      expectedLocalPrivateBundleAuthority: _privateAuthority(
        _senderAccountId,
        _senderRootId,
      ),
      preview: const ContextInvitationPreviewInputV1(
        title: '  Cafe\u0301  ',
        tags: <String>[' News ', 'Technology'],
      ),
      recipientPublicBundle: recipient.publicBundle,
      senderPrivateBundle: sender.privateBundle,
    );

    final opened =
        AccountEnvelopeCrypto.verifyAndOpenContextInvitationPreviewV1(
          envelope: envelope,
          expectedAuthority: ExpectedContextInvitationAuthorityInputV1(
            invitation: authority,
            localRootInstallationId: _recipientRootId,
            localRootAuthorityGeneration: BigInt.one,
          ),
          recipientPrivateBundle: recipient.privateBundle,
          senderPublicBundle: sender.publicBundle,
        );

    expect(opened.title, 'Café');
    expect(opened.tags, <String>['News', 'Technology']);
  });

  test('generated typed error rejects tampering without plaintext', () {
    final sender = _generateInitial(_senderAccountId, _senderRootId);
    final recipient = _generateInitial(_recipientAccountId, _recipientRootId);
    final authority = _invitationAuthority();
    final envelope = AccountEnvelopeCrypto.sealContextInvitationPreviewV1(
      authority: authority,
      expectedLocalPrivateBundleAuthority: _privateAuthority(
        _senderAccountId,
        _senderRootId,
      ),
      preview: const ContextInvitationPreviewInputV1(
        title: 'Private preview',
        tags: <String>[],
      ),
      recipientPublicBundle: recipient.publicBundle,
      senderPrivateBundle: sender.privateBundle,
    );
    envelope[envelope.length - 1] ^= 1;

    expect(
      () => AccountEnvelopeCrypto.verifyAndOpenContextInvitationPreviewV1(
        envelope: envelope,
        expectedAuthority: ExpectedContextInvitationAuthorityInputV1(
          invitation: authority,
          localRootInstallationId: _recipientRootId,
          localRootAuthorityGeneration: BigInt.one,
        ),
        recipientPrivateBundle: recipient.privateBundle,
        senderPublicBundle: sender.publicBundle,
      ),
      throwsA(
        isA<AccountEnvelopeErrorV1>().having(
          (error) => error.code,
          'code',
          AccountEnvelopeErrorCodeV1.signatureInvalid,
        ),
      ),
    );
  });

  test('rotation stays nonpublishable until predecessor authorization', () {
    final initial = _generateInitial(_senderAccountId, _senderRootId);
    final successorKeys = AccountEnvelopeCrypto.generateKeyBundleV1(
      accountId: _senderAccountId,
      generation: BigInt.two,
      rootInstallationId: _senderRootId,
      rootAuthorityGeneration: BigInt.one,
    );
    final successorAuthority = AccountEnvelopePrivateBundleAuthorityInputV1(
      accountId: _senderAccountId,
      generation: BigInt.two,
      rootInstallationId: _senderRootId,
      rootAuthorityGeneration: BigInt.one,
    );
    final candidate = AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
      accountId: _senderAccountId,
      generation: BigInt.two,
      activationKind: AccountEnvelopeActivationKindV1.rotation,
      previousGeneration: BigInt.one,
      expectedLocalPrivateBundleAuthority: successorAuthority,
      privateBundle: successorKeys.privateBundle,
    );
    expect(
      candidate.kind,
      AccountEnvelopePublicBundleCandidateKindV1
          .nonPublishableRotationCandidate,
    );

    final authorized = AccountEnvelopeCrypto.authorizeSuccessorPublicBundleV1(
      expectedPreviousLocalPrivateBundleAuthority: _privateAuthority(
        _senderAccountId,
        _senderRootId,
      ),
      previousPrivateBundle: initial.privateBundle,
      selfSignedSuccessorPublicBundle: candidate.bytes,
    );
    final continuity = AccountEnvelopeCrypto.verifyContinuityResponseV1(
      pinnedPublicBundle: initial.publicBundle,
      continuityPublicBundles: <Uint8List>[
        authorized.authorizedCanonicalSuccessorPublicBundle,
      ],
      manualReanchorAuthorized: false,
    );

    expect(
      continuity.disposition,
      AccountEnvelopeContinuityDispositionV1.rotationChainVerified,
    );
    expect(continuity.verifiedSummary.generation, BigInt.two);
    expect(authorized.retiredPreviousPrivateBundleCandidate, hasLength(155));
  });

  test(
    'committed bundle and Unicode fixture replays through generated API',
    () {
      final fixture =
          jsonDecode(
                File(
                  'test/fixtures/account_envelope_v1.json',
                ).readAsStringSync(),
              )
              as Map<String, Object?>;
      expect(fixture['format'], 'openmls_dart/account-envelope-fixtures/v1');

      final accountId = _fixtureHex(fixture, 'accountIdHex');
      final rootId = _fixtureHex(fixture, 'rootInstallationIdHex');
      final privateBundles =
          fixture['privateBundlesHex']! as Map<String, Object?>;
      final publicBundles =
          fixture['publicBundlesHex']! as Map<String, Object?>;
      final digests = fixture['digestsSha256Hex']! as Map<String, Object?>;
      final firstPrivate = _hexBytes(privateBundles['generation1']! as String);
      final secondPrivate = _hexBytes(privateBundles['generation2']! as String);
      final thirdPrivate = _hexBytes(privateBundles['generation3']! as String);

      final initial = AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
        accountId: accountId,
        generation: BigInt.one,
        activationKind: AccountEnvelopeActivationKindV1.initial,
        previousGeneration: BigInt.zero,
        expectedLocalPrivateBundleAuthority: _privateAuthority(
          accountId,
          rootId,
        ),
        privateBundle: firstPrivate,
      );
      expect(initial.bytes, _hexBytes(publicBundles['initial']! as String));

      final rotationCandidate =
          AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
            accountId: accountId,
            generation: BigInt.two,
            activationKind: AccountEnvelopeActivationKindV1.rotation,
            previousGeneration: BigInt.one,
            expectedLocalPrivateBundleAuthority: _privateAuthority(
              accountId,
              rootId,
              generation: BigInt.two,
            ),
            privateBundle: secondPrivate,
          );
      expect(
        rotationCandidate.kind,
        AccountEnvelopePublicBundleCandidateKindV1
            .nonPublishableRotationCandidate,
      );
      expect(
        rotationCandidate.bytes,
        _hexBytes(publicBundles['rotationCandidate']! as String),
      );

      final authorization =
          AccountEnvelopeCrypto.authorizeSuccessorPublicBundleV1(
            expectedPreviousLocalPrivateBundleAuthority: _privateAuthority(
              accountId,
              rootId,
            ),
            previousPrivateBundle: firstPrivate,
            selfSignedSuccessorPublicBundle: rotationCandidate.bytes,
          );
      expect(
        authorization.authorizedCanonicalSuccessorPublicBundle,
        _hexBytes(publicBundles['authorizedRotation']! as String),
      );
      expect(
        authorization.retiredPreviousPrivateBundleCandidate,
        _fixtureHex(fixture, 'retiredGeneration1PrivateBundleHex'),
      );

      final reset = AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
        accountId: accountId,
        generation: BigInt.from(3),
        activationKind: AccountEnvelopeActivationKindV1.continuityReset,
        resetReason: AccountEnvelopeResetReasonV1.accountRecovery,
        previousGeneration: BigInt.two,
        expectedLocalPrivateBundleAuthority: _privateAuthority(
          accountId,
          rootId,
          generation: BigInt.from(3),
        ),
        privateBundle: thirdPrivate,
      );
      expect(
        reset.bytes,
        _hexBytes(publicBundles['continuityReset']! as String),
      );

      final firstObserved = AccountEnvelopeCrypto.verifyContinuityResponseV1(
        continuityPublicBundles: <Uint8List>[initial.bytes],
        manualReanchorAuthorized: false,
      );
      expect(
        firstObserved.verifiedSummary.digestSha256,
        _hexBytes(digests['initial']! as String),
      );
      final rotated = AccountEnvelopeCrypto.verifyContinuityResponseV1(
        pinnedPublicBundle: initial.bytes,
        continuityPublicBundles: <Uint8List>[
          authorization.authorizedCanonicalSuccessorPublicBundle,
        ],
        manualReanchorAuthorized: false,
      );
      expect(
        rotated.verifiedSummary.digestSha256,
        _hexBytes(digests['authorizedRotation']! as String),
      );
      final resetObserved = AccountEnvelopeCrypto.verifyContinuityResponseV1(
        pinnedPublicBundle:
            authorization.authorizedCanonicalSuccessorPublicBundle,
        continuityPublicBundles: <Uint8List>[reset.bytes],
        manualReanchorAuthorized: false,
      );
      expect(
        resetObserved.disposition,
        AccountEnvelopeContinuityDispositionV1.resetAnchorRequiresAcceptance,
      );
      expect(
        resetObserved.verifiedSummary.digestSha256,
        _hexBytes(digests['continuityReset']! as String),
      );

      final unicode = fixture['unicode17']! as List<Object?>;
      final normalized = unicode[0]! as Map<String, Object?>;
      final sender = _generateInitial(_senderAccountId, _senderRootId);
      final recipient = _generateInitial(_recipientAccountId, _recipientRootId);
      final authority = _invitationAuthority();
      final envelope = AccountEnvelopeCrypto.sealContextInvitationPreviewV1(
        authority: authority,
        expectedLocalPrivateBundleAuthority: _privateAuthority(
          _senderAccountId,
          _senderRootId,
        ),
        preview: ContextInvitationPreviewInputV1(
          title: normalized['inputTitle']! as String,
          tags: (normalized['inputTags']! as List<Object?>).cast<String>(),
        ),
        recipientPublicBundle: recipient.publicBundle,
        senderPrivateBundle: sender.privateBundle,
      );
      final opened =
          AccountEnvelopeCrypto.verifyAndOpenContextInvitationPreviewV1(
            envelope: envelope,
            expectedAuthority: ExpectedContextInvitationAuthorityInputV1(
              invitation: authority,
              localRootInstallationId: _recipientRootId,
              localRootAuthorityGeneration: BigInt.one,
            ),
            recipientPrivateBundle: recipient.privateBundle,
            senderPublicBundle: sender.publicBundle,
          );
      expect(opened.title, normalized['normalizedTitle']);
      expect(opened.tags, normalized['normalizedTags']);

      final duplicate = unicode[1]! as Map<String, Object?>;
      expect(
        () => AccountEnvelopeCrypto.sealContextInvitationPreviewV1(
          authority: authority,
          expectedLocalPrivateBundleAuthority: _privateAuthority(
            _senderAccountId,
            _senderRootId,
          ),
          preview: ContextInvitationPreviewInputV1(
            tags: (duplicate['duplicateTags']! as List<Object?>).cast<String>(),
          ),
          recipientPublicBundle: recipient.publicBundle,
          senderPrivateBundle: sender.privateBundle,
        ),
        throwsA(
          isA<AccountEnvelopeErrorV1>().having(
            (error) => error.code,
            'code',
            AccountEnvelopeErrorCodeV1.plaintextSchemaInvalid,
          ),
        ),
      );
    },
  );

  test(
    'bridge rejects malformed authority and private frames with typed errors',
    () {
      expect(
        () => AccountEnvelopeCrypto.generateKeyBundleV1(
          accountId: Uint8List(15),
          generation: BigInt.one,
          rootInstallationId: _senderRootId,
          rootAuthorityGeneration: BigInt.one,
        ),
        throwsA(
          isA<AccountEnvelopeErrorV1>().having(
            (error) => error.code,
            'code',
            AccountEnvelopeErrorCodeV1.authorityMismatch,
          ),
        ),
      );

      expect(
        () => AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
          accountId: _senderAccountId,
          generation: BigInt.one,
          activationKind: AccountEnvelopeActivationKindV1.initial,
          previousGeneration: BigInt.zero,
          expectedLocalPrivateBundleAuthority: _privateAuthority(
            _senderAccountId,
            _senderRootId,
          ),
          privateBundle: Uint8List(256),
        ),
        throwsA(
          isA<AccountEnvelopeErrorV1>().having(
            (error) => error.code,
            'code',
            AccountEnvelopeErrorCodeV1.privateBundleInvalid,
          ),
        ),
      );
    },
  );
}

({Uint8List privateBundle, Uint8List publicBundle}) _generateInitial(
  Uint8List accountId,
  Uint8List rootId,
) {
  final generated = AccountEnvelopeCrypto.generateKeyBundleV1(
    accountId: accountId,
    generation: BigInt.one,
    rootInstallationId: rootId,
    rootAuthorityGeneration: BigInt.one,
  );
  final publicBundle = AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
    accountId: accountId,
    generation: BigInt.one,
    activationKind: AccountEnvelopeActivationKindV1.initial,
    previousGeneration: BigInt.zero,
    expectedLocalPrivateBundleAuthority: _privateAuthority(accountId, rootId),
    privateBundle: generated.privateBundle,
  );
  expect(
    publicBundle.kind,
    AccountEnvelopePublicBundleCandidateKindV1.canonicalPublicBundle,
  );
  return (
    privateBundle: generated.privateBundle,
    publicBundle: publicBundle.bytes,
  );
}

AccountEnvelopePrivateBundleAuthorityInputV1 _privateAuthority(
  Uint8List accountId,
  Uint8List rootId, {
  BigInt? generation,
}) => AccountEnvelopePrivateBundleAuthorityInputV1(
  accountId: accountId,
  generation: generation ?? BigInt.one,
  rootInstallationId: rootId,
  rootAuthorityGeneration: BigInt.one,
);

ContextInvitationAuthorityInputV1 _invitationAuthority() =>
    ContextInvitationAuthorityInputV1(
      envelopeId: _envelopeId,
      inviteId: _inviteId,
      senderAccountId: _senderAccountId,
      senderGeneration: BigInt.one,
      recipientAccountId: _recipientAccountId,
      recipientGeneration: BigInt.one,
      authorityAttempt: BigInt.one,
      relaySlotVersion: BigInt.one,
      serverCreatedAtUnixMs: BigInt.from(1_800_000_000_000),
      serverExpiresAtUnixMs: BigInt.from(1_800_000_060_000),
      paddingClass: AccountEnvelopePaddingClassV1.bytes2048,
    );

Uint8List _uuid(int firstByte) => Uint8List.fromList(<int>[
  firstByte,
  0x34,
  0x56,
  0x78,
  0x12,
  0x34,
  0x47,
  0x89,
  0x8a,
  0xbc,
  0xde,
  0xf0,
  0x12,
  0x34,
  0x56,
  0x78,
]);

Uint8List _fixtureHex(Map<String, Object?> fixture, String field) =>
    _hexBytes(fixture[field]! as String);

Uint8List _hexBytes(String input) => Uint8List.fromList(<int>[
  for (var index = 0; index < input.length; index += 2)
    int.parse(input.substring(index, index + 2), radix: 16),
]);
