// MS SQL Server `DatabaseClient` implementation backed by ODBC (`odbc-api`)
// with Microsoft ODBC Driver 18 (`sql_password`) or FreeTDS (`ntlm`).
//
// SCOPE (v1):
//   - Full flat CRUD on the framework's entity_types.
//   - Explicit-version optimistic concurrency (per Wayne's choice; an i64
//     `version` column is the concurrency token).
//   - Filter / sort over flat columns via the local filter.rs + sort.rs.
//
// NOT YET (tracked TODOs):
//   - Many-to-many junction inserts/updates/deletes inside CRUD operations.
//     Postgres handles these via tokio_postgres::Transaction; ODBC sessions
//     use BEGIN/COMMIT TRAN on the same pooled ODBC connection.
//   - Nested-nav projections (NavToOne / NavToMany) — see mssql/cte.rs.

#![allow(dead_code)]

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info};

use appfw_provider_mssql::{
    aggregate_group_by as provider_aggregate_group_by,
    aggregate_having as provider_aggregate_having,
    aggregate_order_by as provider_aggregate_order_by,
    aggregate_select_list as provider_aggregate_select_list,
    mssql_connection_config_resolving_auth as provider_mssql_connection_config_resolving_auth,
    mutation_delete_statement as provider_mutation_delete_statement,
    mutation_insert_statement as provider_mutation_insert_statement,
    mutation_update_statement as provider_mutation_update_statement,
    type_param as provider_type_param, MssqlAggregateGroup, MssqlAggregateHaving,
    MssqlAggregateHavingPredicate, MssqlAggregateMetric, MssqlConnection, MssqlConnectionConfig,
    MssqlExecutionClient, MssqlMutationEntity, MssqlMutationField, MssqlRow, MssqlSortField,
    MssqlStoredProcedureCall, MSSQL_POOL_MAX_SIZE,
};
use appfw_runtime::{
    connection_security::{self, Provider},
    extension::UserAuth,
    json::JsonObj,
    provider_keys::FrameworkProvider,
    PolicyAccess, RuntimeAuditEvent, RuntimeAuditQuery, RuntimeProviderIdentity,
    RuntimeProviderPlanInput,
};

use crate::{
    config::app_config::AppConfig,
    data::{
        clients::database_client::{
            DatabaseClient, ProviderAggregatePlan, ProviderPoolStats, ProviderQueryPlan,
            ProviderRoutineArgument, ProviderRoutineCall, ProviderRoutineKind,
            ProviderRoutineReturns,
        },
        clients::mssql::{cte, filter, naming, param},
    },
    product_api::runtime_data_type,
    routes::app_error::AppError,
    schemas::common::{JsonAggregateResult, JsonQueryResult},
    schemas::system::{DataSourceEnvironment, DataType, EntityType, PropertyType},
};

use std::time::Instant;
use time::OffsetDateTime;

use param::SqlParam;

fn validate_connection_security(
    env: &DataSourceEnvironment,
    framework_provider: FrameworkProvider,
) -> Result<connection_security::ConnectionSecurity, AppError> {
    let provider = if framework_provider == FrameworkProvider::FabricSqlAnalytics {
        Provider::FabricSqlAnalytics
    } else {
        Provider::MsSqlServer
    };
    connection_security::validate(
        provider,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| {
        AppError::DataAccess(format!(
            "invalid {} connection security: {e}",
            framework_provider.key()
        ))
    })
}

fn mssql_connection_config(
    env: &DataSourceEnvironment,
    framework_provider: FrameworkProvider,
) -> Result<MssqlConnectionConfig, AppError> {
    let provider = match framework_provider {
        FrameworkProvider::FabricSqlAnalytics => appfw_mssql_auth::PROVIDER_FABRIC,
        _ => appfw_mssql_auth::PROVIDER_MSSQL,
    };
    MssqlConnectionConfig::from_environment(
        provider,
        env.db_host.clone(),
        env.db_port.clone(),
        env.db_name.clone(),
        env.auth_mode.as_deref(),
        env.service_account_name.clone(),
        env.service_account_password.clone(),
        env.entra_tenant_id.clone(),
        env.entra_token_scope.clone(),
    )
    .map_err(|e| AppError::DataAccess(e.to_string()))
}

