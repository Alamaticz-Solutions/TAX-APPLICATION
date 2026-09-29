use chrono::{Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{
    coalesce::{coalesce_domain_changes, CoalescedSelectiveRefresh},
    model::{
        DomainChangeEnvelope, DomainChangeError, DomainChangeKind, DomainChangeSource,
        DomainInvalidationTarget, DomainProjectionWatermark, DomainTargetKind,
        DomainTransportPosition, SelectiveRefreshNotice, DOMAIN_CHANGE_SCHEMA_VERSION,
        SELECTIVE_REFRESH_SCHEMA_VERSION,
    },
    store::{
        DomainChangeAdapterErrorCode, DomainChangeAppendOutcome, DomainChangeBatch,
        DomainChangeStore, DomainChangeStoreError, InMemoryDomainChangeStore, StoredDomainChange,
    },
};

fn source(source_id: &str, projection_id: &str) -> DomainChangeSource {
    DomainChangeSource::try_new(
        source_id,
        projection_id,
        format!("{source_id}.{projection_id}@1"),
    )
    .unwrap()
}

fn view(locator: &str) -> DomainInvalidationTarget {
    DomainInvalidationTarget::try_new(DomainTargetKind::View, locator).unwrap()
}

fn collection(locator: &str) -> DomainInvalidationTarget {
    DomainInvalidationTarget::try_new(DomainTargetKind::Collection, locator).unwrap()
}

fn record() -> DomainInvalidationTarget {
    DomainInvalidationTarget::try_new(
        DomainTargetKind::Record,
        "rl_0123456789abcdef0123456789abcdef",
    )
    .unwrap()
}

fn event_with(
    event_id: &str,
    tenant_id: &str,
    source_id: &str,
    projection_id: &str,
    offset: u64,
    revision: u64,
    targets: Vec<DomainInvalidationTarget>,
    kind: DomainChangeKind,
) -> DomainChangeEnvelope {
    DomainChangeEnvelope::try_new(
        event_id,
        tenant_id,
        format!("request/{event_id}"),
        Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap(),
        source(source_id, projection_id),
        DomainTransportPosition::try_new(format!("{source_id}.{projection_id}.change"), 2, offset)
            .unwrap(),
        kind,
        DomainProjectionWatermark::try_new(
            revision,
            Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap()
                + Duration::seconds(revision as i64),
        )
        .unwrap(),
        targets,
    )
    .unwrap()
}

fn event(event_id: &str, offset: u64, revision: u64) -> DomainChangeEnvelope {
    event_with(
        event_id,
        "tenant-a",
        "source-a",
        "projection-a",
        offset,
        revision,
        vec![view("work-items")],
        DomainChangeKind::Invalidated,
    )
}

fn event_with_observed_at(
    event_id: &str,
    offset: u64,
    revision: u64,
    targets: Vec<DomainInvalidationTarget>,
    kind: DomainChangeKind,
    observed_at: chrono::DateTime<Utc>,
) -> DomainChangeEnvelope {
    let mut value = serde_json::to_value(event_with(
        event_id,
        "tenant-a",
        "source-a",
        "projection-a",
        offset,
        revision,
        targets,
        kind,
    ))
    .unwrap();
    value["projection"]["observed_at"] = serde_json::to_value(observed_at).unwrap();
    serde_json::from_value(value).unwrap()
}

fn stored(sequence: u64, envelope: DomainChangeEnvelope) -> StoredDomainChange {
    let digest = envelope.semantic_digest().unwrap();
    StoredDomainChange::try_new(sequence, digest, envelope).unwrap()
}

fn assert_error_redacted(error: &(impl std::fmt::Debug + std::fmt::Display), sentinels: &[&str]) {
    let rendered = format!("{error:?}\n{error}");
    assert!(!rendered.trim().is_empty());
    let rendered_lower = rendered.to_ascii_lowercase();
    for sentinel in sentinels {
        assert!(
            !rendered_lower.contains(&sentinel.to_ascii_lowercase()),
            "error output disclosed sentinel {sentinel:?}: {rendered}"
        );
    }
}

// DC-01 / SD-N01
#[test]
fn contracts_round_trip_and_reject_wrong_versions_and_unknown_members() {
    let envelope = event("event-1", 1, 1);
    let bytes = serde_json::to_vec(&envelope).unwrap();
    let decoded: DomainChangeEnvelope = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, envelope);

    let mut unknown = serde_json::to_value(&envelope).unwrap();
    unknown["source"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<DomainChangeEnvelope>(unknown).is_err());

    let mut wrong = serde_json::to_value(&envelope).unwrap();
    wrong["schema_version"] = serde_json::json!("appfw.domain_change@2");
    assert!(serde_json::from_value::<DomainChangeEnvelope>(wrong).is_err());
    assert_eq!(DOMAIN_CHANGE_SCHEMA_VERSION, "appfw.domain_change@1");
    assert_eq!(
        SELECTIVE_REFRESH_SCHEMA_VERSION,
        "appfw.selective_refresh@1"
    );
}

