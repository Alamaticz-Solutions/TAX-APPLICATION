use appfw_runtime::fabric::{
    FabricAvailability, FabricComponentKind, FabricComponentRecord, FabricDependency,
    FabricEvidencePosture, FabricHealth, FabricLifecycleState, FabricProvenanceSource,
    FabricRegistry, FabricRegistryError, FabricRelationshipKind, FabricTelemetryEvent,
    FabricTelemetryKind, FabricValidationCode, FABRIC_TELEMETRY_SCHEMA_VERSION,
    FABRIC_TOPOLOGY_SCHEMA_VERSION,
};
use appfw_runtime::observability::RequestContext;

const FRAMEWORK_LOCAL_FIXTURE: &str =
    include_str!("fixtures/fabric/framework-local-topology.v1.json");

fn fixture_records() -> Vec<FabricComponentRecord> {
    serde_json::from_str(FRAMEWORK_LOCAL_FIXTURE).expect("sanitized Framework-local Fabric fixture")
}

#[test]
fn sanitized_nexus_fixture_builds_the_expected_provider_neutral_topology() {
    let registry = FabricRegistry::load(fixture_records()).expect("valid Fabric registry");
    let topology = registry.topology().expect("deterministic topology");

    assert_eq!(registry.len(), 13);
    assert_eq!(topology.schema_version, FABRIC_TOPOLOGY_SCHEMA_VERSION);
    assert_eq!(topology.components.len(), 13);
    assert_eq!(topology.relationships.len(), 23);
    assert!(topology.digest.starts_with("sha256:"));
    assert_eq!(topology.digest.len(), 71);

    let kinds = topology
        .components
        .iter()
        .map(|component| component.kind)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        kinds,
        std::collections::BTreeSet::from([
            FabricComponentKind::Application,
            FabricComponentKind::UserInterface,
            FabricComponentKind::Service,
            FabricComponentKind::Database,
            FabricComponentKind::EventTopic,
            FabricComponentKind::Integration,
            FabricComponentKind::ArtificialIntelligenceService,
        ])
    );

    let serialized = serde_json::to_string(&topology).expect("topology JSON");
    for prohibited in [
        "authorization",
        "access_token",
        "client_secret",
        "mongodb://",
        "postgres://",
        "domain_payload",
    ] {
        assert!(!serialized.to_ascii_lowercase().contains(prohibited));
    }
}

