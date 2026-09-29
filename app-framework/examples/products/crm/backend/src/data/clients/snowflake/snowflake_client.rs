use std::{env, future::Future, pin::Pin, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use inflector::cases::tablecase::{is_table_case, to_table_case};
use serde_json::{json, Value};
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

use appfw_provider_snowflake::{
    aggregate_group_by as provider_aggregate_group_by,
    aggregate_having as provider_aggregate_having,
    aggregate_order_by as provider_aggregate_order_by,
    aggregate_select_list as provider_aggregate_select_list,
    mutation_count_statement as provider_mutation_count_statement,
    mutation_delete_parts as provider_mutation_delete_parts,
    mutation_delete_statement as provider_mutation_delete_statement,
    mutation_insert_parts as provider_mutation_insert_parts,
    mutation_insert_statement as provider_mutation_insert_statement,
    mutation_update_parts as provider_mutation_update_parts,
    mutation_update_statement as provider_mutation_update_statement,
    snowflake_session_config as provider_snowflake_session_config, SnowflakeAggregateGroup,
    SnowflakeAggregateHaving, SnowflakeAggregateHavingPredicate, SnowflakeAggregateMetric,
    SnowflakeConnectionConfig, SnowflakeExecutionClient, SnowflakeMutationEntity,
    SnowflakeMutationField, SnowflakeSessionConfig, SnowflakeSortField,
    SnowflakeStatementBuilder as ProviderSnowflakeStatementBuilder, SnowflakeStoredProcedureCall,
};
use appfw_runtime::{
    connection_security::{self, Provider},
    extension::UserAuth,
    json::JsonObj,
    provider_keys::FrameworkProvider,
    record_locator::RECORD_LOCATOR_FIELD,
    AccessAction, PolicyAccess, RuntimeAuditEvent, RuntimeAuditQuery, RuntimeProviderIdentity,
    RuntimeProviderPlanInput,
};

use crate::{
    config::app_config::AppConfig,
    data::{
        clients::{
            database_client::{
                DatabaseClient, ProviderAggregatePlan, ProviderPoolStats, ProviderQueryPlan,
                ProviderRoutineArgument, ProviderRoutineCall, ProviderRoutineKind,
                ProviderRoutineReturns,
            },
            snowflake::{
                filter, sort,
                statement::{SnowflakeStatement, SnowflakeStatementBuilder},
            },
        },
        query_ir::FilterAst,
    },
    product_api::runtime_data_type,
    routes::app_error::AppError,
    schemas::{
        common::{JsonAggregateResult, JsonQueryResult},
        system::{DataSourceEnvironment, DataType, EntityType, PropertyType},
    },
};

fn physical_table_name(name: &str) -> String {
    if is_table_case(name) {
        name.to_string()
    } else {
        to_table_case(name)
    }
}

#[derive(Clone)]
pub struct SnowflakeClient {
    app_config: Arc<AppConfig>,
    data_source_name: String,
    execution: SnowflakeExecutionClient,
    localstack_endpoint: bool,
}

impl SnowflakeClient {
    pub async fn init(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<SnowflakeClient, AppError> {
        let env = app_config.get_data_source_env(&data_source_name)?;
        let session = session_config(&env)?;
        let localstack_endpoint = session.endpoint.contains("localhost.localstack.cloud");
        let execution = SnowflakeExecutionClient::from_session(session).map_err(AppError::from)?;

        Ok(SnowflakeClient {
            app_config,
            data_source_name,
            execution,
            localstack_endpoint,
        })
    }

    async fn run_raw_query_statement(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<Vec<JsonObj>, AppError> {
        self.execution
            .run_query_statement(statement)
            .await
            .map_err(AppError::from)
    }

    async fn run_entity_query_statement(
        &self,
        statement: SnowflakeStatement,
        entity_type: Arc<EntityType>,
    ) -> Result<Vec<JsonObj>, AppError> {
        let rows = self.run_raw_query_statement(statement).await?;
        let mut coerced = Vec::with_capacity(rows.len());
        for row in rows {
            coerced.push(self.coerce_entity_row(entity_type.clone(), row)?);
        }
        Ok(coerced)
    }

    async fn run_exec_statement(&self, statement: SnowflakeStatement) -> Result<(), AppError> {
        self.execution
            .run_exec_statement(statement)
            .await
            .map_err(AppError::from)
    }

    async fn query_native_rows(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter_val: Option<Value>,
        sort_val: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<(i64, Vec<JsonObj>), AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let alias = "t0";
        let mut filter_builder = SnowflakeStatementBuilder::new();
        let where_sql = self.where_clause(
            entity_type.clone(),
            alias,
            filter_val,
            access.filter.clone(),
            &mut filter_builder,
        )?;
        let order_by = sort::create_if_exists(entity_type.clone(), alias, sort_val)?;
        let select_list = native_select_list(entity_type.clone(), alias);
        let table = table_ref(entity_type.clone());
        let count_sql = format!(
            "SELECT COUNT(*) AS \"count\" FROM {table} AS {alias} {where_sql}",
            table = table,
            alias = quote_ident(alias),
            where_sql = where_sql,
        );
        let count_rows = self
            .run_raw_query_statement(filter_builder.clone().finish(count_sql))
            .await?;
        let total = count_rows
            .first()
            .and_then(|row| row.get("count"))
            .and_then(value_to_i64)
            .unwrap_or(0);

        let data_sql = format!(
      "SELECT {select_list} FROM {table} AS {alias} {where_sql} {order_by} LIMIT {limit} OFFSET {skip}",
      select_list = select_list,
      table = table,
      alias = quote_ident(alias),
      where_sql = where_sql,
      order_by = order_by,
      limit = limit.max(0),
      skip = skip.max(0),
    );
        let rows = self
            .run_entity_query_statement(filter_builder.finish(data_sql), entity_type.clone())
            .await?;
        let mut projected = Vec::with_capacity(rows.len());
        for row in rows {
            projected.push(
                self.project_item(entity_type.clone(), row, selections.clone(), user)
                    .await?,
            );
        }

        Ok((total, projected))
    }

    async fn query_native_rows_plan(
        &self,
        plan: &ProviderQueryPlan,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<(i64, Vec<JsonObj>), AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let alias = "t0";
        let mut filter_builder = SnowflakeStatementBuilder::new();
        let where_sql = self.where_clause_ast(
            plan.entity_type.clone(),
            alias,
            plan.filter.as_ref(),
            plan.access_filter.as_ref(),
            &mut filter_builder,
        )?;
        let order_by = sort::create_if_exists_ast(plan.entity_type.clone(), alias, &plan.sort)?;
        let select_list = native_select_list(plan.entity_type.clone(), alias);
        let table = table_ref(plan.entity_type.clone());
        let count_sql = format!(
            "SELECT COUNT(*) AS \"count\" FROM {table} AS {alias} {where_sql}",
            table = table,
            alias = quote_ident(alias),
            where_sql = where_sql,
        );
        let count_rows = self
            .run_raw_query_statement(filter_builder.clone().finish(count_sql))
            .await?;
        let total = count_rows
            .first()
            .and_then(|row| row.get("count"))
            .and_then(value_to_i64)
            .unwrap_or(0);

        let data_sql = format!(
      "SELECT {select_list} FROM {table} AS {alias} {where_sql} {order_by} LIMIT {limit} OFFSET {skip}",
      select_list = select_list,
      table = table,
      alias = quote_ident(alias),
      where_sql = where_sql,
      order_by = order_by,
      limit = plan.pagination.limit.max(0),
      skip = plan.pagination.skip.max(0),
    );
        let rows = self
            .run_entity_query_statement(filter_builder.finish(data_sql), plan.entity_type.clone())
            .await?;
        let selections = plan.selection.to_json();
        let mut projected = Vec::with_capacity(rows.len());
        for row in rows {
            projected.push(
                self.project_item(plan.entity_type.clone(), row, selections.clone(), user)
                    .await?,
            );
        }

        Ok((total, projected))
    }

    fn where_clause(
        &self,
        entity_type: Arc<EntityType>,
        alias: &str,
        filter_val: Option<Value>,
        access_filter: Option<Value>,
        builder: &mut SnowflakeStatementBuilder,
    ) -> Result<String, AppError> {
        let mut clauses = vec![];
        if let Some(filter_sql) = filter::try_create_filter(
            self.app_config.clone(),
            entity_type.clone(),
            filter_val,
            alias,
            builder,
        )? {
            if !filter_sql.trim().is_empty() && filter_sql.trim() != "1 = 1" {
                clauses.push(format!("({})", filter_sql));
            }
        }
        if let Some(access_sql) = filter::try_create_filter(
            self.app_config.clone(),
            entity_type,
            access_filter,
            alias,
            builder,
        )? {
            if !access_sql.trim().is_empty() && access_sql.trim() != "1 = 1" {
                clauses.push(format!("({})", access_sql));
            }
        }
        Ok(if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        })
    }

    fn where_clause_ast(
        &self,
        entity_type: Arc<EntityType>,
        alias: &str,
        filter_ast: Option<&FilterAst>,
        access_filter_ast: Option<&FilterAst>,
        builder: &mut SnowflakeStatementBuilder,
    ) -> Result<String, AppError> {
        let mut clauses = vec![];
        if let Some(filter_sql) = filter::try_create_filter_ast(
            self.app_config.clone(),
            entity_type.clone(),
            filter_ast,
            alias,
            builder,
        )? {
            if !filter_sql.trim().is_empty() && filter_sql.trim() != "1 = 1" {
                clauses.push(format!("({})", filter_sql));
            }
        }
        if let Some(access_sql) = filter::try_create_filter_ast(
            self.app_config.clone(),
            entity_type,
            access_filter_ast,
            alias,
            builder,
        )? {
            if !access_sql.trim().is_empty() && access_sql.trim() != "1 = 1" {
                clauses.push(format!("({})", access_sql));
            }
        }
        Ok(if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        })
    }

    fn project_item<'a>(
        &'a self,
        entity_type: Arc<EntityType>,
        row: JsonObj,
        selections: Value,
        user: &'a UserAuth,
    ) -> Pin<Box<dyn Future<Output = Result<JsonObj, AppError>> + Send + 'a>> {
        Box::pin(async move {
            let mut output = JsonObj::new();
            let selection_set = selections
                .get("selection_set")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    AppError::Validation("selection_set must be an array".to_string())
                })?;

            for selection in selection_set {
                let selection_name =
                    selection
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            AppError::Validation("selection name must be a string".to_string())
                        })?;
                let prop = AppConfig::get_prop(entity_type.clone(), &selection_name.to_string())?;
                match prop.data_type {
                    DataType::NavToOne | DataType::NavToMany => {
                        let related = self
                            .load_nav(
                                entity_type.clone(),
                                prop.clone(),
                                selection.clone(),
                                &row,
                                user,
                            )
                            .await?;
                        output.insert(prop.name.clone(), related);
                    }
                    DataType::ManyToMany => {
                        let related = self
                            .load_many_to_many(
                                entity_type.clone(),
                                prop.clone(),
                                selection.clone(),
                                &row,
                                user,
                            )
                            .await?;
                        output.insert(prop.name.clone(), related);
                    }
                    _ => {
                        if let Some(value) = row.get(&prop.name).cloned() {
                            output.insert(prop.name.clone(), value);
                        }
                    }
                }
            }
            Ok(output)
        })
    }

    async fn load_nav(
        &self,
        entity_type: Arc<EntityType>,
        prop: Arc<PropertyType>,
        selection: Value,
        row: &JsonObj,
        user: &UserAuth,
    ) -> Result<Value, AppError> {
        let nav = prop.nav_by_fk_property.clone().ok_or_else(|| {
            AppError::Validation(format!("missing navigation metadata for '{}'", prop.name))
        })?;
        let child_entity_type = self
            .app_config
            .get_nav_entity_type(entity_type.clone(), prop.clone())?;
        let child_access = self.app_config.evaluate_user_access(
            child_entity_type.clone(),
            AccessAction::Read,
            user,
        )?;
        if !child_access.allow {
            return Err(AppError::AccessDenied);
        }

        let (filter_val, limit) = if nav.type_name == entity_type.pascal_1
            && nav.schema_name == entity_type.schema_name
        {
            let child_pk_name = self
                .app_config
                .get_primary_key_name(child_entity_type.clone())?;
            let fk_value = row.get(&nav.prop_name).cloned().unwrap_or(Value::Null);
            if fk_value.is_null() {
                return Ok(if prop.data_type == DataType::NavToOne {
                    Value::Null
                } else {
                    Value::Array(vec![])
                });
            }
            (Some(json!({ child_pk_name: fk_value })), 1)
        } else {
            let parent_pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
            let parent_id = row.get(&parent_pk_name).cloned().unwrap_or(Value::Null);
            if parent_id.is_null() {
                return Ok(if prop.data_type == DataType::NavToOne {
                    Value::Null
                } else {
                    Value::Array(vec![])
                });
            }
            (
                Some(json!({ nav.prop_name.clone(): parent_id })),
                if prop.data_type == DataType::NavToOne {
                    1
                } else {
                    250
                },
            )
        };

        let (_, rows) = self
            .query_native_rows(
                child_entity_type,
                selection,
                filter_val,
                None,
                0,
                limit,
                user,
                &child_access,
            )
            .await?;
        if prop.data_type == DataType::NavToOne {
            Ok(rows
                .into_iter()
                .next()
                .map(Value::Object)
                .unwrap_or(Value::Null))
        } else {
            Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
        }
    }

    async fn load_many_to_many(
        &self,
        entity_type: Arc<EntityType>,
        prop: Arc<PropertyType>,
        selection: Value,
        row: &JsonObj,
        user: &UserAuth,
    ) -> Result<Value, AppError> {
        let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
            AppError::Validation(format!("missing many-to-many metadata for '{}'", prop.name))
        })?;
        let target_entity_type = self
            .app_config
            .get_entity_type_by_name(&many_to_many.target_schema, &many_to_many.target_type)?;
        let target_access = self.app_config.evaluate_user_access(
            target_entity_type.clone(),
            AccessAction::Read,
            user,
        )?;
        if !target_access.allow {
            return Err(AppError::AccessDenied);
        }

        let Some(statement) = self.build_many_to_many_load_statement(
            entity_type,
            target_entity_type.clone(),
            prop,
            row,
            &target_access,
        )?
        else {
            return Ok(Value::Array(vec![]));
        };
        let rows = self
            .run_entity_query_statement(statement, target_entity_type.clone())
            .await?;
        let mut projected = Vec::with_capacity(rows.len());
        for row in rows {
            projected.push(
                self.project_item(target_entity_type.clone(), row, selection.clone(), user)
                    .await?,
            );
        }
        Ok(Value::Array(
            projected.into_iter().map(Value::Object).collect(),
        ))
    }

    fn build_many_to_many_load_statement(
        &self,
        entity_type: Arc<EntityType>,
        target_entity_type: Arc<EntityType>,
        prop: Arc<PropertyType>,
        row: &JsonObj,
        target_access: &PolicyAccess,
    ) -> Result<Option<SnowflakeStatement>, AppError> {
        let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
            AppError::Validation(format!("missing many-to-many metadata for '{}'", prop.name))
        })?;
        let parent_pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let target_pk_name = self
            .app_config
            .get_primary_key_name(target_entity_type.clone())?;
        let parent_id = row.get(&parent_pk_name).cloned().unwrap_or(Value::Null);
        if parent_id.is_null() {
            return Ok(None);
        }
        let pk_prop = Self::primary_key_prop(entity_type.clone())?;
        let mut builder = SnowflakeStatementBuilder::new();
        let parent_id = builder.bind_prop(pk_prop, parent_id)?;
        let junction_schema = many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name);
        let target_alias = "t";
        let junction_alias = "j";
        let access_where = self.where_clause(
            target_entity_type.clone(),
            target_alias,
            None,
            target_access.filter.clone(),
            &mut builder,
        )?;
        let access_suffix = if access_where.is_empty() {
            String::new()
        } else {
            format!(" AND {}", access_where.trim_start_matches("WHERE "))
        };
        let sql = format!(
      "SELECT {select_list} FROM {junction_ref} AS {junction_alias} INNER JOIN {target_ref} AS {target_alias} ON {junction_alias}.{foreign_key} = {target_alias}.{target_pk} WHERE {junction_alias}.{local_key} = {parent_id}{access_suffix} ORDER BY {target_alias}.{target_pk} ASC",
      select_list = native_select_list(target_entity_type.clone(), target_alias),
      junction_ref = format!("{}.{}", quote_ident(junction_schema), quote_ident(&physical_table_name(&many_to_many.junction_table))),
      junction_alias = quote_ident(junction_alias),
      target_ref = table_ref(target_entity_type.clone()),
      target_alias = quote_ident(target_alias),
      foreign_key = quote_ident(&many_to_many.foreign_key),
      target_pk = quote_ident(&target_pk_name),
      local_key = quote_ident(&many_to_many.local_key),
      parent_id = parent_id,
      access_suffix = access_suffix,
    );
        Ok(Some(builder.finish(sql)))
    }

    fn coerce_entity_row(
        &self,
        entity_type: Arc<EntityType>,
        row: JsonObj,
    ) -> Result<JsonObj, AppError> {
        let mut out = JsonObj::new();
        for (key, value) in row {
            if let Some(prop) = AppConfig::try_get_prop(entity_type.clone(), &key) {
                out.insert(prop.name.clone(), coerce_value(prop.clone(), value)?);
            } else {
                out.insert(key, value);
            }
        }
        Ok(out)
    }

    fn extract_many_to_many_data(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Vec<(Arc<PropertyType>, Vec<JsonObj>)> {
        entity_type
            .props
            .iter()
            .filter(|p| p.data_type == DataType::ManyToMany)
            .filter_map(|p| {
                let value = input.get(&p.name)?;
                let arr = value.as_array()?;
                let records = arr
                    .iter()
                    .filter_map(|v| v.as_object().cloned())
                    .collect::<Vec<_>>();
                Some((Arc::new(p.clone()), records))
            })
            .collect()
    }

    fn has_many_to_many(entity_type: Arc<EntityType>) -> bool {
        entity_type
            .props
            .iter()
            .any(|p| p.data_type == DataType::ManyToMany)
    }

    fn primary_key_prop(entity_type: Arc<EntityType>) -> Result<Arc<PropertyType>, AppError> {
        entity_type
            .props
            .iter()
            .find(|p| p.is_key)
            .cloned()
            .map(Arc::new)
            .ok_or_else(|| {
                AppError::Validation(format!("{} has no primary key", entity_type.pascal_1))
            })
    }

    fn ensure_input_id(
        &self,
        entity_type: Arc<EntityType>,
        input: &mut JsonObj,
    ) -> Result<Value, AppError> {
        let pk = Self::primary_key_prop(entity_type)?;
        match input.get(&pk.name).cloned() {
            Some(v) if !v.is_null() => Ok(v),
            _ if matches!(
                pk.data_type,
                DataType::Uuid | DataType::String | DataType::ObjectId
            ) =>
            {
                let id = Value::String(Uuid::new_v4().to_string());
                input.insert(pk.name.clone(), id.clone());
                Ok(id)
            }
            _ => Err(AppError::Validation(format!(
                "Snowflake create requires an explicit primary key for non-string key '{}'",
                pk.name
            ))),
        }
    }

    fn mutation_entity(entity_type: Arc<EntityType>) -> SnowflakeMutationEntity {
        SnowflakeMutationEntity {
            schema: entity_type.schema_name.clone(),
            table: entity_type.snake_n.clone(),
        }
    }

    fn mutation_fields(
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Result<Vec<SnowflakeMutationField>, AppError> {
        let mut fields = Vec::new();
        for (name, value) in input {
            let prop = AppConfig::try_get_prop(entity_type.clone(), name)
                .ok_or_else(|| AppError::Validation(format!("Unknown field '{}'", name)))?;
            if !AppConfig::is_native_prop(prop.clone()) {
                continue;
            }
            fields.push(SnowflakeMutationField {
                name: prop.name.clone(),
                data_type: runtime_data_type(prop.data_type),
                is_required: prop.is_required,
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
    ) -> Result<SnowflakeStatement, AppError> {
        let entity = Self::mutation_entity(entity_type.clone());
        let fields = Self::mutation_fields(entity_type, input)?;
        let mut builder = SnowflakeStatementBuilder::new();
        let parts =
            provider_mutation_insert_parts(&fields, builder.inner_mut()).map_err(AppError::from)?;

        Ok(builder.finish(provider_mutation_insert_statement(&entity, &parts)))
    }

    fn build_update(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
        read_version: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<(SnowflakeStatement, Value, SnowflakeStatement), AppError> {
        let entity = Self::mutation_entity(entity_type.clone());
        let fields = Self::mutation_fields(entity_type.clone(), input)?;
        let mut set_builder = SnowflakeStatementBuilder::new();
        let mut where_builder = SnowflakeStatementBuilder::new();
        let update = provider_mutation_update_parts(
            &fields,
            read_version,
            set_builder.inner_mut(),
            where_builder.inner_mut(),
        )
        .map_err(AppError::from)?;
        let mut where_parts = update.where_parts.clone();

        if update.id_value.is_null() {
            return Err(AppError::Validation(
                "update requires the primary key".into(),
            ));
        }
        if update.sets.is_empty() {
            return Err(AppError::Validation(
                "update requires at least one mutable field".into(),
            ));
        }
        if let Some(access_sql) = filter::try_create_filter(
            self.app_config.clone(),
            entity_type.clone(),
            access.filter.clone(),
            "",
            &mut where_builder,
        )? {
            if !access_sql.trim().is_empty() && access_sql.trim() != "1 = 1" {
                where_parts.push(format!("({})", access_sql));
            }
        }

        let where_clause = where_parts
            .into_iter()
            .map(|part| format!("({})", part))
            .collect::<Vec<_>>()
            .join(" AND ");
        let count_statement = where_builder
            .clone()
            .finish(provider_mutation_count_statement(&entity, &where_clause));
        set_builder.append(where_builder);
        let update_statement = set_builder.finish(provider_mutation_update_statement(
            &entity,
            &update.sets,
            &where_clause,
        ));

        Ok((update_statement, update.id_value, count_statement))
    }

    fn build_delete(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
        read_version: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<(SnowflakeStatement, Value, SnowflakeStatement), AppError> {
        let entity = Self::mutation_entity(entity_type.clone());
        let fields = Self::mutation_fields(entity_type.clone(), input)?;
        let mut where_builder = SnowflakeStatementBuilder::new();
        let delete =
            provider_mutation_delete_parts(&fields, read_version, where_builder.inner_mut())
                .map_err(AppError::from)?;
        let mut where_parts = delete.where_parts.clone();

        if delete.id_value.is_null() {
            return Err(AppError::Validation(
                "delete requires the primary key".into(),
            ));
        }
        if let Some(access_sql) = filter::try_create_filter(
            self.app_config.clone(),
            entity_type.clone(),
            access.filter.clone(),
            "",
            &mut where_builder,
        )? {
            if !access_sql.trim().is_empty() && access_sql.trim() != "1 = 1" {
                where_parts.push(format!("({})", access_sql));
            }
        }

        let where_clause = where_parts
            .into_iter()
            .map(|part| format!("({})", part))
            .collect::<Vec<_>>()
            .join(" AND ");
        let count_statement = where_builder
            .clone()
            .finish(provider_mutation_count_statement(&entity, &where_clause));
        let delete_statement =
            where_builder.finish(provider_mutation_delete_statement(&entity, &where_clause));

        Ok((delete_statement, delete.id_value, count_statement))
    }

    async fn matching_count_statement(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<i64, AppError> {
        let rows = self.run_raw_query_statement(statement).await?;
        Ok(rows
            .first()
            .and_then(|r| r.get("count"))
            .and_then(value_to_i64)
            .unwrap_or(0))
    }

    async fn insert_junction_records(
        &self,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
        related_entities: &[JsonObj],
    ) -> Result<(), AppError> {
        let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
            AppError::Validation(format!("missing many-to-many metadata for '{}'", prop.name))
        })?;
        let target_entity_type = self
            .app_config
            .get_entity_type_by_name(&many_to_many.target_schema, &many_to_many.target_type)?;
        let target_pk = self
            .app_config
            .get_primary_key_name(target_entity_type.clone())?;
        let entity_pk = Self::primary_key_prop(entity_type.clone())?;
        let target_pk_prop = Self::primary_key_prop(target_entity_type)?;

        for related in related_entities {
            let statement = self.build_junction_insert_statement(
                entity_type.clone(),
                entity_id.clone(),
                prop.clone(),
                many_to_many.clone(),
                entity_pk.clone(),
                target_pk_prop.clone(),
                &target_pk,
                related,
            )?;
            self.run_exec_statement(statement).await?;
        }
        Ok(())
    }

    fn build_junction_insert_statement(
        &self,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
        many_to_many: crate::schemas::system::ManyToManyProperty,
        entity_pk: Arc<PropertyType>,
        target_pk_prop: Arc<PropertyType>,
        target_pk: &str,
        related: &JsonObj,
    ) -> Result<SnowflakeStatement, AppError> {
        let related_id = related.get(target_pk).cloned().ok_or_else(|| {
            AppError::Validation(format!("related '{}' record is missing id", prop.name))
        })?;
        let junction_schema = many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name);
        let junction_table = format!(
            "{}.{}",
            quote_ident(junction_schema),
            quote_ident(&physical_table_name(&many_to_many.junction_table))
        );
        let mut builder = SnowflakeStatementBuilder::new();
        let new_id = builder.bind_scalar(
            DataType::String,
            "id",
            true,
            Value::String(Uuid::new_v4().to_string()),
        )?;
        let entity_id_select = builder.bind_prop(entity_pk.clone(), entity_id.clone())?;
        let related_id_select = builder.bind_prop(target_pk_prop.clone(), related_id.clone())?;
        let entity_id_exists = builder.bind_prop(entity_pk.clone(), entity_id.clone())?;
        let related_id_exists = builder.bind_prop(target_pk_prop.clone(), related_id)?;
        let sql = format!(
        "INSERT INTO {junction_table} ({id_col}, {local_key}, {foreign_key}, {created_at}) SELECT {new_id}, {entity_id}, {related_id}, CURRENT_TIMESTAMP() WHERE NOT EXISTS (SELECT 1 FROM {junction_table} WHERE {local_key} = {entity_id_exists} AND {foreign_key} = {related_id_exists})",
        junction_table = junction_table,
        id_col = quote_ident("id"),
        local_key = quote_ident(&many_to_many.local_key),
        foreign_key = quote_ident(&many_to_many.foreign_key),
        created_at = quote_ident("created_at"),
        new_id = new_id,
        entity_id = entity_id_select,
        related_id = related_id_select,
        entity_id_exists = entity_id_exists,
        related_id_exists = related_id_exists,
      );
        Ok(builder.finish(sql))
    }

    async fn update_junction_records(
        &self,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
        related_entities: &[JsonObj],
    ) -> Result<(), AppError> {
        let statement = self.build_junction_delete_statement(
            entity_type.clone(),
            entity_id.clone(),
            prop.clone(),
        )?;
        self.run_exec_statement(statement).await?;
        self.insert_junction_records(entity_type, entity_id, prop, related_entities)
            .await
    }

    fn build_junction_delete_statement(
        &self,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
    ) -> Result<SnowflakeStatement, AppError> {
        let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
            AppError::Validation(format!("missing many-to-many metadata for '{}'", prop.name))
        })?;
        let entity_pk = Self::primary_key_prop(entity_type.clone())?;
        let junction_schema = many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name);
        let junction_table = format!(
            "{}.{}",
            quote_ident(junction_schema),
            quote_ident(&physical_table_name(&many_to_many.junction_table))
        );
        let mut builder = SnowflakeStatementBuilder::new();
        let entity_id_placeholder = builder.bind_prop(entity_pk, entity_id)?;
        let sql = format!(
            "DELETE FROM {junction_table} WHERE {local_key} = {entity_id}",
            junction_table = junction_table,
            local_key = quote_ident(&many_to_many.local_key),
            entity_id = entity_id_placeholder,
        );
        Ok(builder.finish(sql))
    }

    async fn delete_junction_records(
        &self,
        entity_type: Arc<EntityType>,
        entity_id: Value,
    ) -> Result<(), AppError> {
        for prop in entity_type
            .props
            .iter()
            .filter(|p| p.data_type == DataType::ManyToMany)
        {
            if let Some(many_to_many) = &prop.many_to_many_property {
                let mut prop = (*prop).clone();
                prop.many_to_many_property = Some(many_to_many.clone());
                let statement = self.build_junction_delete_statement(
                    entity_type.clone(),
                    entity_id.clone(),
                    Arc::new(prop),
                )?;
                self.run_exec_statement(statement).await?;
            }
        }
        Ok(())
    }

    async fn refresh_account_health_provider_routine(
        &self,
        arguments: &[ProviderRoutineArgument],
    ) -> Result<Value, AppError> {
        let account_id = routine_string_argument(arguments, "account_id")?;
        let health_score = routine_f64_argument(arguments, "health_score")?;
        let refreshed_at = OffsetDateTime::now_utc().to_string();

        let mut count_builder = SnowflakeStatementBuilder::new();
        let account_id_count = count_builder.bind_scalar(
            DataType::String,
            "account_id",
            true,
            Value::String(account_id.clone()),
        )?;
        let count_statement = count_builder.finish(format!(
            "SELECT COUNT(*) AS {} FROM {} WHERE {} = {}",
            quote_ident("count"),
            table_ref_name("crm", "accounts"),
            quote_ident("id"),
            account_id_count,
        ));
        if self.matching_count_statement(count_statement).await? == 0 {
            return Err(AppError::DataAccess(format!(
                "account `{account_id}` was not found"
            )));
        }

        let mut builder = SnowflakeStatementBuilder::new();
        let score =
            builder.bind_scalar(DataType::Float64, "health_score", true, json!(health_score))?;
        let refreshed = builder.bind_scalar(
            DataType::DateTime,
            "health_last_refreshed_at",
            true,
            Value::String(refreshed_at.clone()),
        )?;
        let account_id_update = builder.bind_scalar(
            DataType::String,
            "account_id",
            true,
            Value::String(account_id.clone()),
        )?;
        let statement = builder.finish(format!(
            "UPDATE {} SET {} = {}, {} = {}, {} = COALESCE({}, 0) + 1 WHERE {} = {}",
            table_ref_name("crm", "accounts"),
            quote_ident("health_score"),
            score,
            quote_ident("health_last_refreshed_at"),
            refreshed,
            quote_ident("version"),
            quote_ident("version"),
            quote_ident("id"),
            account_id_update,
        ));
        self.run_exec_statement(statement).await?;

        Ok(json!({
            "account_id": account_id,
            "health_score": health_score,
            "refreshed_at": refreshed_at,
            "source": "snowflake-stored-procedure",
        }))
    }
}

impl RuntimeProviderIdentity for SnowflakeClient {
    fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    fn framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::Snowflake
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        self.provider_descriptor().opaque_pool_stats()
    }
}