// DC-02 / SD-N01 / SD-N12
#[tokio::test]
async fn serialized_contracts_expose_only_internal_metadata_and_require_refetch() {
    let store = InMemoryDomainChangeStore::default();
    store.append(event("event-1", 1, 1)).await.unwrap();
    let units = coalesce_domain_changes(&store.read_after(0, 10).await.unwrap()).unwrap();
    let notice = &units[0].notices()[0];
    let json = serde_json::to_value(notice).unwrap();
    assert_eq!(json["authorized_refetch_required"], true);
    for prohibited in ["payload", "fields", "record", "credentials", "resume_id"] {
        assert!(json.get(prohibited).is_none());
    }
    let mut bypass = json;
    bypass["authorized_refetch_required"] = serde_json::json!(false);
    assert!(serde_json::from_value::<SelectiveRefreshNotice>(bypass).is_err());
}

// DC-03 / SD-N02 / SD-N03
#[test]
fn sensitive_corpus_classifies_every_model_and_store_string_slot_exactly() {
    const KEY: &str = "MC4CAQAwBQYDK2VwBCIEIJiGLkWAlHdIeMza9cdqgW0p86icRRK7WydQw1GJH1cJ";
    for value in [
        "bearer abc",
        "password=value",
        "person@example.com",
        "123-45-6789",
        KEY,
    ] {
        let error =
            DomainChangeSource::try_new(value, "projection-a", "source-a.schema@1").unwrap_err();
        assert_eq!(
            error,
            DomainChangeError::SensitiveValue {
                field: "source.source_id"
            }
        );
    }
    assert!(DomainChangeSource::try_new("source-a", "projection-a", "source-a.schema@1").is_ok());

    let sensitive = "password=sentinel";
    assert_eq!(
        DomainTransportPosition::try_new(sensitive, 0, 0).unwrap_err(),
        DomainChangeError::SensitiveValue {
            field: "transport.stream_id"
        }
    );
    assert_eq!(
        DomainInvalidationTarget::try_new(DomainTargetKind::View, sensitive).unwrap_err(),
        DomainChangeError::SensitiveValue {
            field: "targets.locator"
        }
    );
    let valid_source = source("source-a", "projection-a");
    let valid_transport = DomainTransportPosition::try_new("stream-a", 0, 1).unwrap();
    let valid_watermark =
        DomainProjectionWatermark::try_new(1, Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap())
            .unwrap();
    for (event_id, tenant_id, correlation_id, expected_field) in [
        (sensitive, "tenant-a", "request/1", "event_id"),
        ("event-1", sensitive, "request/1", "tenant_id"),
        ("event-1", "tenant-a", sensitive, "correlation_id"),
    ] {
        assert_eq!(
            DomainChangeEnvelope::try_new(
                event_id,
                tenant_id,
                correlation_id,
                Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap(),
                valid_source.clone(),
                valid_transport.clone(),
                DomainChangeKind::Invalidated,
                valid_watermark.clone(),
                vec![view("work-items")],
            )
            .unwrap_err(),
            DomainChangeError::SensitiveValue {
                field: expected_field
            }
        );
    }
}

