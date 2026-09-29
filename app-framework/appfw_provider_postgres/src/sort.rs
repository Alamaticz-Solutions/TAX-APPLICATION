use appfw_runtime::query_ir::RuntimeSortDirection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresSortField {
    pub name: String,
    pub direction: RuntimeSortDirection,
}

pub fn order_by(alias: &str, primary_key: &str, fields: &[PostgresSortField]) -> String {
    if fields.is_empty() {
        return format!("order by {alias}.{primary_key} asc");
    }

    let parts = fields
        .iter()
        .map(|field| format!("{}.{} {}", alias, field.name, field.direction.as_str()))
        .collect::<Vec<_>>();
    format!("order by {}", parts.join(", "))
}

pub fn aggregate_order_by(fields: &[PostgresSortField]) -> String {
    if fields.is_empty() {
        return String::new();
    }

    let parts = fields
        .iter()
        .map(|field| {
            format!(
                "{} {}",
                quote_ident(&field.name),
                field.direction.as_str().to_uppercase()
            )
        })
        .collect::<Vec<_>>();
    format!("ORDER BY {}", parts.join(", "))
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_primary_key_ordering() {
        assert_eq!(order_by("t0", "id", &[]), "order by t0.id asc");
    }

    #[test]
    fn renders_resolved_fields() {
        let fields = vec![
            PostgresSortField {
                name: "name".to_string(),
                direction: RuntimeSortDirection::Desc,
            },
            PostgresSortField {
                name: "age".to_string(),
                direction: RuntimeSortDirection::Asc,
            },
        ];
        assert_eq!(
            order_by("t0", "id", &fields),
            "order by t0.name desc, t0.age asc"
        );
    }

    #[test]
    fn renders_aggregate_ordering() {
        let fields = vec![PostgresSortField {
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
