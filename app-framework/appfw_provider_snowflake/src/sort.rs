use appfw_runtime::query_ir::RuntimeSortDirection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnowflakeSortField {
    pub name: String,
    pub direction: RuntimeSortDirection,
}

pub fn order_by(alias: &str, primary_key: &str, fields: &[SnowflakeSortField]) -> String {
    if fields.is_empty() {
        return format!("ORDER BY {} ASC", qualify(alias, primary_key));
    }

    let parts = fields
        .iter()
        .map(|field| {
            format!(
                "{} {}",
                qualify(alias, &field.name),
                direction_sql(field.direction)
            )
        })
        .collect::<Vec<_>>();
    format!("ORDER BY {}", parts.join(", "))
}

pub fn aggregate_order_by(fields: &[SnowflakeSortField]) -> String {
    if fields.is_empty() {
        return String::new();
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

fn qualify(alias: &str, col: &str) -> String {
    if alias.is_empty() {
        quote_ident(col)
    } else {
        format!("{}.{}", quote_ident(alias), quote_ident(col))
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
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
        assert_eq!(order_by("t0", "id", &[]), "ORDER BY \"t0\".\"id\" ASC");
    }

    #[test]
    fn renders_resolved_fields() {
        let fields = vec![
            SnowflakeSortField {
                name: "name".to_string(),
                direction: RuntimeSortDirection::Desc,
            },
            SnowflakeSortField {
                name: "age".to_string(),
                direction: RuntimeSortDirection::Asc,
            },
        ];
        assert_eq!(
            order_by("t0", "id", &fields),
            "ORDER BY \"t0\".\"name\" DESC, \"t0\".\"age\" ASC"
        );
    }

    #[test]
    fn quotes_embedded_quotes() {
        let fields = vec![SnowflakeSortField {
            name: "bad\"name".to_string(),
            direction: RuntimeSortDirection::Asc,
        }];
        assert_eq!(order_by("", "id", &fields), "ORDER BY \"bad\"\"name\" ASC");
    }

    #[test]
    fn renders_aggregate_ordering() {
        let fields = vec![SnowflakeSortField {
            name: "total\"age".to_string(),
            direction: RuntimeSortDirection::Desc,
        }];
        assert_eq!(aggregate_order_by(&[]), "");
        assert_eq!(
            aggregate_order_by(&fields),
            "ORDER BY \"total\"\"age\" DESC"
        );
    }
}
