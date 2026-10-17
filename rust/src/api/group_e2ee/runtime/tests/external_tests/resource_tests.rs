//! Explicit opt-in measurement, not a stress test or a production transport.
//! Synthetic secret fixtures travel only through child stdin, never to disk.
use super::*;
use crate::external_resync::{self as public, codec};
use serde::{Deserialize, Serialize};
use std::time::Instant;

type SavedRow = (Vec<u8>, Vec<u8>, Option<Vec<u8>>);

#[test]
#[ignore = "explicit follow-up: one final single-member ADD at 100, not a benchmark rerun"]
fn measure_single_add_welcome_at_100() {
    let (mut a, _, _) = setup(99, false);
    let mut recipient = peer(100, 4);
    let kp = create_key_package_with_storage(
        profile(4).unwrap().ciphersuite,
        recipient.signer.to_vec(),
        recipient.id.clone(),
        recipient.public.clone(),
        None,
        vec![],
        1,
    )
    .unwrap();
    apply(&mut recipient, &kp.storage_batch);
    let kp_hash = Sha256::digest(&kp.key_package_bytes).to_vec();
    let p = prepare(
        context(&a),
        a.signer.to_vec(),
        transition(8),
        vec![MlsAuthorizedKeyPackageV1 {
            key_package_bytes: kp.key_package_bytes,
            expected_credential_identity: recipient.id.clone(),
            expected_signature_public_key: recipient.public.clone(),
        }],
        vec![],
        AAD.to_vec(),
        None,
    )
    .unwrap();
    let sizes = serde_json::json!({"operation":"single_add_99_to_100", "previous_leaves":99,
        "resulting_leaves":p.proposed.leaves.len(),"added_members":1,"aad_bytes":AAD.len(),
        "commit_bytes":p.commit.len(),"welcome_bytes":p.welcome.as_ref().unwrap().len(),
        "group_info_bytes":p.group_info.as_ref().unwrap().len(),
        "fixture_note":"99-member base created by one bulk ADD of98; measured final ADD is one member"});
    mutate(&mut a, &p.mutation);
    let merged = merge(context(&a), acceptance(&p)).unwrap();
    mutate(&mut a, &merged);
    let welcome = p.welcome.unwrap();
    let joined = join(JoinInput {
        config: profile(4).unwrap(),
        incarnation_id: vec![8; 16],
        signer: recipient.signer.to_vec(),
        welcome_sha256: Sha256::digest(&welcome).to_vec(),
        welcome,
        tree: None,
        resulting: expected(&a.roster),
        local_leaf: a
            .roster
            .leaves
            .iter()
            .find(|l| l.credential_identity == recipient.id)
            .unwrap()
            .clone(),
        target_key_package_sha256: kp_hash,
        entries: recipient.rows.0.clone(),
        storage_format: 1,
    })
    .unwrap();
    mutate(&mut recipient, &joined.mutation);
    assert_eq!(a.roster, recipient.roster);
    println!("N3A_SINGLE_ADD {sizes}; strict Welcome join passed");
    let out = std::path::PathBuf::from(std::env::var("MLS_N3A_OUTPUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("single-add-100.sizes.json"),
        serde_json::to_vec_pretty(&sizes).unwrap(),
    )
    .unwrap();
}

#[derive(Serialize, Deserialize)]
struct SavedPeer {
    signer: Vec<u8>,
    id: Vec<u8>,
    public: Vec<u8>,
    rows: Vec<SavedRow>,
    epoch: u64,
    leaves: Vec<(u32, Vec<u8>, Vec<u8>)>,
    digest: Vec<u8>,
}
impl Drop for SavedPeer {
    fn drop(&mut self) {
        self.signer.zeroize();
        for (_, value, _) in &mut self.rows {
            value.zeroize();
        }
    }
}
impl SavedPeer {
    fn save(p: &Peer) -> Self {
        Self {
            signer: p.signer.to_vec(),
            id: p.id.clone(),
            public: p.public.clone(),
            rows: p
                .rows
                .0
                .iter()
                .map(|r| (r.key.clone(), r.value.clone(), r.group_id.clone()))
                .collect(),
            epoch: p.roster.epoch,
            leaves: p
                .roster
                .leaves
                .iter()
                .map(|l| {
                    (
                        l.leaf_index,
                        l.credential_identity.clone(),
                        l.signature_public_key.clone(),
                    )
                })
                .collect(),
            digest: p.roster.digest_sha256.clone(),
        }
    }
    fn load(&self) -> Peer {
        Peer {
            signer: Zeroizing::new(self.signer.clone()),
            id: self.id.clone(),
            public: self.public.clone(),
            rows: Entries(
                self.rows
                    .iter()
                    .map(|(key, value, group_id)| MlsStorageEntry {
                        key: key.clone(),
                        value: value.clone(),
                        group_id: group_id.clone(),
                    })
                    .collect(),
            ),
            roster: MlsRosterSummaryV1 {
                group_id: GID.to_vec(),
                epoch: self.epoch,
                digest_sha256: self.digest.clone(),
                leaves: self
                    .leaves
                    .iter()
                    .map(|(i, id, k)| MlsRosterLeafV1 {
                        leaf_index: *i,
                        credential_identity: id.clone(),
                        signature_public_key: k.clone(),
                    })
                    .collect(),
            },
            retention: 4,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Fixture {
    survivor: SavedPeer,
    replacement: SavedPeer,
    old_identity: Vec<u8>,
    candidate: Vec<u8>,
    candidate_hash: Vec<u8>,
    commit: Vec<u8>,
    next_group_info: Vec<u8>,
    input_group_info: Vec<u8>,
    result_epoch: u64,
    result_digest: Vec<u8>,
    author_index: u32,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.candidate.zeroize();
    }
}
fn public_leaf(l: &MlsRosterLeafV1) -> public::Leaf {
    public::Leaf {
        index: l.leaf_index,
        identity: l.credential_identity.clone(),
        signature_key: l.signature_public_key.clone(),
    }
}
fn snapshot(f: &Fixture, a: &Peer) -> public::SnapshotRequest {
    public::SnapshotRequest {
        profile: 2,
        group_info: f.input_group_info.clone(),
        sha256: Sha256::digest(&f.input_group_info).to_vec(),
        expected: public::ExpectedState {
            group_id: GID.to_vec(),
            epoch: a.roster.epoch,
            roster_sha256: a.roster.digest_sha256.clone(),
        },
        expected_signer: public_leaf(
            a.roster
                .leaves
                .iter()
                .find(|l| l.credential_identity == a.id)
                .unwrap(),
        ),
    }
}
fn authority(f: &Fixture, a: &Peer, c: &Peer) -> MlsExternalCommitAcceptanceV1 {
    MlsExternalCommitAcceptanceV1 {
        binding: MlsExternalCommitBindingV1 {
            candidate_id: vec![33; 16],
            incarnation_id: vec![8; 16],
            transition: transition(33),
            previous_state: expected(&a.roster),
            old_leaf: a
                .roster
                .leaves
                .iter()
                .find(|l| l.credential_identity == f.old_identity)
                .unwrap()
                .clone(),
            author: MlsRosterLeafV1 {
                leaf_index: f.author_index,
                credential_identity: c.id.clone(),
                signature_public_key: c.public.clone(),
            },
            input_group_info_sha256: Sha256::digest(&f.input_group_info).to_vec(),
            commit_sha256: Sha256::digest(&f.commit).to_vec(),
            aad_sha256: Sha256::digest(b"N2 opaque authenticated external AAD4").to_vec(),
            resulting_group_info_sha256: Sha256::digest(&f.next_group_info).to_vec(),
        },
        resulting_state: MlsExpectedRosterStateV1 {
            group_id: GID.to_vec(),
            epoch: f.result_epoch,
            digest_sha256: f.result_digest.clone(),
        },
    }
}
fn setup(count: usize, sparse: bool) -> (Peer, Peer, Peer) {
    let mut a = peer(1, 4);
    let created = create(
        profile(4).unwrap(),
        a.signer.to_vec(),
        GID.to_vec(),
        vec![8; 16],
        MlsAuthorizedOwnerV1 {
            expected_credential_identity: a.id.clone(),
            expected_signature_public_key: a.public.clone(),
        },
        None,
        vec![],
        1,
    )
    .unwrap();
    mutate(&mut a, &created);
    let mut additions = Vec::new();
    for i in 1..count {
        let p = peer(((i + 1) % 256) as u8, 4);
        let kp = create_key_package_with_storage(
            profile(4).unwrap().ciphersuite,
            p.signer.to_vec(),
            p.id.clone(),
            p.public.clone(),
            None,
            vec![],
            1,
        )
        .unwrap();
        let mut private = Entries(kp.storage_batch.upserts);
        private.0.iter_mut().for_each(|r| r.value.zeroize());
        additions.push(MlsAuthorizedKeyPackageV1 {
            key_package_bytes: kp.key_package_bytes,
            expected_credential_identity: p.id.clone(),
            expected_signature_public_key: p.public.clone(),
        });
    }
    let p = prepare(
        context(&a),
        a.signer.to_vec(),
        transition(1),
        additions,
        vec![],
        AAD.to_vec(),
        None,
    )
    .unwrap();
    mutate(&mut a, &p.mutation);
    let m = merge(context(&a), acceptance(&p)).unwrap();
    mutate(&mut a, &m);
    if sparse {
        let keep: Vec<_> = (0..100).map(|i| i * 255 / 99).collect();
        let remove = a
            .roster
            .leaves
            .iter()
            .filter(|l| !keep.contains(&l.leaf_index))
            .map(|l| MlsAuthorizedRemovalV1 {
                leaf_index: l.leaf_index,
                expected_credential_identity: l.credential_identity.clone(),
                expected_signature_public_key: l.signature_public_key.clone(),
            })
            .collect();
        let p = prepare(
            context(&a),
            a.signer.to_vec(),
            transition(2),
            vec![],
            remove,
            AAD.to_vec(),
            None,
        )
        .unwrap();
        mutate(&mut a, &p.mutation);
        let m = merge(context(&a), acceptance(&p)).unwrap();
        mutate(&mut a, &m);
    }
    let old_id = a.roster.leaves.last().unwrap().credential_identity[0];
    let old = peer(old_id, 4); // Only stable identity is used; old private state is absent.
    let replacement = peer(old_id, 4);
    (a, old, replacement)
}
fn row_bytes(rows: &[MlsStorageEntry]) -> usize {
    rows.iter()
        .map(|r| r.key.len() + r.value.len() + r.group_id.as_ref().map_or(0, Vec::len))
        .sum()
}
fn rss_kib() -> u64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());
    std::str::from_utf8(&out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
fn checked_response(frame: &[u8]) -> Vec<u8> {
    let response = codec::execute(frame);
    let mut d = minicbor::Decoder::new(&response[12..]);
    assert_eq!(d.array().unwrap(), Some(3));
    d.bytes().unwrap();
    assert_eq!(d.u16().unwrap(), 0);
    response
}
#[test]
#[ignore = "explicit N3a subprocess measurement only"]
fn resource_child() {
    let op = std::env::var("MLS_N3A_OPERATION").unwrap();
    let mut raw = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut raw)
        .unwrap();
    let f: Fixture = serde_json::from_slice(&raw).unwrap();
    raw.zeroize();
    drop(raw);
    let a = f.survivor.load();
    let c = f.replacement.load();
    let receipt = authority(&f, &a, &c);
    // Construct all owned operation inputs before measuring the call.
    enum Job {
        Public(Vec<u8>),
        Prepare(MlsExternalPreparationV1, Vec<u8>),
        Accept(
            MlsExternalTargetContextV1,
            Vec<u8>,
            Vec<u8>,
            MlsExternalCommitAcceptanceV1,
        ),
        Receive(Context, Vec<u8>, MlsExternalCommitAcceptanceV1),
    }
    let base = snapshot(&f, &a);
    let job = match op.as_str() {
        "snapshot" => Job::Public(codec::encode_request(&codec::Request::Snapshot(base)).unwrap()),
        "successor" => Job::Public(
            codec::encode_request(&codec::Request::Successor(Box::new(
                public::SuccessorRequest {
                    base,
                    commit: f.commit.clone(),
                    commit_sha256: receipt.binding.commit_sha256.clone(),
                    expected_aad: b"N2 opaque authenticated external AAD4".to_vec(),
                    authorized_old: public_leaf(&receipt.binding.old_leaf),
                    authorized_new_identity: c.id.clone(),
                    authorized_new_signature_key: c.public.clone(),
                    reported_next_group_info: f.next_group_info.clone(),
                    reported_next_sha256: receipt.binding.resulting_group_info_sha256.clone(),
                },
            )))
            .unwrap(),
        ),
        "prepare" => Job::Prepare(
            MlsExternalPreparationV1 {
                candidate_id: vec![33; 16],
                incarnation_id: vec![8; 16],
                transition: transition(33),
                local_lifecycle_context_sha256: vec![77; 32],
                input_group_info: f.input_group_info.clone(),
                input_group_info_sha256: base.sha256,
                expected_previous_state: expected(&a.roster),
                expected_snapshot_signer: a
                    .roster
                    .leaves
                    .iter()
                    .find(|l| l.credential_identity == a.id)
                    .unwrap()
                    .clone(),
                authorized_old_leaf: receipt.binding.old_leaf.clone(),
                authorized_new_credential_identity: c.id.clone(),
                authorized_new_signature_public_key: c.public.clone(),
                aad: b"N2 opaque authenticated external AAD4".to_vec(),
                retention: 4,
            },
            c.signer.to_vec(),
        ),
        "accept" => Job::Accept(
            target(&c),
            f.candidate.clone(),
            f.candidate_hash.clone(),
            receipt,
        ),
        "receive" => Job::Receive(context(&a), f.commit.clone(), receipt),
        _ => panic!("unknown operation"),
    };
    drop(f);
    drop(a);
    drop(c);
    let before = rss_kib();
    let start = Instant::now();
    let output_bytes = match job {
        Job::Public(frame) => checked_response(&frame).len(),
        Job::Prepare(input, signer) => {
            let r = ext::prepare(input, signer).unwrap();
            let bytes = Zeroizing::new(r.candidate_bytes);
            bytes.len()
        }
        Job::Accept(target, bytes, hash, accepted) => {
            let r = ext::accept(target, bytes, hash, accepted).unwrap();
            let rows = Entries(r.storage_batch.upserts);
            row_bytes(&rows.0)
        }
        Job::Receive(context, commit, accepted) => {
            let r = ext::receive(
                context,
                commit,
                b"N2 opaque authenticated external AAD4".to_vec(),
                accepted,
                None,
            )
            .unwrap();
            let rows = Entries(r.storage_batch.upserts);
            row_bytes(&rows.0)
        }
    };
    let elapsed = start.elapsed().as_micros();
    let after = rss_kib();
    println!(
        "N3A_METRIC {}",
        serde_json::json!({"operation":op,"elapsed_us":elapsed,"rss_before_kib":before,"rss_after_kib":after,"output_bytes":output_bytes})
    );
}
#[test]
#[ignore = "bounded N3a measurements run only through Makefile"]
fn measure_bounded_external_resources() {
    let out = std::path::PathBuf::from(std::env::var("MLS_N3A_OUTPUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for (name, count, sparse) in [
        ("dense100", 100, false),
        ("dense256", 256, false),
        ("sparse100of256", 256, true),
    ] {
        let start = Instant::now();
        let (a, old, c) = setup(count, sparse);
        let prep = input(&a, &old, &c, 33);
        let gi = prep.input_group_info.clone();
        let candidate = ext::prepare(prep, c.signer.to_vec()).unwrap();
        let f = Fixture {
            survivor: SavedPeer::save(&a),
            replacement: SavedPeer::save(&c),
            old_identity: old.id.clone(),
            candidate: candidate.candidate_bytes,
            candidate_hash: candidate.info.candidate_sha256.clone(),
            commit: candidate.info.commit.clone(),
            next_group_info: candidate.info.group_info.clone(),
            input_group_info: gi,
            result_epoch: candidate.info.proposed_resulting_roster.epoch,
            result_digest: candidate
                .info
                .proposed_resulting_roster
                .digest_sha256
                .clone(),
            author_index: candidate.info.binding.author.leaf_index,
        };
        let receipt = authority(&f, &a, &c);
        let promoted = ext::accept(
            target(&c),
            f.candidate.clone(),
            f.candidate_hash.clone(),
            receipt.clone(),
        )
        .unwrap();
        let promoted_rows = Entries(promoted.storage_batch.upserts);
        let base = snapshot(&f, &a);
        let public_snapshot =
            codec::encode_request(&codec::Request::Snapshot(base.clone())).unwrap();
        let public_successor = codec::encode_request(&codec::Request::Successor(Box::new(
            public::SuccessorRequest {
                base,
                commit: f.commit.clone(),
                commit_sha256: receipt.binding.commit_sha256,
                expected_aad: b"N2 opaque authenticated external AAD4".to_vec(),
                authorized_old: public_leaf(&receipt.binding.old_leaf),
                authorized_new_identity: c.id.clone(),
                authorized_new_signature_key: c.public.clone(),
                reported_next_group_info: f.next_group_info.clone(),
                reported_next_sha256: receipt.binding.resulting_group_info_sha256,
            },
        )))
        .unwrap();
        for (operation, bytes) in [
            ("snapshot", &public_snapshot),
            ("successor", &public_successor),
        ] {
            std::fs::write(out.join(format!("{name}.{operation}.request.bin")), bytes).unwrap();
            std::fs::write(
                out.join(format!("{name}.{operation}.response.bin")),
                checked_response(bytes),
            )
            .unwrap();
        }
        let sizes = serde_json::json!({"shape":name,"leaves":a.roster.leaves.len(),"max_leaf_index":a.roster.leaves.last().unwrap().leaf_index,
            "fixture_setup_us":start.elapsed().as_micros(),"input_group_info_bytes":f.input_group_info.len(),"resulting_group_info_bytes":f.next_group_info.len(),
            "external_commit_bytes":f.commit.len(),"aad_bytes":b"N2 opaque authenticated external AAD4".len(),"snapshot_request_bytes":public_snapshot.len(),
            "successor_request_bytes":public_successor.len(),"snapshot_response_bytes":checked_response(&public_snapshot).len(),"successor_response_bytes":checked_response(&public_successor).len(),
            "candidate_bytes":f.candidate.len(),"survivor_row_count":a.rows.0.len(),"survivor_row_bytes":row_bytes(&a.rows.0),
            "promotion_row_count":promoted_rows.0.len(),"promotion_row_bytes":row_bytes(&promoted_rows.0),
            "product_gi_16k_fits":f.input_group_info.len()<=16384 && f.next_group_info.len()<=16384,"product_commit_16k_fits":f.commit.len()<=16384});
        println!("N3A_SIZE {sizes}");
        std::fs::write(
            out.join(format!("{name}.sizes.json")),
            serde_json::to_vec_pretty(&sizes).unwrap(),
        )
        .unwrap();
        let encoded = Zeroizing::new(serde_json::to_vec(&f).unwrap());
        for operation in ["snapshot", "successor", "prepare", "accept", "receive"] {
            let mut child=std::process::Command::new("/usr/bin/time").arg("-l").arg(std::env::current_exe().unwrap())
                .args(["--exact","api::group_e2ee::runtime::tests::external_tests::resource_tests::resource_child","--ignored","--nocapture","--test-threads=1"])
                .env("MLS_N3A_OPERATION",operation).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
            child.stdin.take().unwrap().write_all(&encoded).unwrap();
            let result = child.wait_with_output().unwrap();
            std::fs::write(
                out.join(format!("{name}.{operation}.stdout")),
                &result.stdout,
            )
            .unwrap();
            std::fs::write(out.join(format!("{name}.{operation}.time")), &result.stderr).unwrap();
            assert!(
                result.status.success(),
                "resource child failed: {name}/{operation}; inspect retained logs"
            );
            println!(
                "N3A_CHILD {name}/{operation} {}",
                String::from_utf8_lossy(&result.stdout)
            );
        }
    }
}