// DC-04 / SD-N03
#[test]
fn assignment_provider_jwt_email_ssn_boundaries_are_exact() {
    let header = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        br#"{"alg":"HS256"}"#,
    );
    for value in [
        format!("{header}.e30.c2lnbmF0dXJl"),
        "ghp_AAAAAAAAAAAAAAAAAAAA".to_string(),
        "person@example.com".to_string(),
        "123-45-6789".to_string(),
    ] {
        assert!(DomainChangeSource::try_new(value, "projection-a", "source-a.schema@1").is_err());
    }
    for value in ["ordinary-id", "person@example", "a123-45-6789"] {
        assert!(DomainChangeSource::try_new(value, "projection-a", "source-a.schema@1").is_ok());
    }
}

// DC-05 / SD-N02
#[test]
fn opaque_record_locator_and_target_boundaries_are_exact() {
    assert!(DomainInvalidationTarget::try_new(DomainTargetKind::Record, "raw-record-1").is_err());
    assert!(DomainInvalidationTarget::try_new(
        DomainTargetKind::Record,
        "rl_0123456789abcdef0123456789abcdef"
    )
    .is_ok());
    assert!(DomainInvalidationTarget::try_new(DomainTargetKind::View, "x".repeat(128)).is_ok());
    assert!(DomainInvalidationTarget::try_new(DomainTargetKind::View, "x".repeat(129)).is_err());
    assert_eq!(record().kind(), DomainTargetKind::Record);
}

// DC-06
#[test]
fn event_and_notice_semantic_digests_are_canonical_and_sensitive_to_semantics() {
    let first = event_with(
        "event-1",
        "tenant-a",
        "source-a",
        "projection-a",
        1,
        1,
        vec![view("z-view"), collection("a-collection")],
        DomainChangeKind::Upserted,
    );
    let second = event_with(
        "event-1",
        "tenant-a",
        "source-a",
        "projection-a",
        1,
        1,
        vec![collection("a-collection"), view("z-view")],
        DomainChangeKind::Upserted,
    );
    assert_eq!(
        first.semantic_digest().unwrap(),
        second.semantic_digest().unwrap()
    );
    let changed = event_with(
        "event-1",
        "tenant-a",
        "source-a",
        "projection-a",
        1,
        1,
        vec![collection("a-collection"), view("z-view")],
        DomainChangeKind::Deleted,
    );
    assert_ne!(
        first.semantic_digest().unwrap(),
        changed.semantic_digest().unwrap()
    );
}

// DC-07 / SD-N06
#[tokio::test]
async fn append_replay_and_conflicts_are_scoped_redacted_and_state_immutable() {
    let store = InMemoryDomainChangeStore::default();
    let original = event("sentinel-event", 1, 1);
    assert!(matches!(
        store.append(original.clone()).await.unwrap(),
        DomainChangeAppendOutcome::Inserted(_)
    ));
    assert!(matches!(
        store.append(original.clone()).await.unwrap(),
        DomainChangeAppendOutcome::Duplicate(_)
    ));
    let before = store.snapshot().await;
    let conflict = event_with(
        "sentinel-event",
        "tenant-a",
        "source-a",
        "projection-a",
        2,
        2,
        vec![view("other-view")],
        DomainChangeKind::Deleted,
    );
    let error = store.append(conflict).await.unwrap_err();
    assert_eq!(error, DomainChangeStoreError::EventConflict);
    assert!(!format!("{error:?}{error}").contains("sentinel-event"));
    assert_eq!(store.snapshot().await, before);

    let transport_conflict = event("other-event", 1, 2);
    assert_eq!(
        store.append(transport_conflict).await.unwrap_err(),
        DomainChangeStoreError::TransportPositionConflict
    );
    assert_eq!(store.snapshot().await, before);
}

