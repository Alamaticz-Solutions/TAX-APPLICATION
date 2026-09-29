use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    app_workspace::AppWorkspace,
    bootstrap_types::{
        data_source::{DataSource, DataSourceType, Schema},
        entity_type::{DataType, EntityType, StandardMethod},
    },
    utils::{artifacts, console, context, files},
};

#[derive(Debug, Serialize)]
pub struct PerformanceRecommendations {
    version: u32,
    generated_at: String,
    schemas: Vec<SchemaRecommendations>,
}

#[derive(Debug, Serialize)]
struct SchemaRecommendations {
    schema_name: String,
    data_source_name: String,
    data_source_type: String,
    recommendation_count: usize,
    recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, Serialize)]
struct Recommendation {
    id: String,
    kind: &'static str,
    status: &'static str,
    severity: &'static str,
    provider: String,
    schema_name: String,
    entity_name: String,
    table_name: String,
    index_name: Option<String>,
    columns: Vec<String>,
    reason: String,
    action: String,
    migration_hint: String,
}

pub fn emit(workspace: &AppWorkspace, entity_types: &[EntityType]) -> Result<()> {
    console::step("emit performance recommendations");

    let schemas = load_schemas(workspace)?;
    let data_sources = context::read_data_sources(&files::get_data_sources_file(workspace))?
        .into_iter()
        .map(|data_source| (data_source.name.clone(), data_source))
        .collect::<HashMap<_, _>>();

    let mut schema_recommendations = vec![];
    for schema in schemas {
        if schema.name == "system" {
            continue;
        }
        let data_source = data_sources
            .get(&schema.data_source_name)
            .with_context(|| {
                format!(
                    "schema `{}` references missing data source `{}`",
                    schema.name, schema.data_source_name
                )
            })?;
        let entities = entity_types
            .iter()
            .filter(|entity_type| entity_type.schema_name == schema.name)
            .collect::<Vec<_>>();
        schema_recommendations.push(schema_recommendations_for(&schema, data_source, &entities));
    }

    schema_recommendations.sort_by(|left, right| left.schema_name.cmp(&right.schema_name));

    let report = PerformanceRecommendations {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        schemas: schema_recommendations,
    };

    let json_file = workspace
        .report_root
        .join("performance_recommendations.json");
    artifacts::safe_write(&json_file, serde_json::to_string_pretty(&report)? + "\n")
        .with_context(|| format!("could not write {}", json_file.display()))?;
    console::write(&json_file);

    let md_file = workspace.report_root.join("performance_recommendations.md");
    artifacts::safe_write(&md_file, render_markdown(&report))
        .with_context(|| format!("could not write {}", md_file.display()))?;
    console::write(&md_file);

    Ok(())
}

fn schema_recommendations_for(
    schema: &Schema,
    data_source: &DataSource,
    entities: &[&EntityType],
) -> SchemaRecommendations {
    let mut dedupe = BTreeMap::new();
    for entity_type in entities
        .iter()
        .copied()
        .filter(|entity_type| entity_type.is_table)
        .filter(|entity_type| !is_generated_audit_entity(entity_type))
    {
        for recommendation in entity_recommendations(schema, data_source, entity_type) {
            dedupe
                .entry(recommendation.id.clone())
                .or_insert(recommendation);
        }
    }

    let recommendations = dedupe.into_values().collect::<Vec<_>>();
    SchemaRecommendations {
        schema_name: schema.name.clone(),
        data_source_name: schema.data_source_name.clone(),
        data_source_type: data_source.data_source_type.as_str().to_string(),
        recommendation_count: recommendations.len(),
        recommendations,
    }
}