#[async_trait]
impl DatabaseClient for SnowflakeClient {
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
        if routine.kind != ProviderRoutineKind::Procedure {
            return Err(AppError::DataAccess(format!(
                "provider routine `{}` is not a Snowflake stored procedure",
                routine.method_name
            )));
        }
        if self.localstack_endpoint
            && routine.method_name == "refresh_account_health_stored_procedure"
        {
            return self
                .refresh_account_health_provider_routine(arguments)
                .await;
        }
        let routine_name = routine.snowflake.ok_or_else(|| {
            AppError::DataAccess(format!(
                "provider routine `{}` has no Snowflake binding",
                routine.method_name
            ))
        })?;
        let statement = snowflake_routine_statement(routine_name, arguments)?;

        match routine.returns {
            ProviderRoutineReturns::None => {
                self.execution
                    .call_stored_procedure(statement)
                    .await
                    .map_err(AppError::from)?;
                Ok(Value::Null)
            }
            ProviderRoutineReturns::One => {
                let rows = self
                    .execution
                    .call_stored_procedure(statement)
                    .await
                    .map_err(AppError::from)?;
                Ok(snowflake_routine_one(rows))
            }
            ProviderRoutineReturns::Many => {
                let rows = self
                    .execution
                    .call_stored_procedure(statement)
                    .await
                    .map_err(AppError::from)?;
                Ok(Value::Array(
                    rows.into_iter().map(snowflake_routine_row_value).collect(),
                ))
            }
        }
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
        let request_start = std::time::Instant::now();
        let alias = "t0";
        let mut builder = SnowflakeStatementBuilder::new();
        let groups = snowflake_aggregate_groups(&plan);
        let metrics = snowflake_aggregate_metrics(&plan);
        let having_input = snowflake_aggregate_having_input(&plan);
        let where_sql = self.where_clause_ast(
            plan.entity_type.clone(),
            alias,
            plan.filter.as_ref(),
            plan.access_filter.as_ref(),
            &mut builder,
        )?;
        let select_list =
            provider_aggregate_select_list(&groups, &metrics, alias).map_err(AppError::from)?;
        let group_by = provider_aggregate_group_by(&groups, alias);
        let having =
            provider_aggregate_having(&metrics, having_input.as_ref(), alias, builder.inner_mut())
                .map_err(AppError::from)?;
        let order_by = snowflake_aggregate_order_by(&plan);
        let table = table_ref(plan.entity_type.clone());
        let base_sql = format!(
            "SELECT {select_list} FROM {table} AS {alias} {where_sql} {group_by} {having}",
            select_list = select_list,
            table = table,
            alias = quote_ident(alias),
            where_sql = where_sql,
            group_by = group_by,
            having = having,
        );

