use std::collections::BTreeSet;

use anyhow::{Context, Result};
use inflector::cases::camelcase::to_camel_case;
use serde::Serialize;

use crate::{
    app_workspace::AppWorkspace,
    normalized_config::{
        GeneratorIr, NormalizedEntity, NormalizedProperty, NormalizedRelation,
        NormalizedRelationship, NormalizedSchema,
    },
    utils::{
        artifacts::{self, Artifact, OverwriteMode},
        console,
    },
};

pub fn run(workspace: &AppWorkspace, ir: &GeneratorIr) -> Result<()> {
    let frontend_root = workspace.app_root.join("frontend");
    if !frontend_root.is_dir() {
        return Ok(());
    }

    console::stage("frontend");
    let contracts = ir
        .schemas
        .iter()
        .filter(|schema| !schema.is_system_schema)
        .map(build_contract)
        .collect::<Vec<_>>();

    let target = frontend_root.join("src/generated/appfw-ui-contract.ts");
    let content = render_contract_module(&contracts).context("render frontend UI contract")?;
    artifacts::emit(&Artifact::generated_text(
        target,
        content,
        OverwriteMode::Always,
    ))
    .context("emit frontend UI contract")?;

    let manifest_target = frontend_root.join(".appfw-ui/scaffold-manifest.json");
    let manifest =
        render_scaffold_manifest(&contracts).context("render frontend scaffold manifest")?;
    artifacts::emit(&Artifact::generated_text(
        manifest_target,
        manifest,
        OverwriteMode::Always,
    ))
    .context("emit frontend scaffold manifest")?;

    let workspace_target = frontend_root.join("src/generated/appfw-entity-workspace.tsx");
    let workspace = render_entity_workspace_module();
    artifacts::emit(&Artifact::generated_text(
        workspace_target,
        workspace,
        OverwriteMode::Always,
    ))
    .context("emit frontend entity workspace starter")?;

    console::done("frontend");
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiContract {
    version: u32,
    schema_name: String,
    source: &'static str,
    provider: UiProviderContract,
    design: UiDesignContract,
    auth: UiAuthContract,
    pagination: UiPaginationContract,
    filters: UiFilterContract,
    errors: UiErrorContract,
    entities: Vec<UiEntityContract>,
    relationships: Vec<UiRelationshipContract>,
    workflows: Vec<UiWorkflowContract>,
    view_registry: Vec<UiViewRegistryContract>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiProviderContract {
    data_source_name: String,
    data_source_type: String,
    capability_hints: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiDesignContract {
    token_prefix: &'static str,
    token_css_path: &'static str,
    density: &'static str,
    accessibility_baseline: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiAuthContract {
    required: bool,
    tenant_required: bool,
    local_dev_auth: &'static str,
    headers: Vec<UiHeaderContract>,
    policy_denied: UiErrorShapeContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiHeaderContract {
    name: &'static str,
    purpose: &'static str,
    required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiPaginationContract {
    default_page_size: u32,
    max_page_size: u32,
    cursor_parameter: &'static str,
    offset_parameters: Vec<&'static str>,
    result_fields: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiFilterContract {
    conjunctions: Vec<&'static str>,
    scalar_operators: Vec<&'static str>,
    enum_operators: Vec<&'static str>,
    relationship_operators: Vec<&'static str>,
    sort_spec: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiErrorContract {
    categories: Vec<&'static str>,
    validation: UiErrorShapeContract,
    policy_denied: UiErrorShapeContract,
    request_id_header: &'static str,
    correlation_id_header: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiErrorShapeContract {
    category: &'static str,
    message_field: &'static str,
    detail_field: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityContract {
    schema_name: String,
    type_name: String,
    route_segment: String,
    addressing: UiEntityAddressingContract,
    caption: UiCaption,
    primary_key: String,
    caption_field: String,
    audited: bool,
    read_only: bool,
    facets: Vec<String>,
    fields: Vec<UiFieldContract>,
    relationships: Vec<String>,
    operations: Vec<UiOperationContract>,
    scaffold: UiEntityScaffoldContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityAddressingContract {
    entity_uri: String,
    record_uri_template: String,
    list_route: String,
    route_template: String,
    filtered_list_route: String,
    route_id_kind: &'static str,
    id_kinds: Vec<UiEntityIdKindContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolve_operation: Option<UiEntityOperationRefContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locator_resolve_operation: Option<UiEntityOperationRefContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_operation: Option<UiEntityOperationRefContract>,
    batch_resolution: UiEntityBatchResolutionContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityIdKindContract {
    kind: &'static str,
    field: String,
    public: bool,
    route_safe: bool,
    answer_envelope: bool,
    filter_operators: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityOperationRefContract {
    name: String,
    graphql_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityBatchResolutionContract {
    field: &'static str,
    operator: &'static str,
    max_page_size: u32,
    filter_parameter: &'static str,
    fallback: &'static str,
}

#[derive(Debug, Serialize)]
struct UiCaption {
    singular: String,
    plural: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiFieldContract {
    name: String,
    label: String,
    kind: &'static str,
    data_type: String,
    required: bool,
    read_only: bool,
    is_key: bool,
    is_concurrency_control: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    enum_type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relationship_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relationship: Option<UiFieldRelationshipContract>,
    ui: UiFieldUiContract,
    validation: UiFieldValidationContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiFieldRelationshipContract {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    junction_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    junction_table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    local_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreign_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiFieldUiContract {
    list: bool,
    detail: bool,
    edit: bool,
    sortable: bool,
    filterable: bool,
    form_control: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiFieldValidationContract {
    required: bool,
    read_only: bool,
    concurrency_control: bool,
    client_hint: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiOperationContract {
    name: String,
    kind: &'static str,
    graphql_name: String,
    variables: Vec<UiOperationVariableContract>,
    returns: String,
    returns_shape: &'static str,
    selection_preset: Vec<String>,
    requires_auth: bool,
    requires_tenant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimistic_concurrency_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiOperationVariableContract {
    name: &'static str,
    data_type: &'static str,
    required: bool,
    description: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityScaffoldContract {
    list: UiEntityViewContract,
    detail: UiEntityViewContract,
    edit: UiEntityViewContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiEntityViewContract {
    route: String,
    fields: Vec<String>,
    states: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiRelationshipContract {
    name: String,
    kind: String,
    endpoints: Vec<UiRelationshipEndpointContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<UiRelationshipStorageContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    junction: Option<UiRelationshipJunctionContract>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiRelationshipEndpointContract {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_name: Option<String>,
    entity_name: String,
    field_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiRelationshipStorageContract {
    storage_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_schema: Option<String>,
    owner: String,
    field: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiRelationshipJunctionContract {
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_name: Option<String>,
    entity_name: String,
    left_key: String,
    right_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiWorkflowContract {
    id: &'static str,
    label: &'static str,
    feature_path: &'static str,
    primary_entity: &'static str,
    supporting_entities: Vec<&'static str>,
    proof_points: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiViewRegistryContract {
    view_id: String,
    route: String,
    kind: &'static str,
    supports: UiViewSupportContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiViewSupportContract {
    entity_types: Vec<String>,
    shapes: Vec<&'static str>,
    id_kinds: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_nodes: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiScaffoldManifest {
    version: u32,
    source: &'static str,
    package_kind: &'static str,
    generated_contract: &'static str,
    ownership_manifest: &'static str,
    design_system: UiScaffoldDesignSystem,
    schema_names: Vec<String>,
    contracts: Vec<UiScaffoldContractSummary>,
    generated_roots: Vec<&'static str>,
    scaffold_roots: Vec<&'static str>,
    human_owned_roots: Vec<&'static str>,
    required_package_scripts: Vec<&'static str>,
    verification: UiScaffoldVerification,
    release_evidence: Vec<&'static str>,
    notes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiScaffoldDesignSystem {
    brand: &'static str,
    source: &'static str,
    tokens: &'static str,
    components: &'static str,
    package: &'static str,
    style_import: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiScaffoldContractSummary {
    schema_name: String,
    provider: String,
    data_source_name: String,
    entity_count: usize,
    relationship_count: usize,
    workflow_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiScaffoldVerification {
    package_check: &'static str,
    typecheck: &'static str,
    build: &'static str,
    backend_validation: &'static str,
    generated_drift: &'static str,
}

fn build_contract(schema: &NormalizedSchema) -> UiContract {
    let entities = schema
        .entities
        .iter()
        .filter(|entity| entity.is_table && !entity.is_union)
        .map(build_entity)
        .collect::<Vec<_>>();
    let workflows = workflow_defaults(schema);
    let view_registry = view_registry_defaults(schema, &workflows);

    UiContract {
        version: 1,
        schema_name: schema.name.clone(),
        source: "app_gen",
        provider: UiProviderContract {
            data_source_name: schema.data_source_name.clone(),
            data_source_type: schema.data_source_type.clone(),
            capability_hints: vec![
                "server_policy_enforced",
                "server_validation_enforced",
                "query_cost_limits",
                "cursor_or_offset_pagination",
            ],
        },
        design: UiDesignContract {
            token_prefix: "--pds-",
            token_css_path: "@appfw/pds-health/tokens/pdsTokens.css",
            density: "compact",
            accessibility_baseline: vec![
                "keyboard_navigation",
                "visible_focus",
                "semantic_tables",
                "responsive_text_fit",
            ],
        },
        auth: auth_contract(),
        pagination: pagination_contract(),
        filters: filter_contract(),
        errors: error_contract(),
        relationships: schema
            .relationships
            .iter()
            .map(build_relationship)
            .collect(),
        workflows,
        view_registry,
        entities,
    }
}

fn auth_contract() -> UiAuthContract {
    UiAuthContract {
        required: true,
        tenant_required: true,
        local_dev_auth: "Session storage bearer token for local exploration; production auth is backend-governed.",
        headers: vec![
            UiHeaderContract {
                name: "authorization",
                purpose: "Bearer token forwarded to the generated API",
                required: true,
            },
            UiHeaderContract {
                name: "x-tenant-id",
                purpose: "Tenant context forwarded to policy enforcement",
                required: true,
            },
            UiHeaderContract {
                name: "x-request-id",
                purpose: "Per-request trace identifier",
                required: true,
            },
            UiHeaderContract {
                name: "x-correlation-id",
                purpose: "Cross-service trace identifier",
                required: true,
            },
            UiHeaderContract {
                name: "x-timezone",
                purpose: "Browser timezone for date/time workflows",
                required: true,
            },
        ],
        policy_denied: UiErrorShapeContract {
            category: "policy_denied",
            message_field: "message",
            detail_field: "extensions",
        },
    }
}

fn pagination_contract() -> UiPaginationContract {
    UiPaginationContract {
        default_page_size: 25,
        max_page_size: 100,
        cursor_parameter: "after",
        offset_parameters: vec!["skip", "limit"],
        result_fields: vec![
            "items",
            "skip",
            "limit",
            "page_count",
            "page_index",
            "query_count",
            "next_cursor",
            "previous_cursor",
        ],
    }
}

fn filter_contract() -> UiFilterContract {
    UiFilterContract {
        conjunctions: vec!["and", "or"],
        scalar_operators: vec![
            "eq",
            "ne",
            "in",
            "contains",
            "starts_with",
            "gt",
            "gte",
            "lt",
            "lte",
        ],
        enum_operators: vec!["eq", "ne", "in"],
        relationship_operators: vec!["eq", "in", "exists"],
        sort_spec: "JSON sort array/object accepted by generated query operations",
    }
}

fn error_contract() -> UiErrorContract {
    UiErrorContract {
        categories: vec![
            "validation",
            "policy_denied",
            "auth",
            "provider",
            "network",
            "unknown",
        ],
        validation: UiErrorShapeContract {
            category: "validation",
            message_field: "message",
            detail_field: "extensions.validation",
        },
        policy_denied: UiErrorShapeContract {
            category: "policy_denied",
            message_field: "message",
            detail_field: "extensions",
        },
        request_id_header: "x-request-id",
        correlation_id_header: "x-correlation-id",
    }
}

fn build_entity(entity: &NormalizedEntity) -> UiEntityContract {
    let fields = entity
        .native_properties
        .iter()
        .chain(entity.relationship_properties.iter())
        .map(build_field)
        .collect::<Vec<_>>();

    let route_segment = route_segment(entity);
    let edit_fields = edit_fields(entity);
    let operations = operations(entity);

    UiEntityContract {
        schema_name: entity.schema_name.clone(),
        type_name: entity.name.clone(),
        route_segment: route_segment.clone(),
        addressing: entity_addressing(entity, &route_segment),
        caption: UiCaption {
            singular: entity.caption_singular.clone(),
            plural: entity.caption_plural.clone(),
        },
        primary_key: entity.primary_key.clone().unwrap_or_default(),
        caption_field: caption_field(entity),
        audited: entity.audit_table.is_some(),
        read_only: edit_fields.is_empty(),
        facets: entity.facets.clone(),
        relationships: entity
            .relationship_properties
            .iter()
            .map(|prop| prop.name.clone())
            .collect(),
        operations,
        scaffold: UiEntityScaffoldContract {
            list: UiEntityViewContract {
                route: format!("/data/{route_segment}"),
                fields: list_fields(entity),
                states: standard_states(),
                disabled_reason: None,
            },
            detail: UiEntityViewContract {
                route: format!("/data/{route_segment}/:recordLocator"),
                fields: detail_fields(entity),
                states: standard_states(),
                disabled_reason: None,
            },
            edit: UiEntityViewContract {
                route: format!("/data/{route_segment}/:recordLocator/edit"),
                fields: edit_fields,
                states: edit_states(),
                disabled_reason: (!has_standard_method(entity, "Update"))
                    .then_some("No generated update operation is available for this entity."),
            },
        },
        fields,
    }
}

fn entity_addressing(entity: &NormalizedEntity, route_segment: &str) -> UiEntityAddressingContract {
    UiEntityAddressingContract {
        entity_uri: format!("appfw://entity/{}/{}", entity.schema_name, entity.name),
        record_uri_template: format!(
            "appfw://entity/{}/{}/{{recordLocator}}",
            entity.schema_name, entity.name
        ),
        list_route: format!("/data/{route_segment}"),
        route_template: format!("/data/{route_segment}/:recordLocator"),
        filtered_list_route: format!("/data/{route_segment}?filter=:filterJson"),
        route_id_kind: "record_locator",
        id_kinds: entity_id_kinds(entity),
        resolve_operation: operation_ref(entity, "FindById"),
        locator_resolve_operation: locator_operation_ref(entity),
        query_operation: operation_ref(entity, "Query"),
        batch_resolution: UiEntityBatchResolutionContract {
            field: "record_locator",
            operator: "in",
            max_page_size: 100,
            filter_parameter: "filter",
            fallback: "by_locator_loop",
        },
    }
}

fn entity_id_kinds(entity: &NormalizedEntity) -> Vec<UiEntityIdKindContract> {
    let mut id_kinds = vec![UiEntityIdKindContract {
        kind: "record_locator",
        field: "record_locator".to_string(),
        public: true,
        route_safe: true,
        answer_envelope: true,
        filter_operators: vec!["eq", "in"],
    }];

    if let Some(primary_key) = &entity.primary_key {
        id_kinds.push(UiEntityIdKindContract {
            kind: "primary_key",
            field: primary_key.clone(),
            public: false,
            route_safe: false,
            answer_envelope: false,
            filter_operators: vec![],
        });
    }

    id_kinds
}

fn locator_operation_ref(entity: &NormalizedEntity) -> Option<UiEntityOperationRefContract> {
    let resolve = operation_ref(entity, "FindById")?;
    Some(UiEntityOperationRefContract {
        name: format!("{}_by_locator", resolve.name),
        graphql_name: format!("{}ByLocator", resolve.graphql_name),
    })
}

fn operation_ref(
    entity: &NormalizedEntity,
    standard_method: &str,
) -> Option<UiEntityOperationRefContract> {
    if !has_standard_method(entity, standard_method) {
        return None;
    }
    let name = match standard_method {
        "FindById" => format!("find_{}", entity.snake_singular),
        "Query" => format!("query_{}", entity.table_name),
        _ => return None,
    };
    Some(UiEntityOperationRefContract {
        graphql_name: to_camel_case(&name),
        name,
    })
}

fn route_segment(entity: &NormalizedEntity) -> String {
    match (entity.schema_name.as_str(), entity.name.as_str()) {
        ("crm", "Opportunity") => "pipeline".to_string(),
        _ => entity.table_name.clone(),
    }
}

fn build_field(prop: &NormalizedProperty) -> UiFieldContract {
    UiFieldContract {
        name: prop.name.clone(),
        label: if prop.caption.is_empty() {
            prop.name.clone()
        } else {
            prop.caption.clone()
        },
        kind: field_kind(prop),
        data_type: prop.data_type.clone(),
        required: prop.is_required,
        read_only: prop.is_read_only,
        is_key: prop.is_key,
        is_concurrency_control: prop.is_concurrency_control,
        enum_type_name: prop.enum_type_name.clone(),
        relationship_target: prop.relation.as_ref().and_then(relationship_target),
        relationship: prop.relation.as_ref().map(field_relationship),
        ui: UiFieldUiContract {
            list: list_field(prop),
            detail: detail_field(prop),
            edit: edit_field(prop),
            sortable: sortable_field(prop),
            filterable: filterable_field(prop),
            form_control: form_control(prop),
            format: format_hint(prop),
        },
        validation: UiFieldValidationContract {
            required: prop.is_required,
            read_only: prop.is_read_only,
            concurrency_control: prop.is_concurrency_control,
            client_hint: validation_hint(prop),
        },
    }
}

fn field_kind(prop: &NormalizedProperty) -> &'static str {
    if prop.relation.is_some() {
        "relationship"
    } else if prop.data_type == "Enum"
        || prop.data_type == "EnumArray"
        || prop.enum_type_name.is_some()
    {
        "enum"
    } else {
        "scalar"
    }
}

fn field_relationship(relation: &NormalizedRelation) -> UiFieldRelationshipContract {
    UiFieldRelationshipContract {
        kind: relation.kind,
        schema_name: relation
            .resolved_schema_name
            .clone()
            .or_else(|| relation.schema_name.clone()),
        type_name: relationship_target(relation),
        field_name: relation.prop_name.clone(),
        junction_schema: relation.junction_schema.clone(),
        junction_table: relation.junction_table.clone(),
        local_key: relation.local_key.clone(),
        foreign_key: relation.foreign_key.clone(),
    }
}

fn relationship_target(relation: &NormalizedRelation) -> Option<String> {
    relation
        .resolved_type_name
        .clone()
        .or_else(|| relation.type_name.clone())
}

fn list_field(prop: &NormalizedProperty) -> bool {
    !prop.is_concurrency_control
        && prop.relation.is_none()
        && !matches!(
            prop.data_type.as_str(),
            "Object" | "ObjectArray" | "Json" | "JsonArray"
        )
}

fn detail_field(prop: &NormalizedProperty) -> bool {
    prop.relation.is_none()
}

fn edit_field(prop: &NormalizedProperty) -> bool {
    prop.relation.is_none()
        && !prop.is_key
        && !prop.is_read_only
        && !prop.is_concurrency_control
        && !matches!(
            prop.data_type.as_str(),
            "NavToOne" | "NavToMany" | "Object" | "ObjectArray" | "JsonArray"
        )
}

fn sortable_field(prop: &NormalizedProperty) -> bool {
    prop.relation.is_none()
        && !matches!(
            prop.data_type.as_str(),
            "StringArray" | "EnumArray" | "Object" | "ObjectArray" | "Json" | "JsonArray"
        )
}

fn filterable_field(prop: &NormalizedProperty) -> bool {
    prop.relation.is_none()
        && !matches!(
            prop.data_type.as_str(),
            "Object" | "ObjectArray" | "JsonArray" | "NavToOne" | "NavToMany"
        )
}

fn form_control(prop: &NormalizedProperty) -> &'static str {
    if prop.is_read_only || prop.is_key || prop.is_concurrency_control {
        return "readonly";
    }
    if prop.relation.is_some() {
        return "relationship-picker";
    }
    if prop.enum_type_name.is_some() || prop.data_type == "Enum" {
        return "select";
    }
    match prop.data_type.as_str() {
        "Boolean" => "toggle",
        "Date" => "date",
        "DateTime" => "datetime",
        "Time" => "time",
        "StringArray" | "EnumArray" => "tag-list",
        "Int8" | "Int16" | "Int32" | "Int64" | "Float32" | "Float64" => "number",
        "Json" | "Object" => "code",
        _ => "text",
    }
}

fn format_hint(prop: &NormalizedProperty) -> Option<&'static str> {
    let name = prop.name.as_str();
    if name.contains("email") {
        Some("email")
    } else if name.contains("phone") {
        Some("phone")
    } else if name.contains("url") || name.contains("website") {
        Some("url")
    } else if name.contains("amount") || name.contains("revenue") || name.contains("price") {
        Some("currency")
    } else if prop.data_type == "DateTime" {
        Some("datetime")
    } else if prop.data_type == "Date" {
        Some("date")
    } else {
        None
    }
}

fn validation_hint(prop: &NormalizedProperty) -> &'static str {
    if prop.is_key {
        "Generated primary key; display only."
    } else if prop.is_concurrency_control {
        "Optimistic concurrency token; include on updates."
    } else if prop.is_read_only {
        "Backend-owned value; display only."
    } else if prop.is_required {
        "Required by the generated model."
    } else {
        "Optional field."
    }
}

fn caption_field(entity: &NormalizedEntity) -> String {
    entity
        .native_properties
        .iter()
        .find(|prop| prop.is_caption)
        .or_else(|| {
            entity
                .native_properties
                .iter()
                .find(|prop| prop.data_type == "String" && !prop.is_key)
        })
        .or_else(|| {
            entity
                .native_properties
                .iter()
                .find(|prop| entity.primary_key.as_deref() == Some(prop.name.as_str()))
        })
        .or_else(|| entity.native_properties.first())
        .map(|prop| prop.name.clone())
        .unwrap_or_default()
}

fn list_fields(entity: &NormalizedEntity) -> Vec<String> {
    dedupe(
        std::iter::once(caption_field(entity))
            .chain(entity.primary_key.clone())
            .chain(
                entity
                    .native_properties
                    .iter()
                    .filter(|prop| list_field(prop))
                    .map(|prop| prop.name.clone()),
            )
            .collect(),
    )
}

fn detail_fields(entity: &NormalizedEntity) -> Vec<String> {
    dedupe(
        std::iter::once(caption_field(entity))
            .chain(entity.primary_key.clone())
            .chain(
                entity
                    .native_properties
                    .iter()
                    .filter(|prop| detail_field(prop))
                    .map(|prop| prop.name.clone()),
            )
            .collect(),
    )
}

fn edit_fields(entity: &NormalizedEntity) -> Vec<String> {
    entity
        .native_properties
        .iter()
        .filter(|prop| edit_field(prop))
        .map(|prop| prop.name.clone())
        .collect()
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    for value in values {
        if !value.is_empty() && seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn operations(entity: &NormalizedEntity) -> Vec<UiOperationContract> {
    let mut operations = vec![];

    if has_standard_method(entity, "Query") {
        operations.push(operation(
            entity,
            format!("query_{}", entity.table_name),
            "query",
            vec![
                variable(
                    "filter",
                    "JsonValue",
                    false,
                    "Provider-normalized filter JSON",
                ),
                variable("sort", "JsonValue", false, "Provider-normalized sort JSON"),
                variable("skip", "Int", false, "Offset pagination start"),
                variable("limit", "Int", false, "Requested page size"),
                variable(
                    "after",
                    "String",
                    false,
                    "Cursor returned by the previous page",
                ),
            ],
            format!("{}Connection", entity.name),
            "connection",
            list_fields(entity),
            None,
        ));
    }

    if has_standard_method(entity, "FindById") {
        operations.push(operation(
            entity,
            format!("find_{}", entity.snake_singular),
            "query",
            vec![variable("id", "ID", true, "Primary key value")],
            entity.name.clone(),
            "record",
            detail_fields(entity),
            None,
        ));
    }

    if has_standard_method(entity, "GetAll") {
        operations.push(operation(
            entity,
            format!("get_{}", entity.table_name),
            "query",
            vec![],
            format!("{}List", entity.name),
            "list",
            list_fields(entity),
            None,
        ));
    }

    if has_standard_method(entity, "Query") {
        operations.push(operation(
            entity,
            format!("aggregate_{}", entity.table_name),
            "aggregate",
            vec![
                variable(
                    "filter",
                    "JsonValue",
                    false,
                    "Provider-normalized filter JSON",
                ),
                variable("groupBy", "JsonValue", false, "Aggregate grouping fields"),
                variable("metrics", "JsonValue", false, "Aggregate metric selections"),
                variable("having", "JsonValue", false, "Aggregate post-filter JSON"),
                variable("sort", "JsonValue", false, "Aggregate sort JSON"),
                variable("skip", "Int", false, "Offset pagination start"),
                variable("limit", "Int", false, "Requested page size"),
            ],
            format!("{}Aggregate", entity.name),
            "aggregate",
            vec![],
            None,
        ));
    }

    if has_standard_method(entity, "Create") {
        operations.push(operation(
            entity,
            format!("create_{}", entity.snake_singular),
            "mutation",
            vec![variable("input", "Input", true, "Generated create input")],
            entity.name.clone(),
            "record",
            detail_fields(entity),
            None,
        ));
    }

    if has_standard_method(entity, "Update") {
        operations.push(operation(
            entity,
            format!("update_{}", entity.snake_singular),
            "mutation",
            vec![variable("input", "Input", true, "Generated update input")],
            entity.name.clone(),
            "record",
            detail_fields(entity),
            None,
        ));
    }

    if has_standard_method(entity, "Delete") {
        operations.push(operation(
            entity,
            format!("delete_{}", entity.snake_singular),
            "mutation",
            vec![variable("input", "Input", true, "Generated delete input")],
            "Int64".to_string(),
            "scalar",
            vec![],
            Some("Generated delete actions require product confirmation UX before enabling."),
        ));
    }

    operations
}

fn operation(
    entity: &NormalizedEntity,
    name: String,
    kind: &'static str,
    variables: Vec<UiOperationVariableContract>,
    returns: String,
    returns_shape: &'static str,
    selection_preset: Vec<String>,
    disabled_reason: Option<&'static str>,
) -> UiOperationContract {
    UiOperationContract {
        graphql_name: to_camel_case(&name),
        name,
        kind,
        variables,
        returns,
        returns_shape,
        selection_preset,
        requires_auth: true,
        requires_tenant: true,
        optimistic_concurrency_field: entity
            .native_properties
            .iter()
            .find(|prop| prop.is_concurrency_control)
            .map(|prop| prop.name.clone()),
        disabled_reason,
    }
}

fn variable(
    name: &'static str,
    data_type: &'static str,
    required: bool,
    description: &'static str,
) -> UiOperationVariableContract {
    UiOperationVariableContract {
        name,
        data_type,
        required,
        description,
    }
}

fn has_standard_method(entity: &NormalizedEntity, method: &str) -> bool {
    entity
        .standard_methods
        .iter()
        .any(|candidate| candidate == method)
}

fn standard_states() -> Vec<&'static str> {
    vec!["loading", "empty", "error", "policy_denied"]
}

fn edit_states() -> Vec<&'static str> {
    vec!["loading", "validation", "saving", "error", "policy_denied"]
}

fn build_relationship(relationship: &NormalizedRelationship) -> UiRelationshipContract {
    UiRelationshipContract {
        name: relationship.name.clone(),
        kind: relationship.kind.clone(),
        endpoints: relationship_endpoints(relationship),
        storage: relationship
            .storage
            .as_ref()
            .map(|storage| UiRelationshipStorageContract {
                storage_type: storage.storage_type.clone(),
                owner_schema: storage.owner_schema.clone(),
                owner: storage.owner.clone(),
                field: storage.field.clone(),
            }),
        junction: relationship
            .junction
            .as_ref()
            .map(|junction| UiRelationshipJunctionContract {
                schema_name: junction.schema_name.clone(),
                entity_name: junction.entity_name.clone(),
                left_key: junction.left_key.clone(),
                right_key: junction.right_key.clone(),
            }),
    }
}

fn relationship_endpoints(
    relationship: &NormalizedRelationship,
) -> Vec<UiRelationshipEndpointContract> {
    let mut endpoints = vec![];
    if let Some(endpoint) = &relationship.left {
        endpoints.push(relationship_endpoint("left", endpoint));
    }
    if let Some(endpoint) = &relationship.right {
        endpoints.push(relationship_endpoint("right", endpoint));
    }
    if let Some(endpoint) = &relationship.one {
        endpoints.push(relationship_endpoint("one", endpoint));
    }
    if let Some(endpoint) = &relationship.many {
        endpoints.push(relationship_endpoint("many", endpoint));
    }
    endpoints
}

fn relationship_endpoint(
    role: &'static str,
    endpoint: &crate::normalized_config::NormalizedRelationshipEndpoint,
) -> UiRelationshipEndpointContract {
    UiRelationshipEndpointContract {
        role,
        schema_name: endpoint.schema_name.clone(),
        entity_name: endpoint.entity_name.clone(),
        field_name: endpoint.field_name.clone(),
        caption: endpoint.caption.clone(),
    }
}

fn workflow_defaults(schema: &NormalizedSchema) -> Vec<UiWorkflowContract> {
    if schema.name != "crm" {
        return vec![];
    }

    let entity_names = schema
        .entities
        .iter()
        .map(|entity| entity.name.as_str())
        .collect::<BTreeSet<_>>();

    let mut workflows = vec![];

    if entity_names.contains("Account") {
        workflows.push(UiWorkflowContract {
            id: "accounts",
            label: "Accounts",
            feature_path: "src/features/accounts",
            primary_entity: "Account",
            supporting_entities: existing_entities(
                &entity_names,
                &["Contact", "Address", "Industry", "Activity"],
            ),
            proof_points: vec![
                "relationship_navigation",
                "policy_denied_state",
                "tenant_filtering",
                "field_validation",
            ],
        });
    }

    if entity_names.contains("Opportunity") {
        workflows.push(UiWorkflowContract {
            id: "pipeline",
            label: "Pipeline",
            feature_path: "src/features/pipeline",
            primary_entity: "Opportunity",
            supporting_entities: existing_entities(
                &entity_names,
                &["Account", "OpportunityStage", "Quote"],
            ),
            proof_points: vec![
                "sortable_list",
                "aggregate_summary",
                "pagination",
                "empty_loading_error_states",
            ],
        });
    }

    if entity_names.contains("Activity") {
        workflows.push(UiWorkflowContract {
            id: "activities",
            label: "Activities",
            feature_path: "src/features/activities",
            primary_entity: "Activity",
            supporting_entities: existing_entities(
                &entity_names,
                &["Account", "Contact", "Lead", "Opportunity"],
            ),
            proof_points: vec![
                "date_time_handling",
                "ownership_context",
                "optimistic_refresh",
                "task_status",
            ],
        });
    }

    if entity_names.contains("Account") {
        workflows.push(UiWorkflowContract {
            id: "audit",
            label: "Audit",
            feature_path: "src/features/audit",
            primary_entity: "Account",
            supporting_entities: existing_entities(&entity_names, &["Activity", "Opportunity"]),
            proof_points: vec![
                "read_only_timeline",
                "request_correlation_ids",
                "policy_context",
                "redacted_diagnostics",
            ],
        });
    }

    workflows
}

fn view_registry_defaults(
    schema: &NormalizedSchema,
    workflows: &[UiWorkflowContract],
) -> Vec<UiViewRegistryContract> {
    let mut views = vec![];

    for entity in schema
        .entities
        .iter()
        .filter(|entity| entity.is_table && !entity.is_union)
    {
        let route_segment = route_segment(entity);
        views.push(UiViewRegistryContract {
            view_id: format!("entity:{}:list", route_segment),
            route: format!("/data/{route_segment}"),
            kind: "entity_list",
            supports: UiViewSupportContract {
                entity_types: vec![entity.name.clone()],
                shapes: vec!["entity_ref", "filtered_list"],
                id_kinds: vec!["record_locator"],
                max_nodes: None,
            },
        });
        views.push(UiViewRegistryContract {
            view_id: format!("entity:{}:detail", route_segment),
            route: format!("/data/{route_segment}/:recordLocator"),
            kind: "entity_detail",
            supports: UiViewSupportContract {
                entity_types: vec![entity.name.clone()],
                shapes: vec!["entity_ref", "record_detail"],
                id_kinds: vec!["record_locator"],
                max_nodes: None,
            },
        });
    }

    for workflow in workflows {
        let mut entity_types = vec![workflow.primary_entity.to_string()];
        for entity_name in &workflow.supporting_entities {
            if !entity_types.iter().any(|existing| existing == entity_name) {
                entity_types.push((*entity_name).to_string());
            }
        }
        views.push(UiViewRegistryContract {
            view_id: format!("workflow:{}", workflow.id),
            route: format!("/{}", workflow.id),
            kind: "workflow",
            supports: UiViewSupportContract {
                entity_types,
                shapes: vec!["entity_ref", "filtered_list", "flow_graph"],
                id_kinds: vec!["record_locator"],
                max_nodes: Some(1000),
            },
        });
    }

    views
}

fn existing_entities(
    entity_names: &BTreeSet<&str>,
    candidates: &[&'static str],
) -> Vec<&'static str> {
    candidates
        .iter()
        .copied()
        .filter(|entity_name| entity_names.contains(entity_name))
        .collect()
}

fn build_scaffold_manifest(contracts: &[UiContract]) -> UiScaffoldManifest {
    UiScaffoldManifest {
        version: 1,
        source: "app_gen",
        package_kind: "product_frontend_scaffold",
        generated_contract: "src/generated/appfw-ui-contract.ts",
        ownership_manifest: ".appfw-ui/ownership.json",
        design_system: UiScaffoldDesignSystem {
            brand: "PDS Health",
            source: "appfw_ui/pds_health",
            tokens: "appfw_ui/pds_health/tokens",
            components: "appfw_ui/pds_health/components",
            package: "@appfw/pds-health-components",
            style_import: "@appfw/pds-health-components/styles.css",
        },
        schema_names: contracts
            .iter()
            .map(|contract| contract.schema_name.clone())
            .collect(),
        contracts: contracts
            .iter()
            .map(|contract| UiScaffoldContractSummary {
                schema_name: contract.schema_name.clone(),
                provider: contract.provider.data_source_type.clone(),
                data_source_name: contract.provider.data_source_name.clone(),
                entity_count: contract.entities.len(),
                relationship_count: contract.relationships.len(),
                workflow_count: contract.workflows.len(),
            })
            .collect(),
        generated_roots: vec!["src/generated/**", ".appfw-ui/scaffold-manifest.json"],
        scaffold_roots: vec![
            "src/app/**",
            "src/components/**",
            "src/scaffold/**",
            "src/lib/**",
            "src/styles/**",
            "scripts/check-scaffold.mjs",
        ],
        human_owned_roots: vec!["src/features/**"],
        required_package_scripts: vec!["appfw:check", "typecheck", "build"],
        verification: UiScaffoldVerification {
            package_check: "npm run appfw:check",
            typecheck: "npm run typecheck",
            build: "npm run build",
            backend_validation: "scripts/appfw validate --json",
            generated_drift: "scripts/appfw generate --check --json",
        },
        release_evidence: vec![
            "frontend scaffold manifest check",
            "generated contract drift check",
            "TypeScript typecheck",
            "production frontend build emitted to backend/product_dist",
            "backend validation",
            "handoff JSON",
        ],
        notes: vec![
            "The scaffold manifest is deterministic and regenerated from app_gen.",
            "Product teams customize src/features/**; scaffold upgrades own reusable package surfaces.",
            "The package check is intentionally offline and does not require a running backend.",
        ],
    }
}

fn render_scaffold_manifest(contracts: &[UiContract]) -> Result<String> {
    let manifest = build_scaffold_manifest(contracts);
    Ok(format!("{}\n", serde_json::to_string_pretty(&manifest)?))
}

// Product-neutral, contract-driven entity-workspace starter. This module is the
// same for every product (it is driven entirely by the generated UI contract),
// so it is emitted as a fixed module rather than per-entity codegen. It gives a
// generated product a working list workspace (header + toolbar + data grid +
// pagination + feedback states) wired to the shared PDS components on day one;
// products own data fetching and pass rows/handlers in.
fn render_entity_workspace_module() -> String {
    String::from(
        r##"// Generated by app_gen. Do not hand-edit.
//
// Contract-driven entity-workspace starter wired to the framework-owned PDS
// component library. Products own data fetching, routing, and mutations; they
// pass rows and handlers into these presentational components. The design
// system owns layout, density, accessibility, and feedback presentation.
import type { ReactNode } from "react";
import {
  Button,
  DataGridPagination,
  DataGridShell,
  DataGridToolbar,
  FeedbackState,
  PageHeader,
  Surface,
  type PdsDataGridColumn
} from "@appfw/pds-health-components";
import {
  appfwUiContracts,
  type AppfwUiEntityContract
} from "./appfw-ui-contract";

export type EntityWorkspaceRow = Record<string, unknown>;

export type EntityWorkspaceState = "ready" | "loading" | "empty" | "error" | "policy_denied";

export type EntityWorkspacePage = {
  index: number;
  size: number;
  total?: number | null;
  pageCount?: number | null;
};

export type EntityWorkspaceProps = {
  entity: AppfwUiEntityContract;
  rows: readonly EntityWorkspaceRow[];
  state?: EntityWorkspaceState;
  page?: EntityWorkspacePage;
  search?: ReactNode;
  actions?: ReactNode;
  errorTitle?: string;
  errorDetail?: ReactNode;
  requestId?: ReactNode;
  correlationId?: ReactNode;
  onRetry?: () => void;
  onRowSelect?: (row: EntityWorkspaceRow) => void;
  onFirstPage?: () => void;
  onPreviousPage?: () => void;
  onNextPage?: () => void;
  onLastPage?: () => void;
};

const NUMERIC_DATA_TYPES = new Set(["Int32", "Int64", "Float64", "Decimal", "Number"]);

// Locate a generated entity contract by schema and route segment (or type name).
export function findEntityContract(
  schemaName: string,
  routeOrType: string
): AppfwUiEntityContract | undefined {
  const contract = appfwUiContracts.find((item) => item.schemaName === schemaName);
  return contract?.entities.find(
    (entity) => entity.routeSegment === routeOrType || entity.typeName === routeOrType
  );
}

// Derive list-grid columns from the contract's list-view field set.
export function entityListColumns(
  entity: AppfwUiEntityContract
): PdsDataGridColumn<EntityWorkspaceRow>[] {
  const labels = new Map(entity.fields.map((field) => [field.name, field.label]));
  const dataTypes = new Map(entity.fields.map((field) => [field.name, field.dataType]));
  return entity.scaffold.list.fields.map((name) => ({
    key: name,
    header: labels.get(name) ?? name,
    align: NUMERIC_DATA_TYPES.has(dataTypes.get(name) ?? "") ? ("end" as const) : undefined
  }));
}

function countLabel(
  entity: AppfwUiEntityContract,
  rows: readonly EntityWorkspaceRow[],
  page?: EntityWorkspacePage
): string {
  const total = page?.total ?? rows.length;
  const noun = total === 1 ? entity.caption.singular : entity.caption.plural;
  return `${total} ${noun}`;
}

// A presentational list workspace for a generated entity.
export function EntityWorkspace({
  entity,
  rows,
  state = "ready",
  page,
  search,
  actions,
  errorTitle = "Request failed",
  errorDetail,
  requestId,
  correlationId,
  onRetry,
  onRowSelect,
  onFirstPage,
  onPreviousPage,
  onNextPage,
  onLastPage
}: EntityWorkspaceProps) {
  const columns = entityListColumns(entity);
  const metadata = requestId || correlationId ? { requestId, correlationId } : undefined;
  const summary = countLabel(entity, rows, page);

  let body: ReactNode;
  if (state === "error" || state === "policy_denied") {
    body = (
      <FeedbackState
        kind={state === "policy_denied" ? "denied" : "error"}
        title={state === "policy_denied" ? "Access is restricted" : errorTitle}
        detail={errorDetail}
        metadata={metadata}
        action={
          state === "error" && onRetry ? (
            <Button variant="secondary" onClick={onRetry}>
              Try again
            </Button>
          ) : undefined
        }
      />
    );
  } else if (state === "empty" || (state === "ready" && rows.length === 0)) {
    body = (
      <FeedbackState
        kind="empty"
        title={`No ${entity.caption.plural.toLowerCase()} found`}
        detail="Adjust filters or create a record to get started."
      />
    );
  } else {
    body = (
      <DataGridShell
        ariaLabel={entity.caption.plural}
        columns={columns}
        rows={rows}
        rowKey={entity.primaryKey}
        isLoading={state === "loading"}
        onRowSelect={onRowSelect}
      />
    );
  }

  return (
    <section className="appfw-entity-workspace">
      <PageHeader
        eyebrow={entity.schemaName}
        title={entity.caption.plural}
        subtitle={summary}
        actions={actions}
      />
      <Surface>
        <DataGridToolbar
          ariaLabel={`${entity.caption.plural} controls`}
          search={search}
          summary={summary}
          actions={actions}
        />
        {body}
        {page ? (
          <DataGridPagination
            pageIndex={page.index}
            pageSize={page.size}
            startRow={rows.length ? (page.index - 1) * page.size + 1 : 0}
            endRow={(page.index - 1) * page.size + rows.length}
            totalRows={page.total ?? null}
            pageCount={page.pageCount ?? null}
            onFirstPage={onFirstPage}
            onPreviousPage={onPreviousPage}
            onNextPage={onNextPage}
            onLastPage={onLastPage}
          />
        ) : null}
      </Surface>
    </section>
  );
}
"##,
    )
}

fn render_contract_module(contracts: &[UiContract]) -> Result<String> {
    let mut out = String::from(
        r#"// Generated by app_gen. Do not hand-edit.

export type AppfwUiFieldKind = "scalar" | "enum" | "relationship";

export type AppfwUiOperationKind = "query" | "mutation" | "aggregate";

export type AppfwUiOperationReturnShape =
  | "connection"
  | "record"
  | "list"
  | "aggregate"
  | "scalar";

export type AppfwUiWorkflowId = string;

export type AppfwUiErrorState =
  | "loading"
  | "empty"
  | "validation"
  | "saving"
  | "error"
  | "policy_denied";

export type AppfwUiProviderContract = {
  dataSourceName: string;
  dataSourceType: string;
  capabilityHints: readonly string[];
};

export type AppfwUiDesignContract = {
  tokenPrefix: "--pds-";
  tokenCssPath: string;
  density: "comfortable" | "compact";
  accessibilityBaseline: readonly string[];
};

export type AppfwUiHeaderContract = {
  name: string;
  purpose: string;
  required: boolean;
};

export type AppfwUiErrorShapeContract = {
  category: string;
  messageField: string;
  detailField: string;
};

export type AppfwUiAuthContract = {
  required: boolean;
  tenantRequired: boolean;
  localDevAuth: string;
  headers: readonly AppfwUiHeaderContract[];
  policyDenied: AppfwUiErrorShapeContract;
};

export type AppfwUiPaginationContract = {
  defaultPageSize: number;
  maxPageSize: number;
  cursorParameter: string;
  offsetParameters: readonly string[];
  resultFields: readonly string[];
};

export type AppfwUiFilterContract = {
  conjunctions: readonly string[];
  scalarOperators: readonly string[];
  enumOperators: readonly string[];
  relationshipOperators: readonly string[];
  sortSpec: string;
};

export type AppfwUiErrorContract = {
  categories: readonly string[];
  validation: AppfwUiErrorShapeContract;
  policyDenied: AppfwUiErrorShapeContract;
  requestIdHeader: string;
  correlationIdHeader: string;
};

export type AppfwUiFieldRelationshipContract = {
  kind: string;
  schemaName?: string;
  typeName?: string;
  fieldName?: string;
  junctionSchema?: string;
  junctionTable?: string;
  localKey?: string;
  foreignKey?: string;
};

export type AppfwUiFieldUiContract = {
  list: boolean;
  detail: boolean;
  edit: boolean;
  sortable: boolean;
  filterable: boolean;
  formControl: string;
  format?: string;
};

export type AppfwUiFieldValidationContract = {
  required: boolean;
  readOnly: boolean;
  concurrencyControl: boolean;
  clientHint: string;
};

export type AppfwUiFieldContract = {
  name: string;
  label: string;
  kind: AppfwUiFieldKind;
  dataType: string;
  required: boolean;
  readOnly: boolean;
  isKey: boolean;
  isConcurrencyControl: boolean;
  enumTypeName?: string;
  relationshipTarget?: string;
  relationship?: AppfwUiFieldRelationshipContract;
  ui: AppfwUiFieldUiContract;
  validation: AppfwUiFieldValidationContract;
};

export type AppfwUiOperationVariableContract = {
  name: string;
  dataType: string;
  required: boolean;
  description: string;
};

export type AppfwUiOperationContract = {
  name: string;
  kind: AppfwUiOperationKind;
  graphqlName: string;
  variables: readonly AppfwUiOperationVariableContract[];
  returns: string;
  returnsShape: AppfwUiOperationReturnShape;
  selectionPreset: readonly string[];
  requiresAuth: boolean;
  requiresTenant: boolean;
  optimisticConcurrencyField?: string;
  disabledReason?: string;
};

export type AppfwUiEntityViewContract = {
  route: string;
  fields: readonly string[];
  states: readonly AppfwUiErrorState[];
  disabledReason?: string;
};

export type AppfwUiEntityScaffoldContract = {
  list: AppfwUiEntityViewContract;
  detail: AppfwUiEntityViewContract;
  edit: AppfwUiEntityViewContract;
};

export type AppfwUiEntityIdKindContract = {
  kind: "record_locator" | "primary_key" | string;
  field: string;
  public: boolean;
  routeSafe: boolean;
  answerEnvelope: boolean;
  filterOperators: readonly string[];
};

export type AppfwUiEntityOperationRefContract = {
  name: string;
  graphqlName: string;
};

export type AppfwUiEntityBatchResolutionContract = {
  field: string;
  operator: "in" | string;
  maxPageSize: number;
  filterParameter: string;
  fallback: "by_locator_loop" | string;
};

export type AppfwUiEntityAddressingContract = {
  entityUri: string;
  recordUriTemplate: string;
  listRoute: string;
  routeTemplate: string;
  filteredListRoute: string;
  routeIdKind: "record_locator" | string;
  idKinds: readonly AppfwUiEntityIdKindContract[];
  resolveOperation?: AppfwUiEntityOperationRefContract;
  locatorResolveOperation?: AppfwUiEntityOperationRefContract;
  queryOperation?: AppfwUiEntityOperationRefContract;
  batchResolution: AppfwUiEntityBatchResolutionContract;
};

export type AppfwUiEntityContract = {
  schemaName: string;
  typeName: string;
  routeSegment: string;
  addressing: AppfwUiEntityAddressingContract;
  caption: {
    singular: string;
    plural: string;
  };
  primaryKey: string;
  captionField: string;
  audited: boolean;
  readOnly: boolean;
  facets: readonly string[];
  fields: readonly AppfwUiFieldContract[];
  relationships: readonly string[];
  operations: readonly AppfwUiOperationContract[];
  scaffold: AppfwUiEntityScaffoldContract;
};

export type AppfwUiRelationshipEndpointContract = {
  role: string;
  schemaName?: string;
  entityName: string;
  fieldName: string;
  caption?: string;
};

export type AppfwUiRelationshipStorageContract = {
  storageType: string;
  ownerSchema?: string;
  owner: string;
  field: string;
};

export type AppfwUiRelationshipJunctionContract = {
  schemaName?: string;
  entityName: string;
  leftKey: string;
  rightKey: string;
};

export type AppfwUiRelationshipContract = {
  name: string;
  kind: string;
  endpoints: readonly AppfwUiRelationshipEndpointContract[];
  storage?: AppfwUiRelationshipStorageContract;
  junction?: AppfwUiRelationshipJunctionContract;
};

export type AppfwUiWorkflowContract = {
  id: AppfwUiWorkflowId;
  label: string;
  featurePath: string;
  primaryEntity: string;
  supportingEntities: readonly string[];
  proofPoints: readonly string[];
};

export type AppfwUiViewSupportContract = {
  entityTypes: readonly string[];
  shapes: readonly string[];
  idKinds: readonly string[];
  maxNodes?: number;
};

export type AppfwUiViewRegistryContract = {
  viewId: string;
  route: string;
  kind: "entity_list" | "entity_detail" | "workflow" | string;
  supports: AppfwUiViewSupportContract;
};

export type AppfwUiContract = {
  version: 1;
  schemaName: string;
  source: "app_gen";
  provider: AppfwUiProviderContract;
  design: AppfwUiDesignContract;
  auth: AppfwUiAuthContract;
  pagination: AppfwUiPaginationContract;
  filters: AppfwUiFilterContract;
  errors: AppfwUiErrorContract;
  entities: readonly AppfwUiEntityContract[];
  relationships: readonly AppfwUiRelationshipContract[];
  workflows: readonly AppfwUiWorkflowContract[];
  viewRegistry: readonly AppfwUiViewRegistryContract[];
};

"#,
    );

    let mut variable_names = vec![];
    for contract in contracts {
        let variable_name = contract_variable_name(&contract.schema_name);
        let json = serde_json::to_string_pretty(contract)?;
        out.push_str(&format!(
            "export const {variable_name} = {json} as const satisfies AppfwUiContract;\n\n"
        ));
        variable_names.push(variable_name);
    }

    out.push_str("export const appfwUiContracts = [\n");
    for variable_name in variable_names {
        out.push_str(&format!("  {variable_name},\n"));
    }
    out.push_str("] as const satisfies readonly AppfwUiContract[];\n");

    Ok(out)
}

fn contract_variable_name(schema_name: &str) -> String {
    format!("{}UiContract", to_camel_case(schema_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalized_config::{NormalizedDataAccessExecution, NormalizedSchema};

    fn scalar_property(
        name: &str,
        data_type: &str,
        is_key: bool,
        is_caption: bool,
    ) -> NormalizedProperty {
        NormalizedProperty {
            name: name.to_string(),
            caption: name.to_string(),
            data_type: data_type.to_string(),
            is_key,
            is_caption,
            is_required: is_key || is_caption,
            is_read_only: is_key,
            is_concurrency_control: false,
            enum_type_name: None,
            relation: None,
        }
    }

    fn account_entity() -> NormalizedEntity {
        NormalizedEntity {
            schema_name: "crm".to_string(),
            name: "Account".to_string(),
            table_name: "accounts".to_string(),
            caption_singular: "Account".to_string(),
            caption_plural: "Accounts".to_string(),
            snake_singular: "account".to_string(),
            is_table: true,
            is_union: false,
            base_type: None,
            facets: vec!["audited".to_string()],
            execution: NormalizedDataAccessExecution {
                prepared_statements: false,
            },
            has_standard_methods: true,
            has_custom_methods: false,
            has_generated_handler: true,
            standard_methods: vec!["Query".to_string(), "FindById".to_string()],
            query_standard_methods: vec!["Query".to_string(), "FindById".to_string()],
            mutation_standard_methods: vec![],
            custom_methods: vec![],
            query_custom_methods: vec![],
            mutation_custom_methods: vec![],
            primary_key: Some("id".to_string()),
            audit_table: Some("accounts_audit".to_string()),
            native_properties: vec![
                scalar_property("id", "Uuid", true, false),
                scalar_property("name", "String", false, true),
            ],
            relationship_properties: vec![],
        }
    }

    #[test]
    fn entity_addressing_uses_record_locator_as_route_safe_answer_currency() {
        let entity = account_entity();
        let contract = build_entity(&entity);

        assert_eq!(contract.addressing.entity_uri, "appfw://entity/crm/Account");
        assert_eq!(
            contract.addressing.record_uri_template,
            "appfw://entity/crm/Account/{recordLocator}"
        );
        assert_eq!(contract.addressing.route_id_kind, "record_locator");
        assert_eq!(
            contract.addressing.route_template,
            "/data/accounts/:recordLocator"
        );
        assert_eq!(
            contract.addressing.filtered_list_route,
            "/data/accounts?filter=:filterJson"
        );

        let record_locator = contract
            .addressing
            .id_kinds
            .iter()
            .find(|kind| kind.kind == "record_locator")
            .expect("record locator id kind");
        assert_eq!(record_locator.field, "record_locator");
        assert!(record_locator.public);
        assert!(record_locator.route_safe);
        assert!(record_locator.answer_envelope);
        assert_eq!(record_locator.filter_operators, vec!["eq", "in"]);

        let primary_key = contract
            .addressing
            .id_kinds
            .iter()
            .find(|kind| kind.kind == "primary_key")
            .expect("primary key id kind");
        assert_eq!(primary_key.field, "id");
        assert!(!primary_key.public);
        assert!(!primary_key.route_safe);
        assert!(!primary_key.answer_envelope);
    }

    #[test]
    fn entity_addressing_exposes_query_and_resolution_operation_names() {
        let entity = account_entity();
        let contract = build_entity(&entity);

        let resolve = contract
            .addressing
            .resolve_operation
            .as_ref()
            .expect("resolve operation");
        assert_eq!(resolve.name, "find_account");
        assert_eq!(resolve.graphql_name, "findAccount");

        let locator_resolve = contract
            .addressing
            .locator_resolve_operation
            .as_ref()
            .expect("locator resolve operation");
        assert_eq!(locator_resolve.name, "find_account_by_locator");
        assert_eq!(locator_resolve.graphql_name, "findAccountByLocator");

        let query = contract
            .addressing
            .query_operation
            .as_ref()
            .expect("query operation");
        assert_eq!(query.name, "query_accounts");
        assert_eq!(query.graphql_name, "queryAccounts");

        assert_eq!(contract.addressing.batch_resolution.field, "record_locator");
        assert_eq!(contract.addressing.batch_resolution.operator, "in");
        assert_eq!(contract.addressing.batch_resolution.max_page_size, 100);
        assert_eq!(
            contract.addressing.batch_resolution.fallback,
            "by_locator_loop"
        );
    }

    #[test]
    fn view_registry_declares_record_locator_resolution_support() {
        let schema = NormalizedSchema {
            name: "crm".to_string(),
            data_source_name: "crm_primary".to_string(),
            data_source_type: "Postgres".to_string(),
            is_system_schema: false,
            relationships: vec![],
            entities: vec![account_entity()],
        };
        let workflows = workflow_defaults(&schema);
        let views = view_registry_defaults(&schema, &workflows);

        let detail = views
            .iter()
            .find(|view| view.view_id == "entity:accounts:detail")
            .expect("account detail view");
        assert_eq!(detail.route, "/data/accounts/:recordLocator");
        assert_eq!(detail.supports.id_kinds, vec!["record_locator"]);
        assert!(detail.supports.shapes.contains(&"record_detail"));

        let list = views
            .iter()
            .find(|view| view.view_id == "entity:accounts:list")
            .expect("account list view");
        assert_eq!(list.supports.id_kinds, vec!["record_locator"]);
        assert!(list.supports.shapes.contains(&"filtered_list"));
    }
}
