use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Client;
use postgres_types::ToSql;
use serde_json::Value;
use std::marker::Sync;
use std::sync::Arc;
use std::time::Instant;
use std::vec;
use time::OffsetDateTime;
use tracing::{debug, error, info};

use appfw_provider_postgres::{
    aggregate_group_by as provider_aggregate_group_by,
    aggregate_having as provider_aggregate_having,
    aggregate_order_by as provider_aggregate_order_by,
    aggregate_query_sql as provider_aggregate_query_sql,
    aggregate_select_list as provider_aggregate_select_list,
    cte_page_query_sql as provider_cte_page_query_sql,
    mutation_delete_statement as provider_mutation_delete_statement,
    mutation_insert_statement as provider_mutation_insert_statement,
    mutation_junction_delete_statement as provider_mutation_junction_delete_statement,
    mutation_junction_insert_statement as provider_mutation_junction_insert_statement,
    mutation_junction_related_entity_id as provider_mutation_junction_related_entity_id,
    mutation_update_parts as provider_mutation_update_parts,
    mutation_update_statement as provider_mutation_update_statement,
    postgres_execution_client as provider_postgres_execution_client,
    postgres_physical_table_name as provider_physical_table_name, postgres_runtime_error,
    type_param as provider_type_param,
    validate_postgres_connection_security as provider_validate_postgres_connection_security,
    PostgresAggregateGroup, PostgresAggregateHaving, PostgresAggregateHavingPredicate,
    PostgresAggregateMetric, PostgresAggregateQuery, PostgresConnectionConfig,
    PostgresCtePageQuery, PostgresExecutionClient, PostgresFunctionCall, PostgresJunctionTable,
    PostgresMutationEntity, PostgresMutationField, PostgresSortField, PostgresStoredProcedureCall,
    SqlParam,
};
use appfw_runtime::{
    extension::UserAuth, json::JsonObj, provider_keys::FrameworkProvider, PolicyAccess,
    RuntimeAuditEvent, RuntimeAuditQuery, RuntimeProviderIdentity, RuntimeProviderPlanInput,
};

use super::cte::CTE;
use crate::config::app_config::AppConfig;
use crate::data::clients::database_client::{
    DatabaseClient, ProviderAggregatePlan, ProviderPoolStats, ProviderQueryPlan,
    ProviderRoutineArgument, ProviderRoutineCall, ProviderRoutineKind, ProviderRoutineReturns,
};
use crate::data::clients::postgres::filter;
use crate::product_api::runtime_data_type;
use crate::routes::app_error::{AppError, MetadataError};
use crate::schemas::common::{JsonAggregateResult, JsonQueryResult};
use crate::schemas::system::{DataSourceEnvironment, DataType, EntityType, PropertyType};

fn junction_table(
    entity_type: &EntityType,
    prop: &PropertyType,
) -> Result<PostgresJunctionTable, AppError> {
    let many_to_many = prop.many_to_many_property.as_ref().ok_or_else(|| {
        AppError::InternalServerError(
            anyhow::anyhow!("ManyToMany property missing configuration").into(),
        )
    })?;

    Ok(PostgresJunctionTable {
        schema: many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name)
            .clone(),
        table: provider_physical_table_name(&many_to_many.junction_table),
        local_key: many_to_many.local_key.clone(),
        foreign_key: many_to_many.foreign_key.clone(),
    })
}

pub struct PostgresClient {
    execution: PostgresExecutionClient,
    app_config: Arc<AppConfig>,
    data_source_name: String,
}