// DC-08 / SD-N07
#[test]
fn stored_changes_and_batches_enforce_contiguous_sequences_and_digests() {
    let envelope = event("event-1", 1, 1);
    let digest = envelope.semantic_digest().unwrap();
    assert_eq!(
        StoredDomainChange::try_new(0, digest.clone(), envelope.clone()).unwrap_err(),
        DomainChangeStoreError::StoredSequenceZero
    );
    assert_eq!(
        StoredDomainChange::try_new(1, "A".repeat(64), envelope.clone()).unwrap_err(),
        DomainChangeStoreError::StoredDigestMismatch { sequence: 1 }
    );
    assert_eq!(
        StoredDomainChange::try_new(1, "0".repeat(64), envelope.clone()).unwrap_err(),
        DomainChangeStoreError::StoredDigestMismatch { sequence: 1 }
    );
    let first = stored(1, envelope);
    assert_eq!(
        DomainChangeBatch::try_new(0, vec![first.clone(), first.clone()]).unwrap_err(),
        DomainChangeStoreError::StoredSequenceDuplicate { sequence: 1 }
    );
    let third = stored(3, event("event-3", 3, 3));
    assert_eq!(
        DomainChangeBatch::try_new(0, vec![third, first.clone()]).unwrap_err(),
        DomainChangeStoreError::StoredSequenceNonContiguous {
            expected: 2,
            actual: 3,
        }
    );
    let empty = DomainChangeBatch::try_new(17, vec![]).unwrap();
    assert_eq!(empty.after_sequence(), 17);
    assert_eq!(empty.next_sequence(), 17);
    assert_eq!(
        DomainChangeBatch::try_new(u64::MAX, vec![first]).unwrap_err(),
        DomainChangeStoreError::StoredSequenceOverflow { previous: u64::MAX }
    );
}

// DC-09 / SD-N08
#[tokio::test]
async fn consumer_cursors_are_independent_monotonic_and_clone_continuous() {
    let store = InMemoryDomainChangeStore::default();
    store.append(event("event-1", 1, 1)).await.unwrap();
    store.append(event("event-2", 2, 2)).await.unwrap();
    store
        .advance_consumer_cursor("consumer-a", 1)
        .await
        .unwrap();
    let clone = store.clone();
    clone
        .advance_consumer_cursor("consumer-b", 2)
        .await
        .unwrap();
    assert_eq!(store.consumer_cursor("consumer-a").await.unwrap(), 1);
    assert_eq!(store.consumer_cursor("consumer-b").await.unwrap(), 2);
    let before = store.snapshot().await;
    assert_eq!(
        store
            .advance_consumer_cursor("consumer-a", 0)
            .await
            .unwrap_err(),
        DomainChangeStoreError::CursorRegression
    );
    assert_eq!(store.snapshot().await, before);
}

// DC-10 / SD-N04
#[tokio::test]
async fn coalescing_isolated_by_tenant_source_and_projection_without_leakage() {
    let store = InMemoryDomainChangeStore::default();
    for envelope in [
        event_with(
            "event-a",
            "tenant-a",
            "source-a",
            "projection-a",
            1,
            1,
            vec![view("work-items")],
            DomainChangeKind::Invalidated,
        ),
        event_with(
            "event-b",
            "tenant-b",
            "source-a",
            "projection-a",
            2,
            1,
            vec![view("work-items")],
            DomainChangeKind::Invalidated,
        ),
        event_with(
            "event-c",
            "tenant-a",
            "source-b",
            "projection-b",
            3,
            1,
            vec![view("work-items")],
            DomainChangeKind::Invalidated,
        ),
    ] {
        store.append(envelope).await.unwrap();
    }
    let units = coalesce_domain_changes(&store.read_after(0, 10).await.unwrap()).unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].notices().len(), 3);
    assert_eq!(units[0].through_sequence(), 3);
}

