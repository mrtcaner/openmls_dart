import 'dart:convert';

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

    test('rejects a key package for a different credential identity', () async {
      final alice = TestIdentity.create('alice-credential-check');
      final bob = TestIdentity.create('bob-credential-check');
      final aliceStore = _MemoryMlsStore();
      final bobStore = _MemoryMlsStore();

      final bobKeyPackage = await _createKeyPackage(
        bob,
        bobStore.globalSnapshot,
      );
      bobStore.apply(bobKeyPackage.storageBatch);

      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        credentialIdentity: alice.credentialIdentity,
        signerPublicKey: alice.publicKey,
        storageEntries: aliceStore.globalSnapshot,
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(created.storageBatch);
      final beforeMismatch = aliceStore.fingerprint;

      await expectLater(
        addMembersWithStorage(
          groupId: created.groupId,
          signerBytes: alice.signerBytes,
          keyPackagesBytes: [bobKeyPackage.keyPackageBytes],
          expectedCredentialIdentities: [alice.credentialIdentity],
          aad: utf8.encode('credential-check/add-member'),
          storageEntries: aliceStore.forGroup(created.groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Key package credential identity does not match',
            ),
          ),
        ),
      );
      expect(aliceStore.fingerprint, beforeMismatch);

      await expectLater(
        addMembersWithStorage(
          groupId: created.groupId,
          signerBytes: alice.signerBytes,
          keyPackagesBytes: [bobKeyPackage.keyPackageBytes],
          expectedCredentialIdentities: const [],
          aad: utf8.encode('credential-check/add-member'),
          storageEntries: aliceStore.forGroup(created.groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'does not match expected credential identity count',
            ),
          ),
        ),
      );
      expect(aliceStore.fingerprint, beforeMismatch);
    });

    test('recreates a three-member conversation from stored entries', () async {
      final alice = TestIdentity.create('alice-external');
      final bob = TestIdentity.create('bob-external');
      final charlie = TestIdentity.create('charlie-external');
      final aliceStore = _MemoryMlsStore();
      final bobStore = _MemoryMlsStore();
      final charlieStore = _MemoryMlsStore();

      final bobKeyPackage = await _createKeyPackage(
        bob,
        bobStore.globalSnapshot,
      );
      bobStore.apply(bobKeyPackage.storageBatch);

      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        credentialIdentity: alice.credentialIdentity,
        signerPublicKey: alice.publicKey,
        storageEntries: aliceStore.globalSnapshot,
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(created.storageBatch);

      final addedBob = await addMembersWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        keyPackagesBytes: [bobKeyPackage.keyPackageBytes],
        expectedCredentialIdentities: [bob.credentialIdentity],
        aad: utf8.encode('conversation-1/add-bob'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(addedBob.storageBatch);

      final joinedBob = await joinGroupFromWelcomeWithStorage(
        config: defaultConfig(),
        welcomeBytes: addedBob.welcome,
        signerBytes: bob.signerBytes,
        storageEntries: bobStore.globalSnapshot,
        storageFormatVersion: bobStore.formatVersion,
      );
      bobStore.apply(joinedBob.storageBatch);
      expect(joinedBob.groupId, orderedEquals(created.groupId));

      final aliceBeforeDiscard = aliceStore.fingerprint;
      final discarded = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('discard me'),
        aad: utf8.encode('conversation-1/discarded-before-apply'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(discarded.storageBatch.upserts, isNotEmpty);
      expect(aliceStore.fingerprint, aliceBeforeDiscard);

      final sentToBob = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('hello bob'),
        aad: utf8.encode('conversation-1/message-1'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(sentToBob.storageBatch);

      final bobBeforeAadMismatch = bobStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: created.groupId,
          messageBytes: sentToBob.ciphertext,
          expectedAad: utf8.encode('conversation-1/wrong-message'),
          storageEntries: bobStore.forGroup(created.groupId),
          storageFormatVersion: bobStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Message AAD does not match the expected AAD',
            ),
          ),
        ),
      );
      expect(bobStore.fingerprint, bobBeforeAadMismatch);

      final receivedByBob = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToBob.ciphertext,
        expectedAad: utf8.encode('conversation-1/message-1'),
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(receivedByBob.messageType, ProcessedMessageType.application);
      expect(utf8.decode(receivedByBob.applicationMessage!), 'hello bob');
      expect(receivedByBob.previousEpoch, BigInt.one);
      expect(receivedByBob.resultingEpoch, BigInt.one);
      bobStore.apply(receivedByBob.storageBatch);

      final appliedButUndelivered = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('terminally rejected'),
        aad: utf8.encode('conversation-1/message-2'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(appliedButUndelivered.storageBatch);

      final sentAfterGap = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('after rejected message'),
        aad: utf8.encode('conversation-1/message-3'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(sentAfterGap.storageBatch);

      final receivedAfterGap = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentAfterGap.ciphertext,
        expectedAad: utf8.encode('conversation-1/message-3'),
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(
        utf8.decode(receivedAfterGap.applicationMessage!),
        'after rejected message',
      );
      expect(receivedAfterGap.previousEpoch, BigInt.one);
      expect(receivedAfterGap.resultingEpoch, BigInt.one);
      bobStore.apply(receivedAfterGap.storageBatch);

      final bobBeforeError = bobStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: created.groupId,
          messageBytes: const [1, 2, 3],
          expectedAad: const [],
          storageEntries: bobStore.forGroup(created.groupId),
          storageFormatVersion: bobStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) =>
                error.toString().contains('Failed to deserialize message'),
          ),
        ),
      );
      expect(bobStore.fingerprint, bobBeforeError);

      final charlieKeyPackage = await _createKeyPackage(
        charlie,
        charlieStore.globalSnapshot,
      );
      charlieStore.apply(charlieKeyPackage.storageBatch);

      final charlieCommitAad = utf8.encode('conversation-1/add-charlie');
      final addedCharlie = await addMembersWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        keyPackagesBytes: [charlieKeyPackage.keyPackageBytes],
        expectedCredentialIdentities: [charlie.credentialIdentity],
        aad: charlieCommitAad,
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(addedCharlie.storageBatch);

      final bobBeforeCommitAadMismatch = bobStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: created.groupId,
          messageBytes: addedCharlie.commit,
          expectedAad: utf8.encode('conversation-1/wrong-add-charlie'),
          storageEntries: bobStore.forGroup(created.groupId),
          storageFormatVersion: bobStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Message AAD does not match the expected AAD',
            ),
          ),
        ),
      );
      expect(bobStore.fingerprint, bobBeforeCommitAadMismatch);

      final bobProcessedCommit = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: addedCharlie.commit,
        expectedAad: charlieCommitAad,
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(bobProcessedCommit.messageType, ProcessedMessageType.stagedCommit);
      expect(bobProcessedCommit.hasStagedCommit, isTrue);
      expect(bobProcessedCommit.previousEpoch, BigInt.one);
      expect(bobProcessedCommit.resultingEpoch, BigInt.two);
      bobStore.apply(bobProcessedCommit.storageBatch);

      final joinedCharlie = await joinGroupFromWelcomeWithStorage(
        config: defaultConfig(),
        welcomeBytes: addedCharlie.welcome,
        signerBytes: charlie.signerBytes,
        storageEntries: charlieStore.globalSnapshot,
        storageFormatVersion: charlieStore.formatVersion,
      );
      charlieStore.apply(joinedCharlie.storageBatch);
      expect(joinedCharlie.groupId, orderedEquals(created.groupId));

      final sentToGroup = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: bob.signerBytes,
        message: utf8.encode('hello group'),
        aad: utf8.encode('conversation-1/message-4'),
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      bobStore.apply(sentToGroup.storageBatch);

      final receivedByAlice = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToGroup.ciphertext,
        expectedAad: utf8.encode('conversation-1/message-4'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      final receivedByCharlie = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToGroup.ciphertext,
        expectedAad: utf8.encode('conversation-1/message-4'),
        storageEntries: charlieStore.forGroup(created.groupId),
        storageFormatVersion: charlieStore.formatVersion,
      );
      expect(utf8.decode(receivedByAlice.applicationMessage!), 'hello group');
      expect(utf8.decode(receivedByCharlie.applicationMessage!), 'hello group');
      expect(receivedByAlice.previousEpoch, BigInt.two);
      expect(receivedByAlice.resultingEpoch, BigInt.two);
      expect(receivedByCharlie.previousEpoch, BigInt.two);
      expect(receivedByCharlie.resultingEpoch, BigInt.two);
      aliceStore.apply(receivedByAlice.storageBatch);
      charlieStore.apply(receivedByCharlie.storageBatch);

      final deleteCharlieGroup = await deleteGroupWithStorage(
        groupId: created.groupId,
        storageEntries: charlieStore.forGroup(created.groupId),
        storageFormatVersion: charlieStore.formatVersion,
      );
      expect(deleteCharlieGroup.deletedGroupIds, hasLength(1));
      expect(
        deleteCharlieGroup.deletedGroupIds.single,
        orderedEquals(created.groupId),
      );
      charlieStore.apply(deleteCharlieGroup);
      expect(charlieStore.groupEntries(created.groupId), isEmpty);
      expect(
        charlieStore.globalSnapshot,
        isNotEmpty,
        reason: 'deleting a group must retain installation-global state',
      );
    });

    test('enforces the sender-ratchet forward-distance boundary', () async {
      const maximumForwardDistance = 2;
      final config = _configWithForwardDistance(maximumForwardDistance);

      final atLimit = await _createTwoMemberSession('at-limit', config);
      late CreateMessageWithStorageResult atLimitMessage;
      for (
        var generation = 0;
        generation <= maximumForwardDistance;
        generation++
      ) {
        atLimitMessage = await _createAndApplyMessage(atLimit, generation);
      }

      final receivedAtLimit = await processMessageWithStorage(
        groupId: atLimit.groupId,
        messageBytes: atLimitMessage.ciphertext,
        expectedAad: utf8.encode('at-limit/message-2'),
        storageEntries: atLimit.receiverStore.forGroup(atLimit.groupId),
        storageFormatVersion: atLimit.receiverStore.formatVersion,
      );
      expect(utf8.decode(receivedAtLimit.applicationMessage!), 'message 2');
      atLimit.receiverStore.apply(receivedAtLimit.storageBatch);

      final beyondLimit = await _createTwoMemberSession('beyond-limit', config);
      late CreateMessageWithStorageResult beyondLimitMessage;
      for (
        var generation = 0;
        generation <= maximumForwardDistance + 1;
        generation++
      ) {
        beyondLimitMessage = await _createAndApplyMessage(
          beyondLimit,
          generation,
        );
      }

      final receiverBeforeFailure = beyondLimit.receiverStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: beyondLimit.groupId,
          messageBytes: beyondLimitMessage.ciphertext,
          expectedAad: utf8.encode('beyond-limit/message-3'),
          storageEntries: beyondLimit.receiverStore.forGroup(
            beyondLimit.groupId,
          ),
          storageFormatVersion: beyondLimit.receiverStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Generation is too far in the future to be processed',
            ),
          ),
        ),
      );
      expect(beyondLimit.receiverStore.fingerprint, receiverBeforeFailure);
    });
  });
}