        let count_sql = format!(
            "SELECT COUNT(*) AS \"count\" FROM ({base_sql}) AS {agg_alias}",
            base_sql = base_sql,
            agg_alias = quote_ident("agg")
        );
        let count_rows = self
            .run_raw_query_statement(builder.clone().finish(count_sql))
            .await?;
        let total = count_rows
            .first()
            .and_then(|row| row.get("count"))
            .and_then(value_to_i64)
            .unwrap_or(0);

        let data_sql = format!(
            "{base_sql} {order_by} LIMIT {limit} OFFSET {skip}",
            base_sql = base_sql,
            order_by = order_by,
            limit = plan.pagination.limit.max(0),
            skip = plan.pagination.skip.max(0),
        );
        let rows = self
            .run_raw_query_statement(builder.finish(data_sql))
            .await?;
        let items = rows.into_iter().map(Value::Object).collect();

        Ok(JsonAggregateResult::new(
            plan.pagination.skip,
            plan.pagination.limit,
            total,
            items,
            request_datetime,
            request_start,
        ))
    }

    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<Vec<JsonObj>, AppError> {
        let (plan, user, access) = input.into_parts();
        let (total, rows) = self.query_native_rows_plan(&plan, user, access).await?;
        debug!(
            query_count = total,
            item_count = rows.len(),
            "got Snowflake items from query plan"
        );
        Ok(rows)
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
        let request_start = std::time::Instant::now();
        let skip = plan.pagination.skip;
        let limit = plan.pagination.limit;
        let (total, items) = self.query_native_rows_plan(&plan, user, access).await?;
        Ok(JsonQueryResult::new(
            skip,
            limit,
            total,
            items,
            request_datetime,
            request_start,
        ))
    }

    async fn create_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        mut input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input);
        let id_value = self.ensure_input_id(entity_type.clone(), &mut input)?;
        let statement = self.build_insert(entity_type.clone(), &input)?;
        self.run_exec_statement(statement).await?;
        for (prop, related_entities) in many_to_many_data {
            self.insert_junction_records(
                entity_type.clone(),
                id_value.clone(),
                prop,
                &related_entities,
            )
            .await?;
        }

        self.find_item_json(
            entity_type,
            selections,
            id_value_to_string(id_value)?,
            user,
            access,
        )
        .await?
        .ok_or_else(|| {
            AppError::InternalServerError(anyhow::anyhow!("could not refetch created row").into())
        })
    }

    async fn update_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<JsonObj, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input);
        let (statement, id_value, count_statement) =
            self.build_update(entity_type.clone(), &input, read_version, access)?;
        if self.matching_count_statement(count_statement).await? == 0 {
            return Err(AppError::InvalidKeyOrVersion);
        }
        self.run_exec_statement(statement).await?;
        for (prop, related_entities) in many_to_many_data {
            self.update_junction_records(
                entity_type.clone(),
                id_value.clone(),
                prop,
                &related_entities,
            )
            .await?;
        }

        self.find_item_json(
            entity_type,
            selections,
            id_value_to_string(id_value)?,
            user,
            access,
        )
        .await?
        .ok_or_else(|| {
            AppError::InternalServerError(anyhow::anyhow!("could not refetch updated row").into())
        })
    }

    async fn delete_item_json(
        &self,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        _user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        let (statement, id_value, count_statement) =
            self.build_delete(entity_type.clone(), &input, read_version, access)?;
        if self.matching_count_statement(count_statement).await? == 0 {
            return Err(AppError::InvalidKeyOrVersion);
        }
        if Self::has_many_to_many(entity_type.clone()) {
            self.delete_junction_records(entity_type.clone(), id_value)
                .await?;
        }
        self.run_exec_statement(statement).await?;
        Ok(1)
    }

    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let id_val = if let Ok(n) = id.parse::<i64>() {
            Value::Number(serde_json::Number::from(n))
        } else {
            Value::String(id)
        };
        let (_, rows) = self
            .query_native_rows(
                entity_type,
                selections,
                Some(json!({ pk_name: id_val })),
                None,
                0,
                1,
                user,
                access,
            )
            .await?;
        Ok(rows.into_iter().next())
    }

    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter_val: Option<Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError> {
        let res = self
            .query_items_json(
                entity_type,
                selections,
                filter_val,
                None,
                0,
                250,
                user,
                access,
            )
            .await?;
        Ok(res.items)
    }

    async fn query_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter_val: Option<Value>,
        sort_val: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        let request_datetime = OffsetDateTime::now_utc();
        let request_start = std::time::Instant::now();
        let (total, items) = self
            .query_native_rows(
                entity_type,
                selections,
                filter_val,
                sort_val,
                skip,
                limit,
                user,
                access,
            )
            .await?;
        Ok(JsonQueryResult::new(
            skip,
            limit,
            total,
            items,
            request_datetime,
            request_start,
        ))
    }
}