impl PostgresClient {
    #[tracing::instrument(skip(app_config), fields(data_source = %data_source_name))]
    pub async fn init(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<PostgresClient, AppError> {
        let data_source_env = app_config.get_data_source_env(&data_source_name)?;
        let execution = Self::get_pool(data_source_env)?;
        Ok(PostgresClient {
            execution,
            app_config: app_config.clone(),
            data_source_name,
        })
    }

    // Credentials are resolved by the config loader through the secret provider.
    #[tracing::instrument(skip(data_source_env), fields(host = %data_source_env.db_host, database = %data_source_env.db_name))]
    fn get_pool(
        data_source_env: Arc<DataSourceEnvironment>,
    ) -> Result<PostgresExecutionClient, AppError> {
        let security = provider_validate_postgres_connection_security(
            &data_source_env.name,
            &data_source_env.security_profile,
            &data_source_env.tls_mode,
            &data_source_env.db_host,
        )
        .map_err(AppError::from)?;

        let connection = PostgresConnectionConfig::new(
            data_source_env.db_host.clone(),
            data_source_env.db_port.clone(),
            data_source_env.db_name.clone(),
            data_source_env.service_account_name.clone(),
            data_source_env.service_account_password.clone(),
        );
        let res =
            provider_postgres_execution_client(&connection, &security).map_err(AppError::from)?;
        info!("PostgreSQL connection pool configured");
        Ok(res)
    }

    async fn get_client(&self) -> Result<Client, AppError> {
        self.execution.client().await.map_err(AppError::from)
    }

    /// Extract many-to-many relationships from input data
    fn extract_many_to_many_data(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Result<
        Vec<(
            Arc<crate::schemas::system::PropertyType>,
            Vec<serde_json::Value>,
        )>,
        AppError,
    > {
        let mut many_to_many_data = Vec::new();

        for prop in &entity_type.props {
            if prop.data_type == DataType::ManyToMany {
                if let Some(value) = input.get(&prop.name) {
                    if let Some(array) = value.as_array() {
                        many_to_many_data.push((Arc::new(prop.clone()), array.clone()));
                    }
                }
            }
        }

        Ok(many_to_many_data)
    }

    fn has_many_to_many(entity_type: Arc<EntityType>) -> bool {
        entity_type
            .props
            .iter()
            .any(|prop| prop.data_type == DataType::ManyToMany)
    }

    /// Insert junction table records for a many-to-many relationship
    async fn insert_junction_records(
        &self,
        transaction: &tokio_postgres::Transaction<'_>,
        entity_type: Arc<EntityType>,
        entity_id: i64,
        prop: Arc<crate::schemas::system::PropertyType>,
        related_entities: &[serde_json::Value],
        _user: &UserAuth,
    ) -> Result<(), AppError> {
        let junction = junction_table(&entity_type, &prop)?;

        for (index, related_entity) in related_entities.iter().enumerate() {
            let related_id =
                provider_mutation_junction_related_entity_id(related_entity).map_err(|e| {
                    AppError::Validation(
                        format!("Invalid related entity at index {}: {}", index, e).into(),
                    )
                })?;

            let sql = provider_mutation_junction_insert_statement(&junction);

            let params: Vec<&(dyn ToSql + Sync)> = vec![
                &entity_id,
                &related_id,
                &1i64, // Default user ID - could be enhanced to use actual user ID
            ];

            transaction.execute(&sql, &params).await
        .map_err(|e| {
          error!(schema = %junction.schema, table = %junction.table, error = %e, "junction table insert failed");
          AppError::InternalServerError(anyhow::anyhow!(
            "Failed to insert junction record for {}.{}: {}",
            junction.schema, junction.table, e
          ).into())
        })?;
        }

        debug!(
          schema = %junction.schema,
          table = %junction.table,
          record_count = related_entities.len(),
          "inserted junction records"
        );
        Ok(())
    }

    /// Update junction table records for a many-to-many relationship
    async fn update_junction_records(
        &self,
        transaction: &tokio_postgres::Transaction<'_>,
        entity_type: Arc<EntityType>,
        entity_id: i64,
        prop: Arc<crate::schemas::system::PropertyType>,
        new_related_entities: &[serde_json::Value],
        _user: &UserAuth,
    ) -> Result<(), AppError> {
        let junction = junction_table(&entity_type, &prop)?;

        // First, delete all existing relationships for this entity
        let delete_sql = provider_mutation_junction_delete_statement(&junction);

        let deleted_count = transaction.execute(&delete_sql, &[&entity_id]).await
      .map_err(|e| {
        error!(schema = %junction.schema, table = %junction.table, error = %e, "junction table delete failed");
        AppError::InternalServerError(anyhow::anyhow!(
          "Failed to delete existing junction records for {}.{}: {}",
          junction.schema, junction.table, e
        ).into())
      })?;

        debug!(
          schema = %junction.schema,
          table = %junction.table,
          deleted_count,
          "deleted existing junction records"
        );

        // Then, insert the new relationships
        for (index, related_entity) in new_related_entities.iter().enumerate() {
            let related_id =
                provider_mutation_junction_related_entity_id(related_entity).map_err(|e| {
                    AppError::Validation(
                        format!("Invalid related entity at index {}: {}", index, e).into(),
                    )
                })?;

            let insert_sql = provider_mutation_junction_insert_statement(&junction);

            let params: Vec<&(dyn ToSql + Sync)> = vec![
                &entity_id,
                &related_id,
                &1i64, // Default user ID - could be enhanced to use actual user ID
            ];

            transaction.execute(&insert_sql, &params).await
        .map_err(|e| {
          error!(schema = %junction.schema, table = %junction.table, error = %e, "junction table insert failed");
          AppError::InternalServerError(anyhow::anyhow!(
            "Failed to insert junction record for {}.{}: {}",
            junction.schema, junction.table, e
          ).into())
        })?;
        }

        debug!(
          schema = %junction.schema,
          table = %junction.table,
          record_count = new_related_entities.len(),
          "updated junction records"
        );
        Ok(())
    }

    /// Delete all junction table records for an entity
    async fn delete_junction_records(
        &self,
        transaction: &tokio_postgres::Transaction<'_>,
        entity_type: Arc<EntityType>,
        entity_id: i64,
    ) -> Result<(), AppError> {
        let mut total_deleted = 0u64;

        for prop in &entity_type.props {
            if prop.data_type == DataType::ManyToMany {
                if prop.many_to_many_property.is_some() {
                    let junction = junction_table(&entity_type, prop)?;
                    let delete_sql = provider_mutation_junction_delete_statement(&junction);

                    let deleted_count = transaction.execute(&delete_sql, &[&entity_id]).await
            .map_err(|e| {
              error!(schema = %junction.schema, table = %junction.table, error = %e, "junction table cascade delete failed");
              AppError::InternalServerError(anyhow::anyhow!(
                "Failed to cascade delete junction records for {}.{}: {}",
                junction.schema, junction.table, e
              ).into())
            })?;

                    total_deleted += deleted_count;
                    debug!(
                      schema = %junction.schema,
                      table = %junction.table,
                      deleted_count,
                      "cascade deleted junction records"
                    );
                }
            }
        }

        debug!(total_deleted, "deleted junction records for entity");
        Ok(())
    }

    fn mutation_entity(
        &self,
        entity_type: Arc<EntityType>,
    ) -> Result<PostgresMutationEntity, AppError> {
        Ok(PostgresMutationEntity {
            schema: entity_type.schema_name.clone(),
            table: entity_type.snake_n.clone(),
            primary_key: self.app_config.get_primary_key_name(entity_type)?,
        })
    }

    fn mutation_fields(
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Result<Vec<PostgresMutationField>, AppError> {
        let mut fields = Vec::new();
        for (prop_name, value) in input {
            let prop = AppConfig::try_get_prop(entity_type.clone(), prop_name)
                .ok_or_else(|| AppError::Validation(format!("Unknown field '{}'", prop_name)))?;
            if !AppConfig::is_native_prop(prop.clone()) {
                continue;
            }
            fields.push(PostgresMutationField {
                name: prop.name.clone(),
                data_type: runtime_data_type(prop.data_type),
                is_nullable: !prop.is_required,
                is_key: prop.is_key,
                is_concurrency_control: prop.is_concurrency_control,
                value: value.clone(),
            });
        }
        Ok(fields)
    }

    fn build_insert(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Result<(String, Vec<SqlParam>), AppError> {
        let entity = self.mutation_entity(entity_type.clone())?;
        let fields = Self::mutation_fields(entity_type, input)?;
        provider_mutation_insert_statement(&entity, &fields).map_err(AppError::from)
    }

    fn build_update(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
        read_version: Option<Value>,
        access_filter: Option<Value>,
    ) -> Result<(String, Vec<SqlParam>), AppError> {
        let entity = self.mutation_entity(entity_type.clone())?;
        let fields = Self::mutation_fields(entity_type.clone(), input)?;
        let mut update =
            provider_mutation_update_parts(&fields, read_version).map_err(AppError::from)?;
        let alias = "t0".to_string();
        let access_constraint = filter::try_create_filter(
            self.app_config.clone(),
            entity_type,
            access_filter,
            &alias,
            &mut update.params,
        )?
        .filter(|sql| !sql.trim().is_empty())
        .map(|sql| format!(" AND ({sql})"))
        .unwrap_or_default();
        let sql = provider_mutation_update_statement(&entity, &update, &access_constraint);
        Ok((sql, update.params))
    }

    fn build_delete(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
        read_version: Option<Value>,
        access_filter: Option<Value>,
    ) -> Result<(String, Vec<SqlParam>), AppError> {
        let entity = self.mutation_entity(entity_type.clone())?;
        let fields = Self::mutation_fields(entity_type.clone(), input)?;
        let (_, mut params) =
            provider_mutation_delete_statement(&entity, &fields, read_version.clone(), "")
                .map_err(AppError::from)?;
        let alias = "t0".to_string();
        let access_constraint = filter::try_create_filter(
            self.app_config.clone(),
            entity_type,
            access_filter,
            &alias,
            &mut params,
        )?
        .filter(|sql| !sql.trim().is_empty())
        .map(|sql| format!(" AND ({sql})"))
        .unwrap_or_default();
        let (sql, _) =
            provider_mutation_delete_statement(&entity, &fields, read_version, &access_constraint)
                .map_err(AppError::from)?;
        Ok((sql, params))
    }

    fn get_item_res(
        &self,
        entity_type: Arc<EntityType>,
        selection_parent: serde_json::Value,
        item_obj: &JsonObj,
    ) -> Result<JsonObj, AppError> {
        let mut obj_item: JsonObj = JsonObj::new();

        let selection_set = selection_parent
            .get("selection_set")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection_set must be an array".to_string())
            })?;

        for selection in selection_set {
            // println!("\n > get_item_res: selection {:?}", selection);
            let sel_name = selection
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    MetadataError::InvalidSelection("selection name must be a string".to_string())
                })?
                .to_string();
            let prop = AppConfig::get_prop(entity_type.clone(), &sel_name)?;

            let value = item_obj.get(&prop.name).cloned().unwrap_or(Value::Null);

            match prop.data_type {
                DataType::NavToOne | DataType::NavToMany => match value.as_array() {
                    Some(array_value) => {
                        let ref_entity_type = self
                            .app_config
                            .get_nav_entity_type(entity_type.clone(), prop.clone())?;

                        let obj_child_items = self.get_child_items_res(
                            ref_entity_type.clone(),
                            selection.to_owned(),
                            array_value,
                        )?;

                        if prop.data_type == DataType::NavToOne {
                            if let Some(child_item) = obj_child_items.first() {
                                obj_item.insert(prop.name.to_string(), child_item.clone());
                            }
                        } else {
                            let v: Value = serde_json::Value::Array(obj_child_items);
                            obj_item.insert(prop.name.to_string(), v);
                        }
                    }
                    None => {}
                },
                _ => {
                    obj_item.insert(prop.name.to_string(), value);
                }
            }
        }

        Ok(obj_item)
    }

