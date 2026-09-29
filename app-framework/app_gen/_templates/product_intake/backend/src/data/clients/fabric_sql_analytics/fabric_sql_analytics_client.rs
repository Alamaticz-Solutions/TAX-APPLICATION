use std::{sync::Arc, time::Instant};

use anyhow::Result;
use appfw_provider_mssql::{
    aggregate_group_by as provider_aggregate_group_by,
    aggregate_having as provider_aggregate_having,
    aggregate_order_by as provider_aggregate_order_by,
    aggregate_select_list as provider_aggregate_select_list,
    fabric::{FabricOdbcColumn, FabricOdbcExecutionClient},
    order_by as provider_order_by, MssqlAggregateGroup, MssqlAggregateHaving,
    MssqlAggregateHavingPredicate, MssqlAggregateMetric, MssqlConnectionConfig, MssqlSortField,
};
use appfw_runtime::{
    connection_security::{self, Provider},
    extension::UserAuth,
    json::JsonObj,
    provider_keys::FrameworkProvider,
    PolicyAccess, RuntimeAuditEvent, RuntimeAuditQuery, RuntimeProviderIdentity,
    RuntimeProviderPlanInput,
};
use async_trait::async_trait;
use serde_json::Value;
use time::OffsetDateTime;
use tracing::{debug, info};

use crate::{
    config::app_config::AppConfig,
    data::clients::{
        database_client::{
            DatabaseClient, ProviderAggregatePlan, ProviderPoolStats, ProviderQueryPlan,
        },
        mssql::{filter, naming, param},
    },
    data::query_ir::SelectionTree,
    product_api::runtime_data_type,
    routes::app_error::AppError,
    schemas::{
        common::{JsonAggregateResult, JsonQueryResult},
        system::{DataSourceEnvironment, DataType, EntityType},
    },
};

use param::SqlParam;

pub struct FabricSqlAnalyticsClient {
    execution: FabricOdbcExecutionClient,
    app_config: Arc<AppConfig>,
    data_source_name: String,
}

