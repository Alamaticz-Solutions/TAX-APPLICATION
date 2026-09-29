use inflector::cases::{
    snakecase::to_snake_case,
    tablecase::{is_table_case, to_table_case},
};

pub fn to_snake_case_lenient(value: &str) -> String {
    if looks_like_snake_case(value) {
        value.to_string()
    } else {
        to_snake_case(value)
    }
}

pub fn to_table_case_lenient(value: &str) -> String {
    if is_table_case(value) || looks_like_plural_table_case(value) {
        value.to_string()
    } else {
        to_table_case(value)
    }
}

fn looks_like_plural_table_case(value: &str) -> bool {
    looks_like_snake_case(value) && value.ends_with('s')
}

fn looks_like_snake_case(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.starts_with('_') || value.ends_with('_') {
        return false;
    }
    if value.contains("__") {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_digit_bearing_snake_case_identifiers() {
        assert_eq!(to_snake_case_lenient("t12m_ebitda"), "t12m_ebitda");
        assert_eq!(to_snake_case_lenient("q1_revenue"), "q1_revenue");
    }

    #[test]
    fn converts_camel_and_pascal_case_identifiers() {
        assert_eq!(to_snake_case_lenient("accountName"), "account_name");
        assert_eq!(to_snake_case_lenient("AccountName"), "account_name");
    }

    #[test]
    fn preserves_digit_bearing_table_case_identifiers() {
        assert_eq!(to_table_case_lenient("t12m_ebitdas"), "t12m_ebitdas");
        assert_eq!(to_table_case_lenient("q1_revenues"), "q1_revenues");
    }

    #[test]
    fn converts_camel_and_pascal_case_table_identifiers() {
        assert_eq!(to_table_case_lenient("account_contact"), "account_contacts");
        assert_eq!(to_table_case_lenient("accountContact"), "account_contacts");
        assert_eq!(to_table_case_lenient("AccountContact"), "account_contacts");
    }
}