// DC-11 / SD-N11
#[test]
fn coalescing_reason_revision_and_sequence_ties_are_exact() {
    let base_time = Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap();
    let fixtures = [
        ("reason-upserted", 1, DomainChangeKind::Upserted, 1),
        ("reason-deleted", 2, DomainChangeKind::Deleted, 2),
        ("reason-invalidated", 3, DomainChangeKind::Invalidated, 3),
        (
            "reason-permission",
            4,
            DomainChangeKind::PermissionChanged,
            4,
        ),
        ("revision-higher", 5, DomainChangeKind::Upserted, 5),
        ("revision-higher", 6, DomainChangeKind::Deleted, 6),
        ("revision-lower", 8, DomainChangeKind::PermissionChanged, 8),
        ("revision-lower", 7, DomainChangeKind::Invalidated, 9),
        ("revision-equal", 9, DomainChangeKind::Upserted, 9),
        ("revision-equal", 9, DomainChangeKind::Invalidated, 10),
        ("watermark-first", 10, DomainChangeKind::Upserted, 10),
        (
            "watermark-later",
            10,
            DomainChangeKind::PermissionChanged,
            20,
        ),
        ("watermark-lower", 9, DomainChangeKind::Deleted, 30),
    ];
    let changes = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, (locator, revision, kind, observed_second))| {
            let sequence = index as u64 + 1;
            stored(
                sequence,
                event_with_observed_at(
                    &format!("reason-event-{sequence}"),
                    sequence,
                    revision,
                    vec![view(locator)],
                    kind,
                    base_time + Duration::seconds(observed_second),
                ),
            )
        })
        .collect();
    let batch = DomainChangeBatch::try_new(0, changes).unwrap();
    let units = coalesce_domain_changes(&batch).unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].through_sequence(), 13);
    let notice = &units[0].notices()[0];
    let target = |locator: &str| {
        notice
            .targets()
            .iter()
            .find(|candidate| candidate.target().locator() == locator)
            .unwrap()
    };

    assert_eq!(
        target("reason-upserted").reason(),
        DomainChangeKind::Upserted
    );
    assert_eq!(target("reason-deleted").reason(), DomainChangeKind::Deleted);
    assert_eq!(
        target("reason-invalidated").reason(),
        DomainChangeKind::Invalidated
    );
    assert_eq!(
        target("reason-permission").reason(),
        DomainChangeKind::PermissionChanged
    );
    assert_eq!(
        target("revision-higher").reason(),
        DomainChangeKind::Deleted
    );
    assert_eq!(target("revision-higher").revision(), 6);
    assert_eq!(
        target("revision-lower").reason(),
        DomainChangeKind::PermissionChanged
    );
    assert_eq!(target("revision-lower").revision(), 8);
    assert_eq!(
        target("revision-equal").reason(),
        DomainChangeKind::Invalidated
    );
    assert_eq!(target("revision-equal").revision(), 9);
    assert_eq!(notice.watermark().revision(), 10);
    assert_eq!(
        notice.watermark().observed_at(),
        base_time + Duration::seconds(20)
    );
}

// DC-12 / SD-N09
#[test]
fn canonical_permutations_produce_identical_bytes_digests_and_chunks() {
    let envelopes = [
        event("event-a", 1, 1),
        event("event-b", 2, 2),
        event("event-c", 3, 3),
    ];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let expected = format!(
        "{:?}",
        coalesce_domain_changes(
            &DomainChangeBatch::try_new(
                0,
                envelopes
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, envelope)| stored(index as u64 + 1, envelope))
                    .collect(),
            )
            .unwrap(),
        )
        .unwrap()
    );
    for order in permutations {
        let batch = DomainChangeBatch::try_new(
            0,
            order
                .into_iter()
                .map(|index| stored(index as u64 + 1, envelopes[index].clone()))
                .collect(),
        )
        .unwrap();
        assert_eq!(
            format!("{:?}", coalesce_domain_changes(&batch).unwrap()),
            expected
        );
    }
}