fn session_config(env: &DataSourceEnvironment) -> Result<SnowflakeSessionConfig, AppError> {
    let host = optional_env("SNOWFLAKE_HOST").unwrap_or_else(|| env.db_host.clone());
    connection_security::validate(
        Provider::Snowflake,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &host,
    )
    .map_err(|e| AppError::DataAccess(format!("invalid Snowflake connection security: {e}")))?;
    let connection = SnowflakeConnectionConfig::new(
        host,
        env.db_port.clone(),
        env.db_name.clone(),
        optional_env("SNOWFLAKE_WAREHOUSE"),
        optional_env("SNOWFLAKE_ROLE"),
        env.service_account_password.clone(),
        env::var("SNOWFLAKE_AUTH_TOKEN_TYPE").ok(),
        env::var("SNOWFLAKE_STATEMENT_TIMEOUT_SECS").ok(),
    );
    provider_snowflake_session_config(&connection).map_err(AppError::from)
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn coerce_value(prop: Arc<PropertyType>, value: Value) -> Result<Value, AppError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let s = match value {
        Value::String(s) => s,
        other => return Ok(other),
    };
    match prop.data_type {
        DataType::Boolean => Ok(Value::Bool(matches!(
            s.to_lowercase().as_str(),
            "true" | "1"
        ))),
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            let i = s.parse::<i64>().map_err(|e| {
                AppError::DataAccess(format!(
                    "invalid Snowflake integer for '{}': {}",
                    prop.name, e
                ))
            })?;
            Ok(Value::Number(serde_json::Number::from(i)))
        }
        DataType::Float32 | DataType::Float64 => {
            let f = s.parse::<f64>().map_err(|e| {
                AppError::DataAccess(format!(
                    "invalid Snowflake float for '{}': {}",
                    prop.name, e
                ))
            })?;
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .ok_or_else(|| {
                    AppError::DataAccess(format!("invalid Snowflake float for '{}'", prop.name))
                })
        }
        DataType::DateTime => Ok(coerce_datetime_string(&s)),
        DataType::Object
        | DataType::ObjectArray
        | DataType::Json
        | DataType::JsonArray
        | DataType::StringArray
        | DataType::UuidArray
        | DataType::ObjectIdArray
        | DataType::Int8Array
        | DataType::Int16Array
        | DataType::Int32Array
        | DataType::Int64Array
        | DataType::EnumArray => Ok(serde_json::from_str(&s).unwrap_or(Value::String(s))),
        _ => Ok(Value::String(s)),
    }
}

