//! Bounded read-only inspection tests; uses the existing real pending setup.
use super::*;

fn inspection(a: &Peer) -> v3::NativeReceiveRequestV3 {
    let mut c = context(a);
    v3::NativeReceiveRequestV3::InspectPending(v3::NativeInspectPendingRequestV3 {
        profile_id: 2,
        group_id: c.group_id.clone(),
        incarnation_id: c.incarnation_id.clone(),
        expected_current: c.expected_current.clone(),
        expected_base_group_state_sha256: c.expected_base_sha256.clone(),
        expected_retention: c.expected_retention,
        storage: v2::NativeStorageSnapshotV2 {
            storage_format_version: c.storage_format,
            entries: std::mem::take(&mut c.entries)
                .into_iter()
                .map(|mut r| v2::NativeStorageEntryV2 {
                    key: std::mem::take(&mut r.key),
                    value: std::mem::take(&mut r.value),
                    group_id: r.group_id.take(),
                })
                .collect(),
        },
    })
}
fn assert_pending(frame: &[u8], expected: Option<&MlsPendingCommitBinding>) {
    assert_eq!(&frame[..8], b"KMLS\0\x03\x05\0");
    let mut d = Decoder::new(&frame[12..]);
    assert_eq!(d.map().unwrap(), Some(3));
    assert_eq!((d.u8().unwrap(), d.u16().unwrap()), (0, 3));
    assert_eq!(d.u8().unwrap(), 1);
    assert!(!d.bool().unwrap());
    assert_eq!(d.u8().unwrap(), 2);
    assert_eq!(d.map().unwrap(), Some(1));
    assert_eq!(d.u8().unwrap(), 0);
    if let Some(p) = expected {
        assert_eq!(d.map().unwrap(), Some(10));
        for (key, bytes) in [
            (0, &p.group_id),
            (1, &p.incarnation_id),
            (2, &p.transition.command_id),
            (3, &p.transition.context_sha256),
        ] {
            assert_eq!(d.u8().unwrap(), key);
            assert_eq!(d.bytes().unwrap(), bytes);
        }
        assert_eq!(d.u8().unwrap(), 4);
        assert_eq!(
            v2::native_leaf_into_mls(v2::decode_leaf(&mut d).unwrap()),
            p.author
        );
        assert_eq!(d.u8().unwrap(), 5);
        assert_eq!(
            v2::native_expected_into_mls(v2::decode_expected_roster(&mut d).unwrap()),
            p.previous_state
        );
        for (key, bytes) in [(6, &p.commit_sha256), (7, &p.aad_sha256)] {
            assert_eq!(d.u8().unwrap(), key);
            assert_eq!(d.bytes().unwrap(), bytes);
        }
        for (key, bytes) in [(8, &p.welcome_sha256), (9, &p.group_info_sha256)] {
            assert_eq!(d.u8().unwrap(), key);
            if let Some(bytes) = bytes {
                assert_eq!(d.bytes().unwrap(), bytes);
            } else {
                d.null().unwrap();
            }
        }
    } else {
        d.null().unwrap();
    }
    assert_eq!(
        d.position(),
        frame.len() - 12,
        "no batch/plaintext/extra fields"
    );
}
#[test]
fn inspection_exact_pending_and_none_without_state_changes() {
    let (mut a, _) = pair(4);
    for pending in [false, true] {
        let binding = if pending {
            let p = self_prepare(&a, 44);
            assert!(p.binding.welcome_sha256.is_none());
            assert!(
                p.binding.group_info_sha256.is_some(),
                "no Welcome still has native GI authority"
            );
            mutate(&mut a, &p.mutation);
            Some(p.binding)
        } else {
            None
        };
        let before = context(&a).expected_base_sha256.clone();
        let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&inspection(&a)).unwrap());
        let original = Zeroizing::new(frame.to_vec());
        let decoded = v3::decode_native_receive_request_v3(&frame).unwrap();
        assert_eq!(
            v3::encode_native_receive_request_v3(&decoded).unwrap(),
            *frame
        );
        let actual = Zeroizing::new(v3::execute_native_receive_v3(&frame));
        assert_pending(&actual, binding.as_ref());
        assert_eq!(frame.as_slice(), original.as_slice());
        assert_eq!(context(&a).expected_base_sha256, before);
        assert_eq!(
            inspect_pending(context(&a)).unwrap().map(|p| p.binding),
            binding
        );
    }
}
#[test]
fn inspection_context_and_frame_failures_leave_state_unchanged() {
    let (mut a, _) = pair(4);
    let p = self_prepare(&a, 44);
    mutate(&mut a, &p.mutation);
    let before = context(&a).expected_base_sha256.clone();
    for (case, wanted) in [
        (0, Error::UnsupportedProfile),
        (1, Error::UnsupportedRetention),
        (2, Error::ConfigurationMismatch),
        (3, Error::BaseStateMismatch),
        (4, Error::GroupMismatch),
        (5, Error::GroupMismatch),
        (6, Error::PreviousEpochMismatch),
        (7, Error::PreviousRosterMismatch),
    ] {
        let mut req = inspection(&a);
        let v3::NativeReceiveRequestV3::InspectPending(r) = &mut req else {
            panic!()
        };
        match case {
            0 => r.profile_id = 1,
            1 => r.expected_retention = 3,
            2 => r.expected_retention = 2,
            3 => r.expected_base_group_state_sha256[0] ^= 1,
            4 => r.expected_current.group_id[0] ^= 1,
            5 => r.incarnation_id[0] ^= 1,
            6 => r.expected_current.epoch += 1,
            _ => r.expected_current.digest_sha256[0] ^= 1,
        }
        let frame = Zeroizing::new(v3::encode_unchecked_fixture(&req).unwrap());
        assert_eq!(
            error(&v3::execute_native_receive_v3(&frame)),
            wanted as u16,
            "case {case}"
        );
        assert_eq!(context(&a).expected_base_sha256, before);
    }
    let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&inspection(&a)).unwrap());
    assert_eq!(
        error(&v3::execute_native_receive_v3(&frame[..frame.len() - 1])),
        Error::InvalidFrame as u16
    );
    let mut bad = Zeroizing::new(frame.to_vec());
    bad[12] = 0xbf;
    assert_eq!(
        error(&v3::execute_native_receive_v3(&bad)),
        Error::NoncanonicalEncoding as u16
    );
    let mut oversized = Zeroizing::new(vec![0; v2::NATIVE_RECEIVE_REQUEST_MAX_BYTES + 1]);
    oversized[..12].copy_from_slice(&frame[..12]);
    assert_eq!(
        error(&v3::execute_native_receive_v3(&oversized)),
        Error::LimitExceeded as u16
    );
    assert_eq!(context(&a).expected_base_sha256, before);
    assert_eq!(
        inspect_pending(context(&a)).unwrap().unwrap().binding,
        p.binding
    );
}
#[test]
#[ignore = "exports synthetic pending inspection fixtures for the native ABI harness"]
fn export_frame3_inspection_fixtures() {
    let out = std::path::PathBuf::from(std::env::var("MLS_N3B_ADAPTER_FIXTURES").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let (mut a, _) = pair(4);
    for name in ["inspect_none", "inspect_pending", "inspect_wrong_base"] {
        if name == "inspect_pending" {
            let p = self_prepare(&a, 44);
            mutate(&mut a, &p.mutation);
        }
        let mut req = inspection(&a);
        if name == "inspect_wrong_base" {
            let v3::NativeReceiveRequestV3::InspectPending(r) = &mut req else {
                panic!()
            };
            r.expected_base_group_state_sha256[0] ^= 1;
        }
        let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&req).unwrap());
        let actual = Zeroizing::new(v3::execute_native_receive_v3(&frame));
        if name == "inspect_wrong_base" {
            assert_eq!(error(&actual), Error::BaseStateMismatch as u16);
        } else {
            assert_pending(
                &actual,
                inspect_pending(context(&a))
                    .unwrap()
                    .as_ref()
                    .map(|p| &p.binding),
            );
        }
        std::fs::write(out.join(format!("{name}.request.bin")), &*frame).unwrap();
        std::fs::write(out.join(format!("{name}.response.bin")), &*actual).unwrap();
    }
}