// DC-13 / SD-N10
#[test]
fn independent_64_65_129_boundaries_are_lossless_and_cursor_safe() {
    for axis in ["cause", "correlation", "target"] {
        for count in [64usize, 65, 129] {
            let changes = (1..=count)
                .map(|sequence| {
                    let event_id = if axis == "correlation" || axis == "target" {
                        "shared-event".to_string()
                    } else {
                        format!("event-{sequence}")
                    };
                    let targets = if axis == "target" {
                        vec![view(&format!("view-{sequence}"))]
                    } else {
                        vec![view("work-items")]
                    };
                    let envelope = event_with(
                        &event_id,
                        "tenant-a",
                        "source-a",
                        "projection-a",
                        sequence as u64,
                        sequence as u64,
                        targets,
                        DomainChangeKind::Invalidated,
                    );
                    let mut json = serde_json::to_value(envelope).unwrap();
                    json["correlation_id"] = if axis == "correlation" {
                        serde_json::json!(format!("request/{sequence}"))
                    } else {
                        serde_json::json!("request/shared")
                    };
                    stored(
                        sequence as u64,
                        serde_json::from_value::<DomainChangeEnvelope>(json).unwrap(),
                    )
                })
                .collect();
            let batch = DomainChangeBatch::try_new(0, changes).unwrap();
            let units = coalesce_domain_changes(&batch).unwrap();
            assert_eq!(units.last().unwrap().through_sequence(), count as u64);
            assert_eq!(units.len(), count.div_ceil(64), "{axis}/{count}");
            let observed = units
                .iter()
                .flat_map(|unit| unit.notices())
                .map(|notice| match axis {
                    "cause" => notice.cause_event_ids().len(),
                    "correlation" => notice.correlation_ids().len(),
                    "target" => notice.targets().len(),
                    _ => unreachable!(),
                })
                .sum::<usize>();
            assert_eq!(observed, count, "{axis}/{count}");
        }
    }
}

// DC-14 / SD-N05
#[test]
fn worker_delivery_unit_implements_neither_serialize_nor_deserialize() {
    trait AmbiguousIfSerialize<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
    impl<T: ?Sized + Serialize> AmbiguousIfSerialize<u8> for T {}
    trait AmbiguousIfDeserialize<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfDeserialize<()> for T {}
    impl<T: ?Sized + for<'de> Deserialize<'de>> AmbiguousIfDeserialize<u8> for T {}

    let _ = <CoalescedSelectiveRefresh as AmbiguousIfSerialize<_>>::check;
    let _ = <CoalescedSelectiveRefresh as AmbiguousIfDeserialize<_>>::check;
}

// DC-15 / SD-N12 is completed by the retained spec/index and docs-check.
#[test]
fn provider_neutral_spec_and_index_match_dormant_contract() {
    let spec = include_str!("../../../docs/specs/domain-change-selective-refresh-r1.md");
    let index = include_str!("../../../docs/specs/README.md");
    assert!(spec.contains("selected_contract / dormant_implementation_candidate"));
    assert!(spec.contains("provider-neutral"));
    assert!(spec.contains("client resume ID"));
    assert!(index.contains("domain-change-selective-refresh-r1.md"));
}

#[test]
fn malformed_batch_emits_no_unit_and_errors_are_value_free() {
    let envelope = event("sentinel-event", 1, 1);
    let corrupt = StoredDomainChange::unchecked(1, "0".repeat(64), envelope);
    let batch = DomainChangeBatch::unchecked(0, 1, vec![corrupt]);
    assert_eq!(
        coalesce_domain_changes(&batch).unwrap_err(),
        DomainChangeStoreError::StoredDigestMismatch { sequence: 1 }
    );
    let adapter = DomainChangeStoreError::Adapter {
        code: DomainChangeAdapterErrorCode::Unavailable,
    };
    assert!(!format!("{adapter:?}{adapter}").contains("sentinel-event"));
}