fn coerce_datetime_string(value: &str) -> Value {
    parse_snowflake_timestamp_tz(value)
        .map(|dt| Value::String(dt.to_rfc3339()))
        .unwrap_or_else(|| Value::String(value.to_string()))
}

fn parse_snowflake_timestamp_tz(value: &str) -> Option<chrono::DateTime<Utc>> {
    let epoch = value.split_whitespace().next()?;
    let (secs, nanos) = match epoch.split_once('.') {
        Some((secs, frac)) => {
            let nanos = format!("{frac:0<9}").get(..9)?.parse::<u32>().ok()?;
            (secs.parse::<i64>().ok()?, nanos)
        }
        None => (epoch.parse::<i64>().ok()?, 0),
    };
    Utc.timestamp_opt(secs, nanos).single()
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

fn id_value_to_string(value: Value) -> Result<String, AppError> {
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        other => Err(AppError::Validation(format!(
            "unsupported id value {:?}",
            other
        ))),
    }
}

fn native_select_list(entity_type: Arc<EntityType>, alias: &str) -> String {
    let mut projections = entity_type
        .props
        .iter()
        .filter(|p| AppConfig::is_native_prop(Arc::new((*p).clone())))
        .map(|p| native_select_projection(alias, &p.name, p.data_type, p.is_required))
        .collect::<Vec<_>>();
    if entity_type.is_table && !entity_type.snake_n.ends_with("_audit") {
        projections.push(native_select_projection(
            alias,
            RECORD_LOCATOR_FIELD,
            DataType::String,
            true,
        ));
    }
    projections.join(", ")
}