pub struct MsSqlClient {
    execution: MssqlExecutionClient,
    app_config: Arc<AppConfig>,
    data_source_name: String,
    framework_provider: FrameworkProvider,
    read_only: bool,
}

impl MsSqlClient {
    #[tracing::instrument(skip(app_config), fields(data_source = %data_source_name))]
    pub async fn init(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<MsSqlClient, AppError> {
        Self::init_with_mode(
            app_config,
            data_source_name,
            FrameworkProvider::Mssql,
            false,
        )
        .await
    }

    #[tracing::instrument(skip(app_config), fields(data_source = %data_source_name))]
    pub async fn init_fabric_sql_analytics(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<MsSqlClient, AppError> {
        Self::init_with_mode(
            app_config,
            data_source_name,
            FrameworkProvider::FabricSqlAnalytics,
            true,
        )
        .await
    }

    async fn init_with_mode(
        app_config: Arc<AppConfig>,
        data_source_name: String,
        framework_provider: FrameworkProvider,
        read_only: bool,
    ) -> Result<MsSqlClient, AppError> {
        let env = app_config.get_data_source_env(&data_source_name)?;
        let execution = Self::get_pool(env, framework_provider).await?;
        Ok(MsSqlClient {
            execution,
            app_config,
            data_source_name,
            framework_provider,
            read_only,
        })
    }

    #[tracing::instrument(skip(env), fields(host = %env.db_host, database = %env.db_name))]
    async fn get_pool(
        env: Arc<DataSourceEnvironment>,
        framework_provider: FrameworkProvider,
    ) -> Result<MssqlExecutionClient, AppError> {
        if framework_provider == FrameworkProvider::FabricSqlAnalytics {
            return Err(AppError::DataAccess(
                "FabricSqlAnalytics must use FabricOdbcExecutionClient, not MsSqlClient"
                    .to_string(),
            ));
        }
        let security = validate_connection_security(&env, framework_provider)?;
        let connection = mssql_connection_config(&env, framework_provider)?;
        let execution = MssqlExecutionClient::from_connection_config(&connection, &security)
            .await
            .map_err(AppError::from)?;
        info!(
            provider = framework_provider.key(),
            "MS SQL-compatible connection pool configured"
        );
        Ok(execution)
    }

    fn ensure_writable(&self, operation: &str) -> Result<(), AppError> {
        if self.read_only {
            Err(AppError::DataAccess(format!(
                "{operation} is not supported for FabricSqlAnalytics read-only data sources"
            )))
        } else {
            Ok(())
        }
    }

    fn extract_many_to_many_data(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Vec<(Arc<PropertyType>, Vec<Value>)> {
        entity_type
            .props
            .iter()
            .filter(|prop| prop.data_type == DataType::ManyToMany)
            .filter_map(|prop| {
                input
                    .get(&prop.name)
                    .and_then(|value| value.as_array())
                    .map(|array| (Arc::new(prop.clone()), array.clone()))
            })
            .collect()
    }

    fn has_many_to_many(entity_type: Arc<EntityType>) -> bool {
        entity_type
            .props
            .iter()
            .any(|prop| prop.data_type == DataType::ManyToMany)
    }

    fn extract_entity_id_value(&self, entity_value: &Value) -> Result<Value, AppError> {
        match entity_value {
            Value::Object(obj) => obj.get("id").cloned().ok_or_else(|| {
                AppError::Validation("Related entity must have an 'id' field".into())
            }),
            Value::Number(_) | Value::String(_) => Ok(entity_value.clone()),
            _ => Err(AppError::Validation("Invalid related entity format".into())),
        }
    }

    fn primary_key_prop(entity_type: Arc<EntityType>) -> Result<Arc<PropertyType>, AppError> {
        entity_type
            .props
            .iter()
            .find(|prop| prop.is_key)
            .map(|prop| Arc::new(prop.clone()))
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "Entity '{}' is missing a primary key",
                    entity_type.pascal_1
                ))
            })
    }

    async fn insert_junction_records_on_conn(
        &self,
        conn: &mut MssqlConnection<'_>,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
        related_entities: &[Value],
    ) -> Result<(), AppError> {
        let many_to_many = prop.many_to_many_property.as_ref().ok_or_else(|| {
            AppError::InternalServerError(
                anyhow::anyhow!("ManyToMany property missing configuration").into(),
            )
        })?;
        let target_entity_type = self
            .app_config
            .get_entity_type_by_name(&many_to_many.target_schema, &many_to_many.target_type)?;
        let local_pk_prop = Self::primary_key_prop(entity_type.clone())?;
        let foreign_pk_prop = Self::primary_key_prop(target_entity_type.clone())?;
        let junction_schema = many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name);
        let junction_table = naming::physical_table_name(&many_to_many.junction_table);

        for (index, related_entity) in related_entities.iter().enumerate() {
            let related_id = self.extract_entity_id_value(related_entity).map_err(|e| {
                AppError::Validation(format!("Invalid related entity at index {}: {}", index, e))
            })?;
            let params = vec![
                param::prop_param(local_pk_prop.clone(), entity_id.clone())?,
                param::prop_param(foreign_pk_prop.clone(), related_id)?,
            ];
            let sql = format!(
                "INSERT INTO [{schema}].[{table}] ([{local_key}], [{foreign_key}], [created_at]) \
         SELECT @P1, @P2, SYSDATETIMEOFFSET() \
         WHERE NOT EXISTS ( \
           SELECT 1 FROM [{schema}].[{table}] \
           WHERE [{local_key}] = @P1 AND [{foreign_key}] = @P2 \
         );",
                schema = junction_schema,
                table = junction_table,
                local_key = many_to_many.local_key,
                foreign_key = many_to_many.foreign_key,
            );
            MssqlExecutionClient::run_execute_on_conn(conn, &sql, params)
                .await
                .map_err(AppError::from)?;
        }

        Ok(())
    }

    async fn update_junction_records_on_conn(
        &self,
        conn: &mut MssqlConnection<'_>,
        entity_type: Arc<EntityType>,
        entity_id: Value,
        prop: Arc<PropertyType>,
        related_entities: &[Value],
    ) -> Result<(), AppError> {
        let many_to_many = prop.many_to_many_property.as_ref().ok_or_else(|| {
            AppError::InternalServerError(
                anyhow::anyhow!("ManyToMany property missing configuration").into(),
            )
        })?;
        let local_pk_prop = Self::primary_key_prop(entity_type.clone())?;
        let junction_schema = many_to_many
            .junction_schema
            .as_ref()
            .unwrap_or(&entity_type.schema_name);
        let junction_table = naming::physical_table_name(&many_to_many.junction_table);

        let delete_sql = format!(
            "DELETE FROM [{schema}].[{table}] WHERE [{local_key}] = @P1;",
            schema = junction_schema,
            table = junction_table,
            local_key = many_to_many.local_key,
        );
        MssqlExecutionClient::run_execute_on_conn(
            conn,
            &delete_sql,
            vec![param::prop_param(local_pk_prop, entity_id.clone())?],
        )
        .await
        .map_err(AppError::from)?;

        self.insert_junction_records_on_conn(conn, entity_type, entity_id, prop, related_entities)
            .await
    }

    async fn delete_junction_records_on_conn(
        &self,
        conn: &mut MssqlConnection<'_>,
        entity_type: Arc<EntityType>,
        entity_id: Value,
    ) -> Result<(), AppError> {
        let local_pk_prop = Self::primary_key_prop(entity_type.clone())?;

        for prop in &entity_type.props {
            if prop.data_type != DataType::ManyToMany {
                continue;
            }
            let Some(many_to_many) = &prop.many_to_many_property else {
                continue;
            };
            let junction_schema = many_to_many
                .junction_schema
                .as_ref()
                .unwrap_or(&entity_type.schema_name);
            let junction_table = naming::physical_table_name(&many_to_many.junction_table);
            let sql = format!(
                "DELETE FROM [{schema}].[{table}] WHERE [{local_key}] = @P1;",
                schema = junction_schema,
                table = junction_table,
                local_key = many_to_many.local_key,
            );
            MssqlExecutionClient::run_execute_on_conn(
                conn,
                &sql,
                vec![param::prop_param(local_pk_prop.clone(), entity_id.clone())?],
            )
            .await
            .map_err(AppError::from)?;
        }

        Ok(())
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
    ) -> Result<(String, Vec<SqlParam>), AppError> {
        let entity = self.mutation_entity(entity_type.clone())?;
        let fields = Self::mutation_fields(entity_type, input)?;
        provider_mutation_update_statement(&entity, &fields, read_version, "")
            .map_err(AppError::from)
    }

    fn build_delete(
        &self,
        entity_type: Arc<EntityType>,
        input: &JsonObj,
        read_version: Option<Value>,
    ) -> Result<(String, Vec<SqlParam>), AppError> {
        let entity = self.mutation_entity(entity_type.clone())?;
        let fields = Self::mutation_fields(entity_type, input)?;
        provider_mutation_delete_statement(&entity, &fields, read_version, "")
            .map_err(AppError::from)
    }

    fn mutation_entity(
        &self,
        entity_type: Arc<EntityType>,
    ) -> Result<MssqlMutationEntity, AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let pk_prop = AppConfig::get_prop(entity_type.clone(), &pk_name)?;
        Ok(MssqlMutationEntity {
            schema: naming::entity_schema(&entity_type),
            table: naming::entity_table(&entity_type),
            primary_key: naming::property_column(&pk_prop),
        })
    }

    fn mutation_fields(
        entity_type: Arc<EntityType>,
        input: &JsonObj,
    ) -> Result<Vec<MssqlMutationField>, AppError> {
        let mut fields = Vec::new();
        for (name, value) in input {
            let prop = AppConfig::try_get_prop(entity_type.clone(), name)
                .ok_or_else(|| AppError::Validation(format!("Unknown field '{}'", name)))?;
            if !AppConfig::is_native_prop(prop.clone()) {
                continue;
            }
            fields.push(MssqlMutationField {
                name: naming::property_column(&prop),
                data_type: runtime_data_type(prop.data_type),
                is_nullable: !prop.is_required,
                is_key: prop.is_key,
                is_concurrency_control: prop.is_concurrency_control,
                value: value.clone(),
            });
        }
        Ok(fields)
    }
}