// SD-N06 / SD-N07 / SD-N08
#[tokio::test]
async fn all_error_paths_are_value_free_and_failure_state_is_immutable() {
    const SENTINELS: [&str; 12] = [
        "event-sentinel-1",
        "locator-sentinel-1",
        "consumer-sentinel-1",
        "tenant-sentinel-1",
        "source-sentinel-1",
        "projection-sentinel-1",
        "transport-sentinel-1",
        "correlation/sentinel-1",
        "password=sentinel-secret",
        "raw-backend-sentinel",
        "dsn-sentinel",
        "sql-sentinel",
    ];

    for nested in [
        DomainChangeError::UnsupportedSchemaVersion,
        DomainChangeError::InvalidField {
            field: "event_id",
            reason: "must be non-empty and within the byte limit",
        },
        DomainChangeError::SensitiveValue {
            field: "source.source_id",
        },
        DomainChangeError::DuplicateTarget,
        DomainChangeError::DigestFailure,
    ] {
        assert_error_redacted(&nested, &SENTINELS);
        assert_error_redacted(&DomainChangeStoreError::InvalidEvent(nested), &SENTINELS);
    }

    for code in [
        DomainChangeAdapterErrorCode::Unavailable,
        DomainChangeAdapterErrorCode::Timeout,
        DomainChangeAdapterErrorCode::TransactionFailed,
        DomainChangeAdapterErrorCode::CorruptData,
    ] {
        assert_error_redacted(&code, &SENTINELS);
    }

    for error in [
        DomainChangeStoreError::EventConflict,
        DomainChangeStoreError::TransportPositionConflict,
        DomainChangeStoreError::InvalidBatchLimit,
        DomainChangeStoreError::CursorOutOfRange {
            sequence: 41,
            latest: 40,
        },
        DomainChangeStoreError::CursorRegression,
        DomainChangeStoreError::StoredSequenceZero,
        DomainChangeStoreError::StoredSequenceDuplicate { sequence: 7 },
        DomainChangeStoreError::StoredSequenceNonContiguous {
            expected: 8,
            actual: 9,
        },
        DomainChangeStoreError::StoredSequenceOverflow { previous: u64::MAX },
        DomainChangeStoreError::StoredDigestMismatch { sequence: 7 },
        DomainChangeStoreError::Adapter {
            code: DomainChangeAdapterErrorCode::Unavailable,
        },
        DomainChangeStoreError::Adapter {
            code: DomainChangeAdapterErrorCode::Timeout,
        },
        DomainChangeStoreError::Adapter {
            code: DomainChangeAdapterErrorCode::TransactionFailed,
        },
        DomainChangeStoreError::Adapter {
            code: DomainChangeAdapterErrorCode::CorruptData,
        },
    ] {
        assert_error_redacted(&error, &SENTINELS);
    }

    let store = InMemoryDomainChangeStore::default();
    let empty = store.snapshot().await;
    let invalid =
        event("valid-event", 1, 1).unchecked_with_event_id("password=sentinel-secret".to_string());
    let error = store.append(invalid).await.unwrap_err();
    assert_eq!(
        error,
        DomainChangeStoreError::InvalidEvent(DomainChangeError::SensitiveValue {
            field: "event_id"
        })
    );
    assert_error_redacted(&error, &SENTINELS);
    assert_eq!(store.snapshot().await, empty);

    let original = event_with(
        "event-sentinel-1",
        "tenant-sentinel-1",
        "source-sentinel-1",
        "projection-sentinel-1",
        1,
        1,
        vec![view("locator-sentinel-1")],
        DomainChangeKind::Upserted,
    );
    store.append(original).await.unwrap();
    let one_event = store.snapshot().await;

    let event_conflict = event_with(
        "event-sentinel-1",
        "tenant-sentinel-1",
        "source-sentinel-1",
        "projection-sentinel-1",
        2,
        2,
        vec![view("other-locator")],
        DomainChangeKind::Deleted,
    );
    let error = store.append(event_conflict).await.unwrap_err();
    assert_eq!(error, DomainChangeStoreError::EventConflict);
    assert_error_redacted(&error, &SENTINELS);
    assert_eq!(store.snapshot().await, one_event);

    let transport_conflict = event_with(
        "other-event",
        "other-tenant",
        "source-sentinel-1",
        "projection-sentinel-1",
        1,
        3,
        vec![view("other-locator")],
        DomainChangeKind::Invalidated,
    );
    let error = store.append(transport_conflict).await.unwrap_err();
    assert_eq!(error, DomainChangeStoreError::TransportPositionConflict);
    assert_error_redacted(&error, &SENTINELS);
    assert_eq!(store.snapshot().await, one_event);

    for error in [
        store.read_after(0, 0).await.unwrap_err(),
        store.read_after(0, 1_001).await.unwrap_err(),
        store.read_after(2, 1).await.unwrap_err(),
    ] {
        assert_error_redacted(&error, &SENTINELS);
        assert_eq!(store.snapshot().await, one_event);
    }

    let error = store
        .consumer_cursor("password=sentinel-secret")
        .await
        .unwrap_err();
    assert_eq!(
        error,
        DomainChangeStoreError::InvalidEvent(DomainChangeError::SensitiveValue {
            field: "consumer_id"
        })
    );
    assert_error_redacted(&error, &SENTINELS);
    assert_eq!(store.snapshot().await, one_event);

    store
        .advance_consumer_cursor("consumer-sentinel-1", 1)
        .await
        .unwrap();
    let cursor_one = store.snapshot().await;
    for error in [
        store
            .advance_consumer_cursor("consumer-sentinel-1", 0)
            .await
            .unwrap_err(),
        store
            .advance_consumer_cursor("consumer-sentinel-1", 2)
            .await
            .unwrap_err(),
    ] {
        assert_error_redacted(&error, &SENTINELS);
        assert_eq!(store.snapshot().await, cursor_one);
    }

    for error in [
        DomainChangeSource::try_new(
            "password=sentinel-secret",
            "projection-sentinel-1",
            "source-sentinel-1.schema@1",
        )
        .unwrap_err(),
        DomainInvalidationTarget::try_new(DomainTargetKind::View, "password=sentinel-secret")
            .unwrap_err(),
    ] {
        assert_error_redacted(&error, &SENTINELS);
    }
}

