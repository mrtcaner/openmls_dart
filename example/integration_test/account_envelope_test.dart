import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:openmls/openmls.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(Openmls.init);
  tearDownAll(Openmls.cleanup);

  testWidgets('native account-envelope seal/open fails closed on tampering', (
    tester,
  ) async {
    final senderAccountId = _uuid(0x12);
    final senderRootId = _uuid(0x22);
    final recipientAccountId = _uuid(0x32);
    final recipientRootId = _uuid(0x42);
    final sender = _generateInitial(senderAccountId, senderRootId);
    final recipient = _generateInitial(recipientAccountId, recipientRootId);
    final authority = ContextInvitationAuthorityInputV1(
      envelopeId: _uuid(0x52),
      inviteId: _uuid(0x62),
      senderAccountId: senderAccountId,
      senderGeneration: BigInt.one,
      recipientAccountId: recipientAccountId,
      recipientGeneration: BigInt.one,
      authorityAttempt: BigInt.one,
      relaySlotVersion: BigInt.one,
      serverCreatedAtUnixMs: BigInt.from(1_800_000_000_000),
      serverExpiresAtUnixMs: BigInt.from(1_800_000_060_000),
      paddingClass: AccountEnvelopePaddingClassV1.bytes512,
    );
    final expected = ExpectedContextInvitationAuthorityInputV1(
      invitation: authority,
      localRootInstallationId: recipientRootId,
      localRootAuthorityGeneration: BigInt.one,
    );
    final envelope = AccountEnvelopeCrypto.sealContextInvitationPreviewV1(
      authority: authority,
      expectedLocalPrivateBundleAuthority: _privateAuthority(
        senderAccountId,
        senderRootId,
      ),
      preview: const ContextInvitationPreviewInputV1(
        title: '  Cafe\u0301  ',
        tags: <String>[' News ', 'Technology'],
      ),
      recipientPublicBundle: recipient.publicBundle,
      senderPrivateBundle: sender.privateBundle,
    );
    expect(envelope, hasLength(741));

    final opened =
        AccountEnvelopeCrypto.verifyAndOpenContextInvitationPreviewV1(
          envelope: envelope,
          expectedAuthority: expected,
          recipientPrivateBundle: recipient.privateBundle,
          senderPublicBundle: sender.publicBundle,
        );
    expect(opened.title, 'Café');
    expect(opened.tags, <String>['News', 'Technology']);

    final tampered = Uint8List.fromList(envelope)..last ^= 1;
    expect(
      () => AccountEnvelopeCrypto.verifyAndOpenContextInvitationPreviewV1(
        envelope: tampered,
        expectedAuthority: expected,
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
}

({Uint8List privateBundle, Uint8List publicBundle}) _generateInitial(
  Uint8List accountId,
  Uint8List rootInstallationId,
) {
  final generated = AccountEnvelopeCrypto.generateKeyBundleV1(
    accountId: accountId,
    generation: BigInt.one,
    rootInstallationId: rootInstallationId,
    rootAuthorityGeneration: BigInt.one,
  );
  final public = AccountEnvelopeCrypto.createSelfSignedPublicBundleV1(
    accountId: accountId,
    generation: BigInt.one,
    activationKind: AccountEnvelopeActivationKindV1.initial,
    previousGeneration: BigInt.zero,
    expectedLocalPrivateBundleAuthority: _privateAuthority(
      accountId,
      rootInstallationId,
    ),
    privateBundle: generated.privateBundle,
  );
  expect(
    public.kind,
    AccountEnvelopePublicBundleCandidateKindV1.canonicalPublicBundle,
  );
  return (privateBundle: generated.privateBundle, publicBundle: public.bytes);
}

AccountEnvelopePrivateBundleAuthorityInputV1 _privateAuthority(
  Uint8List accountId,
  Uint8List rootInstallationId,
) => AccountEnvelopePrivateBundleAuthorityInputV1(
  accountId: accountId,
  generation: BigInt.one,
  rootInstallationId: rootInstallationId,
  rootAuthorityGeneration: BigInt.one,
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