fn mssql_aggregate_query(
    app_config: Arc<AppConfig>,
    plan: &ProviderAggregatePlan,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let alias = "t0";
    let groups = mssql_aggregate_groups(plan);
    let metrics = mssql_aggregate_metrics(plan);
    let having_input = mssql_aggregate_having_input(plan);
    let select_list =
        provider_aggregate_select_list(&groups, &metrics, alias).map_err(AppError::from)?;
    let table = naming::table_ref(&plan.entity_type);
    let where_sql = mssql_aggregate_where(app_config, plan, alias, params)?;
    let group_by = provider_aggregate_group_by(&groups, alias);
    let having = provider_aggregate_having(&metrics, having_input.as_ref(), alias, params)
        .map_err(AppError::from)?;
    let order_by = mssql_aggregate_order_by(plan);
    let skip = plan.pagination.skip.max(0);
    let limit = plan.pagination.limit.max(0);

    Ok(format!(
        r#"
        WITH agg AS (
          SELECT {select_list}
          FROM {table} AS {alias}
          {where_sql}
          {group_by}
          {having}
        ),
        paged AS (
          SELECT *
          FROM agg
          {order_by}
          OFFSET {skip} ROWS FETCH NEXT {limit} ROWS ONLY
        )
        SELECT
          (SELECT COUNT_BIG(*) FROM agg) AS [count],
          COALESCE((SELECT * FROM paged FOR JSON PATH), '[]') AS [rows]
        "#,
        select_list = select_list,
        table = table,
        alias = ms_quote_ident(alias),
        where_sql = where_sql,
        group_by = group_by,
        having = having,
        order_by = order_by,
        skip = skip,
        limit = limit,
    ))
}

