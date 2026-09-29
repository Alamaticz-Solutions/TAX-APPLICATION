use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
};

use appfw_runtime::{
    data_access as runtime_data_access,
    extension::UserAuth,
    model_metadata::{RuntimeDataType, RuntimeEntityMetadata, RuntimePropertyMetadata},
    provider_error as runtime_provider_error,
    provider_keys::FrameworkProvider,
    provider_time_period,
    query_cost::{
        QueryCostBudget, RuntimeFilterCostNode, RuntimeQueryCostInput, RuntimeRelationKind,
        RuntimeSelectionCostNode, RuntimeSelectionCostTree,
    },
    query_filter::{
        filter_token, runtime_filter_capabilities_for_provider, runtime_filter_data_types,
    },
    query_ir::{
        aggregate_alias_exists, apply_keyset_cursor_filter, decode_keyset_cursor,
        default_aggregate_metric_alias, encode_keyset_cursor, ensure_aggregate_groupable,
        ensure_aggregate_having_op, ensure_keyset_sort, expect_relationship_filter_object,
        normalize_array_input, normalize_filter_conjunction_items, normalize_object_input,
        parse_sort_specs, scalar_filter_predicates, validate_aggregate_metric_field,
        validate_aggregate_output_alias, validate_unique_aggregate_aliases,
        RuntimeAggregateFieldDescriptor, RuntimeAggregateFunction, RuntimePagination,
        RuntimePaginationPolicy, RuntimeSortDirection, RuntimeSortSpecInput,
    },
    record_audit,
    record_computed::{self, RuntimeComputedKind},
    record_timezone,
    record_validation::{
        is_validator_named, property_validators, render_uniqueness_filter, uniqueness_conflict,
        validate_record, validator_message,
    },
    record_version, AccessAction, PolicyAccess, RuntimeFilterOp, RuntimeJsonAggregateResult,
    RuntimeJsonObj, RuntimeJsonQueryResult, RuntimeProviderAggregatePlan,
    RuntimeProviderDataClient, RuntimeProviderDescriptor, RuntimeProviderIdentity,
    RuntimeProviderMutationKind, RuntimeProviderMutationPlan, RuntimeProviderOperation,
    RuntimeProviderOperationContract, RuntimeProviderOperationCounts,
    RuntimeProviderOperationRequirement, RuntimeProviderOperationSurface, RuntimeProviderPlanInput,
    RuntimeProviderQueryPlan, RuntimeProviderRegistry, RuntimeSaasEndpoint, RuntimeSaasRequest,
    RuntimeSaasResponse, RuntimeSaasResponseBody, RuntimeSaasTransportError,
    RuntimeSyncWorkerConfig,
};
use appfw_saas_core::{SaasHttpMethod, SaasRequestPlan, SaasTransportProtocol};
use serde_json::json;

#[test]
fn runtime_filter_operator_tokens_are_complete_and_round_trip() {
    let cases = [
        (RuntimeFilterOp::Eq, filter_token::EQUALS),
        (RuntimeFilterOp::Ne, filter_token::NOT_EQUALS),
        (RuntimeFilterOp::Lt, filter_token::LESS_THAN),
        (RuntimeFilterOp::Lte, filter_token::LESS_THAN_OR_EQUAL),
        (RuntimeFilterOp::Gt, filter_token::GREATER_THAN),
        (RuntimeFilterOp::Gte, filter_token::GREATER_THAN_OR_EQUAL),
        (RuntimeFilterOp::Regex, filter_token::REGEX),
        (RuntimeFilterOp::StartsWith, filter_token::STARTS_WITH),
        (RuntimeFilterOp::Contains, filter_token::CONTAINS),
        (RuntimeFilterOp::NotContains, filter_token::NOT_CONTAINS),
        (RuntimeFilterOp::EndsWith, filter_token::ENDS_WITH),
        (RuntimeFilterOp::Overlaps, filter_token::OVERLAPS),
        (RuntimeFilterOp::NotOverlaps, filter_token::NOT_OVERLAPS),
        (RuntimeFilterOp::ContainedBy, filter_token::CONTAINED_BY),
        (
            RuntimeFilterOp::NotContainedBy,
            filter_token::NOT_CONTAINED_BY,
        ),
        (RuntimeFilterOp::In, filter_token::IN),
        (RuntimeFilterOp::NotIn, filter_token::NOT_IN),
        (RuntimeFilterOp::Before, filter_token::BEFORE),
        (RuntimeFilterOp::During, filter_token::DURING),
        (RuntimeFilterOp::After, filter_token::AFTER),
        (RuntimeFilterOp::Has, filter_token::HAS),
    ];

    for (op, token) in cases {
        assert_eq!(op.as_filter_token(), token);
        assert_eq!(RuntimeFilterOp::from_token(token).unwrap(), op);
    }
}

#[test]
fn runtime_provider_capability_report_uses_canonical_data_type_order() {
    let capabilities = runtime_filter_capabilities_for_provider(FrameworkProvider::Postgres);

    assert_eq!(
        capabilities
            .data_types
            .iter()
            .map(|capability| capability.data_type)
            .collect::<Vec<_>>(),
        runtime_filter_data_types()
    );

    let object_id_capability = capabilities
        .data_types
        .iter()
        .find(|capability| capability.data_type == RuntimeDataType::ObjectId)
        .expect("ObjectId capability should be present")
        .operators
        .iter()
        .find(|operator| operator.op == filter_token::EQUALS)
        .expect("ObjectId equality capability should be present");

    assert!(!object_id_capability.supported);
    assert!(
        object_id_capability
            .unsupported_reason
            .unwrap_or_default()
            .contains("ObjectId"),
        "unsupported provider capabilities should carry actionable runtime reasons"
    );
}

#[test]
fn query_budget_rejects_nested_relationship_depth_from_runtime_selection() {
    let plan = RuntimeQueryCostInput {
        selection: RuntimeSelectionCostTree {
            fields: vec![RuntimeSelectionCostNode {
                data_type: RuntimeDataType::NavToMany,
                has_target_entity: true,
                children: vec![RuntimeSelectionCostNode {
                    data_type: RuntimeDataType::NavToOne,
                    has_target_entity: true,
                    children: vec![RuntimeSelectionCostNode {
                        data_type: RuntimeDataType::String,
                        has_target_entity: false,
                        children: Vec::new(),
                    }],
                }],
            }],
        },
        ..RuntimeQueryCostInput::default()
    };
    let budget = QueryCostBudget {
        max_relationship_depth: 1,
        ..QueryCostBudget::default()
    };

    let err = budget
        .enforce_query(&plan)
        .expect_err("nested relationship depth should exceed budget");

    assert!(err.to_string().contains("relationship depth 2"));
}

#[test]
fn query_budget_rejects_many_to_many_expansion_from_runtime_filter() {
    let plan = RuntimeQueryCostInput {
        filter: Some(RuntimeFilterCostNode::Relation {
            kind: RuntimeRelationKind::ManyToMany,
            filter: Box::new(RuntimeFilterCostNode::Field {
                data_type: RuntimeDataType::String,
            }),
        }),
        ..RuntimeQueryCostInput::default()
    };
    let budget = QueryCostBudget {
        max_many_to_many_expansions: 0,
        ..QueryCostBudget::default()
    };

    let err = budget
        .enforce_query(&plan)
        .expect_err("many-to-many filter expansion should exceed budget");

    assert!(err.to_string().contains("many-to-many expansion count 1"));
}