    fn get_child_items_res(
        &self,
        ref_entity_type: Arc<EntityType>,
        selection: serde_json::Value,
        array_value: &Vec<Value>,
    ) -> Result<Vec<Value>, AppError> {
        let mut obj_child_items: Vec<Value> = vec![];
        for array_item in array_value.iter() {
            match array_item {
                Value::Object(o) => {
                    let child_res =
                        self.get_item_res(ref_entity_type.clone(), selection.clone(), o)?;
                    let v: Value = serde_json::Value::Object(child_res);
                    obj_child_items.push(v);
                }
                _ => {}
            };
        }
        Ok(obj_child_items)
    }

    #[tracing::instrument(
    skip(self, entity_type, cte, selections, params_vec),
    fields(entity = %entity_type.pascal_1, skip = skip, limit = limit, param_count = params_vec.len())
  )]
    async fn exec_cte_query(
        &self,
        entity_type: Arc<EntityType>,
        cte: CTE,
        selections: serde_json::Value,
        skip: i32,
        limit: i32,
        request_datetime: OffsetDateTime,
        request_start: Instant,
        params_vec: &Vec<SqlParam>,
    ) -> Result<JsonQueryResult, AppError> {
        let sql = provider_cte_page_query_sql(&PostgresCtePageQuery {
            cte_def: cte.def,
            root_cte_name: cte.name,
            skip,
            limit,
        });

        let use_prepared_statements = prepared_statements_enabled(&entity_type);

        debug!(
            sql = %sql,
            prepared_statements = use_prepared_statements,
            "executing CTE query"
        );

        let query_res = if use_prepared_statements {
            self.execution
                .query_json_rows_prepared(&sql, params_vec)
                .await
        } else {
            self.execution.query_json_rows(&sql, params_vec).await
        }
        .map_err(AppError::from)?;

        // .map_err(|e| AppError::InternalServerError(e.into()))?

        debug!(has_row = query_res.is_some(), "CTE query returned");

        match query_res {
            Some(row) => {
                let query_count = row.query_count;
                debug!(query_count, "CTE query count loaded");

                let items: Vec<JsonObj> = match (query_count, row.rows) {
                    (0, _) => vec![],
                    (_, Some(row_value)) => match row_value {
                        Value::Array(val_items) => {
                            let mut items = vec![];
                            for item in val_items {
                                if let Some(item_obj) = item.as_object() {
                                    items.push(self.get_item_res(
                                        entity_type.clone(),
                                        selections.clone(),
                                        item_obj,
                                    )?);
                                }
                            }
                            items
                        }
                        _ => vec![],
                    },
                    (_, None) => vec![],
                };

                Ok(JsonQueryResult::new(
                    skip,
                    limit,
                    query_count,
                    items,
                    request_datetime,
                    request_start,
                ))
            }
            None => {
                error!("CTE query returned no row");
                Err(AppError::InternalServerError(
                    anyhow::anyhow!("error executing query").into(),
                ))
            }
        }
    }

    #[tracing::instrument(skip(self, id_value, entity_type, selections, user, access), fields(entity = %entity_type.pascal_1))]
    async fn fetch_persisted_item(
        &self,
        id_value: Option<Value>,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        match id_value {
            Some(v) => {
                debug!("fetching persisted item");
                let id_str = match v {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    _ => {
                        return Err(AppError::InternalServerError(
                            anyhow::anyhow!("Invalid ID format").into(),
                        ))
                    }
                };
                match self
                    .find_item_json(entity_type.clone(), selections, id_str, user, access)
                    .await
                {
                    Ok(res) => match res {
                        Some(item) => {
                            debug!("fetched persisted item");
                            Ok(item)
                        }
                        None => {
                            error!("persisted item was not found after write");
                            Err(AppError::InternalServerError(
                                anyhow::anyhow!("error executing fetch").into(),
                            ))
                        }
                    },
                    Err(e) => {
                        error!(error = %e, "failed to fetch persisted item");
                        return Err(e);
                    }
                }
            }
            None => Err(AppError::InternalServerError(
                anyhow::anyhow!("error executing fetch").into(),
            )), // should not happen but need to handle with error
        }
    }
}