fn entity_recommendations(
    schema: &Schema,
    data_source: &DataSource,
    entity_type: &EntityType,
) -> Vec<Recommendation> {
    let mut recommendations = vec![];

    if let Some(columns) = entity_type
        .indexes
        .as_ref()
        .filter(|columns| !columns.is_empty())
    {
        recommendations.push(index_recommendation(
            schema,
            data_source,
            entity_type,
            "declared_index",
            declared_index_name(entity_type, columns),
            columns.clone(),
            "Entity config declares this index for frequent filters, sorting, or lookup paths.",
        ));
    }

    for prop in entity_type
        .props
        .iter()
        .filter(|prop| prop.foreign_key.is_some())
    {
        recommendations.push(index_recommendation(
            schema,
            data_source,
            entity_type,
            "foreign_key_index",
            format!("idx_{}_{}", entity_type.snake_n, prop.name),
            vec![prop.name.clone()],
            "Foreign-key columns should be indexed for access filters, joins, and relationship resolvers.",
        ));
    }

    for prop in entity_type
        .props
        .iter()
        .filter(|prop| prop.data_type == DataType::ManyToMany)
    {
        let Some(many_to_many) = &prop.many_to_many_property else {
            continue;
        };
        let junction_table = snake_table(&many_to_many.junction_table);
        for column in [&many_to_many.local_key, &many_to_many.foreign_key] {
            recommendations.push(junction_recommendation(
                schema,
                data_source,
                entity_type,
                &junction_table,
                format!("idx_{}_{}", junction_table, column),
                vec![column.clone()],
            ));
        }
        recommendations.push(junction_recommendation(
            schema,
            data_source,
            entity_type,
            &junction_table,
            format!("uq_{}", junction_table),
            vec![
                many_to_many.local_key.clone(),
                many_to_many.foreign_key.clone(),
            ],
        ));
    }

    if has_facet(entity_type, "audited") {
        for (index_name, columns) in audit_index_specs(entity_type) {
            recommendations.push(recommendation(
                schema,
                data_source,
                entity_type,
                &format!("{}_audit", entity_type.snake_n),
                "audit_index",
                Some(index_name),
                columns,
                "Audited entities should keep timeline, actor, chain, and event-hash lookups efficient.",
            ));
        }
    }

    if is_generated_query_surface(entity_type) && recommendations.is_empty() {
        recommendations.push(review_recommendation(
            schema,
            data_source,
            entity_type,
            "query_index_review",
            "Generated query methods exist but no explicit indexes or relationship indexes were inferred.",
        ));
    }

    dedupe_entity_recommendations(recommendations)
}

fn index_recommendation(
    schema: &Schema,
    data_source: &DataSource,
    entity_type: &EntityType,
    kind: &'static str,
    index_name: String,
    columns: Vec<String>,
    reason: &str,
) -> Recommendation {
    recommendation(
        schema,
        data_source,
        entity_type,
        &entity_type.snake_n,
        kind,
        Some(index_name),
        columns,
        reason,
    )
}

fn junction_recommendation(
    schema: &Schema,
    data_source: &DataSource,
    entity_type: &EntityType,
    junction_table: &str,
    index_name: String,
    columns: Vec<String>,
) -> Recommendation {
    recommendation(
        schema,
        data_source,
        entity_type,
        junction_table,
        "many_to_many_junction_index",
        Some(index_name),
        columns,
        "Many-to-many junction tables need forward, reverse, and uniqueness access paths.",
    )
}

fn review_recommendation(
    schema: &Schema,
    data_source: &DataSource,
    entity_type: &EntityType,
    kind: &'static str,
    reason: &str,
) -> Recommendation {
    recommendation(
        schema,
        data_source,
        entity_type,
        &entity_type.snake_n,
        kind,
        None,
        vec![],
        reason,
    )
}

fn recommendation(
    schema: &Schema,
    data_source: &DataSource,
    entity_type: &EntityType,
    table_name: &str,
    kind: &'static str,
    index_name: Option<String>,
    columns: Vec<String>,
    reason: &str,
) -> Recommendation {
    let provider = data_source.data_source_type.as_str().to_string();
    let status = recommendation_status(data_source.data_source_type, kind);
    let id = format!(
        "{}:{}:{}:{}",
        provider,
        schema.name,
        table_name,
        index_name
            .as_deref()
            .unwrap_or_else(|| if columns.is_empty() { kind } else { "columns" })
    );
    let action = recommendation_action(data_source.data_source_type, status, table_name, &columns);
    let migration_hint = migration_hint(schema, data_source, status);

    Recommendation {
        id,
        kind,
        status,
        severity: if status == "generated" {
            "info"
        } else {
            "review"
        },
        provider,
        schema_name: schema.name.clone(),
        entity_name: entity_type.pascal_1.clone(),
        table_name: table_name.to_string(),
        index_name,
        columns,
        reason: reason.to_string(),
        action,
        migration_hint,
    }
}

fn recommendation_status(provider: DataSourceType, kind: &str) -> &'static str {
    match provider {
        DataSourceType::PostgreSQL | DataSourceType::MsSqlServer => "generated",
        DataSourceType::FabricSqlAnalytics => "analytics_workload_review",
        DataSourceType::MongoDB => {
            if kind == "query_index_review" {
                "review"
            } else {
                "manual_provider_index"
            }
        }
        DataSourceType::Snowflake => "provider_workload_review",
        DataSourceType::Neo4j => "graph_projection_review",
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => "external_api_workload_review",
    }
}

fn recommendation_action(
    provider: DataSourceType,
    status: &str,
    table_name: &str,
    columns: &[String],
) -> String {
    if status == "generated" {
        return "Generated database package DDL already includes this access path; keep migration drift checks green.".to_string();
    }
    match provider {
        DataSourceType::MongoDB if !columns.is_empty() => format!(
            "Review and apply a MongoDB createIndex on collection `{table_name}` for [{}].",
            columns.join(", ")
        ),
        DataSourceType::Snowflake if !columns.is_empty() => format!(
            "Review clustering/search optimization for `{table_name}` on [{}] using observed workload.",
            columns.join(", ")
        ),
        DataSourceType::Neo4j if !columns.is_empty() => format!(
            "Review graph projection indexes or constraints for `{table_name}` read-model lookups on [{}].",
            columns.join(", ")
        ),
        _ => "Review production query patterns and add a provider-native optimization when warranted."
            .to_string(),
    }
}