#[test]
fn query_ir_pagination_and_keyset_helpers_are_runtime_owned() {
    let policy = RuntimePaginationPolicy {
        default_page_size: 25,
        max_page_size: 100,
    };
    assert_eq!(policy.normalize(None, None).expect("defaults"), (0, 25));
    assert!(policy.normalize(Some(0), Some(101)).is_err());

    let sort = ensure_keyset_sort("id", None).expect("primary key sort");
    assert_eq!(sort, json!({ "id": "asc" }));

    let sort_specs =
        parse_sort_specs(Some(json!({ "created_at": "desc" }))).expect("runtime sort specs");
    assert_eq!(
        sort_specs,
        vec![RuntimeSortSpecInput {
            field: "created_at".to_string(),
            direction: RuntimeSortDirection::Desc,
        }]
    );

    let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-2"))
        .expect("keyset cursor");
    let filter = apply_keyset_cursor_filter(
        Some(json!({ "name": { "_starts": "A" } })),
        &sort,
        Some(&cursor),
    )
    .expect("cursor filter");

    assert_eq!(
        filter,
        Some(json!({
            "_and": [
                { "name": { "_starts": "A" } },
                { "id": { "_gt": "account-2" } }
            ]
        }))
    );
}

#[test]
fn query_ir_aggregate_primitives_are_runtime_owned() {
    let age = RuntimeAggregateFieldDescriptor::new("age", RuntimeDataType::Int32);
    let name = RuntimeAggregateFieldDescriptor::new("name", RuntimeDataType::String);
    let contacts = RuntimeAggregateFieldDescriptor::new("contacts", RuntimeDataType::NavToMany);

    assert_eq!(
        RuntimeAggregateFunction::from_name("average").expect("average alias"),
        RuntimeAggregateFunction::Avg
    );
    validate_aggregate_output_alias("total_age").expect("alias");
    validate_unique_aggregate_aliases(["is_active", "total_age"]).expect("unique aliases");
    assert!(aggregate_alias_exists(["count", "total_age"], "count"));

    ensure_aggregate_groupable(&name).expect("groupable scalar");
    validate_aggregate_metric_field(RuntimeAggregateFunction::Sum, Some(&age))
        .expect("numeric metric");
    validate_aggregate_metric_field(RuntimeAggregateFunction::CountDistinct, Some(&name))
        .expect("distinct scalar");
    ensure_aggregate_having_op(RuntimeFilterOp::Gte).expect("having comparison");

    assert!(ensure_aggregate_groupable(&contacts).is_err());
    assert!(validate_aggregate_metric_field(RuntimeAggregateFunction::Sum, Some(&name)).is_err());
    assert!(ensure_aggregate_having_op(RuntimeFilterOp::Contains).is_err());
    assert_eq!(
        default_aggregate_metric_alias(RuntimeAggregateFunction::Sum, Some("age")),
        "sum_age"
    );
}