fn prepared_statements_enabled(entity_type: &EntityType) -> bool {
    entity_type
        .execution
        .as_ref()
        .and_then(|execution| execution.prepared_statements)
        .unwrap_or(false)
}

fn postgres_aggregate_query(
    app_config: Arc<AppConfig>,
    plan: &ProviderAggregatePlan,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let alias = "t0".to_string();
    let groups = postgres_aggregate_groups(plan);
    let metrics = postgres_aggregate_metrics(plan);
    let having_input = postgres_aggregate_having_input(plan);
    let select_list =
        provider_aggregate_select_list(&groups, &metrics, &alias).map_err(AppError::from)?;
    let table = format!(
        "{}.{}",
        plan.entity_type.schema_name, plan.entity_type.snake_n
    );
    let where_sql = postgres_aggregate_where(app_config, plan, &alias, params)?;
    let group_by = provider_aggregate_group_by(&groups, &alias);
    let having = provider_aggregate_having(&metrics, having_input.as_ref(), &alias, params)
        .map_err(AppError::from)?;
    let order_by = postgres_aggregate_order_by(plan);
    let skip = plan.pagination.skip.max(0);
    let limit = plan.pagination.limit.max(0);

    Ok(provider_aggregate_query_sql(&PostgresAggregateQuery {
        select_list,
        table,
        alias,
        where_sql,
        group_by,
        having,
        order_by,
        skip,
        limit,
    }))
}

