use serde_json::Value;

use crate::{query_ir::RuntimePagination, RuntimeJsonObj};

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeProviderQueryPlan<E, Selection, Filter, Sort, AccessFilter> {
    pub entity_type: E,
    pub selection: Selection,
    pub filter: Option<Filter>,
    pub access_filter: Option<AccessFilter>,
    pub sort: Sort,
    pub pagination: RuntimePagination,
    selection_json: Value,
    filter_json: Option<Value>,
    sort_json: Option<Value>,
    access_filter_json: Option<Value>,
}

impl<E, Selection, Filter, Sort, AccessFilter>
    RuntimeProviderQueryPlan<E, Selection, Filter, Sort, AccessFilter>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_type: E,
        selection: Selection,
        filter: Option<Filter>,
        access_filter: Option<AccessFilter>,
        sort: Sort,
        pagination: RuntimePagination,
        selection_json: Value,
        filter_json: Option<Value>,
        sort_json: Option<Value>,
        access_filter_json: Option<Value>,
    ) -> Self {
        Self {
            entity_type,
            selection,
            filter,
            access_filter,
            sort,
            pagination,
            selection_json,
            filter_json,
            sort_json,
            access_filter_json,
        }
    }

    pub fn selection_json(&self) -> Value {
        self.selection_json.clone()
    }

    pub fn filter_json(&self) -> Option<Value> {
        self.filter_json.clone()
    }

    pub fn sort_json(&self) -> Option<Value> {
        self.sort_json.clone()
    }

    pub fn access_filter_json(&self) -> Option<Value> {
        self.access_filter_json.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeProviderAggregatePlan<E, Filter, AccessFilter, GroupBy, Metric, Having, Sort> {
    pub entity_type: E,
    pub filter: Option<Filter>,
    pub access_filter: Option<AccessFilter>,
    pub group_by: Vec<GroupBy>,
    pub metrics: Vec<Metric>,
    pub having: Option<Having>,
    pub sort: Sort,
    pub pagination: RuntimePagination,
    filter_json: Option<Value>,
    access_filter_json: Option<Value>,
}

impl<E, Filter, AccessFilter, GroupBy, Metric, Having, Sort>
    RuntimeProviderAggregatePlan<E, Filter, AccessFilter, GroupBy, Metric, Having, Sort>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_type: E,
        filter: Option<Filter>,
        access_filter: Option<AccessFilter>,
        group_by: Vec<GroupBy>,
        metrics: Vec<Metric>,
        having: Option<Having>,
        sort: Sort,
        pagination: RuntimePagination,
        filter_json: Option<Value>,
        access_filter_json: Option<Value>,
    ) -> Self {
        Self {
            entity_type,
            filter,
            access_filter,
            group_by,
            metrics,
            having,
            sort,
            pagination,
            filter_json,
            access_filter_json,
        }
    }

    pub fn filter_json(&self) -> Option<Value> {
        self.filter_json.clone()
    }

    pub fn access_filter_json(&self) -> Option<Value> {
        self.access_filter_json.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeProviderMutationKind {
    Create,
    Update { read_version: Option<Value> },
    Delete { read_version: Option<Value> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeProviderMutationPlan<E> {
    pub kind: RuntimeProviderMutationKind,
    pub entity: E,
    pub selection: Value,
    pub input: RuntimeJsonObj,
    pub access_filter: Option<Value>,
}

impl<E> RuntimeProviderMutationPlan<E> {
    pub fn new(
        kind: RuntimeProviderMutationKind,
        entity: E,
        selection: Value,
        input: RuntimeJsonObj,
        access_filter: Option<Value>,
    ) -> Self {
        Self {
            kind,
            entity,
            selection,
            input,
            access_filter,
        }
    }

    pub fn selection_json(&self) -> Value {
        self.selection.clone()
    }

    pub fn access_filter_json(&self) -> Option<Value> {
        self.access_filter.clone()
    }

    pub fn read_version(&self) -> Option<Value> {
        match &self.kind {
            RuntimeProviderMutationKind::Create => None,
            RuntimeProviderMutationKind::Update { read_version }
            | RuntimeProviderMutationKind::Delete { read_version } => read_version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map};

    use super::*;

    #[test]
    fn query_plan_preserves_provider_ready_json() {
        let plan = RuntimeProviderQueryPlan::new(
            "Account",
            "selection-ast",
            Some("filter-ast"),
            Some("access-filter-ast"),
            "sort-ast",
            RuntimePagination::new(5, 25).expect("valid pagination"),
            json!({ "name": "accounts", "selection_set": [] }),
            Some(json!({ "name": { "_eq": "Acme" } })),
            Some(json!([{ "field": "name", "direction": "asc" }])),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
        );

        assert_eq!(plan.entity_type, "Account");
        assert_eq!(plan.selection, "selection-ast");
        assert_eq!(plan.filter, Some("filter-ast"));
        assert_eq!(plan.access_filter, Some("access-filter-ast"));
        assert_eq!(plan.sort, "sort-ast");
        assert_eq!(plan.pagination.skip, 5);
        assert_eq!(plan.pagination.limit, 25);
        assert_eq!(
            plan.selection_json(),
            json!({ "name": "accounts", "selection_set": [] })
        );
        assert_eq!(
            plan.filter_json(),
            Some(json!({ "name": { "_eq": "Acme" } }))
        );
        assert_eq!(
            plan.sort_json(),
            Some(json!([{ "field": "name", "direction": "asc" }]))
        );
        assert_eq!(
            plan.access_filter_json(),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
        );
    }

    #[test]
    fn aggregate_plan_preserves_provider_ready_json() {
        let plan = RuntimeProviderAggregatePlan::new(
            "Account",
            Some("filter-ast"),
            Some("access-filter-ast"),
            vec!["group-by-industry"],
            vec!["metric-count"],
            Some("having-ast"),
            "sort-ast",
            RuntimePagination::new(0, 10).expect("valid pagination"),
            Some(json!({ "is_active": { "_eq": true } })),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
        );

        assert_eq!(plan.entity_type, "Account");
        assert_eq!(plan.filter, Some("filter-ast"));
        assert_eq!(plan.access_filter, Some("access-filter-ast"));
        assert_eq!(plan.group_by, vec!["group-by-industry"]);
        assert_eq!(plan.metrics, vec!["metric-count"]);
        assert_eq!(plan.having, Some("having-ast"));
        assert_eq!(plan.sort, "sort-ast");
        assert_eq!(plan.pagination.skip, 0);
        assert_eq!(plan.pagination.limit, 10);
        assert_eq!(
            plan.filter_json(),
            Some(json!({ "is_active": { "_eq": true } }))
        );
        assert_eq!(
            plan.access_filter_json(),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
        );
    }

    #[test]
    fn mutation_plan_preserves_provider_ready_json() {
        let input = Map::from_iter([("id".to_string(), json!("account-1"))]);
        let plan = RuntimeProviderMutationPlan::new(
            RuntimeProviderMutationKind::Update {
                read_version: Some(json!(7)),
            },
            "Account",
            json!({ "name": "accounts", "selection_set": [] }),
            input.clone(),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
        );

        assert_eq!(plan.entity, "Account");
        assert_eq!(
            plan.selection_json(),
            json!({ "name": "accounts", "selection_set": [] })
        );
        assert_eq!(plan.input, input);
        assert_eq!(
            plan.access_filter_json(),
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
        );
        assert_eq!(plan.read_version(), Some(json!(7)));
    }
}