#[test]
fn query_ir_parser_shape_checks_are_runtime_owned() {
    let filter = normalize_object_input(Some(json!(r#"{"name":{"_eq":"Acme"}}"#)), "filter")
        .expect("filter shape")
        .expect("filter object");
    assert!(filter.contains_key("name"));

    let group_by = normalize_array_input(Some(json!("industryId")), "group_by")
        .expect("group_by shape")
        .expect("group_by array");
    assert_eq!(group_by, vec![json!("industryId")]);

    let conjunction = normalize_filter_conjunction_items(
        "_and",
        json!([
            { "name": { "_starts": "A" } },
            { "age": { "_gt": 10 } }
        ]),
    )
    .expect("conjunction items");
    assert_eq!(conjunction.len(), 2);

    let relationship =
        expect_relationship_filter_object("navigation", "industry", json!({ "name": "Tech" }))
            .expect("relationship filter");
    assert_eq!(relationship["name"], json!("Tech"));

    let predicates =
        scalar_filter_predicates("age", json!({ "_gt": 10 })).expect("scalar predicate");
    assert_eq!(predicates[0].op, RuntimeFilterOp::Gt);

    assert!(normalize_object_input(Some(json!([1])), "filter").is_err());
    assert!(normalize_array_input(Some(json!({ "field": "age" })), "group_by").is_err());
    assert!(normalize_filter_conjunction_items("_and", json!({})).is_err());
    assert!(expect_relationship_filter_object("navigation", "industry", json!("Tech")).is_err());
    assert!(scalar_filter_predicates("age", json!({ "_bogus": 1 })).is_err());
}

#[test]
fn policy_access_contracts_are_runtime_owned() {
    let access = PolicyAccess::allow_with_filter(json!({ "billing_state": { "_eq": "CA" } }))
        .and_filter(Some(json!({ "tenant_id": { "_eq": "tenant-1" } })));

    assert_eq!(
        access.filter,
        Some(json!({
            "_and": [
                { "billing_state": { "_eq": "CA" } },
                { "tenant_id": { "_eq": "tenant-1" } }
            ]
        }))
    );

    let denied = PolicyAccess::deny().and_filter(Some(json!({ "tenant_id": "tenant-1" })));
    assert!(!denied.allow);
    assert!(denied.filter.is_none());

    let parsed = PolicyAccess::from_policy_result(
        "crm.account",
        json!({ "allow": true, "filter": { "is_active": { "_eq": true } } }),
    )
    .expect("policy result should parse");
    assert!(parsed.allow);
    assert_eq!(parsed.filter, Some(json!({ "is_active": { "_eq": true } })));

    assert!(PolicyAccess::from_policy_result(
        "crm.account",
        json!({ "allow": true, "filter": [] })
    )
    .is_err());
}

#[test]
fn record_validation_engine_is_runtime_owned() {
    fn property(name: &str, validators: serde_json::Value) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: format!("{}-id", name),
            name: name.to_string(),
            caption: name.to_string(),
            data_type: RuntimeDataType::String,
            is_key: false,
            is_caption: false,
            is_required: name == "email",
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: None,
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: Some(json!({ "validators": validators })),
        }
    }

    let entity = RuntimeEntityMetadata {
        id: "contact-id".to_string(),
        schema_name: "crm".to_string(),
        schema_id: None,
        pascal_1: "Contact".to_string(),
        pascal_n: "Contacts".to_string(),
        snake_1: "contact".to_string(),
        snake_n: "contacts".to_string(),
        caption_1: "Contact".to_string(),
        caption_n: "Contacts".to_string(),
        is_union: false,
        base_type: None,
        is_table: true,
        facets: Vec::new(),
        meta: None,
        standard_methods: Vec::new(),
        custom_methods: Vec::new(),
        properties: vec![
            property(
                "email",
                json!([
                    { "name": "UniquenessValidator", "filter": r#"{ "email": { "_eq": "$email" } }"# },
                    { "name": "StringLength", "min": 3, "message": "bad length" }
                ]),
            ),
            property("status", json!([])),
        ],
    };

    let meta = json!({
        "validators": [
            { "name": "UniquenessValidator", "filter": r#"{ "email": { "_eq": "$email" } }"# },
            { "name": "StringLength", "message": "bad length" }
        ]
    });
    let validators = property_validators("email", Some(&meta)).expect("validators");

    assert!(is_validator_named(validators[0], "uniqueness"));
    assert_eq!(validator_message(validators[1], "default"), "bad length");

    let record = json!({ "id": "1", "email": "a@example.com" })
        .as_object()
        .expect("object")
        .clone();
    validate_record(&entity, &record, AccessAction::Create).expect("valid record");
    let invalid_record = json!({ "id": "2", "email": "" })
        .as_object()
        .expect("object")
        .clone();
    assert!(validate_record(&entity, &invalid_record, AccessAction::Create).is_err());

    let filter =
        render_uniqueness_filter(r#"{ "email": { "_eq": "$email" } }"#, &record).expect("filter");
    assert_eq!(filter, json!({ "email": { "_eq": "a@example.com" } }));

    let same = json!({ "id": "1" }).as_object().expect("object").clone();
    let other = json!({ "id": "2" }).as_object().expect("object").clone();
    assert!(!uniqueness_conflict("id", &record, &[same]));
    assert!(uniqueness_conflict("id", &record, &[other]));
}

#[test]
fn record_timezone_rules_are_runtime_owned() {
    assert_eq!(
        record_timezone::to_client_tz(&json!("2024-01-15T12:00:00Z"), "America/New_York")
            .expect("timezone conversion"),
        json!("2024-01-15T07:00:00-05:00")
    );
    assert_eq!(
        record_timezone::to_utc(&json!("2024-01-15T07:00:00-05:00"), "America/New_York")
            .expect("utc conversion"),
        json!("2024-01-15T12:00:00+00:00")
    );
}

#[test]
fn record_version_rules_are_runtime_owned() {
    let mut version_prop = RuntimePropertyMetadata {
        id: "version-id".to_string(),
        name: "version".to_string(),
        caption: "Version".to_string(),
        data_type: RuntimeDataType::Int64,
        is_key: false,
        is_caption: false,
        is_required: false,
        is_read_only: false,
        is_concurrency_control: true,
        default_value: None,
        foreign_key: None,
        nav_by_fk: None,
        many_to_many: None,
        nested_entity_type: None,
        enum_type_name: None,
        meta: None,
    };
    let entity = RuntimeEntityMetadata {
        id: "account-id".to_string(),
        schema_name: "crm".to_string(),
        schema_id: None,
        pascal_1: "Account".to_string(),
        pascal_n: "Accounts".to_string(),
        snake_1: "account".to_string(),
        snake_n: "accounts".to_string(),
        caption_1: "Account".to_string(),
        caption_n: "Accounts".to_string(),
        is_union: false,
        base_type: None,
        is_table: true,
        facets: Vec::new(),
        meta: None,
        standard_methods: Vec::new(),
        custom_methods: Vec::new(),
        properties: vec![version_prop.clone()],
    };
    let record = json!({ "version": 42 })
        .as_object()
        .expect("object")
        .clone();

    assert_eq!(
        record_version::get_record_version(&entity, &record).expect("version lookup"),
        Some(json!(42))
    );
    assert!(record_version::new_record_version(&version_prop, &record)
        .expect("new version")
        .is_some());

    version_prop.data_type = RuntimeDataType::String;
    assert!(record_version::new_record_version(&version_prop, &record).is_err());
}

#[test]
fn record_computed_rules_are_runtime_owned() {
    let meta = json!({
        "FormatComputed": {
            "propNames": ["firstName", "lastName"],
            "template": "{first_name} <{last_name}>"
        }
    });
    let record = json!({
        "first_name": "Ada",
        "last_name": "Lovelace"
    })
    .as_object()
    .expect("object")
    .clone();

    assert_eq!(
        record_computed::try_compute_property(
            RuntimeComputedKind::Format,
            "label",
            Some(&meta),
            &record,
        )
        .expect("computed value"),
        Some(json!("Ada <Lovelace>"))
    );
    assert!(record_computed::try_compute_property(
        RuntimeComputedKind::Concatenate,
        "full_name",
        None,
        &record,
    )
    .is_err());
}

#[test]
fn record_audit_helpers_are_runtime_owned() {
    let entity = RuntimeEntityMetadata {
        id: "account-id".to_string(),
        schema_name: "crm".to_string(),
        schema_id: None,
        pascal_1: "Account".to_string(),
        pascal_n: "Accounts".to_string(),
        snake_1: "account".to_string(),
        snake_n: "accounts".to_string(),
        caption_1: "Account".to_string(),
        caption_n: "Accounts".to_string(),
        is_union: false,
        base_type: None,
        is_table: true,
        facets: vec!["audited".to_string()],
        meta: None,
        standard_methods: Vec::new(),
        custom_methods: Vec::new(),
        properties: vec![
            RuntimePropertyMetadata {
                id: "id-id".to_string(),
                name: "id".to_string(),
                caption: "Id".to_string(),
                data_type: RuntimeDataType::String,
                is_key: true,
                is_caption: false,
                is_required: true,
                is_read_only: false,
                is_concurrency_control: false,
                default_value: None,
                foreign_key: None,
                nav_by_fk: None,
                many_to_many: None,
                nested_entity_type: None,
                enum_type_name: None,
                meta: None,
            },
            RuntimePropertyMetadata {
                id: "api-token-id".to_string(),
                name: "api_token".to_string(),
                caption: "API Token".to_string(),
                data_type: RuntimeDataType::String,
                is_key: false,
                is_caption: false,
                is_required: false,
                is_read_only: false,
                is_concurrency_control: false,
                default_value: None,
                foreign_key: None,
                nav_by_fk: None,
                many_to_many: None,
                nested_entity_type: None,
                enum_type_name: None,
                meta: None,
            },
        ],
    };
    let record = json!({ "id": "account-1", "api_token": "secret" })
        .as_object()
        .expect("object")
        .clone();

    assert!(record_audit::is_audited(&entity));
    assert_eq!(
        record_audit::record_id(&entity, &record),
        Some("account-1".to_string())
    );
    assert_eq!(
        record_audit::chain_scope(&entity, Some("tenant-1"), Some("account-1")),
        "crm.Account:tenant-1:account-1"
    );

    let redacted = record_audit::redact_record_for_external_response(&entity, record);
    assert_eq!(redacted["api_token"], json!({ "_redacted": true }));
    assert_eq!(
        record_audit::policy_decision_json(
            &PolicyAccess::allow_with_filter(json!({ "tenant_id": "tenant-1" })),
            "allowed"
        ),
        json!({
            "allow": true,
            "filter": { "tenant_id": "tenant-1" },
            "decision": { "reason": "allowed" }
        })
    );

    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "alex".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: Vec::new(),
        token: "do-not-store".to_string(),
    };
    let event = record_audit::RuntimeAuditEvent::entity_mutation(
        &entity,
        AccessAction::Update,
        &user,
        Some("account-1".to_string()),
        Some(json!({ "id": "account-1", "api_token": "old" })),
        Some(json!({ "id": "account-1", "api_token": "new" })),
        &PolicyAccess::allow_all(),
    )
    .finalize(Some("previous-hash".to_string()))
    .expect("finalized event");

    assert_eq!(event.audit_table_name, "accounts_audit");
    assert_eq!(event.chain_scope, "crm.Account:tenant-1:account-1");
    assert_eq!(event.prev_hash.as_deref(), Some("previous-hash"));
    assert!(!event.event_hash.is_empty());

    let query = record_audit::audit_query(&entity, "tenant-1", "account-1", 250);
    assert_eq!(query.schema_name, "crm");
    assert_eq!(query.entity_name, "Account");
    assert_eq!(query.audit_table_name, "accounts_audit");
    assert_eq!(query.tenant_id, "tenant-1");
    assert_eq!(query.record_id, "account-1");
    assert_eq!(query.limit, 100);

    let new_event = record_audit::RuntimeAuditEvent::entity_mutation(
        &entity,
        AccessAction::Update,
        &user,
        Some("account-1".to_string()),
        None,
        Some(json!({ "id": "account-1", "api_token": "newer" })),
        &PolicyAccess::allow_all(),
    );
    let continued = record_audit::continue_record_chain(
        new_event,
        Some(&json!({
            "tenant_id": "tenant-b-sentinel",
            "chain_scope": "crm.Account:tenant-b-sentinel:account-1"
        })),
    );
    assert_eq!(continued.tenant_id.as_deref(), Some("tenant-1"));
    assert_eq!(continued.chain_scope, "crm.Account:tenant-1:account-1");
}

#[tokio::test]
async fn data_access_read_query_helpers_are_runtime_owned() {
    let filter = runtime_data_access::primary_key_in_filter(
        "id",
        vec![json!("account-1"), json!("account-2")],
    )
    .expect("batch id filter");
    assert_eq!(
        filter,
        json!({ "id": { "_in": ["account-1", "account-2"] } })
    );
    assert_eq!(
        runtime_data_access::batch_limit_for_ids(2).expect("batch limit"),
        2
    );

    let pagination =
        RuntimePagination::keyset(Some("previous".to_string()), 1).expect("keyset pagination");
    let diagnostic = runtime_data_access::pagination_diagnostic(&pagination);
    assert_eq!(diagnostic.strategy, "keyset");
    assert!(diagnostic.after_present);

    let items = vec![json!({ "id": "account-2" })
        .as_object()
        .expect("object")
        .clone()];
    let cursors = runtime_data_access::keyset_cursors_from_items(
        &pagination,
        "id",
        RuntimeSortDirection::Asc,
        &items,
    )
    .expect("query cursors");

    assert_eq!(cursors.previous_cursor, Some("previous".to_string()));
    // Cursors are opaque (base64url + HMAC); decode to verify the signed payload value.
    let next = cursors.next_cursor.as_deref().expect("next cursor");
    assert_eq!(
        decode_keyset_cursor(next)
            .expect("decode next cursor")
            .value,
        json!("account-2")
    );

    let finalized = runtime_data_access::finalize_query_items_page(
        &pagination,
        Some(&runtime_data_access::RuntimeReadSort {
            field: "id".to_string(),
            direction: RuntimeSortDirection::Asc,
        }),
        items,
        |mut item| {
            item.insert("evaluated".to_string(), json!(true));
            Ok::<_, appfw_runtime::RuntimeError>(item)
        },
    )
    .expect("finalized read page");
    assert_eq!(finalized.previous_cursor, Some("previous".to_string()));
    assert_eq!(finalized.items[0]["evaluated"], json!(true));

    let mut traced = None;
    let executed = runtime_data_access::execute_list_read(
        RuntimeProviderOperation::GetItems,
        || async { Ok::<_, appfw_runtime::RuntimeError>(finalized.items) },
        |operation, _started_at, counts| traced = Some((operation, counts)),
        |mut item| {
            item.insert("executed".to_string(), json!(true));
            Ok::<_, appfw_runtime::RuntimeError>(item)
        },
    )
    .await
    .expect("executed read provider call");
    assert_eq!(executed[0]["executed"], json!(true));
    assert_eq!(
        traced,
        Some((
            RuntimeProviderOperation::GetItems,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );
}

#[tokio::test]
async fn data_access_aggregate_helpers_are_runtime_owned() {
    #[derive(Debug)]
    struct AggregateResult {
        query_count: i64,
        items: Vec<serde_json::Value>,
    }

    let mut traced = None;
    let result = runtime_data_access::execute_aggregate_read(
        || async {
            Ok::<_, appfw_runtime::RuntimeError>(AggregateResult {
                query_count: 4,
                items: vec![json!({ "total": 7 }), json!({ "total": 9 })],
            })
        },
        |result| result.query_count,
        |result| result.items.len() as i64,
        |operation, _started_at, counts| traced = Some((operation, counts)),
    )
    .await
    .expect("executed aggregate provider call");

    assert_eq!(result.items.len(), 2);
    assert_eq!(
        traced,
        Some((
            RuntimeProviderOperation::AggregateItems,
            RuntimeProviderOperationCounts::new(4, 2)
        ))
    );
}

#[test]
fn data_access_mutation_helpers_are_runtime_owned() {
    let entity = RuntimeEntityMetadata {
        id: "account-id".to_string(),
        schema_name: "crm".to_string(),
        schema_id: None,
        pascal_1: "Account".to_string(),
        pascal_n: "Accounts".to_string(),
        snake_1: "account".to_string(),
        snake_n: "accounts".to_string(),
        caption_1: "Account".to_string(),
        caption_n: "Accounts".to_string(),
        is_union: false,
        base_type: None,
        is_table: true,
        facets: Vec::new(),
        meta: None,
        standard_methods: Vec::new(),
        custom_methods: Vec::new(),
        properties: vec![RuntimePropertyMetadata {
            id: "id-id".to_string(),
            name: "id".to_string(),
            caption: "Id".to_string(),
            data_type: RuntimeDataType::String,
            is_key: true,
            is_caption: false,
            is_required: true,
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: None,
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: None,
        }],
    };
    let input = json!({ "id": "input-id" })
        .as_object()
        .expect("object")
        .clone();
    let provider_result = json!({ "id": "provider-id" })
        .as_object()
        .expect("object")
        .clone();

    assert_eq!(
        runtime_data_access::mutation_record_id(
            &entity,
            runtime_data_access::RuntimeMutationKind::Create,
            Some(&input),
            Some(&provider_result),
            None,
        ),
        Some("provider-id".to_string())
    );
    assert_eq!(
        runtime_data_access::delete_audit_outcome(true, 0),
        Some(runtime_data_access::RuntimeDeleteAuditOutcome::NotApplied)
    );
    assert!(runtime_data_access::should_check_filtered_update_denial(
        Some(&json!({ "tenant_id": "tenant-1" })),
        Some("input-id"),
        None,
    ));
    assert_eq!(
        runtime_data_access::RuntimeMutationKind::Update
            .provider_operation()
            .as_str(),
        "update_item"
    );
}

#[tokio::test]
async fn data_access_mutation_dispatch_helpers_are_runtime_owned() {
    let mut traced = None;
    let deleted = runtime_data_access::execute_mutation(
        runtime_data_access::RuntimeMutationKind::Delete,
        || async { Ok::<_, appfw_runtime::RuntimeError>(2_i64) },
        |deleted_count| *deleted_count,
        |operation, _started_at, counts| traced = Some((operation, counts)),
    )
    .await
    .expect("executed mutation provider call");

    assert_eq!(deleted, 2);
    assert_eq!(
        traced,
        Some((
            RuntimeProviderOperation::DeleteItem,
            RuntimeProviderOperationCounts::new(2, 2)
        ))
    );
}

#[tokio::test]
async fn data_access_audit_append_helpers_are_runtime_owned() {
    let entity = RuntimeEntityMetadata {
        id: "account-id".to_string(),
        schema_name: "crm".to_string(),
        schema_id: None,
        pascal_1: "Account".to_string(),
        pascal_n: "Accounts".to_string(),
        snake_1: "account".to_string(),
        snake_n: "accounts".to_string(),
        caption_1: "Account".to_string(),
        caption_n: "Accounts".to_string(),
        is_union: false,
        base_type: None,
        is_table: true,
        facets: vec!["audited".to_string()],
        meta: None,
        standard_methods: Vec::new(),
        custom_methods: Vec::new(),
        properties: vec![RuntimePropertyMetadata {
            id: "id-id".to_string(),
            name: "id".to_string(),
            caption: "Id".to_string(),
            data_type: RuntimeDataType::String,
            is_key: true,
            is_caption: false,
            is_required: true,
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: None,
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: None,
        }],
    };
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "alex".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: Vec::new(),
        token: "do-not-store".to_string(),
    };

    let mut mutation_event = None;
    runtime_data_access::append_audit_mutation(
        &entity,
        AccessAction::Update,
        &user,
        Some("account-1".to_string()),
        Some(json!({ "id": "account-1", "name": "Old" })),
        Some(json!({ "id": "account-1", "name": "New" })),
        &PolicyAccess::allow_all(),
        |event| {
            mutation_event = Some(event);
            async { Ok::<_, appfw_runtime::RuntimeError>(()) }
        },
    )
    .await
    .expect("appended runtime-owned mutation event");

    let mutation_event = mutation_event.expect("mutation event");
    assert_eq!(mutation_event.action, "update");
    assert_eq!(mutation_event.outcome, "succeeded");

    let mut attempt_event = None;
    runtime_data_access::append_audit_attempt_on_record_chain(
        &entity,
        AccessAction::Update,
        Some(&user),
        "denied",
        Some("account-1".to_string()),
        None,
        Some(json!({ "id": "account-1" })),
        Some(record_audit::operation_decision_json("policy_denied")),
        |event| {
            attempt_event = Some(event);
            async { Ok::<_, appfw_runtime::RuntimeError>(()) }
        },
    )
    .await
    .expect("appended runtime-owned chained attempt event");

    let attempt_event = attempt_event.expect("attempt event");
    assert_eq!(attempt_event.outcome, "denied");
    assert_eq!(attempt_event.chain_scope, "crm.Account:tenant-1:account-1");

    let mut denied_helper_event = None;
    runtime_data_access::append_policy_denied_audit_attempt_on_record_chain(
        &entity,
        AccessAction::Update,
        &user,
        Some("account-1".to_string()),
        Some(json!({ "id": "account-1", "name": "Old" })),
        Some(json!({ "id": "account-1", "name": "Blocked" })),
        &PolicyAccess {
            allow: false,
            filter: Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
        },
        |event| {
            denied_helper_event = Some(event);
            async { Ok::<_, appfw_runtime::RuntimeError>(()) }
        },
    )
    .await
    .expect("appended runtime-owned policy denied attempt event");

    let denied_helper_event = denied_helper_event.expect("denied helper event");
    assert_eq!(denied_helper_event.outcome, "denied");
    assert_eq!(
        denied_helper_event.policy_json.expect("policy json")["decision"]["reason"],
        json!("policy_denied")
    );

    let mut missing_user_event = None;
    runtime_data_access::append_missing_user_audit_attempt(
        &entity,
        AccessAction::Read,
        Some("account-1".to_string()),
        "not authorized",
        |event| {
            missing_user_event = Some(event);
            async { Ok::<_, appfw_runtime::RuntimeError>(()) }
        },
    )
    .await
    .expect("appended runtime-owned missing-user attempt event");

    let missing_user_event = missing_user_event.expect("missing-user event");
    assert_eq!(missing_user_event.outcome, "denied");
    assert_eq!(
        missing_user_event.policy_json.expect("policy json")["decision"]["reason"],
        json!("missing_user")
    );
}

#[tokio::test]
async fn data_access_validation_provider_hooks_are_runtime_owned() {
    let provider = RuntimeOnlyProvider::default();
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: Vec::new(),
        token: "token".to_string(),
    };
    let access = PolicyAccess::allow_with_filter(json!({
        "tenant_id": { "_eq": "tenant-1" },
        "region": { "_eq": "west" }
    }));

    let duplicate = runtime_data_access::validate_primary_key_available(
        &provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [] }),
        Some("account-1".to_string()),
        &user,
        &access,
    )
    .await
    .expect_err("existing primary key should be rejected");

    assert!(matches!(
        duplicate,
        appfw_runtime::RuntimeError::DataStore(appfw_runtime::DataStoreError::DuplicateKey)
    ));
    assert_eq!(
        provider.observed_find_access(),
        Some(access.filter.clone().expect("filtered access"))
    );

    runtime_data_access::validate_primary_key_available(
        &provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [] }),
        None,
        &user,
        &access,
    )
    .await
    .expect("missing primary-key value should skip provider validation");

    runtime_data_access::validate_foreign_key_exists(
        &provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [] }),
        Some("account-1".to_string()),
        &user,
        &access,
    )
    .await
    .expect("existing foreign key should pass");
    assert_eq!(
        provider.observed_find_access(),
        Some(access.filter.clone().expect("filtered access"))
    );

    let same_record = serde_json::Map::from_iter([("id".to_string(), json!("account-1"))]);
    runtime_data_access::validate_unique_record(
        &provider,
        "unique-query",
        &user,
        &access,
        "id",
        &same_record,
        "Account",
        "name",
        "must be unique",
    )
    .await
    .expect("current record should not conflict with itself");

    let different_record = serde_json::Map::from_iter([("id".to_string(), json!("account-2"))]);
    let conflict = runtime_data_access::validate_unique_record(
        &provider,
        "unique-query",
        &user,
        &access,
        "id",
        &different_record,
        "Account",
        "name",
        "must be unique",
    )
    .await
    .expect_err("another record with the same unique value should be rejected");

    assert!(matches!(
        conflict,
        appfw_runtime::RuntimeError::Validation(message)
            if message == "Account.name: must be unique"
    ));
}

