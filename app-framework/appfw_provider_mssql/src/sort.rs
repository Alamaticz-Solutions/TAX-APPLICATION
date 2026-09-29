use appfw_runtime::query_ir::RuntimeSortDirection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MssqlSortField {
    pub name: String,
    pub direction: RuntimeSortDirection,
}

pub fn order_by(alias: &str, primary_key: &str, fields: &[MssqlSortField]) -> String {
    if fields.is_empty() {
        return format!("ORDER BY [{}].[{}] ASC", alias, primary_key);
    }

    let parts = fields
        .iter()
        .map(|field| {
            format!(
                "[{}].[{}] {}",
                alias,
                field.name,
                direction_sql(field.direction)
            )
        })
        .collect::<Vec<_>>();
    format!("ORDER BY {}", parts.join(", "))
}

pub fn aggregate_order_by(fields: &[MssqlSortField]) -> String {
    if fields.is_empty() {
        return "ORDER BY (SELECT NULL)".to_string();
    }

    let parts = fields
        .iter()
        .map(|field| {
            format!(
                "{} {}",
                quote_ident(&field.name),
                direction_sql(field.direction)
            )
        })
        .collect::<Vec<_>>();
    format!("ORDER BY {}", parts.join(", "))
}

fn quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn direction_sql(direction: RuntimeSortDirection) -> &'static str {
    match direction {
        RuntimeSortDirection::Asc => "ASC",
        RuntimeSortDirection::Desc => "DESC",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_primary_key_ordering() {
        assert_eq!(order_by("t0", "id", &[]), "ORDER BY [t0].[id] ASC");
    }

    #[test]
    fn renders_resolved_fields() {
        let fields = vec![
            MssqlSortField {
                name: "name".to_string(),
                direction: RuntimeSortDirection::Desc,
            },
            MssqlSortField {
                name: "age".to_string(),
                direction: RuntimeSortDirection::Asc,
            },
        ];
        assert_eq!(
            order_by("t0", "id", &fields),
            "ORDER BY [t0].[name] DESC, [t0].[age] ASC"
        );
    }

    #[test]
    fn renders_aggregate_ordering() {
        let fields = vec![MssqlSortField {
            name: "total]age".to_string(),
            direction: RuntimeSortDirection::Desc,
        }];
        assert_eq!(aggregate_order_by(&[]), "ORDER BY (SELECT NULL)");
        assert_eq!(aggregate_order_by(&fields), "ORDER BY [total]]age] DESC");
    }
}
