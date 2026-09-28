import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:openmls/openmls.dart';
import 'package:test/test.dart';

import 'test_helpers.dart';

void main() {
  setUpAll(() async {
    await Openmls.init();
  });

  group('caller-owned MLS storage', () {
    test('creates key packages without writing durable state', () async {
      final identity = TestIdentity.create('external-storage');
      final store = _MemoryMlsStore();

      final first = await _createKeyPackage(identity, store.snapshot);

      expect(first.keyPackageBytes, isNotEmpty);
      expect(first.storageBatch.storageFormatVersion, store.formatVersion);
      expect(first.storageBatch.upserts, isNotEmpty);
      expect(
        first.storageBatch.upserts.every((entry) => entry.groupId == null),
        isTrue,
      );
      expect(store.snapshot, isEmpty, reason: 'the Rust call must not persist');

      store.apply(first.storageBatch);
      final persistedCount = store.snapshot.length;
      expect(persistedCount, greaterThan(0));

      final discarded = await _createKeyPackage(identity, store.snapshot);
      expect(discarded.storageBatch.upserts, isNotEmpty);
      expect(
        store.snapshot.length,
        persistedCount,
        reason: 'discarding a batch must leave caller state unchanged',
      );

      final retried = await _createKeyPackage(identity, store.snapshot);
      expect(retried.keyPackageBytes, isNotEmpty);
      expect(
        store.snapshot.length,
        persistedCount,
        reason: 'retrying from the same snapshot must not retain Rust state',
      );
    });

    test(
      'rejects an unknown storage format before running the operation',
      () async {
        final identity = TestIdentity.create('wrong-format');

        await expectLater(
          createKeyPackageWithStorage(
            ciphersuite: ciphersuite,
            signerBytes: identity.signerBytes,
            credentialIdentity: identity.credentialIdentity,
            signerPublicKey: identity.publicKey,
            storageEntries: const [],
            storageFormatVersion: 999,
          ),
          throwsA(
            predicate<Object>(
              (error) => error.toString().contains(
                'Unsupported MLS storage format version',
              ),
            ),
          ),
        );
      },
    );

    test(
      'rejects duplicate opaque keys instead of silently overwriting',
      () async {
        final identity = TestIdentity.create('duplicate-key');
        final first = await _createKeyPackage(identity, const []);
        final entry = first.storageBatch.upserts.first;

        await expectLater(
          _createKeyPackage(identity, [entry, entry]),
          throwsA(
            predicate<Object>(
              (error) => error.toString().contains('Duplicate MLS storage key'),
            ),
          ),
        );
      },
    );

    test(
      'owner initialization validates explicit credential and default retention',
      () async {
        final owner = _Peer('owner', 2);
        final initialized = await owner.create();
        expect(initialized.effectiveRetention, 2);
        expect(defaultConfig().maxPastEpochs, 2);
        expect(initialized.resultingRoster.epoch, BigInt.zero);
        expect(initialized.resultingRoster.leaves, hasLength(1));
        expect(
          initialized.resultingGroupStateSha256,
          orderedEquals(owner.digest),
        );
        final other = _Peer('other', 2);
        final invalid = await createGroupWithStorage(
          config: defaultConfig(),
          signerBytes: owner.identity.signerBytes,
          explicitGroupId: _gid,
          incarnationId: _incarnation,
          expectedOwnerAuthority: owner.authority,
          credentialBytes: other.identity.serializedCredential,
          storageEntries: const [],
          storageFormatVersion: 1,
        );
        expect(
          invalid,
          const CreateGroupWithStorageOutcome.failure(
            MlsErrorCode.localLeafMismatch,
          ),
        );
      },
    );

    test(
      'KeyPackage credential and signature authority cannot be substituted',
      () async {
        final a = _Peer('a', 2);
        final b = _Peer('b', 2);
        final wrong = _Peer('wrong', 2);
        await a.create();
        await b.keyPackage();
        final before = a.store.fingerprint;
        for (final bad in [
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: b.keyPackageBytes!,
            expectedCredentialIdentity: wrong.identity.credentialIdentity,
            expectedSignaturePublicKey: b.identity.publicKey,
          ),
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: b.keyPackageBytes!,
            expectedCredentialIdentity: b.identity.credentialIdentity,
            expectedSignaturePublicKey: wrong.identity.publicKey,
          ),
        ]) {
          final outcome = await addMembersWithStorage(
            context: a.context,
            transition: _transition(),
            signerBytes: a.identity.signerBytes,
            authorizedKeyPackages: [bad],
            aad: _aad,
          );
          expect(
            outcome,
            const AddMembersWithStorageOutcome.failure(
              MlsErrorCode.mlsProtocolRejected,
            ),
          );
          expect(a.store.fingerprint, before);
        }
      },
    );

    test(
      'pending batch persists immediately; live receive survives exact merge',
      () async {
        final (a, b) = await _pair();
        final prior = a.roster;
        final p = await a.prepareSelf();
        expect(p.previousRoster.epoch, prior.epoch);
        expect(p.proposedResultingRoster.epoch, prior.epoch + BigInt.one);
        expect(a.roster.epoch, prior.epoch);
        final retry =
            (await getPendingCommitWithStorage(context: a.context)
                    as GetPendingCommitWithStorageOutcome_Success)
                .field0!;
        expect(retry.commit, orderedEquals(p.commit));
        expect(
          retry.preparationBaseGroupStateSha256,
          orderedEquals(p.preparationBaseGroupStateSha256),
        );
        expect(
          await createMessageWithStorage(
            context: a.context,
            signerBytes: a.identity.signerBytes,
            message: [1],
            aad: _aad,
          ),
          const CreateMessageWithStorageOutcome.failure(
            MlsErrorCode.pendingCommitExists,
          ),
        );
        // Only A pauses outgoing generation. B continues, and A persists B's receive.
        final incoming = await b.send('while pending');
        final receive = await a.receive(incoming);
        expect(utf8.decode(receive.applicationMessage!), 'while pending');
        expect(a.digest, isNot(orderedEquals(p.resultingGroupStateSha256)));
        final wrong = MlsCommitAcceptance(
          binding: p.pendingBinding,
          resultingState: _expected(p.proposedResultingRoster),
          preparationBaseGroupStateSha256: Uint8List(32),
        );
        expect(
          await mergePendingCommitWithStorage(
            context: a.context,
            acceptance: wrong,
          ),
          const MergePendingCommitWithStorageOutcome.failure(
            MlsErrorCode.acceptanceBindingMismatch,
          ),
        );
        await a.accept(p);
        expect(
          await getPendingCommitWithStorage(context: a.context),
          const GetPendingCommitWithStorageOutcome.success(),
        );
        expect(
          await a.receiveOutcome(incoming),
          const ProcessMessageWithStorageOutcome.failure(MlsErrorCode.replay),
        );
        await b.receiveCommit(p);
        expect(
          (await b.receive(await a.send('new epoch'))).applicationMessage,
          orderedEquals(utf8.encode('new epoch')),
        );
      },
    );

    test(
      'definitive discard preserves receive state and never treats missing as success',
      () async {
        final (a, b) = await _pair();
        final p = await a.prepareSelf();
        final incoming = await b.send('survives discard');
        await a.receive(incoming);
        final d =
            (await discardPendingCommitWithStorage(
                      context: a.context,
                      expectedPendingBinding: p.pendingBinding,
                    )
                    as DiscardPendingCommitWithStorageOutcome_Success)
                .field0;
        a.apply(d.storageBatch, d.resultingRoster, d.resultingGroupStateSha256);
        expect(
          await a.receiveOutcome(incoming),
          const ProcessMessageWithStorageOutcome.failure(MlsErrorCode.replay),
        );
        expect(
          await discardPendingCommitWithStorage(
            context: a.context,
            expectedPendingBinding: p.pendingBinding,
          ),
          const DiscardPendingCommitWithStorageOutcome.failure(
            MlsErrorCode.pendingCommitMissing,
          ),
        );
        await a.send('send resumed');
      },
    );

    test(
      'local 2/4 retention gives independent historical receive windows',
      () async {
        final (a, b) = await _pair(receiverRetention: 4);
        final fromA = await a.send('age three');
        final fromB = await b.send('expires at A');
        for (var i = 0; i < 3; i++) {
          final p = await a.prepareSelf();
          await a.accept(p);
          await b.receiveCommit(p);
        }
        final received = await b.receive(fromA);
        expect(received.messageEpoch, fromA.state.epoch);
        expect(received.previousEpoch, b.roster.epoch);
        expect(received.resultingEpoch, b.roster.epoch);
        expect(received.effectiveRetention, 4);
        expect(
          await a.receiveOutcome(fromB),
          const ProcessMessageWithStorageOutcome.failure(
            MlsErrorCode.pastEpochUnavailable,
          ),
        );
        final c = b.context;
        expect(
          await getPendingCommitWithStorage(
            context: MlsGroupOperationContext(
              groupId: c.groupId,
              incarnationId: c.incarnationId,
              expectedCurrentState: c.expectedCurrentState,
              expectedBaseGroupStateSha256: c.expectedBaseGroupStateSha256,
              expectedRetention: 2,
              storageEntries: c.storageEntries,
              storageFormatVersion: 1,
            ),
          ),
          const GetPendingCommitWithStorageOutcome.failure(
            MlsErrorCode.configurationMismatch,
          ),
        );
      },
    );

    test(
      'wrong receive authority returns no batch and a valid retry still works',
      () async {
        final (a, b) = await _pair();
        final incoming = await a.send('retry');
        final before = b.store.fingerprint;
        expect(
          await b.receiveOutcome(incoming, aad: [1]),
          const ProcessMessageWithStorageOutcome.failure(
            MlsErrorCode.aadMismatch,
          ),
        );
        expect(
          await b.receiveOutcome(incoming, hash: Uint8List(32)),
          const ProcessMessageWithStorageOutcome.failure(
            MlsErrorCode.wireHashMismatch,
          ),
        );
        expect(
          await b.receiveOutcome(incoming, sender: b.leaf),
          const ProcessMessageWithStorageOutcome.failure(
            MlsErrorCode.senderMismatch,
          ),
        );
        expect(b.store.fingerprint, before);
        expect(
          (await b.receive(incoming)).applicationMessage,
          orderedEquals(utf8.encode('retry')),
        );
        final c = b.context;
        expect(
          await getPendingCommitWithStorage(
            context: MlsGroupOperationContext(
              groupId: c.groupId,
              incarnationId: c.incarnationId,
              expectedCurrentState: c.expectedCurrentState,
              expectedBaseGroupStateSha256: Uint8List(32),
              expectedRetention: c.expectedRetention,
              storageEntries: c.storageEntries,
              storageFormatVersion: 1,
            ),
          ),
          const GetPendingCommitWithStorageOutcome.failure(
            MlsErrorCode.baseStateMismatch,
          ),
        );
      },
    );

    test('atomic swap and removal authenticate exact leaf authority', () async {
      final (a, b) = await _pair();
      final replacement = _Peer('b', 4);
      await replacement.keyPackage();
      final p =
          (await swapMembersWithStorage(
                    context: a.context,
                    transition: _transition(),
                    signerBytes: a.identity.signerBytes,
                    authorizedKeyPackages: [replacement.addition],
                    authorizedRemovals: [b.removal],
                    aad: _aad,
                  )
                  as SwapMembersWithStorageOutcome_Success)
              .field0;
      a.apply(p.storageBatch, p.previousRoster, p.resultingGroupStateSha256);
      await a.accept(p);
      await b.receiveCommit(p);
      await replacement.join(p);
      expect(
        await createMessageWithStorage(
          context: b.context,
          signerBytes: b.identity.signerBytes,
          message: [1],
          aad: _aad,
        ),
        const CreateMessageWithStorageOutcome.failure(
          MlsErrorCode.inactiveGroup,
        ),
      );
      expect(
        (await replacement.receive(
          await a.send('replacement'),
        )).applicationMessage,
        orderedEquals(utf8.encode('replacement')),
      );
      final removed =
          (await removeMembersWithStorage(
                    context: a.context,
                    transition: _transition(),
                    signerBytes: a.identity.signerBytes,
                    authorizedRemovals: [replacement.removal],
                    aad: _aad,
                  )
                  as RemoveMembersWithStorageOutcome_Success)
              .field0;
      a.apply(
        removed.storageBatch,
        removed.previousRoster,
        removed.resultingGroupStateSha256,
      );
      await a.accept(removed);
      await replacement.receiveCommit(removed);
      expect(a.roster.leaves, hasLength(1));
      final deletion = await deleteGroupWithStorage(
        groupId: _gid,
        storageEntries: a.store.forGroup(_gid),
        storageFormatVersion: 1,
      );
      a.store.apply(deletion);
      expect(
        a.store.groupEntries(_gid),
        isEmpty,
        reason: 'includes wrapper rows',
      );
    });

    test(
      'Welcome requires exact retained KeyPackage hash before consuming it',
      () async {
        final a = _Peer('a', 2);
        final b = _Peer('b', 2);
        await a.create();
        await b.keyPackage();
        final p = await a.add([b]);
        await a.accept(p);
        final before = b.store.fingerprint;
        expect(
          await b.joinOutcome(p, hash: Uint8List(32)),
          const JoinGroupFromWelcomeWithStorageOutcome.failure(
            MlsErrorCode.expectedKeyPackageMismatch,
          ),
        );
        expect(b.store.fingerprint, before);
        await b.join(p);
        expect(b.roster.digestSha256, orderedEquals(a.roster.digestSha256));
      },
    );
  });
}

