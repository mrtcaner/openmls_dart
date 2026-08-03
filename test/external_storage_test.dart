import 'dart:convert';
import 'dart:typed_data';

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
      final groupId = utf8.encode('credential-check-group');

      final bobKeyPackage = await _createKeyPackage(
        bob,
        bobStore.globalSnapshot,
      );
      bobStore.apply(bobKeyPackage.storageBatch);

      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        explicitGroupId: groupId,
        expectedOwnerAuthority: MlsAuthorizedOwnerV1(
          expectedCredentialIdentity: alice.credentialIdentity,
          expectedSignaturePublicKey: alice.publicKey,
        ),
        storageEntries: aliceStore.globalSnapshot,
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(created.storageBatch);
      final beforeMismatch = aliceStore.fingerprint;

      await expectLater(
        addMembersWithStorage(
          groupId: created.groupId,
          signerBytes: alice.signerBytes,
          additions: [
            MlsAuthorizedKeyPackageV1(
              keyPackageBytes: bobKeyPackage.keyPackageBytes,
              expectedCredentialIdentity: utf8.encode(
                'different-authorized-installation',
              ),
              expectedSignaturePublicKey: bob.publicKey,
            ),
          ],
          aad: utf8.encode('credential-check/add-member'),
          expectedPreviousState: _expected(created.resultingRoster),
          storageEntries: aliceStore.forGroup(created.groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Key package credential identity does not match expected authority',
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
      final groupId = utf8.encode('conversation-1');

      final bobKeyPackage = await _createKeyPackage(
        bob,
        bobStore.globalSnapshot,
      );
      bobStore.apply(bobKeyPackage.storageBatch);

      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        explicitGroupId: groupId,
        expectedOwnerAuthority: MlsAuthorizedOwnerV1(
          expectedCredentialIdentity: alice.credentialIdentity,
          expectedSignaturePublicKey: alice.publicKey,
        ),
        storageEntries: aliceStore.globalSnapshot,
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(created.storageBatch);

      final addedBob = await addMembersWithStorage(
        groupId: created.groupId,
        signerBytes: alice.signerBytes,
        additions: [
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: bobKeyPackage.keyPackageBytes,
            expectedCredentialIdentity: bob.credentialIdentity,
            expectedSignaturePublicKey: bob.publicKey,
          ),
        ],
        aad: utf8.encode('conversation-1/add-bob'),
        expectedPreviousState: _expected(created.resultingRoster),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(addedBob.storageBatch);

      final joinedBob = await joinGroupFromWelcomeWithStorage(
        config: defaultConfig(),
        welcomeBytes: addedBob.welcome!,
        signerBytes: bob.signerBytes,
        expectedResultingState: _expected(addedBob.resultingRoster),
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
          expectedPreviousState: _expected(addedBob.resultingRoster),
          expectedResultingState: _expected(addedBob.resultingRoster),
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
        expectedPreviousState: _expected(addedBob.resultingRoster),
        expectedResultingState: _expected(addedBob.resultingRoster),
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
        expectedPreviousState: _expected(addedBob.resultingRoster),
        expectedResultingState: _expected(addedBob.resultingRoster),
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
          expectedPreviousState: _expected(addedBob.resultingRoster),
          expectedResultingState: _expected(addedBob.resultingRoster),
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
        additions: [
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: charlieKeyPackage.keyPackageBytes,
            expectedCredentialIdentity: charlie.credentialIdentity,
            expectedSignaturePublicKey: charlie.publicKey,
          ),
        ],
        aad: charlieCommitAad,
        expectedPreviousState: _expected(addedBob.resultingRoster),
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
          expectedPreviousState: _expected(addedBob.resultingRoster),
          expectedResultingState: _expected(addedCharlie.resultingRoster),
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
        expectedPreviousState: _expected(addedBob.resultingRoster),
        expectedResultingState: _expected(addedCharlie.resultingRoster),
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
        welcomeBytes: addedCharlie.welcome!,
        signerBytes: charlie.signerBytes,
        expectedResultingState: _expected(addedCharlie.resultingRoster),
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
        expectedPreviousState: _expected(addedCharlie.resultingRoster),
        expectedResultingState: _expected(addedCharlie.resultingRoster),
        storageEntries: aliceStore.forGroup(created.groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      final receivedByCharlie = await processMessageWithStorage(
        groupId: created.groupId,
        messageBytes: sentToGroup.ciphertext,
        expectedAad: utf8.encode('conversation-1/message-4'),
        expectedPreviousState: _expected(addedCharlie.resultingRoster),
        expectedResultingState: _expected(addedCharlie.resultingRoster),
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
        expectedPreviousState: _expected(atLimit.roster),
        expectedResultingState: _expected(atLimit.roster),
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
          expectedPreviousState: _expected(beyondLimit.roster),
          expectedResultingState: _expected(beyondLimit.roster),
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

    test('supports strict origin-aligned variable-roster operations', () async {
      final alice = TestIdentity.create('group-alice');
      final bob = TestIdentity.create('group-bob');
      final charlie = TestIdentity.create('group-charlie');
      final aliceStore = _MemoryMlsStore();
      final bobStore = _MemoryMlsStore();
      final charlieStore = _MemoryMlsStore();
      final groupId = utf8.encode('server-issued-group-1');

      final bobKeyPackage = await _createKeyPackage(
        bob,
        bobStore.globalSnapshot,
      );
      bobStore.apply(bobKeyPackage.storageBatch);
      final charlieKeyPackage = await _createKeyPackage(
        charlie,
        charlieStore.globalSnapshot,
      );
      charlieStore.apply(charlieKeyPackage.storageBatch);

      final beforeCreate = aliceStore.fingerprint;
      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        explicitGroupId: groupId,
        expectedOwnerAuthority: MlsAuthorizedOwnerV1(
          expectedCredentialIdentity: alice.credentialIdentity,
          expectedSignaturePublicKey: alice.publicKey,
        ),
        storageEntries: aliceStore.globalSnapshot,
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(aliceStore.fingerprint, beforeCreate);
      expect(created.groupId, orderedEquals(groupId));
      _expectRoster(
        created.resultingRoster,
        groupId: groupId,
        epoch: 0,
        identities: [alice.credentialIdentity],
      );
      aliceStore.apply(created.storageBatch);

      final wrongPreviousDigest = Uint8List.fromList(
        created.resultingRoster.digestSha256,
      );
      wrongPreviousDigest[0] ^= 0xff;
      final beforeWrongPrevious = aliceStore.fingerprint;
      await expectLater(
        addMembersWithStorage(
          groupId: groupId,
          signerBytes: alice.signerBytes,
          additions: [
            MlsAuthorizedKeyPackageV1(
              keyPackageBytes: bobKeyPackage.keyPackageBytes,
              expectedCredentialIdentity: bob.credentialIdentity,
              expectedSignaturePublicKey: bob.publicKey,
            ),
          ],
          aad: utf8.encode('group-1/add-bob'),
          expectedPreviousState: MlsExpectedRosterStateV1(
            groupId: created.resultingRoster.groupId,
            epoch: created.resultingRoster.epoch,
            digestSha256: wrongPreviousDigest,
          ),
          storageEntries: aliceStore.forGroup(groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'previous MLS roster digest does not match',
            ),
          ),
        ),
      );
      expect(aliceStore.fingerprint, beforeWrongPrevious);

      final wrongBobSignatureKey = Uint8List.fromList(bob.publicKey);
      wrongBobSignatureKey[0] ^= 0xff;
      final beforeWrongAddition = aliceStore.fingerprint;
      await expectLater(
        addMembersWithStorage(
          groupId: groupId,
          signerBytes: alice.signerBytes,
          additions: [
            MlsAuthorizedKeyPackageV1(
              keyPackageBytes: bobKeyPackage.keyPackageBytes,
              expectedCredentialIdentity: bob.credentialIdentity,
              expectedSignaturePublicKey: wrongBobSignatureKey,
            ),
          ],
          aad: utf8.encode('group-1/add-bob'),
          expectedPreviousState: _expected(created.resultingRoster),
          storageEntries: aliceStore.forGroup(groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'signature key does not match expected authority',
            ),
          ),
        ),
      );
      expect(aliceStore.fingerprint, beforeWrongAddition);

      final addBaseDigest = mlsGroupStateDigest(
        groupId: groupId,
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      final beforeAdd = aliceStore.fingerprint;
      final addedBob = await addMembersWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        additions: [
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: bobKeyPackage.keyPackageBytes,
            expectedCredentialIdentity: bob.credentialIdentity,
            expectedSignaturePublicKey: bob.publicKey,
          ),
        ],
        aad: utf8.encode('group-1/add-bob'),
        expectedPreviousState: _expected(created.resultingRoster),
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(aliceStore.fingerprint, beforeAdd);
      expect(addedBob.baseGroupStateSha256, orderedEquals(addBaseDigest));
      expect(addedBob.commitSha256, hasLength(32));
      _expectRoster(
        addedBob.resultingRoster,
        groupId: groupId,
        epoch: 1,
        identities: [alice.credentialIdentity, bob.credentialIdentity],
      );
      aliceStore.apply(addedBob.storageBatch);

      final bobBeforeWrongSigner = bobStore.fingerprint;
      await expectLater(
        joinGroupFromWelcomeWithStorage(
          config: defaultConfig(),
          welcomeBytes: addedBob.welcome!,
          signerBytes: charlie.signerBytes,
          expectedResultingState: _expected(addedBob.resultingRoster),
          storageEntries: bobStore.globalSnapshot,
          storageFormatVersion: bobStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Signer public key does not match expected installation authority',
            ),
          ),
        ),
      );
      expect(bobStore.fingerprint, bobBeforeWrongSigner);

      final wrongJoinDigest = Uint8List.fromList(
        addedBob.resultingRoster.digestSha256,
      );
      wrongJoinDigest[0] ^= 0xff;
      final bobBeforeWrongJoin = bobStore.fingerprint;
      await expectLater(
        joinGroupFromWelcomeWithStorage(
          config: defaultConfig(),
          welcomeBytes: addedBob.welcome!,
          signerBytes: bob.signerBytes,
          expectedResultingState: MlsExpectedRosterStateV1(
            groupId: addedBob.resultingRoster.groupId,
            epoch: addedBob.resultingRoster.epoch,
            digestSha256: wrongJoinDigest,
          ),
          storageEntries: bobStore.globalSnapshot,
          storageFormatVersion: bobStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'resulting MLS roster digest does not match',
            ),
          ),
        ),
      );
      expect(bobStore.fingerprint, bobBeforeWrongJoin);

      final joinedBob = await joinGroupFromWelcomeWithStorage(
        config: defaultConfig(),
        welcomeBytes: addedBob.welcome!,
        signerBytes: bob.signerBytes,
        expectedResultingState: _expected(addedBob.resultingRoster),
        storageEntries: bobStore.globalSnapshot,
        storageFormatVersion: bobStore.formatVersion,
      );
      _expectSameRoster(joinedBob.resultingRoster, addedBob.resultingRoster);
      bobStore.apply(joinedBob.storageBatch);

      final applicationAad = utf8.encode('group-1/application-1');
      final sent = await createMessageWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('hello variable roster'),
        aad: applicationAad,
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      aliceStore.apply(sent.storageBatch);
      final received = await processMessageWithStorage(
        groupId: groupId,
        messageBytes: sent.ciphertext,
        expectedAad: applicationAad,
        expectedPreviousState: _expected(addedBob.resultingRoster),
        expectedResultingState: _expected(addedBob.resultingRoster),
        storageEntries: bobStore.forGroup(groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(
        utf8.decode(received.applicationMessage!),
        'hello variable roster',
      );
      _expectSameRoster(received.previousRoster, received.resultingRoster);
      bobStore.apply(received.storageBatch);

      final bobLeaf = addedBob.resultingRoster.leaves.singleWhere(
        (leaf) =>
            base64Encode(leaf.credentialIdentity) ==
            base64Encode(bob.credentialIdentity),
      );
      final swapAad = utf8.encode('group-1/swap-bob-charlie');
      final beforeSwap = aliceStore.fingerprint;
      final swapped = await swapMembersWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        removals: [
          MlsAuthorizedRemovalV1(
            leafIndex: bobLeaf.leafIndex,
            expectedCredentialIdentity: bobLeaf.credentialIdentity,
            expectedSignaturePublicKey: bobLeaf.signaturePublicKey,
          ),
        ],
        additions: [
          MlsAuthorizedKeyPackageV1(
            keyPackageBytes: charlieKeyPackage.keyPackageBytes,
            expectedCredentialIdentity: charlie.credentialIdentity,
            expectedSignaturePublicKey: charlie.publicKey,
          ),
        ],
        aad: swapAad,
        expectedPreviousState: _expected(addedBob.resultingRoster),
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(aliceStore.fingerprint, beforeSwap);
      _expectRoster(
        swapped.resultingRoster,
        groupId: groupId,
        epoch: 2,
        identities: [alice.credentialIdentity, charlie.credentialIdentity],
      );
      aliceStore.apply(swapped.storageBatch);

      final bobProcessedSwap = await processMessageWithStorage(
        groupId: groupId,
        messageBytes: swapped.commit,
        expectedAad: swapAad,
        expectedPreviousState: _expected(addedBob.resultingRoster),
        expectedResultingState: _expected(swapped.resultingRoster),
        storageEntries: bobStore.forGroup(groupId),
        storageFormatVersion: bobStore.formatVersion,
      );
      expect(bobProcessedSwap.resultingEpoch, BigInt.two);
      bobStore.apply(bobProcessedSwap.storageBatch);

      final joinedCharlie = await joinGroupFromWelcomeWithStorage(
        config: defaultConfig(),
        welcomeBytes: swapped.welcome!,
        signerBytes: charlie.signerBytes,
        expectedResultingState: _expected(swapped.resultingRoster),
        storageEntries: charlieStore.globalSnapshot,
        storageFormatVersion: charlieStore.formatVersion,
      );
      _expectSameRoster(joinedCharlie.resultingRoster, swapped.resultingRoster);
      charlieStore.apply(joinedCharlie.storageBatch);

      final aliceLeaf = swapped.resultingRoster.leaves.singleWhere(
        (leaf) =>
            base64Encode(leaf.credentialIdentity) ==
            base64Encode(alice.credentialIdentity),
      );
      final selfUpdateAad = utf8.encode('group-1/alice-self-update');
      final selfUpdated = await selfUpdateWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        aad: selfUpdateAad,
        expectedPreviousState: _expected(swapped.resultingRoster),
        expectedSelfAuthority: MlsAuthorizedSelfV1(
          leafIndex: aliceLeaf.leafIndex,
          expectedCredentialIdentity: aliceLeaf.credentialIdentity,
          expectedSignaturePublicKey: aliceLeaf.signaturePublicKey,
        ),
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(selfUpdated.resultingRoster.leaves, hasLength(2));
      _expectRoster(
        selfUpdated.resultingRoster,
        groupId: groupId,
        epoch: 3,
        identities: [alice.credentialIdentity, charlie.credentialIdentity],
      );
      aliceStore.apply(selfUpdated.storageBatch);

      final wrongResult = _expected(selfUpdated.resultingRoster);
      final wrongDigest = List<int>.from(wrongResult.digestSha256);
      wrongDigest[0] ^= 0xff;
      final charlieBeforeWrongResult = charlieStore.fingerprint;
      await expectLater(
        processMessageWithStorage(
          groupId: groupId,
          messageBytes: selfUpdated.commit,
          expectedAad: selfUpdateAad,
          expectedPreviousState: _expected(swapped.resultingRoster),
          expectedResultingState: MlsExpectedRosterStateV1(
            groupId: wrongResult.groupId,
            epoch: wrongResult.epoch,
            digestSha256: Uint8List.fromList(wrongDigest),
          ),
          storageEntries: charlieStore.forGroup(groupId),
          storageFormatVersion: charlieStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'resulting MLS roster digest does not match',
            ),
          ),
        ),
      );
      expect(charlieStore.fingerprint, charlieBeforeWrongResult);
      final charlieProcessedUpdate = await processMessageWithStorage(
        groupId: groupId,
        messageBytes: selfUpdated.commit,
        expectedAad: selfUpdateAad,
        expectedPreviousState: _expected(swapped.resultingRoster),
        expectedResultingState: _expected(selfUpdated.resultingRoster),
        storageEntries: charlieStore.forGroup(groupId),
        storageFormatVersion: charlieStore.formatVersion,
      );
      charlieStore.apply(charlieProcessedUpdate.storageBatch);

      final charlieLeaf = selfUpdated.resultingRoster.leaves.singleWhere(
        (leaf) =>
            base64Encode(leaf.credentialIdentity) ==
            base64Encode(charlie.credentialIdentity),
      );
      final removeAad = utf8.encode('group-1/remove-charlie');
      final wrongCharlieSignatureKey = Uint8List.fromList(
        charlieLeaf.signaturePublicKey,
      );
      wrongCharlieSignatureKey[0] ^= 0xff;
      final beforeWrongRemoval = aliceStore.fingerprint;
      await expectLater(
        removeMembersWithStorage(
          groupId: groupId,
          signerBytes: alice.signerBytes,
          removals: [
            MlsAuthorizedRemovalV1(
              leafIndex: charlieLeaf.leafIndex,
              expectedCredentialIdentity: charlieLeaf.credentialIdentity,
              expectedSignaturePublicKey: wrongCharlieSignatureKey,
            ),
          ],
          aad: removeAad,
          expectedPreviousState: _expected(selfUpdated.resultingRoster),
          storageEntries: aliceStore.forGroup(groupId),
          storageFormatVersion: aliceStore.formatVersion,
        ),
        throwsA(
          predicate<Object>(
            (error) => error.toString().contains(
              'Authorized removal does not match current leaf authority',
            ),
          ),
        ),
      );
      expect(aliceStore.fingerprint, beforeWrongRemoval);

      final removedCharlie = await removeMembersWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        removals: [
          MlsAuthorizedRemovalV1(
            leafIndex: charlieLeaf.leafIndex,
            expectedCredentialIdentity: charlieLeaf.credentialIdentity,
            expectedSignaturePublicKey: charlieLeaf.signaturePublicKey,
          ),
        ],
        aad: removeAad,
        expectedPreviousState: _expected(selfUpdated.resultingRoster),
        storageEntries: aliceStore.forGroup(groupId),
        storageFormatVersion: aliceStore.formatVersion,
      );
      expect(removedCharlie.welcome, isNull);
      _expectRoster(
        removedCharlie.resultingRoster,
        groupId: groupId,
        epoch: 4,
        identities: [alice.credentialIdentity],
      );
      aliceStore.apply(removedCharlie.storageBatch);

      final charlieProcessedRemoval = await processMessageWithStorage(
        groupId: groupId,
        messageBytes: removedCharlie.commit,
        expectedAad: removeAad,
        expectedPreviousState: _expected(selfUpdated.resultingRoster),
        expectedResultingState: _expected(removedCharlie.resultingRoster),
        storageEntries: charlieStore.forGroup(groupId),
        storageFormatVersion: charlieStore.formatVersion,
      );
      _expectSameRoster(
        charlieProcessedRemoval.resultingRoster,
        removedCharlie.resultingRoster,
      );
      charlieStore.apply(charlieProcessedRemoval.storageBatch);
    });

    test('binds a deferred candidate to its exact group snapshot', () async {
      final alice = TestIdentity.create('candidate-alice');
      final store = _MemoryMlsStore();
      final groupId = utf8.encode('candidate-group');
      final created = await createGroupWithStorage(
        config: defaultConfig(),
        signerBytes: alice.signerBytes,
        explicitGroupId: groupId,
        expectedOwnerAuthority: MlsAuthorizedOwnerV1(
          expectedCredentialIdentity: alice.credentialIdentity,
          expectedSignaturePublicKey: alice.publicKey,
        ),
        storageEntries: store.globalSnapshot,
        storageFormatVersion: store.formatVersion,
      );
      store.apply(created.storageBatch);
      final owner = created.resultingRoster.leaves.single;
      final candidate = await selfUpdateWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        aad: utf8.encode('candidate/self-update'),
        expectedPreviousState: _expected(created.resultingRoster),
        expectedSelfAuthority: MlsAuthorizedSelfV1(
          leafIndex: owner.leafIndex,
          expectedCredentialIdentity: owner.credentialIdentity,
          expectedSignaturePublicKey: owner.signaturePublicKey,
        ),
        storageEntries: store.forGroup(groupId),
        storageFormatVersion: store.formatVersion,
      );
      final retainedCommit = List<int>.from(candidate.commit);
      final retainedBatchFingerprint = _batchFingerprint(
        candidate.storageBatch,
      );
      expect(
        candidate.storageBatch.upserts.every((entry) => entry.groupId != null),
        isTrue,
        reason: 'group candidate batches must not rewrite global inventory',
      );

      expect(
        mlsGroupStateDigest(
          groupId: groupId,
          storageEntries: store.forGroup(groupId),
          storageFormatVersion: store.formatVersion,
        ),
        orderedEquals(candidate.baseGroupStateSha256),
      );

      final message = await createMessageWithStorage(
        groupId: groupId,
        signerBytes: alice.signerBytes,
        message: utf8.encode('accepted before the candidate fence'),
        aad: utf8.encode('candidate/application'),
        storageEntries: store.forGroup(groupId),
        storageFormatVersion: store.formatVersion,
      );
      store.apply(message.storageBatch);

      expect(
        mlsGroupStateDigest(
          groupId: groupId,
          storageEntries: store.forGroup(groupId),
          storageFormatVersion: store.formatVersion,
        ),
        isNot(orderedEquals(candidate.baseGroupStateSha256)),
      );
      expect(candidate.commit, orderedEquals(retainedCommit));
      expect(
        _batchFingerprint(candidate.storageBatch),
        retainedBatchFingerprint,
      );
    });
  });
}

