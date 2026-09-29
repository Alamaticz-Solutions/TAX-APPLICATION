use appfw_runtime::identifier::to_table_case_lenient;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCteJunctionJoin {
    pub junction_schema: String,
    pub junction_table: String,
    pub relationship_name: String,
    pub source_alias: String,
    pub source_pk: String,
    pub local_key: String,
    pub target_cte: String,
    pub target_pk: String,
    pub foreign_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCteNavigationJoin {
    pub cte_name: String,
    pub left_alias: String,
    pub left_key: String,
    pub right_alias: String,
    pub right_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCteDefinition {
    pub ref_cte_defs: Vec<String>,
    pub cte_name: String,
    pub select_fields: String,
    pub ref_cte_selects: Vec<String>,
    pub schema_name: String,
    pub table: String,
    pub alias: String,
    pub ref_cte_joins: Vec<String>,
    pub filter_clause: String,
    pub pk_name: String,
    pub sort_clause: String,
    pub limit_clause: String,
    pub performance_many_to_many_count: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCtePageQuery {
    pub cte_def: String,
    pub root_cte_name: String,
    pub skip: i32,
    pub limit: i32,
}

/// Controls whether the page query computes the expensive total `COUNT(*)` over the root CTE.
///
/// `COUNT(*)` over the root CTE forces Postgres to materialize and count every matching row on
/// every page request, which dominates cost on large tables and defeats the point of keyset
/// pagination. Callers that do not need an exact total (e.g. keyset/infinite-scroll) should use
/// [`PostgresPageCountMode::Skip`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum PostgresPageCountMode {
    /// Compute an exact `COUNT(*)` over the root CTE. This is the back-compatible default used by
    /// [`page_query_sql`] and is appropriate for offset pagination where a total/page count is
    /// surfaced to the client.
    #[default]
    Exact,
    /// Skip the total count entirely. The `count` column is emitted as the sentinel value `-1`
    /// (meaning "not computed") so the result shape stays a two-column `(count, rows)` row.
    /// Use this for keyset pagination, where the per-page count scan is pure overhead.
    Skip,
}

impl PostgresPageCountMode {
    /// SQL expression for the `count` column, given the root CTE name.
    fn count_select(self, root_cte_name: &str) -> String {
        match self {
            PostgresPageCountMode::Exact => {
                format!("(SELECT COUNT(*) FROM {root_cte_name})")
            }
            // Sentinel: count was intentionally not computed. Cast keeps the column type bigint so
            // the execution layer can read it as an i64 without a NULL/typing surprise.
            PostgresPageCountMode::Skip => "(-1)::bigint".to_string(),
        }
    }
}

pub fn physical_table_name(name: &str) -> String {
    to_table_case_lenient(name)
}

pub fn json_agg_expr(cte_name: &str, relationship_name: &str) -> String {
    format!(
        "COALESCE(json_agg({cte_name}) FILTER (WHERE {cte_name}.id IS NOT NULL), '[]'::json) as {relationship_name}"
    )
}

pub fn junction_source_join(join: &PostgresCteJunctionJoin) -> String {
    format!(
        "left join {schema}.{table} junction_{relationship} on {source_alias}.{source_pk} = junction_{relationship}.{local_key} /* INDEX HINT: idx_{table}_{local_key} */",
        schema = join.junction_schema,
        table = join.junction_table,
        relationship = join.relationship_name,
        source_alias = join.source_alias,
        source_pk = join.source_pk,
        local_key = join.local_key,
    )
}

pub fn junction_target_join(join: &PostgresCteJunctionJoin) -> String {
    format!(
        "left join {target_cte} on junction_{relationship}.{foreign_key} = {target_cte}.{target_pk} /* INDEX HINT: idx_{table}_{foreign_key} */",
        target_cte = join.target_cte,
        relationship = join.relationship_name,
        foreign_key = join.foreign_key,
        target_pk = join.target_pk,
        table = join.junction_table,
    )
}

pub fn navigation_join(join: &PostgresCteNavigationJoin) -> String {
    format!(
        "left join {cte_name} on {left_alias}.{left_key} = {right_alias}.{right_key}",
        cte_name = join.cte_name,
        left_alias = join.left_alias,
        left_key = join.left_key,
        right_alias = join.right_alias,
        right_key = join.right_key,
    )
}

pub fn definition_sql(definition: &PostgresCteDefinition) -> String {
    let ref_cte_defs_str = if definition.ref_cte_defs.is_empty() {
        String::new()
    } else {
        format!("{}, ", definition.ref_cte_defs.join(", "))
    };

    let ref_cte_selects_str = if definition.ref_cte_selects.is_empty() {
        String::new()
    } else {
        format!(", {}", definition.ref_cte_selects.join(", "))
    };

    let ref_cte_joins_str = definition.ref_cte_joins.join("\n");
    let performance_comment = definition
        .performance_many_to_many_count
        .map(|count| {
            format!(
                "\n          /* PERFORMANCE: ManyToMany relationships detected ({count}), ensure junction table indexes exist */"
            )
        })
        .unwrap_or_default();

    format!(
        r#"
          {ref_cte_defs_str}
          {cte_name} AS
          (
            select {select_fields}{ref_cte_selects_str}
            from {schema_name}.{table} {alias}
                  {ref_cte_joins_str}
            {filter_clause}
            group by {alias}.{pk_name}
            {sort_clause}
            {limit_clause}
          ){performance_comment}
        "#,
        cte_name = definition.cte_name,
        select_fields = definition.select_fields,
        schema_name = definition.schema_name,
        table = definition.table,
        alias = definition.alias,
        filter_clause = definition.filter_clause,
        pk_name = definition.pk_name,
        sort_clause = definition.sort_clause,
        limit_clause = definition.limit_clause,
    )
}

/// Build the page query SQL with an exact total `COUNT(*)`.
///
/// This is the back-compatible default. For large tables / keyset pagination prefer
/// [`page_query_sql_with_count`] with [`PostgresPageCountMode::Skip`] to avoid the per-page
/// count scan.
pub fn page_query_sql(query: &PostgresCtePageQuery) -> String {
    page_query_sql_with_count(query, PostgresPageCountMode::Exact)
}

/// Build the page query SQL, choosing whether to compute the expensive total count.
///
/// The result is always a single row with two columns `(count, rows)`:
/// - `count`: the exact total when [`PostgresPageCountMode::Exact`], or the sentinel `-1` when
///   [`PostgresPageCountMode::Skip`].
/// - `rows`: the page of rows for the requested `OFFSET`/`LIMIT` window.
pub fn page_query_sql_with_count(
    query: &PostgresCtePageQuery,
    count_mode: PostgresPageCountMode,
) -> String {
    format!(
        r#"
    WITH
    {cte_def}
    SELECT
      {count_select} as count,
      (SELECT json_agg(t.*) FROM (SELECT * FROM {root_cte_name} OFFSET {skip} LIMIT {limit}) AS t) AS rows
    "#,
        cte_def = query.cte_def,
        count_select = count_mode.count_select(&query.root_cte_name),
        root_cte_name = query.root_cte_name,
        skip = query.skip,
        limit = query.limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_physical_table_names() {
        assert_eq!(physical_table_name("AccountContact"), "account_contacts");
        assert_eq!(physical_table_name("account_contacts"), "account_contacts");
        assert_eq!(physical_table_name("t12m_ebitdas"), "t12m_ebitdas");
    }

    #[test]
    fn renders_json_aggregation_expression() {
        assert_eq!(
            json_agg_expr("contact_accounts_cte", "accounts"),
            "COALESCE(json_agg(contact_accounts_cte) FILTER (WHERE contact_accounts_cte.id IS NOT NULL), '[]'::json) as accounts"
        );
    }

    #[test]
    fn renders_many_to_many_junction_joins() {
        let join = PostgresCteJunctionJoin {
            junction_schema: "crm".to_string(),
            junction_table: "account_contacts".to_string(),
            relationship_name: "contacts".to_string(),
            source_alias: "t0".to_string(),
            source_pk: "id".to_string(),
            local_key: "account_id".to_string(),
            target_cte: "contact_contacts_account_cte".to_string(),
            target_pk: "id".to_string(),
            foreign_key: "contact_id".to_string(),
        };

        assert_eq!(
            junction_source_join(&join),
            "left join crm.account_contacts junction_contacts on t0.id = junction_contacts.account_id /* INDEX HINT: idx_account_contacts_account_id */"
        );
        assert_eq!(
            junction_target_join(&join),
            "left join contact_contacts_account_cte on junction_contacts.contact_id = contact_contacts_account_cte.id /* INDEX HINT: idx_account_contacts_contact_id */"
        );
    }

    #[test]
    fn renders_navigation_join() {
        let join = PostgresCteNavigationJoin {
            cte_name: "contact_cte".to_string(),
            left_alias: "t0".to_string(),
            left_key: "primary_contact_id".to_string(),
            right_alias: "contact_cte".to_string(),
            right_key: "id".to_string(),
        };

        assert_eq!(
            navigation_join(&join),
            "left join contact_cte on t0.primary_contact_id = contact_cte.id"
        );
    }

    #[test]
    fn renders_cte_definition_with_optional_performance_comment() {
        let definition = PostgresCteDefinition {
            ref_cte_defs: vec!["child_cte AS (select id from crm.contacts)".to_string()],
            cte_name: "t0_cte".to_string(),
            select_fields: "t0.id,t0.name".to_string(),
            ref_cte_selects: vec![json_agg_expr("child_cte", "contacts")],
            schema_name: "crm".to_string(),
            table: "accounts".to_string(),
            alias: "t0".to_string(),
            ref_cte_joins: vec!["left join child_cte on child_cte.account_id = t0.id".to_string()],
            filter_clause: "WHERE t0.active = $1".to_string(),
            pk_name: "id".to_string(),
            sort_clause: "ORDER BY t0.id ASC".to_string(),
            limit_clause: "LIMIT 50".to_string(),
            performance_many_to_many_count: Some(2),
        };

        let sql = definition_sql(&definition);
        assert!(sql.contains("child_cte AS (select id from crm.contacts),"));
        assert!(sql.contains("t0_cte AS"));
        assert!(sql.contains("select t0.id,t0.name, COALESCE(json_agg(child_cte)"));
        assert!(sql.contains("from crm.accounts t0"));
        assert!(sql.contains("left join child_cte on child_cte.account_id = t0.id"));
        assert!(sql.contains("WHERE t0.active = $1"));
        assert!(sql.contains("group by t0.id"));
        assert!(sql.contains("ORDER BY t0.id ASC"));
        assert!(sql.contains("LIMIT 50"));
        assert!(sql.contains("ManyToMany relationships detected (2)"));
    }

    #[test]
    fn renders_paged_cte_query() {
        let sql = page_query_sql(&PostgresCtePageQuery {
            cte_def: "t0_cte AS (SELECT id FROM crm.accounts)".to_string(),
            root_cte_name: "t0_cte".to_string(),
            skip: 10,
            limit: 25,
        });

        assert!(sql.contains("WITH"));
        assert!(sql.contains("t0_cte AS (SELECT id FROM crm.accounts)"));
        assert!(sql.contains("(SELECT COUNT(*) FROM t0_cte) as count"));
        assert!(sql.contains(
            "(SELECT json_agg(t.*) FROM (SELECT * FROM t0_cte OFFSET 10 LIMIT 25) AS t) AS rows"
        ));
    }

    #[test]
    fn default_page_query_keeps_exact_count_for_back_compat() {
        let query = PostgresCtePageQuery {
            cte_def: "t0_cte AS (SELECT id FROM crm.accounts)".to_string(),
            root_cte_name: "t0_cte".to_string(),
            skip: 0,
            limit: 50,
        };
        // page_query_sql and the explicit Exact mode must produce identical SQL.
        assert_eq!(
            page_query_sql(&query),
            page_query_sql_with_count(&query, PostgresPageCountMode::Exact)
        );
        assert!(page_query_sql(&query).contains("(SELECT COUNT(*) FROM t0_cte) as count"));
    }

    #[test]
    fn skip_count_mode_omits_count_scan_and_emits_sentinel() {
        let sql = page_query_sql_with_count(
            &PostgresCtePageQuery {
                cte_def: "t0_cte AS (SELECT id FROM crm.accounts)".to_string(),
                root_cte_name: "t0_cte".to_string(),
                skip: 0,
                limit: 25,
            },
            PostgresPageCountMode::Skip,
        );

        // No COUNT(*) scan over the root CTE: the expensive per-page count is skipped.
        assert!(!sql.contains("COUNT(*)"));
        // Sentinel keeps the two-column (count, rows) shape; -1 means "count not computed".
        assert!(sql.contains("(-1)::bigint as count"));
        // The page window is still produced exactly as before.
        assert!(sql.contains(
            "(SELECT json_agg(t.*) FROM (SELECT * FROM t0_cte OFFSET 0 LIMIT 25) AS t) AS rows"
        ));
    }
}