final _gid = Uint8List.fromList(List.filled(16, 54));
final _incarnation = Uint8List.fromList(List.filled(16, 8));
final _aad = Uint8List.fromList(utf8.encode('package-v4-test-aad'));
var _command = 0;
Uint8List _hash(List<int> value) =>
    Uint8List.fromList(sha256.convert(value).bytes);
MlsTransitionContext _transition() => MlsTransitionContext(
  commandId: Uint8List.fromList(utf8.encode('command-${++_command}')),
  contextSha256: Uint8List.fromList(List.filled(32, 7)),
);
MlsExpectedRosterStateV1 _expected(MlsRosterSummaryV1 s) =>
    MlsExpectedRosterStateV1(
      groupId: s.groupId,
      epoch: s.epoch,
      digestSha256: s.digestSha256,
    );
MlsGroupConfig _config(int retention) {
  final c = defaultConfig();
  return MlsGroupConfig(
    ciphersuite: c.ciphersuite,
    wireFormatPolicy: c.wireFormatPolicy,
    useRatchetTreeExtension: c.useRatchetTreeExtension,
    maxPastEpochs: retention,
    paddingSize: c.paddingSize,
    senderRatchetMaxOutOfOrder: c.senderRatchetMaxOutOfOrder,
    senderRatchetMaxForwardDistance: c.senderRatchetMaxForwardDistance,
    numberOfResumptionPsks: c.numberOfResumptionPsks,
  );
}

