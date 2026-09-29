use appfw_runtime::{
    model_metadata::RuntimeDataType, query_filter::RuntimeFilterOp,
    query_ir::RuntimeAggregateFunction, RuntimeError,
};
use serde_json::Value;

use crate::param::{type_param, SqlParam};

#[derive(Debug, Clone, PartialEq)]
pub struct PostgresAggregateGroup {
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostgresAggregateMetric {
    pub function: RuntimeAggregateFunction,
    pub field_name: Option<String>,
    pub alias: String,
    pub value_data_type: RuntimeDataType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PostgresAggregateHaving {
    pub predicates: Vec<PostgresAggregateHavingPredicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostgresAggregateHavingPredicate {
    pub metric_alias: String,
    pub op: RuntimeFilterOp,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresAggregateQuery {
    pub select_list: String,
    pub table: String,
    pub alias: String,
    pub where_sql: String,
    pub group_by: String,
    pub having: String,
    pub order_by: String,
    pub skip: i32,
    pub limit: i32,
}

pub fn aggregate_query(query: &PostgresAggregateQuery) -> String {
    format!(
        r#"
        WITH agg AS (
          SELECT {select_list}
          FROM {table} {alias}
          {where_sql}
          {group_by}
          {having}
        ),
        paged AS (
          SELECT *
          FROM agg
          {order_by}
          OFFSET {skip}
          LIMIT {limit}
        )
        SELECT
          (SELECT COUNT(*) FROM agg) AS count,
          COALESCE((SELECT json_agg(row_to_json(paged)) FROM paged), '[]'::json) AS rows
        "#,
        select_list = query.select_list,
        table = query.table,
        alias = query.alias,
        where_sql = query.where_sql,
        group_by = query.group_by,
        having = query.having,
        order_by = query.order_by,
        skip = query.skip,
        limit = query.limit,
    )
}

pub fn aggregate_select_list(
    groups: &[PostgresAggregateGroup],
    metrics: &[PostgresAggregateMetric],
    alias: &str,
) -> Result<String, RuntimeError> {
    let mut fields = Vec::new();
    for group in groups {
        fields.push(format!(
            "{} AS {}",
            qualified(alias, &group.name),
            quote_ident(&group.alias)
        ));
    }
    for metric in metrics {
        fields.push(format!(
            "{} AS {}",
            aggregate_expr(metric, alias)?,
            quote_ident(&metric.alias)
        ));
    }
    Ok(fields.join(", "))
}

pub fn aggregate_group_by(groups: &[PostgresAggregateGroup], alias: &str) -> String {
    if groups.is_empty() {
        String::new()
    } else {
        format!(
            "GROUP BY {}",
            groups
                .iter()
                .map(|group| qualified(alias, &group.name))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn aggregate_having(
    metrics: &[PostgresAggregateMetric],
    having: Option<&PostgresAggregateHaving>,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    let Some(having) = having else {
        return Ok(String::new());
    };
    if having.predicates.is_empty() {
        return Ok(String::new());
    }

    let mut clauses = Vec::with_capacity(having.predicates.len());
    for predicate in &having.predicates {
        let metric = metrics
            .iter()
            .find(|metric| metric.alias == predicate.metric_alias)
            .ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "having references unknown metric alias '{}'",
                    predicate.metric_alias
                ))
            })?;
        let expr = aggregate_expr(metric, alias)?;
        clauses.push(having_predicate_sql(
            &expr,
            metric,
            predicate.op,
            predicate.value.clone(),
            params,
        )?);
    }

    Ok(format!("HAVING {}", clauses.join(" AND ")))
}

fn having_predicate_sql(
    expr: &str,
    metric: &PostgresAggregateMetric,
    op: RuntimeFilterOp,
    value: Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    match op {
        RuntimeFilterOp::Eq
        | RuntimeFilterOp::Ne
        | RuntimeFilterOp::Lt
        | RuntimeFilterOp::Lte
        | RuntimeFilterOp::Gt
        | RuntimeFilterOp::Gte => {
            let placeholder = having_placeholder(metric, value, params)?;
            Ok(format!("{} {} {}", expr, sql_operator(op)?, placeholder))
        }
        RuntimeFilterOp::In | RuntimeFilterOp::NotIn => {
            let values = value.as_array().ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "having list for '{}' must be an array",
                    metric.alias
                ))
            })?;
            if values.is_empty() {
                return Err(RuntimeError::Validation(format!(
                    "having list for '{}' must not be empty",
                    metric.alias
                )));
            }
            let mut placeholders = Vec::with_capacity(values.len());
            for value in values {
                placeholders.push(having_placeholder(metric, value.clone(), params)?);
            }
            Ok(format!(
                "{} {} ({})",
                expr,
                if matches!(op, RuntimeFilterOp::In) {
                    "IN"
                } else {
                    "NOT IN"
                },
                placeholders.join(", ")
            ))
        }
        _ => Err(RuntimeError::Validation(format!(
            "unsupported aggregate having operator {:?}",
            op
        ))),
    }
}

fn having_placeholder(
    metric: &PostgresAggregateMetric,
    value: Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    params.push(type_param(
        metric.value_data_type,
        false,
        &metric.alias,
        value,
    )?);
    Ok(format!("${}", params.len()))
}