#[tokio::test]
async fn data_access_audit_reads_require_policy_visible_source_record() {
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: Vec::new(),
        token: "token".to_string(),
    };
    let access = PolicyAccess::allow_with_filter(json!({
        "_and": [
            { "tenant_id": { "_eq": "tenant-1" } },
            { "region": { "_eq": "west" } }
        ]
    }));
    let query = record_audit::RuntimeAuditQuery::new(
        "crm",
        "Account",
        "accounts_audit",
        "tenant-1",
        "account-1",
        10,
    );
    let provider = RuntimeOnlyProvider {
        find_visible: false,
        ..RuntimeOnlyProvider::default()
    };

    let events = runtime_data_access::execute_audit_events_read(
        &provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [{ "name": "id" }] }),
        "account-1".to_string(),
        &user,
        &access,
        query.clone(),
    )
    .await
    .expect("policy-hidden record produces an empty audit timeline");

    assert!(events.is_empty());
    assert_eq!(provider.observed_audit_query_count(), 0);
    assert_eq!(
        provider.observed_find_access(),
        Some(access.filter.clone().expect("filtered access"))
    );

    let visible_provider = RuntimeOnlyProvider::default();
    let events = runtime_data_access::execute_audit_events_read(
        &visible_provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [{ "name": "id" }] }),
        "account-1".to_string(),
        &user,
        &access,
        query,
    )
    .await
    .expect("policy-visible record proceeds to its tenant-bound audit timeline");
    assert!(events.is_empty());
    assert_eq!(visible_provider.observed_audit_query_count(), 1);
    assert_eq!(
        visible_provider.observed_find_access(),
        Some(access.filter.clone().expect("filtered access"))
    );

    let mismatched_query = record_audit::RuntimeAuditQuery::new(
        "crm",
        "Account",
        "accounts_audit",
        "tenant-b-sentinel",
        "account-1",
        10,
    );
    let err = runtime_data_access::execute_audit_events_read(
        &provider,
        "Account",
        json!({ "name": "accounts", "selection_set": [{ "name": "id" }] }),
        "account-1".to_string(),
        &user,
        &PolicyAccess::allow_all(),
        mismatched_query,
    )
    .await
    .expect_err("audit query tenant must match the authenticated tenant");
    assert!(!err.to_string().contains("tenant-b-sentinel"));
    assert_eq!(provider.observed_audit_query_count(), 0);
}

