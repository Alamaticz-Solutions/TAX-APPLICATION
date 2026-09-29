use appfw_runtime::identifier::to_table_case_lenient;

pub fn collection_name(schema_name: &str, collection_name: &str) -> String {
    format!("{schema_name}.{collection_name}")
}

pub fn entity_collection_name(schema_name: &str, entity_table_name: &str) -> String {
    collection_name(schema_name, entity_table_name)
}

pub fn physical_collection_name(name: &str) -> String {
    to_table_case_lenient(name)
}

pub fn junction_collection_name(schema_name: &str, junction_table: &str) -> String {
    collection_name(schema_name, &physical_collection_name(junction_table))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_names_include_schema_and_normalize_junction_tables() {
        assert_eq!(collection_name("crm", "accounts"), "crm.accounts");
        assert_eq!(
            entity_collection_name("crm", "account_contacts"),
            "crm.account_contacts"
        );
        assert_eq!(
            junction_collection_name("crm", "AccountContact"),
            "crm.account_contacts"
        );
        assert_eq!(
            junction_collection_name("finance", "t12m_ebitdas"),
            "finance.t12m_ebitdas"
        );
    }
}