fn aggregate_expr(metric: &PostgresAggregateMetric, alias: &str) -> Result<String, RuntimeError> {
    let expr = match metric.function {
        RuntimeAggregateFunction::Count => metric
            .field_name
            .as_ref()
            .map(|field| format!("COUNT({})", qualified(alias, field)))
            .unwrap_or_else(|| "COUNT(*)".to_string()),
        RuntimeAggregateFunction::CountDistinct => {
            format!(
                "COUNT(DISTINCT {})",
                qualified(alias, &metric_field(metric)?)
            )
        }
        RuntimeAggregateFunction::Sum => {
            format!("SUM({})", qualified(alias, &metric_field(metric)?))
        }
        RuntimeAggregateFunction::Avg => {
            format!("AVG({})", qualified(alias, &metric_field(metric)?))
        }
        RuntimeAggregateFunction::Min => {
            format!("MIN({})", qualified(alias, &metric_field(metric)?))
        }
        RuntimeAggregateFunction::Max => {
            format!("MAX({})", qualified(alias, &metric_field(metric)?))
        }
    };
    Ok(expr)
}

fn metric_field(metric: &PostgresAggregateMetric) -> Result<String, RuntimeError> {
    metric.field_name.clone().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "{} metric '{}' requires a field",
            metric.function.as_str(),
            metric.alias
        ))
    })
}

fn sql_operator(op: RuntimeFilterOp) -> Result<&'static str, RuntimeError> {
    match op {
        RuntimeFilterOp::Eq => Ok("="),
        RuntimeFilterOp::Ne => Ok("<>"),
        RuntimeFilterOp::Lt => Ok("<"),
        RuntimeFilterOp::Lte => Ok("<="),
        RuntimeFilterOp::Gt => Ok(">"),
        RuntimeFilterOp::Gte => Ok(">="),
        _ => Err(RuntimeError::Validation(format!(
            "unsupported aggregate comparison operator {:?}",
            op
        ))),
    }
}

fn qualified(alias: &str, col: &str) -> String {
    format!("{}.{}", quote_ident(alias), quote_ident(col))
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use appfw_runtime::{
        model_metadata::RuntimeDataType, query_filter::RuntimeFilterOp,
        query_ir::RuntimeAggregateFunction,
    };
    use serde_json::json;

    use super::*;

    #[test]
    fn renders_select_group_and_having() {
        let groups = vec![PostgresAggregateGroup {
            name: "region".to_string(),
            alias: "region".to_string(),
        }];
        let metrics = vec![PostgresAggregateMetric {
            function: RuntimeAggregateFunction::Sum,
            field_name: Some("amount".to_string()),
            alias: "total_amount".to_string(),
            value_data_type: RuntimeDataType::Int64,
        }];
        let having = PostgresAggregateHaving {
            predicates: vec![PostgresAggregateHavingPredicate {
                metric_alias: "total_amount".to_string(),
                op: RuntimeFilterOp::Gte,
                value: json!(100),
            }],
        };
        let mut params = Vec::new();

        assert_eq!(
            aggregate_select_list(&groups, &metrics, "t0").expect("select"),
            "\"t0\".\"region\" AS \"region\", SUM(\"t0\".\"amount\") AS \"total_amount\""
        );
        assert_eq!(
            aggregate_group_by(&groups, "t0"),
            "GROUP BY \"t0\".\"region\""
        );
        assert_eq!(
            aggregate_having(&metrics, Some(&having), "t0", &mut params).expect("having"),
            "HAVING SUM(\"t0\".\"amount\") >= $1"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn renders_aggregate_query_envelope() {
        let sql = aggregate_query(&PostgresAggregateQuery {
            select_list: "\"t0\".\"region\" AS \"region\", COUNT(*) AS \"count\"".to_string(),
            table: "crm.accounts".to_string(),
            alias: "t0".to_string(),
            where_sql: "WHERE t0.active = $1".to_string(),
            group_by: "GROUP BY \"t0\".\"region\"".to_string(),
            having: "HAVING COUNT(*) > $2".to_string(),
            order_by: "ORDER BY \"count\" DESC".to_string(),
            skip: 20,
            limit: 50,
        });

        assert!(sql.contains("WITH agg AS"));
        assert!(sql.contains("SELECT \"t0\".\"region\" AS \"region\", COUNT(*) AS \"count\""));
        assert!(sql.contains("FROM crm.accounts t0"));
        assert!(sql.contains("WHERE t0.active = $1"));
        assert!(sql.contains("GROUP BY \"t0\".\"region\""));
        assert!(sql.contains("HAVING COUNT(*) > $2"));
        assert!(sql.contains("ORDER BY \"count\" DESC"));
        assert!(sql.contains("OFFSET 20"));
        assert!(sql.contains("LIMIT 50"));
        assert!(sql.contains("(SELECT COUNT(*) FROM agg) AS count"));
        assert!(sql.contains(
            "COALESCE((SELECT json_agg(row_to_json(paged)) FROM paged), '[]'::json) AS rows"
        ));
    }
}