#[test]
fn provider_bridge_contracts_are_runtime_owned() {
    let descriptor = RuntimeProviderDescriptor::new(FrameworkProvider::Mssql, "crm_primary");

    assert_eq!(descriptor.provider_key(), "mssql");
    assert_eq!(descriptor.data_source_name(), "crm_primary");
    assert_eq!(
        descriptor.unsupported_explain_diagnostic(),
        json!({
            "status": "unsupported",
            "message": "safe provider EXPLAIN diagnostics are not implemented for this provider",
            "provider": "mssql",
            "data_source": "crm_primary"
        })
    );
    assert_eq!(RuntimeProviderOperation::QueryItems.as_str(), "query_items");
    assert_eq!(
        RuntimeProviderOperationCounts::from_counts(-1, 4),
        RuntimeProviderOperationCounts::new(0, 4)
    );

    struct TestProvider;

    impl RuntimeProviderIdentity for TestProvider {
        fn data_source_name(&self) -> &str {
            "crm_primary"
        }

        fn framework_provider(&self) -> FrameworkProvider {
            FrameworkProvider::Postgres
        }
    }

    let provider = TestProvider;
    assert_eq!(provider.provider_descriptor().provider_key(), "postgres");
    assert_eq!(provider.pool_stats().data_source, "crm_primary");
    assert!(provider.provider_declares_operation(RuntimeProviderOperation::HealthCheck));
    assert_eq!(
        provider
            .provider_operation_contracts()
            .iter()
            .map(|contract| contract.operation)
            .collect::<Vec<_>>(),
        RuntimeProviderOperation::ALL
    );
    assert_eq!(
        provider.provider_operation_contracts()[0],
        RuntimeProviderOperationContract::required(
            RuntimeProviderOperation::HealthCheck,
            RuntimeProviderOperationSurface::RuntimeNative,
        )
    );
    let query_contract = provider
        .provider_operation_contracts()
        .iter()
        .find(|contract| contract.operation == RuntimeProviderOperation::QueryItems)
        .expect("query items contract");
    assert_eq!(
        query_contract.requirement,
        RuntimeProviderOperationRequirement::Required
    );
    assert_eq!(
        query_contract.surface,
        RuntimeProviderOperationSurface::ProductPlanAdapter
    );
    let audit_contract = provider
        .provider_operation_contracts()
        .iter()
        .find(|contract| contract.operation == RuntimeProviderOperation::AppendAuditEvent)
        .expect("audit append contract");
    assert_eq!(audit_contract.surface_key(), "runtime_native");
}