fn mssql_aggregate_where(
    app_config: Arc<AppConfig>,
    plan: &ProviderAggregatePlan,
    alias: &str,
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

fn mssql_aggregate_groups(plan: &ProviderAggregatePlan) -> Vec<MssqlAggregateGroup> {
    plan.group_by
        .iter()
        .map(|group| MssqlAggregateGroup {
            name: naming::property_column(&group.prop),
            alias: group.alias.clone(),
        })
        .collect()
}

fn mssql_aggregate_metrics(plan: &ProviderAggregatePlan) -> Vec<MssqlAggregateMetric> {
    plan.metrics
        .iter()
        .map(|metric| MssqlAggregateMetric {
            function: metric.function,
            field_name: metric
                .prop
                .as_ref()
                .map(|prop| naming::property_column(prop)),
            alias: metric.alias.clone(),
            value_data_type: runtime_data_type(metric.value_data_type()),
        })
        .collect()
}

fn mssql_aggregate_having_input(plan: &ProviderAggregatePlan) -> Option<MssqlAggregateHaving> {
    plan.having.as_ref().map(|having| MssqlAggregateHaving {
        predicates: having
            .predicates
            .iter()
            .map(|predicate| MssqlAggregateHavingPredicate {
                metric_alias: predicate.metric_alias.clone(),
                op: predicate.op,
                value: predicate.value.clone(),
            })
            .collect(),
    })
}

fn mssql_aggregate_order_by(plan: &ProviderAggregatePlan) -> String {
    let fields = plan
        .sort
        .specs
        .iter()
        .map(|spec| MssqlSortField {
            name: spec.alias.clone(),
            direction: spec.direction,
        })
        .collect::<Vec<_>>();
    provider_aggregate_order_by(&fields)
}

fn ms_quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

impl RuntimeProviderIdentity for MsSqlClient {
    fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    fn framework_provider(&self) -> FrameworkProvider {
        self.framework_provider
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        let state = self.execution.pool().state();
        let provider = self.provider_descriptor();
        ProviderPoolStats::instrumented(
            provider.provider_key(),
            provider.data_source_name(),
            MSSQL_POOL_MAX_SIZE as u64,
            state.connections as u64,
            state.idle_connections as u64,
            0,
        )
    }
}

#[async_trait]
impl DatabaseClient for MsSqlClient {
    async fn health_check(&self) -> Result<(), AppError> {
        self.execution.health_check().await.map_err(AppError::from)
    }

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), AppError> {
        self.ensure_writable("audit append")?;
        self.execution
            .append_audit_event(event)
            .await
            .map_err(AppError::from)
    }

    async fn query_audit_events(&self, query: RuntimeAuditQuery) -> Result<Vec<Value>, AppError> {
        if self.read_only {
            return Err(AppError::DataAccess(
                "audit timeline query is not supported for FabricSqlAnalytics read-only data sources"
                    .to_string(),
            ));
        }
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
        if self.read_only && routine.returns == ProviderRoutineReturns::None {
            return Err(AppError::DataAccess(format!(
                "provider routine `{}` is not allowed for FabricSqlAnalytics because it returns no read payload",
                routine.method_name
            )));
        }
        if routine.kind != ProviderRoutineKind::Procedure {
            return Err(AppError::DataAccess(format!(
                "provider routine `{}` is not an MS SQL Server stored procedure",
                routine.method_name
            )));
        }
        let routine_name = routine.mssql.ok_or_else(|| {
            AppError::DataAccess(format!(
                "provider routine `{}` has no MS SQL Server binding",
                routine.method_name
            ))
        })?;
        let params = mssql_routine_params(arguments)?;
        let call =
            MssqlStoredProcedureCall::new(routine_name.schema, routine_name.name, params.len())
                .map_err(AppError::from)?;

        match routine.returns {
            ProviderRoutineReturns::None => {
                self.execution
                    .execute_stored_procedure(&call, params)
                    .await
                    .map_err(AppError::from)?;
                Ok(Value::Null)
            }
            ProviderRoutineReturns::One => {
                let rows = self
                    .execution
                    .query_stored_procedure(&call, params)
                    .await
                    .map_err(AppError::from)?;
                rows.first()
                    .map(mssql_routine_row_to_json)
                    .transpose()
                    .map(|value| value.unwrap_or(Value::Null))
            }
            ProviderRoutineReturns::Many => {
                let rows = self
                    .execution
                    .query_stored_procedure(&call, params)
                    .await
                    .map_err(AppError::from)?;
                rows.iter()
                    .map(mssql_routine_row_to_json)
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
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
        let mut plan_access = access.clone();
        plan_access.filter = plan.access_filter_json();

        let sql = cte::build_json_query(
            self.app_config.clone(),
            plan.entity_type.clone(),
            plan.selection_json(),
            plan.filter_json(),
            plan.sort_json(),
            plan.pagination.skip,
            plan.pagination.limit,
            user,
            &plan_access,
            &mut params,
        )?;

        let rows = self
            .execution
            .query_json_rows(&sql, params)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("query returned no result row").into(),
                )
            })?;

        Ok(JsonQueryResult::new(
            plan.pagination.skip,
            plan.pagination.limit,
            rows.total,
            rows.items,
            request_datetime,
            request_start,
        ))
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
        let mut params: Vec<SqlParam> = Vec::new();
        let sql = mssql_aggregate_query(self.app_config.clone(), &plan, &mut params)?;
        let rows = self
            .execution
            .aggregate_json_rows(&sql, params)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("aggregate query returned no row").into(),
                )
            })?;

        Ok(JsonAggregateResult::new(
            plan.pagination.skip,
            plan.pagination.limit,
            rows.total,
            rows.items,
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
        selections: Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        debug!("creating MS SQL item");
        self.ensure_writable("create")?;
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input);
        let pk_prop = Self::primary_key_prop(entity_type.clone())?;

        let id_value = if many_to_many_data.is_empty() {
            let (sql, params) = self.build_insert(entity_type.clone(), &input)?;
            let rows = self
                .execution
                .run_query(&sql, params)
                .await
                .map_err(AppError::from)?;
            let row = rows.into_iter().next().ok_or_else(|| {
                AppError::InternalServerError(anyhow::anyhow!("INSERT returned no row").into())
            })?;
            extract_output_value(&row, pk_prop.clone())?
        } else {
            let mut conn = self.execution.connection().await.map_err(AppError::from)?;
            MssqlExecutionClient::run_batch_on_conn(&mut conn, "BEGIN TRANSACTION")
                .await
                .map_err(AppError::from)?;

            let result = async {
                let (sql, params) = self.build_insert(entity_type.clone(), &input)?;
                let rows = MssqlExecutionClient::run_query_on_conn(&mut conn, &sql, params)
                    .await
                    .map_err(AppError::from)?;
                let row = rows.into_iter().next().ok_or_else(|| {
                    AppError::InternalServerError(anyhow::anyhow!("INSERT returned no row").into())
                })?;
                let id_value = extract_output_value(&row, pk_prop.clone())?;

                for (prop, related_entities) in many_to_many_data {
                    self.insert_junction_records_on_conn(
                        &mut conn,
                        entity_type.clone(),
                        id_value.clone(),
                        prop,
                        &related_entities,
                    )
                    .await?;
                }

                Ok::<Value, AppError>(id_value)
            }
            .await;

            match result {
                Ok(id_value) => {
                    MssqlExecutionClient::run_batch_on_conn(&mut conn, "COMMIT TRANSACTION")
                        .await
                        .map_err(AppError::from)?;
                    let _ =
                        MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                    id_value
                }
                Err(e) => {
                    let _ =
                        MssqlExecutionClient::run_batch_on_conn(&mut conn, "ROLLBACK TRANSACTION")
                            .await;
                    let _ =
                        MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                    return Err(e);
                }
            }
        };

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

    #[tracing::instrument(
    skip(self, entity_type, selections, input, user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn update_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<JsonObj, AppError> {
        debug!("updating MS SQL item");
        self.ensure_writable("update")?;
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let many_to_many_data = self.extract_many_to_many_data(entity_type.clone(), &input);
        let pk_prop = Self::primary_key_prop(entity_type.clone())?;

        let id_value = if many_to_many_data.is_empty() {
            let (sql, params) = self.build_update(entity_type.clone(), &input, read_version)?;
            let rows = self
                .execution
                .run_query(&sql, params)
                .await
                .map_err(AppError::from)?;
            let row = rows
                .into_iter()
                .next()
                .ok_or(AppError::InvalidKeyOrVersion)?;
            extract_output_value(&row, pk_prop.clone())?
        } else {
            let mut conn = self.execution.connection().await.map_err(AppError::from)?;
            MssqlExecutionClient::run_batch_on_conn(&mut conn, "BEGIN TRANSACTION")
                .await
                .map_err(AppError::from)?;

            let result = async {
                let (sql, params) = self.build_update(entity_type.clone(), &input, read_version)?;
                let rows = MssqlExecutionClient::run_query_on_conn(&mut conn, &sql, params)
                    .await
                    .map_err(AppError::from)?;
                let row = rows
                    .into_iter()
                    .next()
                    .ok_or(AppError::InvalidKeyOrVersion)?;
                let id_value = extract_output_value(&row, pk_prop.clone())?;

                for (prop, related_entities) in many_to_many_data {
                    self.update_junction_records_on_conn(
                        &mut conn,
                        entity_type.clone(),
                        id_value.clone(),
                        prop,
                        &related_entities,
                    )
                    .await?;
                }

                Ok::<Value, AppError>(id_value)
            }
            .await;

            match result {
                Ok(id_value) => {
                    MssqlExecutionClient::run_batch_on_conn(&mut conn, "COMMIT TRANSACTION")
                        .await
                        .map_err(AppError::from)?;
                    let _ =
                        MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                    id_value
                }
                Err(e) => {
                    let _ =
                        MssqlExecutionClient::run_batch_on_conn(&mut conn, "ROLLBACK TRANSACTION")
                            .await;
                    let _ =
                        MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                    return Err(e);
                }
            }
        };

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

    #[tracing::instrument(
    skip(self, entity_type, input, _user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn delete_item_json(
        &self,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        _user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, AppError> {
        debug!("deleting MS SQL item");
        self.ensure_writable("delete")?;
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        if !Self::has_many_to_many(entity_type.clone()) {
            let (sql, params) = self.build_delete(entity_type, &input, read_version)?;
            let rows = self
                .execution
                .run_query(&sql, params)
                .await
                .map_err(AppError::from)?;
            return if rows.is_empty() {
                Err(AppError::InvalidKeyOrVersion)
            } else {
                Ok(1)
            };
        }

        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let entity_id = input
            .get(&pk_name)
            .cloned()
            .ok_or_else(|| AppError::Validation("Entity ID is required for deletion".into()))?;
        let mut conn = self.execution.connection().await.map_err(AppError::from)?;
        MssqlExecutionClient::run_batch_on_conn(&mut conn, "BEGIN TRANSACTION")
            .await
            .map_err(AppError::from)?;

        let result = async {
            self.delete_junction_records_on_conn(&mut conn, entity_type.clone(), entity_id)
                .await?;
            let (sql, params) = self.build_delete(entity_type, &input, read_version)?;
            let rows = MssqlExecutionClient::run_query_on_conn(&mut conn, &sql, params)
                .await
                .map_err(AppError::from)?;
            if rows.is_empty() {
                Err(AppError::InvalidKeyOrVersion)
            } else {
                Ok(1)
            }
        }
        .await;

        match result {
            Ok(count) => {
                MssqlExecutionClient::run_batch_on_conn(&mut conn, "COMMIT TRANSACTION")
                    .await
                    .map_err(AppError::from)?;
                let _ = MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                Ok(count)
            }
            Err(e) => {
                let _ = MssqlExecutionClient::run_batch_on_conn(&mut conn, "ROLLBACK TRANSACTION")
                    .await;
                let _ = MssqlExecutionClient::rollback_open_transactions_on_conn(&mut conn).await;
                Err(e)
            }
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, id, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_id = !id.is_empty())
  )]
    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError> {
        debug!("finding MS SQL item");
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let id_val = if let Ok(n) = id.parse::<i64>() {
            Value::Number(serde_json::Number::from(n))
        } else {
            Value::String(id)
        };
        let filter_val = Some(serde_json::json!({ pk_name: id_val }));

        let res = self
            .query_items_json(
                entity_type,
                selections,
                filter_val,
                None,
                0,
                1,
                user,
                access,
            )
            .await?;
        Ok(res.items.into_iter().next())
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, filter_val, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow)
  )]
    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter_val: Option<Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError> {
        debug!("getting MS SQL items");
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

    #[tracing::instrument(
    skip(self, entity_type, selections, filter_val, sort_val, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, skip = skip, limit = limit)
  )]
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
        debug!("querying MS SQL items");
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();

        let mut params: Vec<SqlParam> = vec![];

        let sql = cte::build_json_query(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            filter_val,
            sort_val,
            skip,
            limit,
            user,
            access,
            &mut params,
        );
        let sql = sql?;

        let rows = self
            .execution
            .query_json_rows(&sql, params)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("query returned no result row").into(),
                )
            })?;

        Ok(JsonQueryResult::new(
            skip,
            limit,
            rows.total,
            rows.items,
            request_datetime,
            request_start,
        ))
    }
}