MlsGroupConfig _configWithForwardDistance(int maximumForwardDistance) {
  final defaults = defaultConfig();
  return MlsGroupConfig(
    ciphersuite: defaults.ciphersuite,
    wireFormatPolicy: defaults.wireFormatPolicy,
    useRatchetTreeExtension: defaults.useRatchetTreeExtension,
    maxPastEpochs: defaults.maxPastEpochs,
    paddingSize: defaults.paddingSize,
    senderRatchetMaxOutOfOrder: defaults.senderRatchetMaxOutOfOrder,
    senderRatchetMaxForwardDistance: maximumForwardDistance,
    numberOfResumptionPsks: defaults.numberOfResumptionPsks,
  );
}

Future<_TwoMemberSession> _createTwoMemberSession(
  String label,
  MlsGroupConfig config,
) async {
  final sender = TestIdentity.create('$label-sender');
  final receiver = TestIdentity.create('$label-receiver');
  final senderStore = _MemoryMlsStore();
  final receiverStore = _MemoryMlsStore();

  final receiverKeyPackage = await _createKeyPackage(
    receiver,
    receiverStore.globalSnapshot,
  );
  receiverStore.apply(receiverKeyPackage.storageBatch);

  final created = await createGroupWithStorage(
    config: config,
    signerBytes: sender.signerBytes,
    credentialIdentity: sender.credentialIdentity,
    signerPublicKey: sender.publicKey,
    storageEntries: senderStore.globalSnapshot,
    storageFormatVersion: senderStore.formatVersion,
  );
  senderStore.apply(created.storageBatch);

  final added = await addMembersWithStorage(
    groupId: created.groupId,
    signerBytes: sender.signerBytes,
    keyPackagesBytes: [receiverKeyPackage.keyPackageBytes],
    expectedCredentialIdentities: [receiver.credentialIdentity],
    aad: utf8.encode('$label/add-receiver'),
    storageEntries: senderStore.forGroup(created.groupId),
    storageFormatVersion: senderStore.formatVersion,
  );
  senderStore.apply(added.storageBatch);

  final joined = await joinGroupFromWelcomeWithStorage(
    config: config,
    welcomeBytes: added.welcome,
    signerBytes: receiver.signerBytes,
    storageEntries: receiverStore.globalSnapshot,
    storageFormatVersion: receiverStore.formatVersion,
  );
  receiverStore.apply(joined.storageBatch);

  return _TwoMemberSession(
    label: label,
    sender: sender,
    senderStore: senderStore,
    receiverStore: receiverStore,
    groupId: created.groupId,
  );
}

Future<CreateMessageWithStorageResult> _createAndApplyMessage(
  _TwoMemberSession session,
  int generation,
) async {
  final message = await createMessageWithStorage(
    groupId: session.groupId,
    signerBytes: session.sender.signerBytes,
    message: utf8.encode('message $generation'),
    aad: utf8.encode('${session.label}/message-$generation'),
    storageEntries: session.senderStore.forGroup(session.groupId),
    storageFormatVersion: session.senderStore.formatVersion,
  );
  session.senderStore.apply(message.storageBatch);
  return message;
}

class _TwoMemberSession {
  const _TwoMemberSession({
    required this.label,
    required this.sender,
    required this.senderStore,
    required this.receiverStore,
    required this.groupId,
  });

  final String label;
  final TestIdentity sender;
  final _MemoryMlsStore senderStore;
  final _MemoryMlsStore receiverStore;
  final List<int> groupId;
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