#[test]
fn saas_transport_contracts_are_runtime_owned() {
    let endpoint = RuntimeSaasEndpoint::new(
        FrameworkProvider::Salesforce,
        "salesforce_primary",
        "https://example.my.salesforce.com",
    )
    .expect("valid endpoint");
    let plan = SaasRequestPlan::new(
        "salesforce.health.fetch_limits",
        SaasTransportProtocol::RestJson,
        SaasHttpMethod::Get,
        "/services/data/v67.0/limits",
    );
    let request = RuntimeSaasRequest::new(endpoint, plan).expect("runtime SaaS request");

    assert_eq!(
        request.url().expect("request url").as_str(),
        "https://example.my.salesforce.com/services/data/v67.0/limits"
    );
    assert_eq!(
        RuntimeSaasEndpoint::new(
            FrameworkProvider::Postgres,
            "pg_primary",
            "https://db.example"
        )
        .unwrap_err(),
        RuntimeSaasTransportError::NonExternalApiProvider
    );

    let response = RuntimeSaasResponse::new(
        200,
        BTreeMap::from([
            ("Authorization".to_string(), "Bearer secret".to_string()),
            ("Retry-After".to_string(), "1".to_string()),
        ]),
        RuntimeSaasResponseBody::Json(json!([{ "id": "001" }])),
        Some(1),
    );
    let serialized_metadata = serde_json::to_string(&response.metadata).expect("metadata JSON");

    assert!(response.metadata.rate_limit.is_limited());
    assert!(serialized_metadata.contains("Authorization"));
    assert!(!serialized_metadata.contains("Bearer secret"));
}

#[test]
fn sync_worker_config_contract_is_runtime_owned_and_fail_closed() {
    let config: RuntimeSyncWorkerConfig = serde_yaml::from_str(
        r#"
version: 1
enabled: false
worker_count: 0
workers: []
"#,
    )
    .expect("sync worker config");

    config.validate().expect("fail-closed config is valid");
    assert_eq!(config.planned_worker_count(), 0);
    assert_eq!(config.planned_object_count(), 0);
}

#[test]
fn provider_result_contracts_are_runtime_owned() {
    let query_result = RuntimeJsonQueryResult::new(
        10,
        5,
        12,
        vec![json!({ "id": "account-1" })
            .as_object()
            .expect("object")
            .clone()],
        "2026-05-29T00:00:00Z",
        std::time::Instant::now(),
    );
    assert_eq!(query_result.page_index, 2);
    assert_eq!(query_result.query_count, 12);
    assert_eq!(query_result.items[0]["id"], json!("account-1"));

    let aggregate_result = RuntimeJsonAggregateResult::new(
        0,
        10,
        2,
        vec![json!({ "count": 7 })],
        "2026-05-29T00:00:00Z",
        std::time::Instant::now(),
    );
    assert_eq!(aggregate_result.page_count, 1);
    assert_eq!(aggregate_result.items, vec![json!({ "count": 7 })]);
}

#[test]
fn provider_plan_input_contract_is_runtime_owned() {
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: vec!["appfw:data.read".to_string()],
        token: "token".to_string(),
    };
    let access = PolicyAccess::allow_with_filter(json!({ "tenant_id": { "_eq": "tenant-1" } }));
    let input = RuntimeProviderPlanInput::new("query-plan", &user, &access);

    assert_eq!(input.plan(), &"query-plan");
    assert_eq!(input.user().user_name, "casey");
    assert_eq!(
        input.access().filter,
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
    );

    let mapped = input.map_plan(|plan| format!("{plan}:runtime"));
    let (plan, mapped_user, mapped_access) = mapped.into_parts();
    assert_eq!(plan, "query-plan:runtime");
    assert_eq!(mapped_user.tenant_id, "tenant-1");
    assert!(mapped_access.allow);
}

#[test]
fn provider_mutation_plan_contract_is_runtime_owned() {
    let plan = RuntimeProviderMutationPlan::new(
        RuntimeProviderMutationKind::Delete {
            read_version: Some(json!(9)),
        },
        "Account",
        json!({ "name": "accounts", "selection_set": [] }),
        serde_json::Map::from_iter([("id".to_string(), json!("account-1"))]),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
    );

    assert_eq!(plan.entity, "Account");
    assert_eq!(
        plan.selection_json(),
        json!({ "name": "accounts", "selection_set": [] })
    );
    assert_eq!(plan.input["id"], json!("account-1"));
    assert_eq!(
        plan.access_filter_json(),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
    );
    assert_eq!(plan.read_version(), Some(json!(9)));
}

#[test]
fn provider_query_plan_contract_is_runtime_owned() {
    let plan = RuntimeProviderQueryPlan::new(
        "Account",
        "selection-tree",
        Some("filter-ast"),
        Some("access-filter-ast"),
        "sort-ast",
        RuntimePagination::new(10, 50).expect("valid pagination"),
        json!({ "name": "accounts", "selection_set": [] }),
        Some(json!({ "name": { "_eq": "Acme" } })),
        Some(json!([{ "field": "name", "direction": "asc" }])),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
    );

    assert_eq!(plan.entity_type, "Account");
    assert_eq!(plan.selection, "selection-tree");
    assert_eq!(plan.filter, Some("filter-ast"));
    assert_eq!(plan.access_filter, Some("access-filter-ast"));
    assert_eq!(plan.sort, "sort-ast");
    assert_eq!(plan.pagination.skip, 10);
    assert_eq!(plan.pagination.limit, 50);
    assert_eq!(
        plan.selection_json(),
        json!({ "name": "accounts", "selection_set": [] })
    );
    assert_eq!(
        plan.filter_json(),
        Some(json!({ "name": { "_eq": "Acme" } }))
    );
    assert_eq!(
        plan.sort_json(),
        Some(json!([{ "field": "name", "direction": "asc" }]))
    );
    assert_eq!(
        plan.access_filter_json(),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
    );
}