#[test]
fn accessors_preserve_internal_contract_without_public_fields() {
    let envelope = event("event-1", 1, 1);
    assert_eq!(
        envelope.occurred_at(),
        Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap()
    );
    assert_eq!(
        envelope.source().schema_version(),
        "source-a.projection-a@1"
    );
    assert_eq!(envelope.targets()[0].locator(), "work-items");
    let stored = stored(1, envelope);
    assert_eq!(stored.sequence(), 1);
    assert_eq!(stored.semantic_digest().len(), 64);
    assert_eq!(stored.envelope().event_id(), "event-1");
    let batch = DomainChangeBatch::try_new(0, vec![stored]).unwrap();
    let units = coalesce_domain_changes(&batch).unwrap();
    let notice = &units[0].notices()[0];
    assert_eq!(notice.tenant_id(), "tenant-a");
    assert_eq!(notice.source().source_id(), "source-a");
    assert_eq!(notice.watermark().revision(), 1);
    assert_eq!(notice.targets()[0].target().locator(), "work-items");
    assert_eq!(notice.correlation_ids(), &["request/event-1"]);
    assert!(notice.authorized_refetch_required());
    assert_eq!(notice.semantic_digest().unwrap().len(), 64);
    for error in [
        DomainChangeAdapterErrorCode::Unavailable,
        DomainChangeAdapterErrorCode::Timeout,
        DomainChangeAdapterErrorCode::TransactionFailed,
        DomainChangeAdapterErrorCode::CorruptData,
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn rejected_deserialization_cannot_install_unvalidated_event_identity() {
    let envelope = event("event-1", 1, 1).unchecked_with_event_id("password=sentinel".into());
    assert!(serde_json::from_value::<DomainChangeEnvelope>(
        serde_json::to_value(envelope).unwrap()
    )
    .is_err());
}