fn native_select_projection(
    alias: &str,
    name: &str,
    data_type: DataType,
    is_required: bool,
) -> String {
    let qualified = format!("{}.{}", quote_ident(alias), quote_ident(name));
    let expression = if !is_required
        && matches!(
            data_type,
            DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::Float32
                | DataType::Float64
                | DataType::Date
                | DataType::Time
                | DataType::DateTime
        ) {
        format!("TO_VARCHAR({qualified})")
    } else {
        qualified
    };
    format!("{expression} AS {}", quote_ident(name))
}

fn snowflake_routine_statement(
    routine_name: crate::data::clients::database_client::ProviderRoutineName,
    arguments: &[ProviderRoutineArgument],
) -> Result<SnowflakeStatement, AppError> {
    let mut builder = ProviderSnowflakeStatementBuilder::new();
    let mut expressions = Vec::with_capacity(arguments.len());
    for argument in arguments {
        expressions.push(builder.bind_scalar(
            argument.data_type,
            argument.name,
            argument.required,
            argument.value.clone(),
        )?);
    }
    let call =
        SnowflakeStoredProcedureCall::new(routine_name.schema, routine_name.name, expressions)
            .map_err(AppError::from)?;
    call.statement(builder).map_err(AppError::from)
}

fn snowflake_routine_one(rows: Vec<JsonObj>) -> Value {
    rows.into_iter()
        .next()
        .map(snowflake_routine_row_value)
        .unwrap_or(Value::Null)
}