MlsExpectedRosterStateV1 _expected(MlsRosterSummaryV1 summary) =>
    MlsExpectedRosterStateV1(
      groupId: summary.groupId,
      epoch: summary.epoch,
      digestSha256: summary.digestSha256,
    );

void _expectRoster(
  MlsRosterSummaryV1 summary, {
  required List<int> groupId,
  required int epoch,
  required List<List<int>> identities,
}) {
  expect(summary.groupId, orderedEquals(groupId));
  expect(summary.epoch, BigInt.from(epoch));
  expect(summary.digestSha256, hasLength(32));
  expect(
    summary.digestSha256,
    orderedEquals(
      mlsRosterDigestV1(
        groupId: summary.groupId,
        epoch: summary.epoch,
        leaves: summary.leaves,
      ),
    ),
  );
  expect(
    summary.leaves.map((leaf) => base64Encode(leaf.credentialIdentity)),
    unorderedEquals(identities.map(base64Encode)),
  );
  final sortedIndexes = summary.leaves.map((leaf) => leaf.leafIndex).toList()
    ..sort();
  expect(summary.leaves.map((leaf) => leaf.leafIndex), sortedIndexes);
}

void _expectSameRoster(MlsRosterSummaryV1 actual, MlsRosterSummaryV1 expected) {
  expect(actual.groupId, orderedEquals(expected.groupId));
  expect(actual.epoch, expected.epoch);
  expect(actual.digestSha256, orderedEquals(expected.digestSha256));
  expect(actual.leaves, hasLength(expected.leaves.length));
  for (var index = 0; index < actual.leaves.length; index++) {
    expect(actual.leaves[index].leafIndex, expected.leaves[index].leafIndex);
    expect(
      actual.leaves[index].credentialIdentity,
      orderedEquals(expected.leaves[index].credentialIdentity),
    );
    expect(
      actual.leaves[index].signaturePublicKey,
      orderedEquals(expected.leaves[index].signaturePublicKey),
    );
  }
}