impl FabricSqlAnalyticsClient {
    #[tracing::instrument(skip(app_config), fields(data_source = %data_source_name))]
    pub async fn init(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<FabricSqlAnalyticsClient, AppError> {
        let env = app_config.get_data_source_env(&data_source_name)?;
        let execution = Self::get_execution(env).await?;
        Ok(FabricSqlAnalyticsClient {
            execution,
            app_config,
            data_source_name,
        })
    }

    #[tracing::instrument(skip(env), fields(host = %env.db_host, database = %env.db_name))]
    async fn get_execution(
        env: Arc<DataSourceEnvironment>,
    ) -> Result<FabricOdbcExecutionClient, AppError> {
        validate_connection_security(&env)?;
        let connection = fabric_connection_config(&env)?;
        let execution = FabricOdbcExecutionClient::from_connection_config(&connection)
            .await
            .map_err(AppError::from)?;
        info!("Fabric SQL analytics ODBC execution client configured");
        Ok(execution)
    }

    fn read_only_error(operation: &str) -> AppError {
        AppError::DataAccess(format!(
            "{operation} is not supported for FabricSqlAnalytics read-only data sources"
        ))
    }
}

fn validate_connection_security(
    env: &DataSourceEnvironment,
) -> Result<connection_security::ConnectionSecurity, AppError> {
    connection_security::validate(
        Provider::FabricSqlAnalytics,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| {
        AppError::DataAccess(format!(
            "invalid FabricSqlAnalytics connection security: {e}"
        ))
    })
}

fn fabric_connection_config(
    env: &DataSourceEnvironment,
) -> Result<MssqlConnectionConfig, AppError> {
    MssqlConnectionConfig::from_environment(
        appfw_mssql_auth::PROVIDER_FABRIC,
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

fn fabric_aggregate_query(
    app_config: Arc<AppConfig>,
    plan: &ProviderAggregatePlan,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let alias = "t0";
    let groups = fabric_aggregate_groups(plan);
    let metrics = fabric_aggregate_metrics(plan);
    let having_input = fabric_aggregate_having_input(plan);
    let select_list =
        provider_aggregate_select_list(&groups, &metrics, alias).map_err(AppError::from)?;
    let table = naming::table_ref(&plan.entity_type);
    let where_sql = fabric_aggregate_where(app_config, plan, alias, params)?;
    let group_by = provider_aggregate_group_by(&groups, alias);
    let having = provider_aggregate_having(&metrics, having_input.as_ref(), alias, params)
        .map_err(AppError::from)?;
    let order_by = fabric_aggregate_order_by(plan);
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
        alias = naming::quote_ident(alias),
        where_sql = where_sql,
        group_by = group_by,
        having = having,
        order_by = order_by,
        skip = skip,
        limit = limit,
    ))
}

fn fabric_aggregate_where(
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

fn fabric_aggregate_groups(plan: &ProviderAggregatePlan) -> Vec<MssqlAggregateGroup> {
    plan.group_by
        .iter()
        .map(|group| MssqlAggregateGroup {
            name: naming::property_column(&group.prop),
            alias: group.alias.clone(),
        })
        .collect()
}

fn fabric_aggregate_metrics(plan: &ProviderAggregatePlan) -> Vec<MssqlAggregateMetric> {
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

fn fabric_aggregate_having_input(plan: &ProviderAggregatePlan) -> Option<MssqlAggregateHaving> {
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

fn fabric_aggregate_order_by(plan: &ProviderAggregatePlan) -> String {
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

struct FabricFlatQuery {
    count_sql: String,
    count_params: Vec<SqlParam>,
    rows_sql: String,
    rows_params: Vec<SqlParam>,
    columns: Vec<FabricOdbcColumn>,
}

fn fabric_flat_query(
    app_config: Arc<AppConfig>,
    plan: &ProviderQueryPlan,
) -> Result<FabricFlatQuery, AppError> {
    let alias = "t0";
    let table = naming::table_ref(&plan.entity_type);
    let (select_list, columns) = fabric_select_list(&plan.selection, alias)?;
    let mut where_params = Vec::new();
    let where_sql = fabric_query_where(app_config, plan, alias, &mut where_params)?;
    let order_by = fabric_order_by(plan, alias)?;
    let skip = plan.pagination.skip.max(0);
    let limit = plan.pagination.limit.max(0);

    let count_sql = format!(
        "SELECT COUNT_BIG(*) AS [count] FROM {table} AS {alias} {where_sql}",
        table = table,
        alias = naming::quote_ident(alias),
        where_sql = where_sql,
    );
    let rows_sql = format!(
        "SELECT {select_list} FROM {table} AS {alias} {where_sql} {order_by} OFFSET {skip} ROWS FETCH NEXT {limit} ROWS ONLY",
        select_list = select_list,
        table = table,
        alias = naming::quote_ident(alias),
        where_sql = where_sql,
        order_by = order_by,
        skip = skip,
        limit = limit,
    );

    Ok(FabricFlatQuery {
        count_sql,
        count_params: where_params.clone(),
        rows_sql,
        rows_params: where_params,
        columns,
    })
}

fn fabric_select_list(
    selection: &SelectionTree,
    alias: &str,
) -> Result<(String, Vec<FabricOdbcColumn>), AppError> {
    let mut expressions = Vec::new();
    let mut columns = Vec::new();

    for node in &selection.fields {
        match node.prop.data_type {
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany => {
                return Err(AppError::DataAccess(format!(
                    "FabricSqlAnalytics flat read model `{}` does not support relationship projection `{}`; select scalar fields or expose a custom method",
                    selection.name, node.prop.name
                )));
            }
            _ => {}
        }
        if !node.children.is_empty() {
            return Err(AppError::DataAccess(format!(
                "FabricSqlAnalytics flat read model `{}` does not support nested projection `{}`",
                selection.name, node.prop.name
            )));
        }

        expressions.push(format!(
            "{}.{} AS {}",
            naming::quote_ident(alias),
            naming::quote_ident(&naming::property_column(&node.prop)),
            naming::quote_ident(&node.prop.name)
        ));
        columns.push(FabricOdbcColumn {
            name: node.prop.name.clone(),
            data_type: runtime_data_type(node.prop.data_type),
        });
    }

    if expressions.is_empty() {
        return Err(AppError::DataAccess(format!(
            "FabricSqlAnalytics flat read model `{}` requires at least one scalar selection",
            selection.name
        )));
    }

    Ok((expressions.join(", "), columns))
}

fn fabric_query_where(
    app_config: Arc<AppConfig>,
    plan: &ProviderQueryPlan,
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

fn fabric_order_by(plan: &ProviderQueryPlan, alias: &str) -> Result<String, AppError> {
    let fields = plan
        .sort
        .specs
        .iter()
        .map(|spec| MssqlSortField {
            name: naming::property_column(&spec.prop),
            direction: spec.direction,
        })
        .collect::<Vec<_>>();
    let default_column = fabric_default_order_column(&plan.entity_type, &plan.selection)?;
    Ok(provider_order_by(alias, &default_column, &fields))
}

fn fabric_default_order_column(
    entity_type: &EntityType,
    selection: &SelectionTree,
) -> Result<String, AppError> {
    if let Some(prop) = entity_type.props.iter().find(|prop| prop.is_key) {
        return Ok(naming::property_column(prop));
    }
    if let Some(node) = selection.fields.iter().find(|node| {
        !matches!(
            node.prop.data_type,
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany
        )
    }) {
        return Ok(naming::property_column(&node.prop));
    }
    entity_type
        .props
        .iter()
        .find(|prop| {
            !matches!(
                prop.data_type,
                DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany
            )
        })
        .map(naming::property_column)
        .ok_or_else(|| {
            AppError::DataAccess(format!(
                "FabricSqlAnalytics entity `{}` has no scalar column available for default ordering",
                entity_type.pascal_1
            ))
        })
}

fn fabric_runtime_query_plan(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    selections: Value,
    filter_val: Option<Value>,
    sort_val: Option<Value>,
    skip: i32,
    limit: i32,
    access: &PolicyAccess,
) -> Result<ProviderQueryPlan, AppError> {
    crate::data::query_ir::QueryPlan::new(
        app_config,
        entity_type,
        selections,
        filter_val,
        sort_val,
        skip,
        limit,
        access,
    )
    .map(crate::data::query_ir::QueryPlan::into_runtime_provider_plan)
}

impl RuntimeProviderIdentity for FabricSqlAnalyticsClient {
    fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    fn framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::FabricSqlAnalytics
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        let provider = self.provider_descriptor();
        ProviderPoolStats::opaque(provider.provider_key(), provider.data_source_name())
    }
}

#[async_trait]
impl DatabaseClient for FabricSqlAnalyticsClient {
    async fn health_check(&self) -> Result<(), AppError> {
        self.execution.health_check().await.map_err(AppError::from)
    }

    async fn append_audit_event(&self, _event: RuntimeAuditEvent) -> Result<(), AppError> {
        Err(Self::read_only_error("audit append"))
    }

    async fn query_audit_events(&self, _query: RuntimeAuditQuery) -> Result<Vec<Value>, AppError> {
        Err(AppError::DataAccess(
            "audit timeline query is not supported for FabricSqlAnalytics read-only data sources"
                .to_string(),
        ))
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
        let (plan, _user, access) = input.into_parts();
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let flat_query = fabric_flat_query(self.app_config.clone(), &plan)?;

        let rows = self
            .execution
            .query_flat_json_rows(
                &flat_query.count_sql,
                flat_query.count_params,
                &flat_query.rows_sql,
                flat_query.rows_params,
                flat_query.columns,
            )
            .await
            .map_err(AppError::from)?;

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
        let sql = fabric_aggregate_query(self.app_config.clone(), &plan, &mut params)?;
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

    async fn create_item_json(
        &self,
        _entity_type: Arc<EntityType>,
        _selections: Value,
        _input: JsonObj,
        _user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        Err(Self::read_only_error("create"))
    }

    async fn update_item_json(
        &self,
        _entity_type: Arc<EntityType>,
        _selections: Value,
        _input: JsonObj,
        _user: &UserAuth,
        access: &PolicyAccess,
        _read_version: Option<Value>,
    ) -> Result<JsonObj, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        Err(Self::read_only_error("update"))
    }

    async fn delete_item_json(
        &self,
        _entity_type: Arc<EntityType>,
        _input: JsonObj,
        _user: &UserAuth,
        access: &PolicyAccess,
        _read_version: Option<Value>,
    ) -> Result<i64, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }
        Err(Self::read_only_error("delete"))
    }

    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError> {
        debug!("finding Fabric SQL analytics item");
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

    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter_val: Option<Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError> {
        debug!("getting Fabric SQL analytics items");
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
        _user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, AppError> {
        debug!("querying Fabric SQL analytics items");
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let plan = fabric_runtime_query_plan(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            filter_val,
            sort_val,
            skip,
            limit,
            access,
        )?;
        let flat_query = fabric_flat_query(self.app_config.clone(), &plan)?;

        let rows = self
            .execution
            .query_flat_json_rows(
                &flat_query.count_sql,
                flat_query.count_params,
                &flat_query.rows_sql,
                flat_query.rows_params,
                flat_query.columns,
            )
            .await
            .map_err(AppError::from)?;

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