fn snowflake_routine_row_value(row: JsonObj) -> Value {
    if row.len() == 1 {
        row.into_iter()
            .next()
            .map(|(_, value)| value)
            .unwrap_or(Value::Null)
    } else {
        Value::Object(row)
    }
}

fn snowflake_aggregate_groups(plan: &ProviderAggregatePlan) -> Vec<SnowflakeAggregateGroup> {
    plan.group_by
        .iter()
        .map(|group| SnowflakeAggregateGroup {
            name: group.prop.name.clone(),
            alias: group.alias.clone(),
        })
        .collect()
}

fn snowflake_aggregate_metrics(plan: &ProviderAggregatePlan) -> Vec<SnowflakeAggregateMetric> {
    plan.metrics
        .iter()
        .map(|metric| SnowflakeAggregateMetric {
            function: metric.function,
            field_name: metric.prop.as_ref().map(|prop| prop.name.clone()),
            alias: metric.alias.clone(),
            value_data_type: runtime_data_type(metric.value_data_type()),
        })
        .collect()
}

fn snowflake_aggregate_having_input(
    plan: &ProviderAggregatePlan,
) -> Option<SnowflakeAggregateHaving> {
    plan.having.as_ref().map(|having| SnowflakeAggregateHaving {
        predicates: having
            .predicates
            .iter()
            .map(|predicate| SnowflakeAggregateHavingPredicate {
                metric_alias: predicate.metric_alias.clone(),
                op: predicate.op,
                value: predicate.value.clone(),
            })
            .collect(),
    })
}