fn postgres_aggregate_where(
    app_config: Arc<AppConfig>,
    plan: &ProviderAggregatePlan,
    alias: &String,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let mut clauses = Vec::new();
    if let Some(filter_sql) = filter::try_create_filter_ast(
        app_config.clone(),
        plan.entity_type.clone(),
        plan.filter.as_ref(),
        alias,
        params,
    )? {
        if !filter_sql.trim().is_empty() {
            clauses.push(filter_sql);
        }
    }
    if let Some(access_sql) = filter::try_create_filter_ast(
        app_config,
        plan.entity_type.clone(),
        plan.access_filter.as_ref(),
        alias,
        params,
    )? {
        if !access_sql.trim().is_empty() {
            clauses.push(access_sql);
        }
    }
    Ok(if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    })
}

fn postgres_aggregate_groups(plan: &ProviderAggregatePlan) -> Vec<PostgresAggregateGroup> {
    plan.group_by
        .iter()
        .map(|group| PostgresAggregateGroup {
            name: group.prop.name.clone(),
            alias: group.alias.clone(),
        })
        .collect()
}

fn postgres_aggregate_metrics(plan: &ProviderAggregatePlan) -> Vec<PostgresAggregateMetric> {
    plan.metrics
        .iter()
        .map(|metric| PostgresAggregateMetric {
            function: metric.function,
            field_name: metric.prop.as_ref().map(|prop| prop.name.clone()),
            alias: metric.alias.clone(),
            value_data_type: runtime_data_type(metric.value_data_type()),
        })
        .collect()
}

fn postgres_aggregate_having_input(
    plan: &ProviderAggregatePlan,
) -> Option<PostgresAggregateHaving> {
    plan.having.as_ref().map(|having| PostgresAggregateHaving {
        predicates: having
            .predicates
            .iter()
            .map(|predicate| PostgresAggregateHavingPredicate {
                metric_alias: predicate.metric_alias.clone(),
                op: predicate.op,
                value: predicate.value.clone(),
            })
            .collect(),
    })
}

fn postgres_aggregate_order_by(plan: &ProviderAggregatePlan) -> String {
    let fields = plan
        .sort
        .specs
        .iter()
        .map(|spec| PostgresSortField {
            name: spec.alias.clone(),
            direction: spec.direction,
        })
        .collect::<Vec<_>>();
    provider_aggregate_order_by(&fields)
}

fn postgres_app_error(error: tokio_postgres::Error) -> AppError {
    AppError::from(postgres_runtime_error(error))
}

impl RuntimeProviderIdentity for PostgresClient {
    fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    fn framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::Postgres
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        let status = self.execution.pool().status();
        let provider = self.provider_descriptor();
        ProviderPoolStats::instrumented(
            provider.provider_key(),
            provider.data_source_name(),
            status.max_size as u64,
            status.size as u64,
            status.available as u64,
            status.waiting as u64,
        )
    }
}