fn migration_hint(schema: &Schema, data_source: &DataSource, status: &str) -> String {
    if status == "generated" {
        return "Run `scripts/appfw migrate drift --json` and apply generated migrations through your release workflow.".to_string();
    }
    format!(
        "Use `scripts/appfw migrate new --schema {} --data-source {} --phase expand --name add-performance-indexes` for provider-specific changes.",
        schema.name, data_source.name
    )
}

fn audit_index_specs(entity_type: &EntityType) -> Vec<(String, Vec<String>)> {
    vec![
        (
            format!("idx_{}_audit_record", entity_type.snake_n),
            vec!["record_id".to_string(), "occurred_at".to_string()],
        ),
        (
            format!("idx_{}_audit_actor", entity_type.snake_n),
            vec!["actor_user_name".to_string(), "occurred_at".to_string()],
        ),
        (
            format!("idx_{}_audit_chain", entity_type.snake_n),
            vec!["chain_scope".to_string(), "occurred_at".to_string()],
        ),
        (
            format!("idx_{}_audit_event_hash", entity_type.snake_n),
            vec!["event_hash".to_string()],
        ),
    ]
}

fn declared_index_name(entity_type: &EntityType, columns: &[String]) -> String {
    format!("idx_{}_{}", entity_type.snake_n, columns.join("_"))
}

fn is_generated_audit_entity(entity_type: &EntityType) -> bool {
    entity_type
        .meta
        .as_ref()
        .and_then(|meta| meta.get("generatedAuditEntity"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn is_generated_query_surface(entity_type: &EntityType) -> bool {
    entity_type
        .standard_methods
        .as_ref()
        .is_some_and(|methods| {
            methods
                .iter()
                .any(|method| *method == StandardMethod::Query)
        })
}

fn has_facet(entity_type: &EntityType, facet: &str) -> bool {
    entity_type
        .facets
        .as_ref()
        .is_some_and(|facets| facets.iter().any(|value| value == facet))
}

fn dedupe_entity_recommendations(recommendations: Vec<Recommendation>) -> Vec<Recommendation> {
    let mut seen = BTreeSet::new();
    recommendations
        .into_iter()
        .filter(|recommendation| {
            let key = format!(
                "{}:{}:{}",
                recommendation.provider,
                recommendation.table_name,
                recommendation.columns.join(",")
            );
            seen.insert(key)
        })
        .collect()
}

fn load_schemas(workspace: &AppWorkspace) -> Result<Vec<Schema>> {
    let mut schemas = vec![];
    for schema_dir_name in
        files::get_schema_dir_names(workspace).map_err(|err| anyhow::anyhow!("{err}"))?
    {
        let schema_file = files::get_schema_file(workspace, &schema_dir_name);
        schemas.push(
            context::read_schema(&schema_file)
                .with_context(|| format!("read schema config {}", schema_file.display()))?,
        );
    }
    Ok(schemas)
}

fn snake_table(value: &str) -> String {
    use inflector::cases::tablecase::to_table_case;
    to_table_case(value)
}

fn render_markdown(report: &PerformanceRecommendations) -> String {
    let mut output = String::from("# Performance Recommendations\n\n");
    output.push_str(
        "Generated from application config. Use this as review input for migrations, provider tuning, and release handoff.\n\n",
    );
    output.push_str("| Schema | Data Source | Provider | Recommendations |\n");
    output.push_str("| --- | --- | --- | --- |\n");
    for schema in &report.schemas {
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            schema.schema_name,
            schema.data_source_name,
            schema.data_source_type,
            schema.recommendation_count
        ));
    }
    output.push('\n');

    for schema in &report.schemas {
        output.push_str(&format!("## {}\n\n", schema.schema_name));
        if schema.recommendations.is_empty() {
            output.push_str("No recommendations emitted.\n\n");
            continue;
        }
        output.push_str("| Kind | Status | Entity/Table | Columns | Action |\n");
        output.push_str("| --- | --- | --- | --- | --- |\n");
        for recommendation in &schema.recommendations {
            output.push_str(&format!(
                "| {} | {} | {} / {} | {} | {} |\n",
                recommendation.kind,
                recommendation.status,
                recommendation.entity_name,
                recommendation.table_name,
                if recommendation.columns.is_empty() {
                    "-".to_string()
                } else {
                    recommendation.columns.join(", ")
                },
                recommendation.action.replace('|', "\\|")
            ));
        }
        output.push('\n');
    }

    output
}