#[test]
fn provider_aggregate_plan_contract_is_runtime_owned() {
    let plan = RuntimeProviderAggregatePlan::new(
        "Account",
        Some("filter-ast"),
        Some("access-filter-ast"),
        vec!["group-by-industry"],
        vec!["metric-count"],
        Some("having-ast"),
        "sort-ast",
        RuntimePagination::new(0, 10).expect("valid pagination"),
        Some(json!({ "is_active": { "_eq": true } })),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
    );

    assert_eq!(plan.entity_type, "Account");
    assert_eq!(plan.filter, Some("filter-ast"));
    assert_eq!(plan.access_filter, Some("access-filter-ast"));
    assert_eq!(plan.group_by, vec!["group-by-industry"]);
    assert_eq!(plan.metrics, vec!["metric-count"]);
    assert_eq!(plan.having, Some("having-ast"));
    assert_eq!(plan.sort, "sort-ast");
    assert_eq!(plan.pagination.skip, 0);
    assert_eq!(plan.pagination.limit, 10);
    assert_eq!(
        plan.filter_json(),
        Some(json!({ "is_active": { "_eq": true } }))
    );
    assert_eq!(
        plan.access_filter_json(),
        Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
    );
}

#[derive(Clone)]
struct RuntimeOnlyProvider {
    contracts: &'static [RuntimeProviderOperationContract],
    observed_find_access: Arc<Mutex<Option<serde_json::Value>>>,
    observed_audit_queries: Arc<AtomicUsize>,
    find_visible: bool,
}

impl Default for RuntimeOnlyProvider {
    fn default() -> Self {
        Self {
            contracts: RuntimeProviderOperationContract::DATABASE_CLIENT,
            observed_find_access: Arc::new(Mutex::new(None)),
            observed_audit_queries: Arc::new(AtomicUsize::new(0)),
            find_visible: true,
        }
    }
}

impl RuntimeOnlyProvider {
    fn observed_find_access(&self) -> Option<serde_json::Value> {
        self.observed_find_access
            .lock()
            .expect("find access lock")
            .clone()
    }

    fn observed_audit_query_count(&self) -> usize {
        self.observed_audit_queries.load(Ordering::SeqCst)
    }
}

impl RuntimeProviderIdentity for RuntimeOnlyProvider {
    fn data_source_name(&self) -> &str {
        "crm_primary"
    }

    fn framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::Mssql
    }

    fn provider_operation_contracts(&self) -> &'static [RuntimeProviderOperationContract] {
        self.contracts
    }
}

#[async_trait::async_trait]
impl RuntimeProviderDataClient for RuntimeOnlyProvider {
    type Error = appfw_runtime::RuntimeError;
    type Entity = &'static str;
    type QueryPlan = &'static str;
    type AggregatePlan = &'static str;
    type MutationPlan = &'static str;

    async fn health_check(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn create_item_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        Ok(serde_json::Map::from_iter([(
            "operation".to_string(),
            json!("create"),
        )]))
    }

    async fn update_item_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        Ok(serde_json::Map::from_iter([(
            "operation".to_string(),
            json!("update"),
        )]))
    }

    async fn delete_item_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<i64, Self::Error> {
        Ok(1)
    }

    async fn get_items_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        Ok(vec![serde_json::Map::from_iter([(
            "id".to_string(),
            json!("account-1"),
        )])])
    }

    async fn query_items_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<RuntimeJsonQueryResult, Self::Error> {
        Ok(RuntimeJsonQueryResult::new(
            0,
            10,
            1,
            vec![serde_json::Map::from_iter([(
                "id".to_string(),
                json!("account-1"),
            )])],
            "2026-05-29T00:00:00Z",
            std::time::Instant::now(),
        ))
    }

    async fn batch_get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        self.get_items_plan_json(input).await
    }

    async fn aggregate_items_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, Self::AggregatePlan>,
    ) -> Result<RuntimeJsonAggregateResult, Self::Error> {
        Ok(RuntimeJsonAggregateResult::new(
            0,
            10,
            1,
            vec![json!({ "count": 1 })],
            "2026-05-29T00:00:00Z",
            std::time::Instant::now(),
        ))
    }

    async fn explain_query_plan(
        &self,
        _input: RuntimeProviderPlanInput<'_, &Self::QueryPlan>,
    ) -> Result<serde_json::Value, Self::Error> {
        Ok(json!({ "status": "ok" }))
    }

    async fn append_audit_event(
        &self,
        _event: record_audit::RuntimeAuditEvent,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn query_audit_events(
        &self,
        _query: record_audit::RuntimeAuditQuery,
    ) -> Result<Vec<serde_json::Value>, Self::Error> {
        self.observed_audit_queries.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    async fn create_item_json(
        &self,
        _entity_type: Self::Entity,
        _selections: serde_json::Value,
        input: RuntimeJsonObj,
        _user: &UserAuth,
        _access: &PolicyAccess,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        Ok(input)
    }

    async fn update_item_json(
        &self,
        _entity_type: Self::Entity,
        _selections: serde_json::Value,
        input: RuntimeJsonObj,
        _user: &UserAuth,
        _access: &PolicyAccess,
        _read_version: Option<serde_json::Value>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        Ok(input)
    }

    async fn delete_item_json(
        &self,
        _entity_type: Self::Entity,
        _input: RuntimeJsonObj,
        _user: &UserAuth,
        _access: &PolicyAccess,
        _read_version: Option<serde_json::Value>,
    ) -> Result<i64, Self::Error> {
        Ok(1)
    }

    async fn find_item_json(
        &self,
        _entity_type: Self::Entity,
        _selections: serde_json::Value,
        _id: String,
        _user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<RuntimeJsonObj>, Self::Error> {
        *self.observed_find_access.lock().expect("find access lock") = access.filter.clone();
        Ok(self
            .find_visible
            .then(|| serde_json::Map::from_iter([("id".to_string(), json!("account-1"))])))
    }

    async fn get_items_json(
        &self,
        _entity_type: Self::Entity,
        _selections: serde_json::Value,
        _filter: Option<serde_json::Value>,
        _user: &UserAuth,
        _access: &PolicyAccess,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        Ok(vec![serde_json::Map::from_iter([(
            "id".to_string(),
            json!("account-1"),
        )])])
    }

    async fn query_items_json(
        &self,
        _entity_type: Self::Entity,
        _selections: serde_json::Value,
        _filter: Option<serde_json::Value>,
        _sort: Option<serde_json::Value>,
        skip: i32,
        limit: i32,
        _user: &UserAuth,
        _access: &PolicyAccess,
    ) -> Result<RuntimeJsonQueryResult, Self::Error> {
        Ok(RuntimeJsonQueryResult::new(
            skip,
            limit,
            0,
            Vec::new(),
            "2026-05-29T00:00:00Z",
            std::time::Instant::now(),
        ))
    }
}

#[tokio::test]
async fn provider_data_client_contract_is_runtime_owned() {
    let provider = RuntimeOnlyProvider::default();
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: vec!["appfw:data.read".to_string()],
        token: "token".to_string(),
    };
    let access = PolicyAccess {
        allow: true,
        filter: None,
    };

    assert_eq!(
        provider.provider_descriptor(),
        RuntimeProviderDescriptor::new(FrameworkProvider::Mssql, "crm_primary")
    );
    provider.health_check().await.expect("health check");
    let query_result = provider
        .query_items_plan_json(RuntimeProviderPlanInput::new("query-plan", &user, &access))
        .await
        .expect("query result");
    assert_eq!(query_result.query_count, 1);
    assert_eq!(query_result.items[0]["id"], json!("account-1"));

    let aggregate_result = provider
        .aggregate_items_plan_json(RuntimeProviderPlanInput::new(
            "aggregate-plan",
            &user,
            &access,
        ))
        .await
        .expect("aggregate result");
    assert_eq!(aggregate_result.items, vec![json!({ "count": 1 })]);
}

#[tokio::test]
async fn data_access_provider_dispatch_enforces_declared_operations() {
    const QUERY_ONLY_CONTRACTS: &[RuntimeProviderOperationContract] =
        &[RuntimeProviderOperationContract::required(
            RuntimeProviderOperation::QueryItems,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        )];

    let provider = RuntimeOnlyProvider {
        contracts: QUERY_ONLY_CONTRACTS,
        ..RuntimeOnlyProvider::default()
    };
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: vec!["appfw:data.read".to_string()],
        token: "token".to_string(),
    };
    let access = PolicyAccess {
        allow: true,
        filter: None,
    };

    let query_result = appfw_runtime::data_access::provider_query_items_plan_json(
        &provider,
        RuntimeProviderPlanInput::new("query-plan", &user, &access),
    )
    .await
    .expect("declared query operation should dispatch");
    assert_eq!(query_result.query_count, 1);

    let err = appfw_runtime::data_access::provider_aggregate_items_plan_json(
        &provider,
        RuntimeProviderPlanInput::new("aggregate-plan", &user, &access),
    )
    .await
    .expect_err("undeclared aggregate operation should be rejected");

    assert_eq!(err.category(), "data_access");
    assert!(err.to_string().contains("aggregate_items"));
    assert!(err.to_string().contains("crm_primary"));
}