fn snowflake_aggregate_order_by(plan: &ProviderAggregatePlan) -> String {
    let fields = plan
        .sort
        .specs
        .iter()
        .map(|spec| SnowflakeSortField {
            name: spec.alias.clone(),
            direction: spec.direction,
        })
        .collect::<Vec<_>>();
    provider_aggregate_order_by(&fields)
}

fn table_ref(entity_type: Arc<EntityType>) -> String {
    table_ref_name(&entity_type.schema_name, &entity_type.snake_n)
}

fn table_ref_name(schema_name: &str, table_name: &str) -> String {
    format!("{}.{}", quote_ident(schema_name), quote_ident(table_name))
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn routine_argument<'a>(
    arguments: &'a [ProviderRoutineArgument],
    name: &str,
) -> Result<&'a Value, AppError> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
        .ok_or_else(|| AppError::Validation(format!("missing routine argument `{name}`")))
}

fn routine_string_argument(
    arguments: &[ProviderRoutineArgument],
    name: &str,
) -> Result<String, AppError> {
    routine_argument(arguments, name)?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| AppError::Validation(format!("routine argument `{name}` must be a string")))
}

fn routine_f64_argument(
    arguments: &[ProviderRoutineArgument],
    name: &str,
) -> Result<f64, AppError> {
    routine_argument(arguments, name)?
        .as_f64()
        .ok_or_else(|| AppError::Validation(format!("routine argument `{name}` must be a number")))
}