fn mssql_routine_params(arguments: &[ProviderRoutineArgument]) -> Result<Vec<SqlParam>, AppError> {
    arguments
        .iter()
        .map(|argument| {
            provider_type_param(
                argument.data_type,
                !argument.required,
                argument.name,
                argument.value.clone(),
            )
            .map_err(AppError::from)
        })
        .collect()
}

fn mssql_routine_row_to_json(row: &MssqlRow) -> Result<Value, AppError> {
    let account_id = row
        .try_get::<String, _>("account_id")
        .map_err(|e| AppError::DataAccess(e.to_string()))?
        .ok_or_else(|| AppError::DataAccess("routine result account_id was NULL".into()))?;
    let health_score = row
        .try_get::<f64, _>("health_score")
        .map_err(|e| AppError::DataAccess(e.to_string()))?
        .ok_or_else(|| AppError::DataAccess("routine result health_score was NULL".into()))?;
    let refreshed_at = row
        .try_get::<chrono::DateTime<chrono::FixedOffset>, _>("refreshed_at")
        .ok()
        .flatten()
        .map(|value| value.to_rfc3339())
        .or_else(|| {
            row.try_get::<String, _>("refreshed_at")
                .ok()
                .flatten()
                .map(|value| value.to_string())
        })
        .ok_or_else(|| AppError::DataAccess("routine result refreshed_at was NULL".into()))?;
    let source = row
        .try_get::<String, _>("source")
        .map_err(|e| AppError::DataAccess(e.to_string()))?
        .ok_or_else(|| AppError::DataAccess("routine result source was NULL".into()))?;

    Ok(serde_json::json!({
        "account_id": account_id,
        "health_score": health_score,
        "refreshed_at": refreshed_at,
        "source": source,
    }))
}

