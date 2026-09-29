use appfw_runtime::{
    model_metadata::RuntimeDataType, query_filter::RuntimeFilterOp,
    query_ir::RuntimeAggregateFunction, RuntimeError,
};
use serde_json::Value;

use crate::param::{param_placeholder, type_param, SqlParam};

#[derive(Debug, Clone, PartialEq)]
pub struct MssqlAggregateGroup {
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MssqlAggregateMetric {
    pub function: RuntimeAggregateFunction,
    pub field_name: Option<String>,
    pub alias: String,
    pub value_data_type: RuntimeDataType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MssqlAggregateHaving {
    pub predicates: Vec<MssqlAggregateHavingPredicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MssqlAggregateHavingPredicate {
    pub metric_alias: String,
    pub op: RuntimeFilterOp,
    pub value: Value,
}

pub fn aggregate_select_list(
    groups: &[MssqlAggregateGroup],
    metrics: &[MssqlAggregateMetric],
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

pub fn aggregate_group_by(groups: &[MssqlAggregateGroup], alias: &str) -> String {
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
    metrics: &[MssqlAggregateMetric],
    having: Option<&MssqlAggregateHaving>,
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
    metric: &MssqlAggregateMetric,
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
    metric: &MssqlAggregateMetric,
    value: Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    params.push(type_param(
        metric.value_data_type,
        false,
        &metric.alias,
        value,
    )?);
    Ok(param_placeholder(params.len()))
}

fn aggregate_expr(metric: &MssqlAggregateMetric, alias: &str) -> Result<String, RuntimeError> {
    let expr = match metric.function {
        RuntimeAggregateFunction::Count => metric
            .field_name
            .as_ref()
            .map(|field| format!("COUNT({})", qualified(alias, field)))
            .unwrap_or_else(|| "COUNT_BIG(*)".to_string()),
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
            format!(
                "AVG(CAST({} AS float))",
                qualified(alias, &metric_field(metric)?)
            )
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

fn metric_field(metric: &MssqlAggregateMetric) -> Result<String, RuntimeError> {
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
    format!("[{}]", name.replace(']', "]]"))
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
        let groups = vec![MssqlAggregateGroup {
            name: "region".to_string(),
            alias: "region".to_string(),
        }];
        let metrics = vec![MssqlAggregateMetric {
            function: RuntimeAggregateFunction::Avg,
            field_name: Some("amount".to_string()),
            alias: "average_amount".to_string(),
            value_data_type: RuntimeDataType::Float64,
        }];
        let having = MssqlAggregateHaving {
            predicates: vec![MssqlAggregateHavingPredicate {
                metric_alias: "average_amount".to_string(),
                op: RuntimeFilterOp::Lt,
                value: json!(25.5),
            }],
        };
        let mut params = Vec::new();

        assert_eq!(
            aggregate_select_list(&groups, &metrics, "t0").expect("select"),
            "[t0].[region] AS [region], AVG(CAST([t0].[amount] AS float)) AS [average_amount]"
        );
        assert_eq!(aggregate_group_by(&groups, "t0"), "GROUP BY [t0].[region]");
        assert_eq!(
            aggregate_having(&metrics, Some(&having), "t0", &mut params).expect("having"),
            "HAVING AVG(CAST([t0].[amount] AS float)) < @P1"
        );
        assert_eq!(params.len(), 1);
    }
}
