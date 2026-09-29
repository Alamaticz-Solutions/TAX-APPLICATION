use appfw_runtime::fabric::{
    FabricAvailability, FabricClassification, FabricComponentKind, FabricComponentRecord,
    FabricEnvironment, FabricEvidenceKind, FabricEvidencePosture, FabricHealth,
    FabricManagementReadError, FabricObservationSource, FabricReadScope, FabricRegistry,
    FabricRelationshipKind, FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION,
    FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION,
};

const FRAMEWORK_LOCAL_FIXTURE: &str =
    include_str!("fixtures/fabric/framework-local-topology.v1.json");

fn fixture_records() -> Vec<FabricComponentRecord> {
    serde_json::from_str(FRAMEWORK_LOCAL_FIXTURE).expect("sanitized Framework-local Fabric fixture")
}

fn local_scope() -> FabricReadScope {
    FabricReadScope::new(
        [FabricEnvironment::Local],
        [
            FabricClassification::Public,
            FabricClassification::Internal,
            FabricClassification::Confidential,
            FabricClassification::Restricted,
            FabricClassification::Phi,
        ],
    )
    .expect("valid local management scope")
}

#[test]
fn overview_is_a_deterministic_management_projection_of_the_registry() {
    let records = fixture_records();
    let forward = FabricRegistry::load(records.clone())
        .expect("forward registry")
        .management_overview(&local_scope())
        .expect("forward management overview");
    let mut reversed_records = records;
    reversed_records.reverse();
    let reversed = FabricRegistry::load(reversed_records)
        .expect("reversed registry")
        .management_overview(&local_scope())
        .expect("reversed management overview");

    assert_eq!(forward, reversed);
    assert_eq!(
        forward.schema_version,
        FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION
    );
    assert!(forward.projection_digest.starts_with("sha256:"));
    assert_eq!(forward.projection_digest.len(), 71);
    assert_eq!(forward.summary.component_count, 13);
    assert_eq!(forward.summary.relationship_count, 23);
    assert_eq!(forward.summary.owner_count, 4);
    assert_eq!(forward.summary.evidence.total, 13);
    assert_eq!(
        forward
            .summary
            .evidence
            .by_kind
            .get(&FabricEvidenceKind::Contract),
        Some(&11)
    );
    assert_eq!(
        forward
            .summary
            .evidence
            .by_kind
            .get(&FabricEvidenceKind::Test),
        Some(&2)
    );
    assert_eq!(
        forward
            .summary
            .evidence
            .by_posture
            .get(&FabricEvidencePosture::Declared),
        Some(&13)
    );
    for missing in [
        FabricEvidenceKind::Source,
        FabricEvidenceKind::Health,
        FabricEvidenceKind::Operations,
        FabricEvidenceKind::Security,
        FabricEvidenceKind::Approval,
    ] {
        assert!(forward.summary.evidence.missing_kinds.contains(&missing));
    }
    assert_eq!(
        forward
            .summary
            .by_kind
            .get(&FabricComponentKind::UserInterface),
        Some(&2)
    );
    assert_eq!(
        forward.summary.by_health.get(&FabricHealth::Unknown),
        Some(&13)
    );
    assert!(forward
        .components
        .windows(2)
        .all(|pair| pair[0].component_id < pair[1].component_id));

    let bytes = serde_json::to_vec(&forward).expect("overview JSON bytes");
    assert_eq!(
        serde_json::from_slice::<appfw_runtime::fabric::FabricManagementOverview>(&bytes)
            .expect("closed overview round trip"),
        forward
    );
    let mut open_overview = serde_json::to_value(&forward).expect("overview JSON value");
    open_overview
        .as_object_mut()
        .expect("overview object")
        .insert("live_security_passed".to_string(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<appfw_runtime::fabric::FabricManagementOverview>(open_overview)
            .is_err()
    );
}

#[test]
fn authorized_scope_filters_components_and_relationships_without_leaking_hidden_ids() {
    let registry = FabricRegistry::load(fixture_records()).expect("valid registry");
    let internal =
        FabricReadScope::new([FabricEnvironment::Local], [FabricClassification::Internal])
            .expect("internal-only scope");
    let overview = registry
        .management_overview(&internal)
        .expect("scoped management overview");

    assert_eq!(overview.summary.component_count, 6);
    assert_eq!(overview.summary.relationship_count, 9);
    assert!(overview
        .components
        .iter()
        .all(|component| component.classification == FabricClassification::Internal));
    let serialized = serde_json::to_string(&overview).expect("overview JSON");
    for hidden in [
        "framework-app-store",
        "framework-app-ai-adapter",
        "system-a-store",
        "system-b-store",
        "system-a-topic",
        "system-b-topic",
        "shared-ai",
    ] {
        assert!(
            !serialized.contains(hidden),
            "hidden identifier leaked: {hidden}"
        );
    }
    assert_eq!(
        registry.component_detail(&internal, "framework-app-store"),
        Err(FabricManagementReadError::NotAvailable)
    );
}

#[test]
fn component_detail_exposes_provenance_evidence_health_and_typed_drill_in_edges() {
    let registry = FabricRegistry::load(fixture_records()).expect("valid registry");
    let detail = registry
        .component_detail(&local_scope(), "framework-app-service")
        .expect("authorized backend detail");

    assert_eq!(
        detail.schema_version,
        FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION
    );
    assert!(detail.projection_digest.starts_with("sha256:"));
    assert_eq!(detail.component_id, "framework-app-service");
    assert_eq!(detail.evidence.len(), 1);
    assert_eq!(detail.evidence_coverage.total, 1);
    assert_eq!(detail.incoming_relationships.len(), 3);
    assert_eq!(detail.outgoing_relationships.len(), 8);
    assert!(detail
        .outgoing_relationships
        .iter()
        .any(|relationship| relationship.kind == FabricRelationshipKind::ReadsFrom));
    assert!(detail
        .outgoing_relationships
        .iter()
        .any(|relationship| relationship.kind == FabricRelationshipKind::ConsumesFrom));

    let serialized = serde_json::to_string(&detail).expect("detail JSON");
    for prohibited in [
        "authorization",
        "access_token",
        "refresh_token",
        "client_secret",
        "domain_payload",
        "mongodb://",
        "postgres://",
    ] {
        assert!(!serialized.to_ascii_lowercase().contains(prohibited));
    }
}

#[test]
fn owner_and_component_filters_are_bounded_and_fail_closed() {
    assert_eq!(
        FabricReadScope::new([], [FabricClassification::Internal]),
        Err(FabricManagementReadError::InvalidScope)
    );
    assert_eq!(
        local_scope().restrict_to_owners(Vec::<String>::new()),
        Err(FabricManagementReadError::InvalidScope)
    );
    assert_eq!(
        local_scope().restrict_to_components(["INVALID"]),
        Err(FabricManagementReadError::InvalidScope)
    );
    assert_eq!(
        local_scope().restrict_to_components((0..1_025).map(|index| format!("component-{index}"))),
        Err(FabricManagementReadError::InvalidScope)
    );
    assert_eq!(
        local_scope().restrict_to_components(std::iter::repeat_n("framework-app", 1_025)),
        Err(FabricManagementReadError::InvalidScope)
    );

    let registry = FabricRegistry::load(fixture_records()).expect("valid registry");
    let product_owned = local_scope()
        .restrict_to_owners(["product-engineering"])
        .expect("owner scope");
    let overview = registry
        .management_overview(&product_owned)
        .expect("product-owned overview");
    assert_eq!(overview.summary.component_count, 6);
    assert_eq!(overview.summary.owner_count, 1);
    assert!(overview
        .components
        .iter()
        .all(|component| component.owner_id == "product-engineering"));
}

#[test]
fn chained_restrictions_are_monotonic_and_disjoint_filters_fail_closed() {
    let registry = FabricRegistry::load(fixture_records()).expect("valid registry");
    let broad = local_scope()
        .restrict_to_owners(["product-engineering", "enterprise-data-platform"])
        .expect("broad owner scope");
    let broad_overview = registry
        .management_overview(&broad)
        .expect("broad overview");
    let narrow = broad
        .clone()
        .restrict_to_owners(["enterprise-data-platform", "integration-engineering"])
        .expect("intersection remains authorized");
    let narrow_overview = registry
        .management_overview(&narrow)
        .expect("narrow overview");

    assert!(narrow_overview.summary.component_count < broad_overview.summary.component_count);
    assert!(
        narrow_overview.summary.relationship_count <= broad_overview.summary.relationship_count
    );
    assert!(narrow_overview
        .components
        .iter()
        .all(|component| component.owner_id == "enterprise-data-platform"));
    assert_eq!(
        narrow.restrict_to_owners(["product-engineering"]),
        Err(FabricManagementReadError::InvalidScope)
    );

    let components = local_scope()
        .restrict_to_components([
            "framework-app",
            "framework-app-service",
            "framework-app-web",
        ])
        .expect("broad component scope")
        .restrict_to_components(["framework-app-service", "system-b-integration"])
        .expect("component intersection");
    let component_overview = registry
        .management_overview(&components)
        .expect("component overview");
    assert_eq!(component_overview.summary.component_count, 1);
    assert_eq!(
        component_overview.components[0].component_id,
        "framework-app-service"
    );
}

#[test]
fn management_projection_preserves_health_source_and_evidence_binding() {
    let mut records = fixture_records();
    let backend = records
        .iter_mut()
        .find(|record| record.component_id == "framework-app-service")
        .expect("backend fixture");
    backend.evidence[0].kind = FabricEvidenceKind::Health;
    backend.evidence[0].posture = FabricEvidencePosture::Verified;
    backend.evidence[0].sha256 = Some("a".repeat(64));
    backend.health.health = FabricHealth::Healthy;
    backend.health.availability = FabricAvailability::Available;
    backend.health.source = FabricObservationSource::Observed;
    backend.health.evidence_id = Some(backend.evidence[0].evidence_id.clone());
    let registry = FabricRegistry::load(records).expect("observed-health registry");
    let overview = registry
        .management_overview(&local_scope())
        .expect("observed overview");
    let card = overview
        .components
        .iter()
        .find(|component| component.component_id == "framework-app-service")
        .expect("backend card");
    assert_eq!(card.health_source, FabricObservationSource::Observed);
    assert_eq!(
        card.health_evidence_id.as_deref(),
        Some("fixture.framework-app.backend")
    );

    let detail = registry
        .component_detail(&local_scope(), "framework-app-service")
        .expect("backend detail");
    assert_eq!(detail.health_source, FabricObservationSource::Observed);
    assert_eq!(
        detail.health_evidence_id.as_deref(),
        Some("fixture.framework-app.backend")
    );
    assert!(serde_json::to_string(&detail)
        .expect("detail JSON")
        .contains("\"health_source\":\"observed\""));
}