String _batchFingerprint(MlsStorageBatch batch) {
  final rows = batch.upserts.map((entry) {
    final encodedGroupId = entry.groupId == null
        ? '-'
        : base64Encode(entry.groupId!);
    return [
      base64Encode(entry.key),
      base64Encode(entry.value),
      encodedGroupId,
    ].join(':');
  }).toList()..sort();
  final deletes = batch.deletes.map(base64Encode).toList()..sort();
  return '${rows.join('|')}#${deletes.join('|')}';
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
    explicitGroupId: utf8.encode('$label-group'),
    expectedOwnerAuthority: MlsAuthorizedOwnerV1(
      expectedCredentialIdentity: sender.credentialIdentity,
      expectedSignaturePublicKey: sender.publicKey,
    ),
    storageEntries: senderStore.globalSnapshot,
    storageFormatVersion: senderStore.formatVersion,
  );
  senderStore.apply(created.storageBatch);

  final added = await addMembersWithStorage(
    groupId: created.groupId,
    signerBytes: sender.signerBytes,
    additions: [
      MlsAuthorizedKeyPackageV1(
        keyPackageBytes: receiverKeyPackage.keyPackageBytes,
        expectedCredentialIdentity: receiver.credentialIdentity,
        expectedSignaturePublicKey: receiver.publicKey,
      ),
    ],
    aad: utf8.encode('$label/add-receiver'),
    expectedPreviousState: _expected(created.resultingRoster),
    storageEntries: senderStore.forGroup(created.groupId),
    storageFormatVersion: senderStore.formatVersion,
  );
  senderStore.apply(added.storageBatch);

  final joined = await joinGroupFromWelcomeWithStorage(
    config: config,
    welcomeBytes: added.welcome!,
    signerBytes: receiver.signerBytes,
    expectedResultingState: _expected(added.resultingRoster),
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
    roster: added.resultingRoster,
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
    required this.roster,
  });

  final String label;
  final TestIdentity sender;
  final _MemoryMlsStore senderStore;
  final _MemoryMlsStore receiverStore;
  final List<int> groupId;
  final MlsRosterSummaryV1 roster;
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