class _Wire {
  _Wire(this.bytes, this.sender, this.state);
  final List<int> bytes;
  final MlsRosterLeafV1 sender;
  final MlsExpectedRosterStateV1 state;
}

class _Peer {
  _Peer(String name, this.retention)
    : identity = TestIdentity.create(name.padRight(45, '_'));
  final TestIdentity identity;
  final int retention;
  final store = _MemoryMlsStore();
  late MlsRosterSummaryV1 roster;
  Uint8List? keyPackageBytes;
  Uint8List get digest => mlsGroupStateDigest(
    groupId: _gid,
    storageEntries: store.forGroup(_gid),
    storageFormatVersion: 1,
  );
  MlsGroupOperationContext get context => MlsGroupOperationContext(
    groupId: _gid,
    incarnationId: _incarnation,
    expectedCurrentState: _expected(roster),
    expectedBaseGroupStateSha256: digest,
    expectedRetention: retention,
    storageEntries: store.forGroup(_gid),
    storageFormatVersion: 1,
  );
  MlsAuthorizedOwnerV1 get authority => MlsAuthorizedOwnerV1(
    expectedCredentialIdentity: identity.credentialIdentity,
    expectedSignaturePublicKey: identity.publicKey,
  );
  MlsRosterLeafV1 get leaf => roster.leaves.firstWhere(
    (v) =>
        base64Encode(v.signaturePublicKey) == base64Encode(identity.publicKey),
  );
  MlsAuthorizedRemovalV1 get removal => MlsAuthorizedRemovalV1(
    leafIndex: leaf.leafIndex,
    expectedCredentialIdentity: leaf.credentialIdentity,
    expectedSignaturePublicKey: leaf.signaturePublicKey,
  );
  MlsAuthorizedKeyPackageV1 get addition => MlsAuthorizedKeyPackageV1(
    keyPackageBytes: keyPackageBytes!,
    expectedCredentialIdentity: identity.credentialIdentity,
    expectedSignaturePublicKey: identity.publicKey,
  );
  void apply(
    MlsStorageBatch batch,
    MlsRosterSummaryV1 resulting,
    List<int> hash,
  ) {
    store.apply(batch);
    roster = resulting;
    expect(digest, orderedEquals(hash));
    expect(
      mlsRosterDigestV1(
        groupId: roster.groupId,
        epoch: roster.epoch,
        leaves: roster.leaves,
      ),
      orderedEquals(roster.digestSha256),
    );
  }