#[async_trait]
impl DatabaseClient for PostgresClient {
    async fn health_check(&self) -> Result<(), AppError> {
        self.execution.health_check().await.map_err(AppError::from)
    }

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), AppError> {
        self.execution
            .append_audit_event(event)
            .await
            .map_err(AppError::from)
    }

    async fn query_audit_events(&self, query: RuntimeAuditQuery) -> Result<Vec<Value>, AppError> {
        self.execution
            .query_audit_events(query)
            .await
            .map_err(AppError::from)
    }

    async fn call_provider_routine_json(
        &self,
        routine: &ProviderRoutineCall,
        arguments: &[ProviderRoutineArgument],
        _user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Value, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        let routine_name = routine.postgres.ok_or_else(|| {
            AppError::DataAccess(format!(
                "provider routine `{}` has no PostgreSQL binding",
                routine.method_name
            ))
        })?;
        let params = arguments
            .iter()
            .map(|argument| {
                provider_type_param(
                    argument.data_type,
                    !argument.required,
                    argument.name,
                    argument.value.clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)?;

        match routine.kind {
            ProviderRoutineKind::Function => {
                let call =
                    PostgresFunctionCall::new(routine_name.schema, routine_name.name, params.len())
                        .map_err(AppError::from)?;
                match routine.returns {
                    ProviderRoutineReturns::None => {
                        self.execution
                            .query_function(&call, &params)
                            .await
                            .map_err(AppError::from)?;
                        Ok(Value::Null)
                    }
                    ProviderRoutineReturns::One => {
                        let rows = self
                            .execution
                            .query_prepared(&call.json_one_statement()?, &params)
                            .await
                            .map_err(AppError::from)?;
                        rows.first()
                            .map(|row| row.try_get::<_, Value>(0))
                            .transpose()
                            .map_err(|err| AppError::DataAccess(err.to_string()))
                            .map(|value| value.unwrap_or(Value::Null))
                    }
                    ProviderRoutineReturns::Many => {
                        let rows = self
                            .execution
                            .query_prepared(&call.json_many_statement()?, &params)
                            .await
                            .map_err(AppError::from)?;
                        rows.first()
                            .map(|row| row.try_get::<_, Value>(0))
                            .transpose()
                            .map_err(|err| AppError::DataAccess(err.to_string()))
                            .map(|value| value.unwrap_or_else(|| Value::Array(vec![])))
                    }
                }
            }
            ProviderRoutineKind::Procedure => {
                let call = PostgresStoredProcedureCall::new(
                    routine_name.schema,
                    routine_name.name,
                    params.len(),
                )
                .map_err(AppError::from)?;
                match routine.returns {
                    ProviderRoutineReturns::None => {
                        self.execution
                            .call_stored_procedure(&call, &params)
                            .await
                            .map_err(AppError::from)?;
                        Ok(Value::Null)
                    }
                    ProviderRoutineReturns::One => {
                        let rows = self
                            .execution
                            .query_prepared(&call.json_result_statement()?, &params)
                            .await
                            .map_err(AppError::from)?;
                        rows.first()
                            .map(|row| row.try_get::<_, Value>(0))
                            .transpose()
                            .map_err(|err| AppError::DataAccess(err.to_string()))
                            .map(|value| value.unwrap_or(Value::Null))
                    }
                    ProviderRoutineReturns::Many => {
                        let rows = self
                            .execution
                            .query_prepared(&call.json_result_statement()?, &params)
                            .await
                            .map_err(AppError::from)?;
                        rows.first()
                            .map(|row| row.try_get::<_, Value>(0))
                            .transpose()
                            .map_err(|err| AppError::DataAccess(err.to_string()))
                            .map(|value| value.unwrap_or_else(|| Value::Array(vec![])))
                    }
                }
            }
        }
    }

    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<Vec<JsonObj>, AppError> {
        Ok(self.query_items_plan_json(input).await?.items)
    }

    async fn query_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<JsonQueryResult, AppError> {
        let (plan, user, access) = input.into_parts();
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let mut params: Vec<SqlParam> = vec![];

        let cte = CTE::new(
            self.app_config.clone(),
            plan.entity_type.clone(),
            plan.selection_json(),
            plan.filter_json(),
            plan.sort_json(),
            user,
            &mut params,
        )?;
        debug!(
            param_count = params.len(),
            "created PostgreSQL QueryPlan CTE"
        );

        self.exec_cte_query(
            plan.entity_type.clone(),
            cte,
            plan.selection_json(),
            plan.pagination.skip,
            plan.pagination.limit,
            request_datetime,
            request_start,
            &mut params,
        )
        .await
    }

    async fn aggregate_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderAggregatePlan>,
    ) -> Result<JsonAggregateResult, AppError> {
        let (plan, _user, access) = input.into_parts();
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let mut params: Vec<SqlParam> = vec![];
        let sql = postgres_aggregate_query(self.app_config.clone(), &plan, &mut params)?;
        debug!(sql = %sql, "executing PostgreSQL aggregate query");
        let rows = self
            .execution
            .aggregate_json_rows(&sql, &params)
            .await
            .map_err(AppError::from)?;

        Ok(JsonAggregateResult::new(
            plan.pagination.skip,
            plan.pagination.limit,
            rows.total,
            rows.rows,
            request_datetime,
            request_start,
        ))
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, input, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow)
  )]
    async fn create_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        debug!("creating PostgreSQL item");

        if access.allow {
            // Extract many-to-many relationships before processing
            let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input)?;
            let has_many_to_many = !many_to_many_data.is_empty();

            if has_many_to_many {
                // Use transaction for entities with many-to-many relationships
                let mut client = self.get_client().await?;
                let transaction = client.transaction().await.map_err(postgres_app_error)?;

                // Insert the main entity
                let (sql, params_vec) = self.build_insert(entity_type.clone(), &input)?;
                let params: Vec<&(dyn ToSql + Sync)> = params_vec
                    .iter()
                    .map(|v| v.as_ref() as &(dyn ToSql + Sync))
                    .collect();
                debug!(sql = %sql, param_count = params_vec.len(), "executing PostgreSQL insert");

                let maybe_item = transaction
                    .query_opt(&sql, &params)
                    .await
                    .map_err(postgres_app_error)?;

                let row_item = maybe_item.ok_or_else(|| {
                    AppError::InternalServerError(anyhow::anyhow!("Failed to create entity").into())
                })?;
                let id_value: Option<serde_json::Value> = row_item.get(0);

                // Extract the entity ID for junction table insertions
                let entity_id = if let Some(id_json) = &id_value {
                    id_json.as_i64().ok_or_else(|| {
                        AppError::InternalServerError(
                            anyhow::anyhow!("Invalid entity ID returned").into(),
                        )
                    })?
                } else {
                    return Err(AppError::InternalServerError(
                        anyhow::anyhow!("No entity ID returned").into(),
                    ));
                };

                // Insert junction table records for many-to-many relationships
                for (prop, related_entities) in many_to_many_data {
                    self.insert_junction_records(
                        &transaction,
                        entity_type.clone(),
                        entity_id,
                        prop,
                        &related_entities,
                        user,
                    )
                    .await?;
                }

                // Commit the transaction
                transaction.commit().await.map_err(postgres_app_error)?;

                // Fetch and return the complete entity with relationships
                self.fetch_persisted_item(id_value, entity_type.clone(), selections, user, access)
                    .await
            } else {
                // Use simple execution for entities without many-to-many relationships
                let (sql, params_vec) = self.build_insert(entity_type.clone(), &input)?;
                let params: Vec<&(dyn ToSql + Sync)> = params_vec
                    .iter()
                    .map(|v| v.as_ref() as &(dyn ToSql + Sync))
                    .collect();
                debug!(sql = %sql, param_count = params_vec.len(), "executing PostgreSQL insert");

                let client = self.get_client().await?;
                let maybe_item = client.query_opt(&sql, &params).await.map_err(|e| {
                    error!(error = %e, "PostgreSQL insert failed");
                    postgres_app_error(e)
                })?;
                debug!("PostgreSQL insert executed");

                let row_item = maybe_item.ok_or_else(|| {
                    AppError::InternalServerError(anyhow::anyhow!("Failed to create entity").into())
                })?;
                let id_value: Option<serde_json::Value> = row_item.get(0);
                debug!(has_id = id_value.is_some(), "PostgreSQL insert returned id");

                // Fetch and return the complete entity
                self.fetch_persisted_item(id_value, entity_type.clone(), selections, user, access)
                    .await
            }
        } else {
            debug!("PostgreSQL create denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, input, user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn update_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<JsonObj, AppError> {
        debug!("updating PostgreSQL item");

        if access.allow {
            // Extract many-to-many relationships before processing
            let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input)?;

            // Start a transaction for atomicity
            let mut client = self.get_client().await?;
            let transaction = client.transaction().await.map_err(postgres_app_error)?;

            // Update the main entity
            let (sql, params_vec) = self.build_update(
                entity_type.clone(),
                &input,
                read_version,
                access.filter.clone(),
            )?;
            let params: Vec<&(dyn ToSql + Sync)> = params_vec
                .iter()
                .map(|v| v.as_ref() as &(dyn ToSql + Sync))
                .collect();

            debug!(sql = %sql, param_count = params_vec.len(), "executing PostgreSQL update");

            let maybe_item = transaction
                .query_opt(&sql, &params)
                .await
                .map_err(postgres_app_error)?;

            match maybe_item {
                Some(row_item) => {
                    let id_value: Option<serde_json::Value> = row_item.get(0);

                    if !many_to_many_data.is_empty() {
                        let entity_id = if let Some(id_json) = &id_value {
                            id_json.as_i64().ok_or_else(|| {
                                AppError::InternalServerError(
                                    anyhow::anyhow!("Invalid entity ID returned").into(),
                                )
                            })?
                        } else {
                            return Err(AppError::InternalServerError(
                                anyhow::anyhow!("No entity ID returned").into(),
                            ));
                        };

                        // Update junction table records for many-to-many relationships
                        for (prop, related_entities) in many_to_many_data {
                            self.update_junction_records(
                                &transaction,
                                entity_type.clone(),
                                entity_id,
                                prop,
                                &related_entities,
                                user,
                            )
                            .await?;
                        }
                    }

                    // Commit the transaction
                    transaction.commit().await.map_err(postgres_app_error)?;

                    // Fetch and return the complete entity with relationships
                    self.fetch_persisted_item(
                        id_value,
                        entity_type.clone(),
                        selections,
                        user,
                        access,
                    )
                    .await
                }
                None if access.filter.is_some() => Err(AppError::AccessDenied),
                None => Err(AppError::InvalidKeyOrVersion),
            }
        } else {
            debug!("PostgreSQL update denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, input, user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn delete_item_json(
        &self,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, AppError> {
        debug!("deleting PostgreSQL item");

        if access.allow {
            // Start a transaction for atomicity
            let mut client = self.get_client().await?;
            let transaction = client.transaction().await.map_err(postgres_app_error)?;

            if Self::has_many_to_many(entity_type.clone()) {
                // First, delete all junction table records for this entity.
                let entity_id = if let Some(id_value) = input.get("id") {
                    provider_mutation_junction_related_entity_id(id_value)
                        .map_err(AppError::Validation)?
                } else {
                    return Err(AppError::Validation(
                        "Entity ID is required for deletion".into(),
                    ));
                };
                self.delete_junction_records(&transaction, entity_type.clone(), entity_id)
                    .await?;
            }

            // Then delete the main entity
            let (sql, params_vec) = self.build_delete(
                entity_type.clone(),
                &input,
                read_version,
                access.filter.clone(),
            )?;
            let params: Vec<&(dyn ToSql + Sync)> = params_vec
                .iter()
                .map(|v| v.as_ref() as &(dyn ToSql + Sync))
                .collect();

            debug!(sql = %sql, param_count = params_vec.len(), "executing PostgreSQL delete");

            let maybe_item = transaction
                .query_opt(&sql, &params)
                .await
                .map_err(postgres_app_error)?;

            let row_item = maybe_item.ok_or_else(|| AppError::InvalidKeyOrVersion)?;
            let id_value: Option<serde_json::Value> = row_item.get(0);

            // Commit the transaction
            transaction.commit().await.map_err(postgres_app_error)?;

            match id_value {
                Some(_) => Ok(1),
                None => Ok(0),
            }
        } else {
            debug!("PostgreSQL delete denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, id, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_id = !id.is_empty())
  )]
    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError> {
        debug!("finding PostgreSQL item");

        if access.allow {
            let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
            // Convert string ID to appropriate type for database query
            let id_value = if let Ok(int_id) = id.parse::<i64>() {
                serde_json::Value::Number(serde_json::Number::from(int_id))
            } else {
                serde_json::Value::String(id)
            };
            let filter = Some(serde_json::json!({ &pk_name: id_value }));

            let res = self
                .query_items_json(
                    entity_type.clone(),
                    selections,
                    filter,
                    None,
                    0,
                    1,
                    user,
                    access,
                )
                .await?;

            Ok(res.items.into_iter().next())
        } else {
            debug!("PostgreSQL find denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, filter, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow)
  )]
    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<serde_json::Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError> {
        debug!("getting PostgreSQL items");

        if access.allow {
            let res = self
                .query_items_json(
                    entity_type.clone(),
                    selections,
                    filter,
                    None,
                    0,
                    250,
                    user,
                    access,
                )
                .await?;
            // println!("\n PostgresClient: query_items res:: {:?}", res);
            // println!("\n\n");
            Ok(res.items)
        } else {
            debug!("PostgreSQL get-items denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, filter, sort, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, skip = skip, limit = limit)
  )]
    async fn query_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, AppError> {
        debug!("querying PostgreSQL items");

        if access.allow {
            // Metrics
            let request_datetime = OffsetDateTime::now_utc();
            let request_start = Instant::now();

            let mut params: Vec<SqlParam> = vec![];

            let cte = CTE::new(
                self.app_config.clone(),
                entity_type.clone(),
                selections.clone(),
                filter,
                sort,
                user,
                &mut params,
            )?;
            debug!(param_count = params.len(), "created PostgreSQL CTE");

            // Access policy applied at each CTE level. If access is allowed, the CTE will be Some(CTE)
            let result = self
                .exec_cte_query(
                    entity_type,
                    cte,
                    selections,
                    skip,
                    limit,
                    request_datetime,
                    request_start,
                    &mut params,
                )
                .await;
            debug!("completed PostgreSQL CTE query");
            result
        } else {
            debug!("PostgreSQL query denied by access policy");
            Err(AppError::AccessDenied)
        }
    }
}