#[tokio::test]
async fn data_access_plan_read_orchestration_is_runtime_owned() {
    let provider = RuntimeOnlyProvider::default();
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: vec!["appfw:data.read".to_string()],
        token: "token".to_string(),
    };
    let access = PolicyAccess {
        allow: true,
        filter: None,
    };
    let pagination = RuntimePagination::keyset(None, 1).expect("keyset pagination");
    let sort = runtime_data_access::RuntimeReadSort {
        field: "id".to_string(),
        direction: RuntimeSortDirection::Asc,
    };
    let mut query_traced = None;

    let query_result = runtime_data_access::execute_query_items_plan_read(
        &provider,
        "query-plan",
        &pagination,
        Some(&sort),
        &user,
        &access,
        |operation, _started_at, counts| query_traced = Some((operation, counts)),
        |mut record| {
            record.insert("evaluated".to_string(), json!(true));
            Ok::<RuntimeJsonObj, appfw_runtime::RuntimeError>(record)
        },
    )
    .await
    .expect("query read should dispatch and finalize in runtime");

    assert_eq!(query_result.items[0]["evaluated"], json!(true));
    // Opaque cursor: decode to verify the signed payload value.
    let next = query_result.next_cursor.as_deref().expect("next cursor");
    assert_eq!(
        decode_keyset_cursor(next)
            .expect("decode next cursor")
            .value,
        json!("account-1")
    );
    assert_eq!(
        query_traced,
        Some((
            RuntimeProviderOperation::QueryItems,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );

    let mut aggregate_traced = None;
    let aggregate_result = runtime_data_access::execute_aggregate_items_plan_read(
        &provider,
        "aggregate-plan",
        &user,
        &access,
        |operation, _started_at, counts| aggregate_traced = Some((operation, counts)),
    )
    .await
    .expect("aggregate read should dispatch and trace in runtime");

    assert_eq!(aggregate_result.items, vec![json!({ "count": 1 })]);
    assert_eq!(
        aggregate_traced,
        Some((
            RuntimeProviderOperation::AggregateItems,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );
}

#[tokio::test]
async fn data_access_plan_mutation_orchestration_is_runtime_owned() {
    let provider = RuntimeOnlyProvider::default();
    let user = UserAuth {
        tenant_id: "tenant-1".to_string(),
        user_name: "casey".to_string(),
        timezone: "UTC".to_string(),
        principal_type: appfw_runtime::RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: Some("http".to_string()),
        roles: vec!["admin".to_string()],
        scopes: vec!["appfw:data.write".to_string()],
        token: "token".to_string(),
    };
    let access = PolicyAccess {
        allow: true,
        filter: None,
    };

    let mut create_traced = None;
    let created = runtime_data_access::execute_create_item_plan_mutation(
        &provider,
        "create-plan",
        &user,
        &access,
        |operation, _started_at, counts| create_traced = Some((operation, counts)),
    )
    .await
    .expect("create mutation should dispatch through runtime");
    assert_eq!(created["operation"], json!("create"));
    assert_eq!(
        create_traced,
        Some((
            RuntimeProviderOperation::CreateItem,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );

    let mut update_traced = None;
    let updated = runtime_data_access::execute_update_item_plan_mutation(
        &provider,
        "update-plan",
        &user,
        &access,
        |operation, _started_at, counts| update_traced = Some((operation, counts)),
    )
    .await
    .expect("update mutation should dispatch through runtime");
    assert_eq!(updated["operation"], json!("update"));
    assert_eq!(
        update_traced,
        Some((
            RuntimeProviderOperation::UpdateItem,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );

    let mut delete_traced = None;
    let deleted = runtime_data_access::execute_delete_item_plan_mutation(
        &provider,
        "delete-plan",
        &user,
        &access,
        |operation, _started_at, counts| delete_traced = Some((operation, counts)),
    )
    .await
    .expect("delete mutation should dispatch through runtime");
    assert_eq!(deleted, 1);
    assert_eq!(
        delete_traced,
        Some((
            RuntimeProviderOperation::DeleteItem,
            RuntimeProviderOperationCounts::new(1, 1)
        ))
    );
}

#[tokio::test]
async fn provider_registry_contract_is_runtime_owned() {
    let registry = RuntimeProviderRegistry::<String, appfw_runtime::RuntimeError>::new()
        .register(FrameworkProvider::Mssql, |data_source_name| async move {
            Ok(format!("mssql:{data_source_name}"))
        });

    assert!(registry.has_provider(FrameworkProvider::Mssql));
    assert_eq!(
        registry
            .create(FrameworkProvider::Mssql, "crm_primary")
            .await
            .expect("registered provider"),
        "mssql:crm_primary"
    );
}

#[test]
fn provider_error_contracts_are_runtime_owned() {
    assert_eq!(
        runtime_provider_error::classify_postgres_code("23502", Some("name"))
            .expect("postgres code")
            .to_string(),
        "required field: name"
    );
    assert_eq!(
        runtime_provider_error::classify_mssql_code(2627)
            .expect("mssql code")
            .to_string(),
        "duplicate key: record already exists"
    );
    assert_eq!(
        runtime_provider_error::classify_mongo_code(121)
            .expect("mongo code")
            .to_string(),
        "required field: unknown"
    );
    assert_eq!(
        runtime_provider_error::normalize_provider_error(
            FrameworkProvider::Snowflake,
            "Snowflake SQL API returned 422: foreign key constraint failed"
        )
        .to_string(),
        "foreign key: record references a missing related record"
    );

    let log_message = runtime_provider_error::provider_error_log_message(
        "connection failed password=hunter2 Authorization: Bearer jwt.value",
    );
    assert!(log_message.contains("[REDACTED]"));
    assert!(!log_message.contains("hunter2"));
    assert!(!log_message.contains("jwt.value"));
}

#[test]
fn provider_time_period_support_is_runtime_owned() {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 5, 29)
        .expect("date")
        .and_hms_opt(15, 45, 0)
        .expect("time");

    let (start, end) = provider_time_period::get_period_at("_this_wk", RuntimeDataType::Date, now)
        .expect("week period");
    assert_eq!(start, json!("2026-05-24"));
    assert_eq!(end, json!("2026-05-31"));

    let (start, end) =
        provider_time_period::get_period_at("_next_hr", RuntimeDataType::DateTime, now)
            .expect("hour period");
    assert_eq!(start, json!("2026-05-29T16:00:00+00:00"));
    assert_eq!(end, json!("2026-05-29T17:00:00+00:00"));
}
