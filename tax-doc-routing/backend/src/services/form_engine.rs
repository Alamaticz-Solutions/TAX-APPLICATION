//! The dynamic form engine (docs/architecture/dynamic-form-engine-design.md): resolves which
//! fields a screen shows, in what order, and whether each is currently required — driven
//! entirely by the `RequestScreen`/`RequestField`/`RequestScreenField`/`Condition`/
//! `ConditionClause`/`FieldOptionList`/`FieldOptionValue` tables. Nothing here hardcodes a field
//! name, a document type, or a business rule: every fact this module uses came from a database
//! row, the same way `services::authz` never hardcodes a role.
//!
//! `resolve_screen_fields` is the one place a `Condition`/`ConditionClause` is evaluated. The
//! frontend never evaluates a condition itself, the same way it never evaluates an RBAC grant
//! itself (see `services::authz`).

use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Value};

use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, UserAuth},
    schemas::tax_routing::{
        ConditionClauseProjection, ConditionProjection, DocumentTypeRouteProjection,
        FieldOptionValueProjection, RequestFieldProjection, RequestScreenFieldProjection,
        RequestScreenProjection,
    },
    services::authz,
};

fn field(name: &str) -> Value {
    json!({ "name": name, "selection_set": [] })
}

fn selection(name: &str, fields: &[Value]) -> Value {
    json!({ "name": name, "selection_set": fields })
}

async fn find_one<T>(
    data_access: &Arc<DataAccess>,
    schema_entity: &Arc<EntityType>,
    selections: Value,
    filter: Value,
    user: Option<UserAuth>,
) -> HandlerResult<Option<T>>
where
    T: Send + Sync + async_graphql::OutputType + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
{
    let res = data_access
        .query_items::<T>(schema_entity.clone(), selections, Some(filter), None, 0, 1, None, user)
        .await?;
    Ok(res.items.into_iter().next())
}

async fn find_many<T>(
    data_access: &Arc<DataAccess>,
    schema_entity: &Arc<EntityType>,
    selections: Value,
    filter: Value,
    sort: Option<Value>,
    user: Option<UserAuth>,
) -> HandlerResult<Vec<T>>
where
    T: Send + Sync + async_graphql::OutputType + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
{
    let res = data_access
        .query_items::<T>(schema_entity.clone(), selections, Some(filter), sort, 0, 250, None, user)
        .await?;
    Ok(res.items)
}

/// One resolved field on a screen, for the frontend to render — no condition, no requirement
/// string, no document-type name: just "show this, call it required or not."
#[derive(serde::Serialize)]
struct ResolvedField {
    field_code: String,
    label: String,
    help_text: Option<String>,
    answer_shape: String,
    data_type: String,
    section: String,
    sort_order: i32,
    visible: bool,
    required: bool,
    editable: bool,
    options: Option<Vec<ResolvedOption>>,
}

#[derive(serde::Serialize)]
struct ResolvedOption {
    code: String,
    label: String,
}

/// Evaluates one clause against the currently-selected document type's requirement flags.
/// `subject_kind = "field"` (testing another *answered* field, not a document-type attribute) is
/// modeled (docs/architecture/dynamic-form-engine-design.md section 4) but nothing seeded today
/// uses it — every launch condition tests a document-type attribute, because that is what every
/// field in business spec 5.5's Visibility column actually depends on. A "field" clause with no
/// evaluator here fails closed (returns false) rather than silently passing.
fn evaluate_clause(clause: &ConditionClauseProjection, doc_type: Option<&DocumentTypeRouteProjection>) -> bool {
    let Some(subject_kind) = clause.subject_kind.as_deref() else { return false };
    if subject_kind != "document_type_attribute" {
        return false;
    }
    let Some(doc_type) = doc_type else { return false };
    let Some(attribute) = clause.subject_attribute.as_deref() else { return false };
    let actual: Option<bool> = match attribute {
        "allows_entity" => doc_type.allows_entity,
        "requires_k1_attachments" => doc_type.requires_k1_attachments,
        "requires_form_8308_attachments" => doc_type.requires_form_8308_attachments,
        "supports_password_protection" => doc_type.supports_password_protection,
        "is_external_only" => doc_type.is_external_only,
        _ => None,
    };
    let expected = clause.value.as_deref() == Some("true");
    match clause.comparison.as_deref() {
        Some("is") => actual == Some(expected),
        Some("is_not") => actual != Some(expected),
        _ => false,
    }
}

async fn evaluate_condition(
    data_access: &Arc<DataAccess>,
    condition_entity: &Arc<EntityType>,
    clause_entity: &Arc<EntityType>,
    condition_id: &str,
    doc_type: Option<&DocumentTypeRouteProjection>,
    user: Option<UserAuth>,
) -> HandlerResult<bool> {
    let condition: Option<ConditionProjection> = find_one(
        data_access,
        condition_entity,
        selection("condition", &[field("id"), field("join_kind")]),
        json!({ "id": { "_eq": condition_id } }),
        user.clone(),
    )
    .await?;
    let Some(condition) = condition else { return Ok(false) };

    let clauses: Vec<ConditionClauseProjection> = find_many(
        data_access,
        clause_entity,
        selection(
            "condition_clause",
            &[
                field("id"),
                field("subject_kind"),
                field("subject_attribute"),
                field("comparison"),
                field("value"),
                field("sort_order"),
            ],
        ),
        json!({ "condition_id": { "_eq": condition_id } }),
        None,
        user,
    )
    .await?;

    if clauses.is_empty() {
        return Ok(false);
    }
    let results = clauses.iter().map(|c| evaluate_clause(c, doc_type));
    Ok(match condition.join_kind.as_deref() {
        Some("or") => results.fold(false, |acc, r| acc || r),
        _ => results.fold(true, |acc, r| acc && r), // default "and"
    })
}

