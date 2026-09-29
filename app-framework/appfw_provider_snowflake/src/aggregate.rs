use appfw_runtime::{
    model_metadata::RuntimeDataType, query_filter::RuntimeFilterOp,
    query_ir::RuntimeAggregateFunction, RuntimeError,
};
use serde_json::Value;

use crate::statement::SnowflakeStatementBuilder;

#[derive(Debug, Clone, PartialEq)]
pub struct SnowflakeAggregateGroup {
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnowflakeAggregateMetric {
    pub function: RuntimeAggregateFunction,
    pub field_name: Option<String>,
    pub alias: String,
    pub value_data_type: RuntimeDataType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SnowflakeAggregateHaving {
    pub predicates: Vec<SnowflakeAggregateHavingPredicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnowflakeAggregateHavingPredicate {
    pub metric_alias: String,
    pub op: RuntimeFilterOp,
    pub value: Value,
}

pub fn aggregate_select_list(
    groups: &[SnowflakeAggregateGroup],
    metrics: &[SnowflakeAggregateMetric],
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

pub fn aggregate_group_by(groups: &[SnowflakeAggregateGroup], alias: &str) -> String {
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
    metrics: &[SnowflakeAggregateMetric],
    having: Option<&SnowflakeAggregateHaving>,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
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
            builder,
        )?);
    }

    Ok(format!("HAVING {}", clauses.join(" AND ")))
}

fn having_predicate_sql(
    expr: &str,
    metric: &SnowflakeAggregateMetric,
    op: RuntimeFilterOp,
    value: Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    match op {
        RuntimeFilterOp::Eq
        | RuntimeFilterOp::Ne
        | RuntimeFilterOp::Lt
        | RuntimeFilterOp::Lte
        | RuntimeFilterOp::Gt
        | RuntimeFilterOp::Gte => {
            let placeholder = having_placeholder(metric, value, builder)?;
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
                placeholders.push(having_placeholder(metric, value.clone(), builder)?);
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
    metric: &SnowflakeAggregateMetric,
    value: Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let placeholder = builder.bind_scalar(metric.value_data_type, &metric.alias, false, value)?;
    let placeholder = match metric.value_data_type {
        RuntimeDataType::Float32 | RuntimeDataType::Float64 if placeholder == "?" => {
            "TO_DOUBLE(?)".to_string()
        }
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
            if placeholder == "?" =>
        {
            "TO_NUMBER(?)".to_string()
        }
        _ => placeholder,
    };
    Ok(placeholder)
}

fn aggregate_expr(metric: &SnowflakeAggregateMetric, alias: &str) -> Result<String, RuntimeError> {
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

fn metric_field(metric: &SnowflakeAggregateMetric) -> Result<String, RuntimeError> {
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
        let groups = vec![SnowflakeAggregateGroup {
            name: "region".to_string(),
            alias: "region".to_string(),
        }];
        let metrics = vec![SnowflakeAggregateMetric {
            function: RuntimeAggregateFunction::CountDistinct,
            field_name: Some("account_id".to_string()),
            alias: "account_count".to_string(),
            value_data_type: RuntimeDataType::Int64,
        }];
        let having = SnowflakeAggregateHaving {
            predicates: vec![SnowflakeAggregateHavingPredicate {
                metric_alias: "account_count".to_string(),
                op: RuntimeFilterOp::In,
                value: json!([2, 3]),
            }],
        };
        let mut builder = SnowflakeStatementBuilder::new();

        assert_eq!(
            aggregate_select_list(&groups, &metrics, "t0").expect("select"),
            "\"t0\".\"region\" AS \"region\", COUNT(DISTINCT \"t0\".\"account_id\") AS \"account_count\""
        );
        assert_eq!(
            aggregate_group_by(&groups, "t0"),
            "GROUP BY \"t0\".\"region\""
        );
        assert_eq!(
            aggregate_having(&metrics, Some(&having), "t0", &mut builder).expect("having"),
            "HAVING COUNT(DISTINCT \"t0\".\"account_id\") IN (TO_NUMBER(?), TO_NUMBER(?))"
        );
    }
}