  Future<CreateGroupWithStorageResult> create() async {
    final c =
        (await createGroupWithStorage(
                  config: _config(retention),
                  signerBytes: identity.signerBytes,
                  explicitGroupId: _gid,
                  incarnationId: _incarnation,
                  expectedOwnerAuthority: authority,
                  credentialBytes: identity.serializedCredential,
                  storageEntries: store.globalSnapshot,
                  storageFormatVersion: 1,
                )
                as CreateGroupWithStorageOutcome_Success)
            .field0;
    apply(c.storageBatch, c.resultingRoster, c.resultingGroupStateSha256);
    return c;
  }

  Future<void> keyPackage() async {
    final k = await _createKeyPackage(identity, store.globalSnapshot);
    store.apply(k.storageBatch);
    keyPackageBytes = k.keyPackageBytes;
  }

  Future<PendingCommitWithStorageResult> add(List<_Peer> peers) async {
    final p =
        (await addMembersWithStorage(
                  context: context,
                  transition: _transition(),
                  signerBytes: identity.signerBytes,
                  authorizedKeyPackages: peers.map((p) => p.addition).toList(),
                  aad: _aad,
                )
                as AddMembersWithStorageOutcome_Success)
            .field0;
    apply(p.storageBatch, p.previousRoster, p.resultingGroupStateSha256);
    return p;
  }