pub async fn resolve_screen_fields(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    screen_code: String,
    document_type_code: String,
) -> HandlerResult<serde_json::Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "request_screen_field.read")?;

    let config = &data_access.app_config;
    let schema = "tax_routing".to_string();
    let screen_entity = config.get_entity_type(&schema, &"RequestScreen".to_string())?;
    let screen_field_entity = config.get_entity_type(&schema, &"RequestScreenField".to_string())?;
    let field_entity = config.get_entity_type(&schema, &"RequestField".to_string())?;
    let condition_entity = config.get_entity_type(&schema, &"Condition".to_string())?;
    let clause_entity = config.get_entity_type(&schema, &"ConditionClause".to_string())?;
    let doc_type_entity = config.get_entity_type(&schema, &"DocumentTypeRoute".to_string())?;
    let option_value_entity = config.get_entity_type(&schema, &"FieldOptionValue".to_string())?;

    let screen: Option<RequestScreenProjection> = find_one(
        data_access,
        &screen_entity,
        selection("request_screen", &[field("id"), field("code")]),
        json!({ "code": { "_eq": screen_code.clone() } }),
        Some(user.clone()),
    )
    .await?;
    let Some(screen) = screen else {
        return Ok(json!({ "screen_code": screen_code, "fields": [] }));
    };
    let screen_id = screen.id.clone().unwrap_or_default();

    let doc_type: Option<DocumentTypeRouteProjection> = if document_type_code.is_empty() {
        None
    } else {
        find_one(
            data_access,
            &doc_type_entity,
            selection(
                "document_type_route",
                &[
                    field("id"),
                    field("allows_entity"),
                    field("requires_k1_attachments"),
                    field("requires_form_8308_attachments"),
                    field("supports_password_protection"),
                    field("is_external_only"),
                ],
            ),
            json!({ "document_type": { "_eq": document_type_code } }),
            Some(user.clone()),
        )
        .await?
    };

    let screen_fields: Vec<RequestScreenFieldProjection> = find_many(
        data_access,
        &screen_field_entity,
        selection(
            "request_screen_field",
            &[
                field("id"),
                field("field_id"),
                field("requirement"),
                field("condition_id"),
                field("section"),
                field("sort_order"),
            ],
        ),
        json!({ "screen_id": { "_eq": screen_id } }),
        Some(json!({ "sort_order": "asc" })),
        Some(user.clone()),
    )
    .await?;

    // Every RequestField this screen might reference, fetched once rather than N+1 per row.
    let all_fields: Vec<RequestFieldProjection> = find_many(
        data_access,
        &field_entity,
        selection(
            "request_field",
            &[
                field("id"),
                field("code"),
                field("label"),
                field("help_text"),
                field("answer_shape"),
                field("data_type"),
                field("option_list_id"),
                field("is_active"),
            ],
        ),
        json!({}),
        None,
        Some(user.clone()),
    )
    .await?;
    let fields_by_id: HashMap<String, &RequestFieldProjection> = all_fields
        .iter()
        .filter_map(|f| f.id.clone().map(|id| (id, f)))
        .collect();

    let mut resolved = Vec::with_capacity(screen_fields.len());
    for sf in &screen_fields {
        let Some(field_id) = &sf.field_id else { continue };
        let Some(rf) = fields_by_id.get(field_id) else { continue };
        if rf.is_active == Some(false) {
            continue;
        }
        let requirement = sf.requirement.as_deref().unwrap_or("optional");

        let (visible, required, editable) = match requirement {
            "mandatory" => (true, true, true),
            "optional" => (true, false, true),
            "readonly" => (true, false, false),
            "conditional" => {
                let ok = if let Some(condition_id) = &sf.condition_id {
                    evaluate_condition(
                        data_access,
                        &condition_entity,
                        &clause_entity,
                        condition_id,
                        doc_type.as_ref(),
                        Some(user.clone()),
                    )
                    .await?
                } else {
                    false
                };
                (ok, ok, true)
            }
            _ => (true, false, true),
        };

        let options = if let Some(option_list_id) = &rf.option_list_id {
            let values: Vec<FieldOptionValueProjection> = find_many(
                data_access,
                &option_value_entity,
                selection(
                    "field_option_value",
                    &[field("id"), field("code"), field("label"), field("sort_order"), field("is_active")],
                ),
                json!({ "_and": [{ "option_list_id": { "_eq": option_list_id } }, { "is_active": { "_eq": true } }] }),
                Some(json!({ "sort_order": "asc" })),
                Some(user.clone()),
            )
            .await?;
            Some(
                values
                    .into_iter()
                    .map(|v| ResolvedOption { code: v.code.unwrap_or_default(), label: v.label.unwrap_or_default() })
                    .collect(),
            )
        } else {
            None
        };

        resolved.push(ResolvedField {
            field_code: rf.code.clone().unwrap_or_default(),
            label: rf.label.clone().unwrap_or_default(),
            help_text: rf.help_text.clone(),
            answer_shape: rf.answer_shape.clone().unwrap_or_else(|| "single".to_string()),
            data_type: rf.data_type.clone().unwrap_or_else(|| "string".to_string()),
            section: sf.section.clone().unwrap_or_default(),
            sort_order: sf.sort_order.unwrap_or(0),
            visible,
            required,
            editable,
            options,
        });
    }

    Ok(json!({ "screen_code": screen_code, "fields": resolved }))
}
