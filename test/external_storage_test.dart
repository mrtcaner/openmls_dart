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
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(discarded.storageBatch.upserts, isNotEmpty);
      expect(aliceStore.fingerprint, aliceBeforeDiscard);

      final sentToBob = await createMessageWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('hello bob'),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(sentToBob.storageBatch);

      final receivedByBob = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToBob.ciphertext,
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(receivedByBob.messageType, ProcessedMessageType.application);
      expect(utf8.decode(receivedByBob.applicationMessage!), 'hello bob');
      bobStore.apply(receivedByBob.storageBatch);

      final bobBeforeError = bobStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: created.groupId,
          messageBytes: const [1, 2, 3],
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

      final addedCharlie = await addMembersWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        keyPackagesBytes: [charlieKeyPackage.keyPackageBytes],
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(addedCharlie.storageBatch);

      final bobProcessedCommit = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: addedCharlie.commit,
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(bobProcessedCommit.messageType, ProcessedMessageType.stagedCommit);
      expect(bobProcessedCommit.hasStagedCommit, isTrue);
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
        storageEntries: bobStore.forGroup(created.groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      bobStore.apply(sentToGroup.storageBatch);

      final receivedByAlice = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToGroup.ciphertext,
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      final receivedByCharlie = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToGroup.ciphertext,
        storageEntries: charlieStore.forGroup(created.groupId),
        storageFormatVersion: charlieStore.formatVersion,
      );
      expect(utf8.decode(receivedByAlice.applicationMessage!), 'hello group');
      expect(utf8.decode(receivedByCharlie.applicationMessage!), 'hello group');
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
  });
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
