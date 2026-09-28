//! Private native-codec transport and paired host measurements; not a new ABI.
use super::*;
use crate::native_receive_v1::*;
use serde_json::{Value, json};
use std::time::Instant;

fn native_leaf(l: &Leaf) -> NativeLeafAuthorityV1 {
    NativeLeafAuthorityV1 {
        leaf_index: l.0,
        credential_identity: l.1.clone(),
        signature_public_key: l.2.clone(),
    }
}
fn native_roster(r: &Roster) -> NativeRosterSummaryV1 {
    NativeRosterSummaryV1 {
        group_id: GID.to_vec(),
        epoch: r.epoch,
        leaves: r.leaves.iter().map(native_leaf).collect(),
        digest_sha256: r.digest.clone(),
    }
}
fn native_expected(r: &Roster) -> NativeExpectedRosterStateV1 {
    NativeExpectedRosterStateV1 {
        group_id: GID.to_vec(),
        epoch: r.epoch,
        digest_sha256: r.digest.clone(),
    }
}
fn request(store: &Store, wire: &[u8], sender: &Leaf) -> NativeReceiveRequestV1 {
    NativeReceiveRequestV1::Process {
        operation: NativeReceiveOperationV1::Application,
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        group_id: GID.to_vec(),
        message_bytes: wire.to_vec(),
        expected_aad: AAD.to_vec(),
        expected_sender: native_leaf(sender),
        expected_previous_state: native_expected(&store.roster),
        expected_resulting_state: native_expected(&store.roster),
        expected_base_group_state_sha256: store.digest(),
        storage: NativeStorageSnapshotV1 {
            storage_format_version: 1,
            entries: store
                .rows
                .iter()
                .map(|r| NativeStorageEntryV1 {
                    key: r.0.clone(),
                    value: r.1.clone(),
                    group_id: r.2.clone(),
                })
                .collect(),
        },
    }
}
// This seam reuses the released bounded binary codec but deliberately does NOT
// claim that profile-v1 accepts retention. The explicit test argument controls
// a private operation; production execute_native_receive_v1 remains unchanged.
fn execute_private(
    frame: &[u8],
    authority: &Store,
    message_epoch: u64,
    retain: u32,
) -> Result<(Store, Vec<u8>, Value), ProofError> {
    let start = Instant::now();
    let req = decode_native_receive_request_v1(frame).map_err(|_| ProofError::NativeLimit)?;
    let decode_ns = start.elapsed().as_nanos();
    let NativeReceiveRequestV1::Process {
        operation,
        group_id,
        message_bytes,
        expected_aad,
        expected_sender,
        expected_previous_state,
        expected_resulting_state,
        expected_base_group_state_sha256,
        storage,
        ..
    } = &req
    else {
        return Err(ProofError::Authority);
    };
    if *operation != NativeReceiveOperationV1::Application
        || group_id != GID
        || expected_previous_state != &native_expected(&authority.roster)
        || expected_resulting_state != expected_previous_state
    {
        return Err(ProofError::Authority);
    }
    let mut snapshot = authority.clone();
    snapshot.rows = storage
        .entries
        .iter()
        .map(|e| (e.key.clone(), e.value.clone(), e.group_id.clone()))
        .collect();
    if &snapshot.digest() != expected_base_group_state_sha256 {
        return Err(ProofError::BaseMismatch);
    }
    let sender = (
        expected_sender.leaf_index,
        expected_sender.credential_identity.clone(),
        expected_sender.signature_public_key.clone(),
    );
    let start = Instant::now();
    let (next, plaintext, batch) = receive_application(
        &snapshot,
        message_bytes,
        expected_aad,
        &sender,
        message_epoch,
        retain,
    )?;
    let receive_ns = start.elapsed().as_nanos();
    let rewrite_bytes: usize = batch.upserts.iter().map(|e| e.value.len()).sum();
    let outcome = NativeReceiveOutcomeV1::success(NativeReceiveSuccessV1::Application {
        sender: native_leaf(&sender),
        previous_roster: native_roster(&snapshot.roster),
        resulting_roster: native_roster(&next.roster),
        resulting_group_state_sha256: next.digest(),
        plaintext,
        storage_batch: NativeStorageBatchV1 {
            storage_format_version: 1,
            upserts: batch
                .upserts
                .iter()
                .map(|e| NativeStorageEntryV1 {
                    key: e.key.clone(),
                    value: e.value.clone(),
                    group_id: e.group_id.clone(),
                })
                .collect(),
            deletes: batch.deletes.clone(),
            deleted_group_ids: batch.deleted_group_ids.clone(),
        },
    });
    assert!(!outcome.state_applied);
    let start = Instant::now();
    let result =
        encode_native_receive_outcome_v1(Some(NativeReceiveOperationV1::Application), &outcome)
            .map_err(|_| ProofError::NativeLimit)?;
    let encode_ns = start.elapsed().as_nanos();
    Ok((
        next,
        result.clone(),
        json!({"request_bytes":frame.len(),"result_bytes":result.len(),"decode_ns":decode_ns,"receive_ns":receive_ns,"encode_ns":encode_ns,"upsert_count":batch.upserts.len(),"delete_count":batch.deletes.len(),"rewrite_value_bytes":rewrite_bytes,"decoded_value_bytes":snapshot.rows.iter().map(|r|r.1.len()).sum::<usize>(),"result_value_copy_bytes":rewrite_bytes}),
    ))
}
fn removal(leaf: &Leaf) -> MlsAuthorizedRemovalV1 {
    MlsAuthorizedRemovalV1 {
        leaf_index: leaf.0,
        expected_credential_identity: leaf.1.clone(),
        expected_signature_public_key: leaf.2.clone(),
    }
}
fn advance(peers: &mut [Peer], retain: u32, command: u64) {
    let p = prepare(&peers[0], command, vec![], vec![], true).unwrap();
    let b = p.pending.clone().unwrap();
    peers[0].store = settle(&p, &p.digest(), &b, accepted(&b), retain).unwrap();
    for peer in peers.iter_mut().skip(1) {
        peer.store = receive_commit(&peer.store, &b, retain).unwrap();
    }
}
fn swap(peers: &mut [Peer], retain: u32, command: u64) {
    let old = peers[0].store.roster.leaves.last().unwrap().clone();
    let mut newcomer = peer(201);
    let addition = package(&mut newcomer);
    let p = prepare(
        &peers[0],
        command,
        vec![addition],
        vec![removal(&old)],
        false,
    )
    .unwrap();
    let b = p.pending.clone().unwrap();
    peers[0].store = settle(&p, &p.digest(), &b, accepted(&b), retain).unwrap();
    for peer in peers.iter_mut().skip(1) {
        peer.store = receive_commit(&peer.store, &b, retain).unwrap();
    }
    let joined = legacy_join_group_from_welcome_with_storage(
        config(retain),
        b.welcome.clone().unwrap(),
        None,
        newcomer.signer.to_vec(),
        b.resulting.expected(),
        newcomer.store.entries(),
        1,
    )
    .unwrap();
    newcomer.store.apply(&joined.storage_batch);
    newcomer.store.roster = Roster::from(&joined.resulting_roster);
    *peers.last_mut().unwrap() = newcomer;
}
#[test]
fn transition_competition_leaf_reuse_and_native_guard() {
    for retain in [2, 4] {
        let mut peers = fixture(4, retain);
        let old_epoch = peers[0].store.roster.epoch;
        let old_sender = peers[0].store.roster.leaves[3].clone();
        let old_wire = send(&mut peers[3], b"historical-occupant");
        let losing = prepare(&peers[0], 10, vec![], vec![], true).unwrap();
        let lb = losing.pending.clone().unwrap();
        let winning = prepare(&peers[1], 11, vec![], vec![], true).unwrap();
        let wb = winning.pending.clone().unwrap();
        assert!(matches!(
            receive_commit(&losing, &wb, retain),
            Err(ProofError::PendingControl)
        ));
        let discarded = settle(&losing, &losing.digest(), &lb, Decision::Rejected, retain).unwrap();
        peers[0].store = receive_commit(&discarded, &wb, retain).unwrap();
        peers[1].store = settle(&winning, &winning.digest(), &wb, accepted(&wb), retain).unwrap();
        for peer in peers.iter_mut().skip(2) {
            peer.store = receive_commit(&peer.store, &wb, retain).unwrap();
        }
        swap(&mut peers, retain, 12);
        let new_sender = peers[0]
            .store
            .roster
            .leaves
            .iter()
            .find(|l| l.0 == old_sender.0)
            .unwrap()
            .clone();
        assert_ne!(old_sender, new_sender);
        let mut wrong_key_only = old_sender.clone();
        wrong_key_only.2[0] ^= 1;
        assert!(
            matches!(
                receive_application(
                    &peers[0].store,
                    &old_wire,
                    AAD,
                    &wrong_key_only,
                    old_epoch,
                    retain
                ),
                Err(ProofError::Authority)
            ),
            "same historical identity/index cannot authorize a different signature key"
        );
        assert!(
            receive_application(
                &peers[0].store,
                &old_wire,
                AAD,
                &new_sender,
                old_epoch,
                retain
            )
            .is_err()
        );
        let (next, plain, _) = receive_application(
            &peers[0].store,
            &old_wire,
            AAD,
            &old_sender,
            old_epoch,
            retain,
        )
        .unwrap();
        assert_eq!(plain, b"historical-occupant");
        assert_eq!(next.roster, peers[0].store.roster);
        let frame =
            encode_native_receive_request_v1(&request(&peers[0].store, &old_wire, &old_sender))
                .unwrap();
        let prod = execute_native_receive_v1(&frame);
        let expected = encode_native_receive_outcome_v1(
            Some(NativeReceiveOperationV1::Application),
            &NativeReceiveOutcomeV1::failure(NativeReceiveErrorCodeV1::ConfigurationMismatch),
        )
        .unwrap();
        assert_eq!(prod, expected);
        execute_private(&frame, &peers[0].store, old_epoch, retain).unwrap();
        // Removal evicts the target; old secrets cannot bypass inactivity.
        let retired = peers[3].store.clone();
        let wire = send(&mut peers[1], b"before-remove");
        let sender = peers[0].store.roster.leaves[1].clone();
        let ep = peers[0].store.roster.epoch;
        let p = prepare(
            &peers[0],
            13,
            vec![],
            vec![removal(peers[0].store.roster.leaves.last().unwrap())],
            false,
        )
        .unwrap();
        let b = p.pending.clone().unwrap();
        let removed = receive_commit(&retired, &b, retain).unwrap();
        assert!(matches!(
            receive_application(&removed, &wire, AAD, &sender, ep, retain),
            Err(ProofError::Inactive)
        ));
        assert!(
            receive_commit(&retired, &lb, retain).is_err(),
            "old Commit cannot replace required current control"
        );
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Sample {
    epoch: u64,
    sender: Leaf,
    wire: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct Dataset {
    count: usize,
    retain: u32,
    activity: String,
    store: Store,
    current: Vec<Sample>,
    oldest: Vec<Sample>,
    evicted: Vec<Sample>,
    manifest: Value,
}
fn stats(store: &Store) -> Value {
    let secrets = store
        .rows
        .iter()
        .find(|r| String::from_utf8_lossy(&r.0).contains("MessageSecrets"))
        .unwrap();
    let v: Value = serde_json::from_slice(&secrets.1).unwrap();
    fn inventory(tree: &Value) -> Value {
        let apps = tree["application_sender_ratchets"].as_array().unwrap();
        let handshakes = tree["handshake_sender_ratchets"].as_array().unwrap();
        let skipped: usize = apps
            .iter()
            .chain(handshakes)
            .filter_map(|r| r["DecryptionRatchet"]["past_secrets"].as_array())
            .map(|a| a.iter().filter(|x| !x.is_null()).count())
            .sum();
        json!({"leaf_capacity":tree["leaf_nodes"].as_array().unwrap().len(),"tree_size":tree["size"],"application_ratchets":apps.iter().filter(|r|!r.is_null()).count(),"handshake_ratchets":handshakes.iter().filter(|r|!r.is_null()).count(),"available_skipped_keys":skipped})
    }
    let inventory = json!({"current":inventory(&v["message_secrets"]["secret_tree"]),"past":v["past_epoch_trees"].as_array().unwrap().iter().map(|e|json!({"epoch":e["epoch"],"tree":inventory(&e["message_secrets"]["secret_tree"])})).collect::<Vec<_>>()});
    // Record actual schema-derived inventory without retaining extra secret dumps.
    fn shape(v: &Value) -> Value {
        match v {
            Value::Array(a) => {
                json!({"length":a.len(),"non_null":a.iter().filter(|x|!x.is_null()).count(),"first":a.first().map(shape)})
            }
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, v)| (k.clone(), shape(v))).collect())
            }
            _ => Value::Null,
        }
    }
    json!({"inventory":inventory,"rows":store.rows.len(),"snapshot_value_bytes":store.rows.iter().map(|r|r.1.len()).sum::<usize>(),"snapshot_key_bytes":store.rows.iter().map(|r|r.0.len()).sum::<usize>(),"message_secrets_bytes":secrets.1.len(),"message_secrets_shape":shape(&v),"historical_authority_json_bytes":serde_json::to_vec(&store.history).unwrap().len(),"pending_metadata_json_bytes":serde_json::to_vec(&store.pending).unwrap().len(),"max_row_bytes":store.rows.iter().map(|r|r.1.len()).max(),"snapshot_test_json_bytes":serde_json::to_vec(store).unwrap().len()})
}
fn generate(count: usize, retain: u32, active: bool) -> Dataset {
    let mut peers = fixture(count, retain);
    let mut messages = BTreeMap::<u64, Vec<Sample>>::new();
    for step in 0..7 {
        let epoch = peers[0].store.roster.epoch;
        let senders = if active { count } else { 2 };
        for i in 1..senders {
            let leaf = peers[0]
                .store
                .roster
                .leaves
                .iter()
                .find(|l| l.1 == peers[i].identity)
                .unwrap()
                .clone();
            let wires: Vec<_> = (0..5).map(|_| send(&mut peers[i], &[0x61; 128])).collect();
            if active {
                for j in [4, 0, 2] {
                    peers[0].store =
                        receive_application(&peers[0].store, &wires[j], AAD, &leaf, epoch, retain)
                            .unwrap()
                            .0;
                }
            }
            if i == 1 {
                messages.insert(
                    epoch,
                    [1, 3]
                        .iter()
                        .map(|j| Sample {
                            epoch,
                            sender: leaf.clone(),
                            wire: wires[*j].clone(),
                        })
                        .collect(),
                );
            }
        }
        if step < 6 {
            if step == 2 {
                swap(&mut peers, retain, 100 + step);
            } else {
                advance(&mut peers, retain, 100 + step);
            }
        }
    }
    let current = peers[0].store.roster.epoch;
    let pending = prepare(&peers[0], 200, vec![], vec![], true).unwrap();
    Dataset {
        count,
        retain,
        activity: if active { "active" } else { "quiescent" }.into(),
        manifest: stats(&pending),
        store: pending,
        current: messages.remove(&current).unwrap(),
        oldest: messages.remove(&(current - retain as u64)).unwrap(),
        evicted: messages.remove(&(current - retain as u64 - 1)).unwrap(),
    }
}
fn rss_kib() -> u64 {
    let out = Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
#[test]
fn measure_child() {
    let Ok(path) = std::env::var("MLS_PROOF_DATASET") else {
        return;
    };
    let baseline = rss_kib();
    let decode = Instant::now();
    let bytes = Zeroizing::new(fs::read(&path).unwrap());
    let d: Dataset = serde_json::from_slice(&bytes).unwrap();
    let snapshot_decode_ns = decode.elapsed().as_nanos();
    let mut records = vec![];
    for (name, samples) in [("current", &d.current), ("oldest", &d.oldest)] {
        let mut store = d.store.clone();
        for (index, s) in samples.iter().enumerate() {
            let frame = Zeroizing::new(
                encode_native_receive_request_v1(&request(&store, &s.wire, &s.sender)).unwrap(),
            );
            let (next, result, metrics) =
                execute_private(&frame, &store, s.epoch, d.retain).unwrap();
            assert!(!result.is_empty());
            store = next;
            records.push(json!({"case":name,"sample":index,"metrics":metrics}));
        }
    }
    for s in &d.evicted {
        let error = receive_application(&d.store, &s.wire, AAD, &s.sender, s.epoch, d.retain)
            .err()
            .unwrap();
        assert_eq!(error, ProofError::PastEpochUnavailable);
    }
    let report = json!({"count":d.count,"retain":d.retain,"activity":d.activity,"baseline_rss_kib":baseline,"end_rss_kib":rss_kib(),"snapshot_read_decode_ns":snapshot_decode_ns,"samples":records,"manifest":d.manifest,"fixture_sha256":format!("{:x}",Sha256::digest(&*bytes)),"evicted":"PastEpochUnavailable/no output"});
    fs::write(
        std::env::var("MLS_PROOF_REPORT").unwrap(),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
fn paired_retention_measurements() {
    if std::env::var("MLS_PROOF_MEASURE").as_deref() != Ok("1") {
        return;
    }
    let dir = proof_dir().join(format!("paired-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for count in [4, 100] {
        for active in [false, true] {
            for retain in [2, 4] {
                let dataset = generate(count, retain, active);
                let bytes = Zeroizing::new(serde_json::to_vec(&dataset).unwrap());
                fs::write(
                    dir.join(format!("fixture-{count}-{active}-{retain}.json")),
                    &bytes,
                )
                .unwrap();
            }
            for rep in 0..3 {
                for retain in if rep % 2 == 0 { [2, 4] } else { [4, 2] } {
                    let stem = format!("run-{count}-{active}-{retain}-{rep}");
                    let result = Command::new("/usr/bin/time")
                        .args([
                            "-l",
                            std::env::current_exe().unwrap().to_str().unwrap(),
                            "--exact",
                            "api::group_e2ee::pending_retention_proof::measurement::measure_child",
                            "--nocapture",
                        ])
                        .env_remove("MLS_PROOF_MEASURE")
                        .env(
                            "MLS_PROOF_DATASET",
                            dir.join(format!("fixture-{count}-{active}-{retain}.json")),
                        )
                        .env("MLS_PROOF_REPORT", dir.join(format!("{stem}.json")))
                        .output()
                        .unwrap();
                    fs::write(dir.join(format!("{stem}.stdout")), &result.stdout).unwrap();
                    fs::write(dir.join(format!("{stem}.stderr")), &result.stderr).unwrap();
                    assert!(
                        result.status.success(),
                        "measurement failed: {stem}; raw evidence in {}",
                        dir.display()
                    );
                }
            }
        }
    }
    println!("P2 paired measurement evidence {}", dir.display());
}