/// Extract the id column from a row that came back via OUTPUT INSERTED.[id]
/// or OUTPUT DELETED.[id] while preserving the JSON type needed for any
/// follow-up junction-table parameter binding.
fn extract_output_value(row: &MssqlRow, pk_prop: Arc<PropertyType>) -> Result<Value, AppError> {
    match pk_prop.data_type {
        DataType::Uuid => {
            if let Ok(Some(v)) = row.try_get::<uuid::Uuid, _>("id") {
                Ok(Value::String(v.to_string().to_lowercase()))
            } else if let Ok(Some(v)) = row.try_get::<String, _>("id") {
                Ok(Value::String(v.to_lowercase()))
            } else {
                Err(AppError::InternalServerError(
                    anyhow::anyhow!("could not read UUID id column from OUTPUT").into(),
                ))
            }
        }
        DataType::Int8 | DataType::Int16 => row
            .try_get::<i16, _>("id")
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e).into()))?
            .map(|v| Value::Number(serde_json::Number::from(v)))
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("id column from OUTPUT was NULL").into(),
                )
            }),
        DataType::Int32 => row
            .try_get::<i32, _>("id")
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e).into()))?
            .map(|v| Value::Number(serde_json::Number::from(v)))
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("id column from OUTPUT was NULL").into(),
                )
            }),
        DataType::Int64 => row
            .try_get::<i64, _>("id")
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e).into()))?
            .map(|v| Value::Number(serde_json::Number::from(v)))
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("id column from OUTPUT was NULL").into(),
                )
            }),
        DataType::String | DataType::ObjectId => row
            .try_get::<String, _>("id")
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e).into()))?
            .map(|v| Value::String(v))
            .ok_or_else(|| {
                AppError::InternalServerError(
                    anyhow::anyhow!("id column from OUTPUT was NULL").into(),
                )
            }),
        _ => Err(AppError::InternalServerError(
            anyhow::anyhow!(
                "unsupported primary-key type {:?} for MSSQL OUTPUT id",
                pk_prop.data_type
            )
            .into(),
        )),
    }
}

fn id_value_to_string(value: Value) -> Result<String, AppError> {
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(AppError::InternalServerError(
            anyhow::anyhow!("invalid id value from MSSQL OUTPUT").into(),
        )),
    }
}