  Future<PendingCommitWithStorageResult> prepareSelf() async {
    final p =
        (await selfUpdateWithStorage(
                  context: context,
                  transition: _transition(),
                  signerBytes: identity.signerBytes,
                  expectedSelfAuthority: MlsAuthorizedSelfV1(
                    leafIndex: leaf.leafIndex,
                    expectedCredentialIdentity: leaf.credentialIdentity,
                    expectedSignaturePublicKey: leaf.signaturePublicKey,
                  ),
                  aad: _aad,
                )
                as SelfUpdateWithStorageOutcome_Success)
            .field0;
    apply(p.storageBatch, p.previousRoster, p.resultingGroupStateSha256);
    return p;
  }

  Future<void> accept(PendingCommitWithStorageResult p) async {
    final m =
        (await mergePendingCommitWithStorage(
                  context: context,
                  acceptance: MlsCommitAcceptance(
                    binding: p.pendingBinding,
                    resultingState: _expected(p.proposedResultingRoster),
                    preparationBaseGroupStateSha256:
                        p.preparationBaseGroupStateSha256,
                  ),
                )
                as MergePendingCommitWithStorageOutcome_Success)
            .field0;
    apply(m.storageBatch, m.resultingRoster, m.resultingGroupStateSha256);
  }

  Future<JoinGroupFromWelcomeWithStorageOutcome> joinOutcome(
    PendingCommitWithStorageResult p, {
    Uint8List? hash,
  }) {
    final local = p.proposedResultingRoster.leaves.firstWhere(
      (v) =>
          base64Encode(v.signaturePublicKey) ==
          base64Encode(identity.publicKey),
    );
    return joinGroupFromWelcomeWithStorage(
      config: _config(retention),
      incarnationId: _incarnation,
      welcomeBytes: p.welcome!,
      expectedWelcomeSha256: _hash(p.welcome!),
      signerBytes: identity.signerBytes,
      expectedResultingState: _expected(p.proposedResultingRoster),
      expectedLocalLeaf: local,
      expectedTargetKeyPackageSha256: hash ?? _hash(keyPackageBytes!),
      storageEntries: store.globalSnapshot,
      storageFormatVersion: 1,
    );
  }

  Future<void> join(PendingCommitWithStorageResult p) async {
    final j =
        (await joinOutcome(p) as JoinGroupFromWelcomeWithStorageOutcome_Success)
            .field0;
    expect(j.consumedKeyPackageSha256, orderedEquals(_hash(keyPackageBytes!)));
    apply(j.storageBatch, j.resultingRoster, j.resultingGroupStateSha256);
  }

  Future<_Wire> send(String text) async {
    final m =
        (await createMessageWithStorage(
                  context: context,
                  signerBytes: identity.signerBytes,
                  message: utf8.encode(text),
                  aad: _aad,
                )
                as CreateMessageWithStorageOutcome_Success)
            .field0;
    apply(m.storageBatch, roster, m.resultingGroupStateSha256);
    return _Wire(m.ciphertext, leaf, _expected(roster));
  }