#[test]
fn topology_order_and_digest_do_not_depend_on_input_order() {
    let records = fixture_records();
    let expected = FabricRegistry::load(records.clone())
        .expect("forward fixture")
        .topology()
        .expect("forward topology");

    let mut reversed = records;
    reversed.reverse();
    let actual = FabricRegistry::load(reversed)
        .expect("reversed fixture")
        .topology()
        .expect("reversed topology");

    assert_eq!(actual, expected);
    assert!(actual
        .components
        .windows(2)
        .all(|pair| pair[0].component_id < pair[1].component_id));
    assert!(actual
        .relationships
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
}

#[test]
fn registry_reduces_valid_lifecycle_history_to_the_latest_revision() {
    let mut records = fixture_records();
    let mut next = records
        .iter()
        .find(|record| record.component_id == "framework-app-mobile")
        .expect("mobile fixture")
        .clone();
    next.revision = 2;
    next.lifecycle = FabricLifecycleState::Active;
    next.version = "fixture-2".to_string();
    records.insert(0, next);

    let registry = FabricRegistry::load(records).expect("valid lifecycle history");
    let current = registry
        .component("framework-app-mobile")
        .expect("latest mobile record");
    assert_eq!(current.revision, 2);
    assert_eq!(current.lifecycle, FabricLifecycleState::Active);
}

#[test]
fn registry_rejects_dangling_and_kind_invalid_relationships() {
    let mut dangling = fixture_records();
    let backend = dangling
        .iter_mut()
        .find(|record| record.component_id == "framework-app-service")
        .expect("backend fixture");
    backend.dependencies.push(FabricDependency {
        component_id: "missing-database".to_string(),
        relationship: FabricRelationshipKind::ReadsFrom,
    });
    assert!(matches!(
        FabricRegistry::load(dangling),
        Err(FabricRegistryError::DanglingRelationship { .. })
    ));

    let mut invalid = fixture_records();
    let web = invalid
        .iter_mut()
        .find(|record| record.component_id == "framework-app-web")
        .expect("web fixture");
    web.dependencies = vec![FabricDependency {
        component_id: "framework-app-store".to_string(),
        relationship: FabricRelationshipKind::ReadsFrom,
    }];
    assert!(matches!(
        FabricRegistry::load(invalid),
        Err(FabricRegistryError::InvalidRelationship { .. })
    ));

    let mut cross_owner_containment = fixture_records();
    let web = cross_owner_containment
        .iter_mut()
        .find(|record| record.component_id == "framework-app-web")
        .expect("web fixture");
    web.owner_id = "shared-user-interface-owner".to_string();
    assert!(matches!(
        FabricRegistry::load(cross_owner_containment),
        Err(FabricRegistryError::InvalidRelationship { .. })
    ));
}

#[test]
fn registry_rejects_duplicate_or_conflicting_identity_history() {
    let mut duplicate = fixture_records();
    duplicate.push(duplicate[0].clone());
    assert!(matches!(
        FabricRegistry::load(duplicate),
        Err(FabricRegistryError::DuplicateRevision { .. })
    ));

    let mut conflict = fixture_records();
    let mut conflicting = conflict[0].clone();
    conflicting.revision = 2;
    conflicting.kind = FabricComponentKind::Service;
    conflict.push(conflicting);
    assert!(matches!(
        FabricRegistry::load(conflict),
        Err(FabricRegistryError::IdentityConflict { field: "kind", .. })
    ));
}

#[test]
fn registry_rejects_lifecycle_regression_and_invalid_provenance() {
    let mut lifecycle = fixture_records();
    let mut regression = lifecycle[0].clone();
    regression.revision = 2;
    regression.lifecycle = FabricLifecycleState::Registered;
    lifecycle.push(regression);
    assert!(matches!(
        FabricRegistry::load(lifecycle),
        Err(FabricRegistryError::InvalidLifecycleTransition { .. })
    ));

    let mut provenance = fixture_records();
    provenance[0].provenance.source = FabricProvenanceSource::Observed;
    provenance[0].provenance.sha256 = None;
    assert!(matches!(
        FabricRegistry::load(provenance),
        Err(FabricRegistryError::InvalidField {
            field: "provenance.sha256",
            ..
        })
    ));
}

#[test]
fn registry_rejects_sensitive_values_and_open_payload_shapes() {
    let mut sensitive = fixture_records();
    sensitive[0].name = "Authorization: Bearer local-secret".to_string();
    assert!(matches!(
        FabricRegistry::load(sensitive),
        Err(FabricRegistryError::SensitiveValue { field: "name" })
    ));

    let mut credential_uri = fixture_records();
    credential_uri[0].provenance.source_ref =
        "https://fixture-user:fixture-password@example.invalid/source".to_string();
    assert!(matches!(
        FabricRegistry::load(credential_uri),
        Err(FabricRegistryError::SensitiveValue {
            field: "provenance.source_ref"
        })
    ));

    let mut open_shape = serde_json::to_value(&fixture_records()[0]).expect("record JSON");
    open_shape.as_object_mut().expect("record object").insert(
        "domain_payload".to_string(),
        serde_json::json!({ "worker": "example" }),
    );
    let error = serde_json::from_value::<FabricComponentRecord>(open_shape)
        .expect_err("unknown payload fields must be closed");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn telemetry_is_closed_payload_free_and_uses_the_stable_vocabulary() {
    let request_context = RequestContext::new(
        "REQ:550E8400-E29B-41D4-A716-446655440000",
        "NXF:request/550E8400-E29B-41D4-A716-446655440000",
    );
    let loaded =
        FabricTelemetryEvent::registry_loaded(&request_context).expect("safe registry event");
    assert_eq!(loaded.request_id(), request_context.request_id);
    assert_eq!(loaded.correlation_id(), request_context.correlation_id);
    assert_eq!(
        serde_json::to_value(&loaded).expect("registry telemetry JSON"),
        serde_json::json!({
            "schema_version": "appfw.fabric.telemetry@1",
            "event": "app.fabric.registry.loaded",
            "request_id": "REQ:550E8400-E29B-41D4-A716-446655440000",
            "correlation_id": "NXF:request/550E8400-E29B-41D4-A716-446655440000",
            "component_id": null,
            "validation_code": null,
            "health": null,
            "availability": null,
            "evidence_posture": null
        })
    );

    let projected =
        FabricTelemetryEvent::topology_projected(&request_context).expect("safe projection event");
    let health = FabricTelemetryEvent::component_health_changed(
        &request_context,
        "framework-app-service",
        FabricHealth::Degraded,
        FabricAvailability::Available,
    )
    .expect("safe health event");
    assert_eq!(health.schema_version(), FABRIC_TELEMETRY_SCHEMA_VERSION);
    assert_eq!(health.event(), FabricTelemetryKind::ComponentHealthChanged);

    let evidence = FabricTelemetryEvent::evidence_posture_changed(
        &request_context,
        "framework-app-service",
        FabricEvidencePosture::Stale,
    )
    .expect("safe evidence event");
    assert_eq!(
        evidence.event(),
        FabricTelemetryKind::EvidencePostureChanged
    );

    let validation_error = FabricRegistryError::DanglingRelationship {
        component_id: "framework-app-service".to_string(),
        target_id: "missing-database".to_string(),
    };
    let validation = FabricTelemetryEvent::validation_failed(&request_context, &validation_error)
        .expect("safe validation event");
    assert_eq!(
        validation.validation_code(),
        Some(FabricValidationCode::DanglingRelationship)
    );

    let events = [loaded, projected, health, evidence, validation];
    assert_eq!(
        events
            .iter()
            .map(FabricTelemetryEvent::event)
            .collect::<Vec<_>>(),
        vec![
            FabricTelemetryKind::RegistryLoaded,
            FabricTelemetryKind::TopologyProjected,
            FabricTelemetryKind::ComponentHealthChanged,
            FabricTelemetryKind::EvidencePostureChanged,
            FabricTelemetryKind::ValidationFailed,
        ]
    );
    let serialized = serde_json::to_value(events).expect("telemetry JSON");
    for event in serialized.as_array().expect("telemetry array") {
        serde_json::from_value::<FabricTelemetryEvent>(event.clone())
            .expect("constructed telemetry must round-trip through wire validation");
    }
    let serialized = serde_json::to_string(&serialized).expect("telemetry text");
    assert!(!serialized.contains("local-secret"));
    assert!(!serialized.contains("domain_payload"));
    assert!(!serialized.contains("payload"));

    let sensitive = RequestContext::new("request-1", "access_token");
    assert!(FabricTelemetryEvent::registry_loaded(&sensitive).is_err());
    let malformed = RequestContext::new("request id with spaces", "correlation-1");
    assert!(FabricTelemetryEvent::registry_loaded(&malformed).is_err());
    assert!(FabricTelemetryEvent::component_health_changed(
        &request_context,
        "framework-app-service",
        FabricHealth::Healthy,
        FabricAvailability::Unavailable,
    )
    .is_err());
}

#[test]
fn registry_rejects_encoded_private_keys_in_every_metadata_family() {
    const COMPACT_ED25519_PKCS8: &str =
        "MC4CAQAwBQYDK2VwBCIEIJiGLkWAlHdIeMza9cdqgW0p86icRRK7WydQw1GJH1cJ";

    for mutate in [
        |record: &mut FabricComponentRecord| record.name = COMPACT_ED25519_PKCS8.to_string(),
        |record: &mut FabricComponentRecord| record.version = COMPACT_ED25519_PKCS8.to_string(),
        |record: &mut FabricComponentRecord| {
            record.provenance.source_ref = format!("fixture:{COMPACT_ED25519_PKCS8}")
        },
        |record: &mut FabricComponentRecord| {
            record.evidence[0].uri = format!("fixture:{COMPACT_ED25519_PKCS8}")
        },
    ] {
        let mut records = fixture_records();
        mutate(&mut records[0]);
        assert!(matches!(
            FabricRegistry::load(records),
            Err(FabricRegistryError::SensitiveValue { .. })
        ));
    }

    let sensitive_context = RequestContext::new("request-1", COMPACT_ED25519_PKCS8);
    assert!(FabricTelemetryEvent::registry_loaded(&sensitive_context).is_err());
}

#[test]
fn observed_health_requires_verified_health_evidence_and_coherent_posture() {
    let mut records = fixture_records();
    let application = &mut records[0];
    application.health.source = appfw_runtime::fabric::FabricObservationSource::Observed;
    application.health.health = FabricHealth::Healthy;
    application.health.availability = FabricAvailability::Available;
    application.health.evidence_id = Some(application.evidence[0].evidence_id.clone());
    assert!(matches!(
        FabricRegistry::load(records.clone()),
        Err(FabricRegistryError::InvalidField {
            field: "health.evidence_id",
            ..
        })
    ));

    records[0].evidence[0].posture = FabricEvidencePosture::Verified;
    records[0].evidence[0].sha256 = Some("a".repeat(64));
    assert!(matches!(
        FabricRegistry::load(records.clone()),
        Err(FabricRegistryError::InvalidField {
            field: "health.evidence_id",
            ..
        })
    ));

    records[0].evidence[0].kind = appfw_runtime::fabric::FabricEvidenceKind::Health;
    records[0].evidence[0].posture = FabricEvidencePosture::Declared;
    records[0].evidence[0].sha256 = None;
    assert!(matches!(
        FabricRegistry::load(records.clone()),
        Err(FabricRegistryError::InvalidField {
            field: "health.evidence_id",
            ..
        })
    ));

    records[0].evidence[0].posture = FabricEvidencePosture::Verified;
    records[0].evidence[0].sha256 = Some("a".repeat(64));
    FabricRegistry::load(records.clone()).expect("verified health evidence");

    records[0].health.availability = FabricAvailability::Unavailable;
    assert!(matches!(
        FabricRegistry::load(records.clone()),
        Err(FabricRegistryError::InvalidField {
            field: "health",
            ..
        })
    ));
    records[0].health.health = FabricHealth::Unhealthy;
    records[0].health.availability = FabricAvailability::Available;
    assert!(matches!(
        FabricRegistry::load(records),
        Err(FabricRegistryError::InvalidField {
            field: "health",
            ..
        })
    ));
}

#[test]
fn health_availability_matrix_is_exhaustive_and_truthful() {
    let health_values = [
        FabricHealth::Unknown,
        FabricHealth::Healthy,
        FabricHealth::Degraded,
        FabricHealth::Unhealthy,
        FabricHealth::NotApplicable,
    ];
    let availability_values = [
        FabricAvailability::Unknown,
        FabricAvailability::Available,
        FabricAvailability::Unavailable,
        FabricAvailability::NotApplicable,
    ];

    for health in health_values {
        for availability in availability_values {
            let expected = matches!(
                (health, availability),
                (FabricHealth::Unknown, FabricAvailability::Unknown)
                    | (FabricHealth::Healthy, FabricAvailability::Available)
                    | (FabricHealth::Degraded, FabricAvailability::Available)
                    | (FabricHealth::Unhealthy, FabricAvailability::Unavailable)
                    | (
                        FabricHealth::NotApplicable,
                        FabricAvailability::NotApplicable
                    )
            );
            let mut records = fixture_records();
            records[0].health.health = health;
            records[0].health.availability = availability;
            records[0].lifecycle = if health == FabricHealth::NotApplicable
                && availability == FabricAvailability::NotApplicable
            {
                FabricLifecycleState::Retired
            } else {
                FabricLifecycleState::Registered
            };
            assert_eq!(
                FabricRegistry::load(records).is_ok(),
                expected,
                "unexpected health matrix result for {health:?}/{availability:?}"
            );
        }
    }
}

#[test]
fn telemetry_deserialization_enforces_schema_and_event_specific_fields() {
    let valid = serde_json::json!({
        "schema_version": "appfw.fabric.telemetry@1",
        "event": "app.fabric.component.health.changed",
        "request_id": "request-1",
        "correlation_id": "correlation-1",
        "component_id": "framework-app-service",
        "validation_code": null,
        "health": "degraded",
        "availability": "available",
        "evidence_posture": null
    });
    serde_json::from_value::<FabricTelemetryEvent>(valid.clone()).expect("closed health telemetry");

    for invalid in [
        {
            let mut value = valid.clone();
            value["schema_version"] = serde_json::json!("appfw.fabric.telemetry@2");
            value
        },
        {
            let mut value = valid.clone();
            value["event"] = serde_json::json!("app.fabric.registry.loaded");
            value
        },
        {
            let mut value = valid.clone();
            value["availability"] = serde_json::json!("unavailable");
            value
        },
        {
            let mut value = valid;
            value["evidence_posture"] = serde_json::json!("verified");
            value
        },
    ] {
        assert!(serde_json::from_value::<FabricTelemetryEvent>(invalid).is_err());
    }

    let base = serde_json::json!({
        "schema_version": "appfw.fabric.telemetry@1",
        "event": "app.fabric.validation.failed",
        "request_id": "request-1",
        "correlation_id": "correlation-1",
        "component_id": null,
        "validation_code": "invalid_field",
        "health": null,
        "availability": null,
        "evidence_posture": null
    });
    serde_json::from_value::<FabricTelemetryEvent>(base.clone()).expect("validation event");
    for (field, value) in [
        ("validation_code", serde_json::Value::Null),
        ("component_id", serde_json::json!("framework-app-service")),
        ("health", serde_json::json!("healthy")),
        ("evidence_posture", serde_json::json!("verified")),
    ] {
        let mut invalid = base.clone();
        invalid[field] = value;
        assert!(serde_json::from_value::<FabricTelemetryEvent>(invalid).is_err());
    }

    let evidence = serde_json::json!({
        "schema_version": "appfw.fabric.telemetry@1",
        "event": "app.fabric.evidence.posture.changed",
        "request_id": "request-1",
        "correlation_id": "correlation-1",
        "component_id": "framework-app-service",
        "validation_code": null,
        "health": null,
        "availability": null,
        "evidence_posture": "verified"
    });
    serde_json::from_value::<FabricTelemetryEvent>(evidence.clone()).expect("evidence event");
    for field in ["component_id", "evidence_posture"] {
        let mut invalid = evidence.clone();
        invalid[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<FabricTelemetryEvent>(invalid).is_err());
    }

    for (field, value) in [
        ("request_id", serde_json::json!("request id with spaces")),
        ("correlation_id", serde_json::json!("access_token")),
        ("component_id", serde_json::json!("Invalid Component")),
    ] {
        let mut invalid = evidence.clone();
        invalid[field] = value;
        assert!(serde_json::from_value::<FabricTelemetryEvent>(invalid).is_err());
    }
}

#[test]
fn topology_distinguishes_product_composition_from_shared_dependencies_and_data_flow() {
    let topology = FabricRegistry::load(fixture_records())
        .expect("valid Fabric registry")
        .topology()
        .expect("topology");

    let relation = |source: &str, target: &str, kind: FabricRelationshipKind| {
        topology.relationships.iter().any(|relationship| {
            relationship.source_component_id == source
                && relationship.target_component_id == target
                && relationship.kind == kind
        })
    };

    let contained = topology
        .relationships
        .iter()
        .filter(|relationship| {
            relationship.source_component_id == "framework-app"
                && relationship.kind == FabricRelationshipKind::Contains
        })
        .map(|relationship| relationship.target_component_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        contained,
        std::collections::BTreeSet::from([
            "framework-app-service",
            "framework-app-mobile",
            "framework-app-ai-adapter",
            "framework-app-store",
            "framework-app-web",
        ])
    );
    for shared in ["system-a-integration", "system-b-integration", "shared-ai"] {
        assert!(relation(
            "framework-app",
            shared,
            FabricRelationshipKind::DependsOn
        ));
        assert!(!relation(
            "framework-app",
            shared,
            FabricRelationshipKind::Contains
        ));
    }

    for (adapter, projection, topic) in [
        ("system-a-integration", "system-a-store", "system-a-topic"),
        ("system-b-integration", "system-b-store", "system-b-topic"),
    ] {
        assert!(relation(
            adapter,
            projection,
            FabricRelationshipKind::WritesTo
        ));
        assert!(relation(
            adapter,
            topic,
            FabricRelationshipKind::PublishesTo
        ));
        assert!(relation(
            "framework-app-service",
            projection,
            FabricRelationshipKind::ReadsFrom
        ));
        assert!(relation(
            "framework-app-service",
            topic,
            FabricRelationshipKind::ConsumesFrom
        ));
        assert!(!relation(
            projection,
            topic,
            FabricRelationshipKind::PublishesTo
        ));
    }

    assert!(relation(
        "framework-app-ai-adapter",
        "shared-ai",
        FabricRelationshipKind::Invokes
    ));
    assert!(!relation(
        "framework-app-service",
        "shared-ai",
        FabricRelationshipKind::Invokes
    ));
}