  Future<ProcessMessageWithStorageOutcome> receiveOutcome(
    _Wire w, {
    List<int>? aad,
    List<int>? hash,
    MlsRosterLeafV1? sender,
  }) => processMessageWithStorage(
    context: context,
    expectedKind: MlsReceiveKind.application,
    messageBytes: w.bytes,
    expectedMessageSha256: hash ?? _hash(w.bytes),
    expectedAad: aad ?? _aad,
    expectedSender: sender ?? w.sender,
    expectedMessageState: w.state,
    expectedResultingState: _expected(roster),
  );
  Future<ProcessMessageWithStorageResult> receive(_Wire w) async {
    final r =
        (await receiveOutcome(w) as ProcessMessageWithStorageOutcome_Success)
            .field0;
    apply(r.storageBatch, r.resultingRoster, r.resultingGroupStateSha256);
    return r;
  }

  Future<void> receiveCommit(PendingCommitWithStorageResult p) async {
    final r =
        (await processMessageWithStorage(
                  context: context,
                  expectedKind: MlsReceiveKind.commit,
                  messageBytes: p.commit,
                  expectedMessageSha256: _hash(p.commit),
                  expectedAad: _aad,
                  expectedSender: p.pendingBinding.author,
                  expectedMessageState: p.pendingBinding.previousState,
                  expectedResultingState: _expected(p.proposedResultingRoster),
                )
                as ProcessMessageWithStorageOutcome_Success)
            .field0;
    expect(r.messageEpoch, p.previousRoster.epoch);
    apply(r.storageBatch, r.resultingRoster, r.resultingGroupStateSha256);
  }
}

Future<(_Peer, _Peer)> _pair({int receiverRetention = 2}) async {
  final a = _Peer('a', 2);
  final b = _Peer('b', receiverRetention);
  await a.create();
  await b.keyPackage();
  final p = await a.add([b]);
  await a.accept(p);
  await b.join(p);
  return (a, b);
}

Future<CreateKeyPackageWithStorageResult> _createKeyPackage(
  TestIdentity identity,
  List<MlsStorageEntry> storageEntries,
) => createKeyPackageWithStorage(
  ciphersuite: ciphersuite,
  signerBytes: identity.signerBytes,
  credentialIdentity: identity.credentialIdentity,
  signerPublicKey: identity.publicKey,
  storageEntries: storageEntries,
  storageFormatVersion: mlsStorageFormatVersion(),
);

class _MemoryMlsStore {
  final int formatVersion = mlsStorageFormatVersion();
  final Map<String, MlsStorageEntry> _entries = {};

  List<MlsStorageEntry> get snapshot => List.unmodifiable(_entries.values);

  List<MlsStorageEntry> get globalSnapshot => List.unmodifiable(
    _entries.values.where((entry) => entry.groupId == null),
  );

  List<MlsStorageEntry> forGroup(List<int> groupId) {
    final encodedGroupId = base64Encode(groupId);
    return List.unmodifiable(
      _entries.values.where(
        (entry) =>
            entry.groupId == null ||
            base64Encode(entry.groupId!) == encodedGroupId,
      ),
    );
  }

  List<MlsStorageEntry> groupEntries(List<int> groupId) {
    final encodedGroupId = base64Encode(groupId);
    return List.unmodifiable(
      _entries.values.where(
        (entry) =>
            entry.groupId != null &&
            base64Encode(entry.groupId!) == encodedGroupId,
      ),
    );
  }

  String get fingerprint {
    final rows =
        _entries.entries
            .map(
              (row) => [
                row.key,
                base64Encode(row.value.value),
                if (row.value.groupId == null)
                  '-'
                else
                  base64Encode(row.value.groupId!),
              ].join(':'),
            )
            .toList()
          ..sort();
    return rows.join('|');
  }

  void apply(MlsStorageBatch batch) {
    if (batch.storageFormatVersion != formatVersion) {
      throw StateError('Unexpected MLS storage format');
    }

    for (final key in batch.deletes) {
      _entries.remove(base64Encode(key));
    }
    for (final groupId in batch.deletedGroupIds) {
      _entries.removeWhere(
        (_, entry) =>
            entry.groupId != null &&
            base64Encode(entry.groupId!) == base64Encode(groupId),
      );
    }
    for (final entry in batch.upserts) {
      _entries[base64Encode(entry.key)] = entry;
    }
  }
}
