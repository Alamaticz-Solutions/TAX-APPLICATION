use std::env;

use serde::Serialize;

use crate::{model_metadata::RuntimeDataType, RuntimeError};

const DEFAULT_MAX_QUERY_COST: u32 = 1_000;
const DEFAULT_MAX_AGGREGATE_COST: u32 = 1_500;
const DEFAULT_MAX_RELATIONSHIP_DEPTH: u32 = 4;
const DEFAULT_MAX_MANY_TO_MANY_EXPANSIONS: u32 = 4;
const DEFAULT_MAX_FILTER_PREDICATES: u32 = 25;
const DEFAULT_MAX_SORT_SPECS: u32 = 3;
const DEFAULT_MAX_AGGREGATE_OUTPUTS: u32 = 12;
const DEFAULT_MAX_SELECTED_FIELDS: u32 = 80;
const DEFAULT_MAX_OFFSET_ROWS: u32 = 5_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryCostBudget {
    pub max_query_cost: u32,
    pub max_aggregate_cost: u32,
    pub max_relationship_depth: u32,
    pub max_many_to_many_expansions: u32,
    pub max_filter_predicates: u32,
    pub max_sort_specs: u32,
    pub max_aggregate_outputs: u32,
    pub max_selected_fields: u32,
    pub max_offset_rows: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct QueryCost {
    pub score: u32,
    pub selected_fields: u32,
    pub relationship_depth: u32,
    pub relationship_edges: u32,
    pub many_to_many_expansions: u32,
    pub filter_predicates: u32,
    pub sort_specs: u32,
    pub aggregate_outputs: u32,
    pub offset_rows: u32,
    pub page_size: u32,
    pub pagination_cost: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeQueryCostInput {
    pub selection: RuntimeSelectionCostTree,
    pub filter: Option<RuntimeFilterCostNode>,
    pub access_filter: Option<RuntimeFilterCostNode>,
    pub sort_specs: usize,
    pub skip: i32,
    pub limit: i32,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeAggregateCostInput {
    pub filter: Option<RuntimeFilterCostNode>,
    pub access_filter: Option<RuntimeFilterCostNode>,
    pub sort_specs: usize,
    pub group_by_count: usize,
    pub metric_count: usize,
    pub having_predicates: usize,
    pub skip: i32,
    pub limit: i32,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeSelectionCostTree {
    pub fields: Vec<RuntimeSelectionCostNode>,
}

#[derive(Clone, Debug)]
pub struct RuntimeSelectionCostNode {
    pub data_type: RuntimeDataType,
    pub has_target_entity: bool,
    pub children: Vec<RuntimeSelectionCostNode>,
}

#[derive(Clone, Debug)]
pub enum RuntimeFilterCostNode {
    All,
    And(Vec<RuntimeFilterCostNode>),
    Or(Vec<RuntimeFilterCostNode>),
    Field {
        data_type: RuntimeDataType,
    },
    Relation {
        kind: RuntimeRelationKind,
        filter: Box<RuntimeFilterCostNode>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRelationKind {
    NavByForeignKey,
    ManyToMany,
}

impl QueryCostBudget {
    pub fn standard() -> Self {
        Self {
            max_query_cost: DEFAULT_MAX_QUERY_COST,
            max_aggregate_cost: DEFAULT_MAX_AGGREGATE_COST,
            max_relationship_depth: DEFAULT_MAX_RELATIONSHIP_DEPTH,
            max_many_to_many_expansions: DEFAULT_MAX_MANY_TO_MANY_EXPANSIONS,
            max_filter_predicates: DEFAULT_MAX_FILTER_PREDICATES,
            max_sort_specs: DEFAULT_MAX_SORT_SPECS,
            max_aggregate_outputs: DEFAULT_MAX_AGGREGATE_OUTPUTS,
            max_selected_fields: DEFAULT_MAX_SELECTED_FIELDS,
            max_offset_rows: DEFAULT_MAX_OFFSET_ROWS,
        }
    }

    pub fn from_env() -> Self {
        Self {
            max_query_cost: env_u32("APP_QUERY_COST_MAX", DEFAULT_MAX_QUERY_COST),
            max_aggregate_cost: env_u32("APP_AGGREGATE_QUERY_COST_MAX", DEFAULT_MAX_AGGREGATE_COST),
            max_relationship_depth: env_u32(
                "APP_QUERY_RELATIONSHIP_DEPTH_MAX",
                DEFAULT_MAX_RELATIONSHIP_DEPTH,
            ),
            max_many_to_many_expansions: env_u32(
                "APP_QUERY_MANY_TO_MANY_EXPANSION_MAX",
                DEFAULT_MAX_MANY_TO_MANY_EXPANSIONS,
            ),
            max_filter_predicates: env_u32(
                "APP_QUERY_FILTER_PREDICATES_MAX",
                DEFAULT_MAX_FILTER_PREDICATES,
            ),
            max_sort_specs: env_u32("APP_QUERY_SORT_SPECS_MAX", DEFAULT_MAX_SORT_SPECS),
            max_aggregate_outputs: env_u32(
                "APP_AGGREGATE_OUTPUTS_MAX",
                DEFAULT_MAX_AGGREGATE_OUTPUTS,
            ),
            max_selected_fields: env_u32(
                "APP_QUERY_SELECTED_FIELDS_MAX",
                DEFAULT_MAX_SELECTED_FIELDS,
            ),
            max_offset_rows: env_u32("APP_QUERY_OFFSET_ROWS_MAX", DEFAULT_MAX_OFFSET_ROWS),
        }
    }

    pub fn enforce_query(&self, plan: &RuntimeQueryCostInput) -> Result<QueryCost, RuntimeError> {
        let cost = QueryCost::for_query(plan);
        self.enforce_cost(&cost)?;
        if cost.score > self.max_query_cost {
            return Err(RuntimeError::Validation(format!(
                "query cost {} exceeds budget {}",
                cost.score, self.max_query_cost
            )));
        }
        Ok(cost)
    }

    pub fn enforce_aggregate(
        &self,
        plan: &RuntimeAggregateCostInput,
    ) -> Result<QueryCost, RuntimeError> {
        let cost = QueryCost::for_aggregate(plan);
        self.enforce_cost(&cost)?;
        if cost.score > self.max_aggregate_cost {
            return Err(RuntimeError::Validation(format!(
                "aggregate query cost {} exceeds budget {}",
                cost.score, self.max_aggregate_cost
            )));
        }
        Ok(cost)
    }

    pub fn enforce_cost(&self, cost: &QueryCost) -> Result<(), RuntimeError> {
        if cost.relationship_depth > self.max_relationship_depth {
            return Err(RuntimeError::Validation(format!(
                "relationship depth {} exceeds budget {}",
                cost.relationship_depth, self.max_relationship_depth
            )));
        }
        if cost.many_to_many_expansions > self.max_many_to_many_expansions {
            return Err(RuntimeError::Validation(format!(
                "many-to-many expansion count {} exceeds budget {}",
                cost.many_to_many_expansions, self.max_many_to_many_expansions
            )));
        }
        if cost.filter_predicates > self.max_filter_predicates {
            return Err(RuntimeError::Validation(format!(
                "filter predicate count {} exceeds budget {}",
                cost.filter_predicates, self.max_filter_predicates
            )));
        }
        if cost.sort_specs > self.max_sort_specs {
            return Err(RuntimeError::Validation(format!(
                "sort spec count {} exceeds budget {}",
                cost.sort_specs, self.max_sort_specs
            )));
        }
        if cost.aggregate_outputs > self.max_aggregate_outputs {
            return Err(RuntimeError::Validation(format!(
                "aggregate output count {} exceeds budget {}",
                cost.aggregate_outputs, self.max_aggregate_outputs
            )));
        }
        if cost.selected_fields > self.max_selected_fields {
            return Err(RuntimeError::Validation(format!(
                "selected field count {} exceeds budget {}",
                cost.selected_fields, self.max_selected_fields
            )));
        }
        if cost.offset_rows > self.max_offset_rows {
            return Err(RuntimeError::Validation(format!(
                "offset row count {} exceeds budget {}",
                cost.offset_rows, self.max_offset_rows
            )));
        }
        Ok(())
    }
}

impl Default for QueryCostBudget {
    fn default() -> Self {
        Self::standard()
    }
}

impl QueryCost {
    pub fn for_query(plan: &RuntimeQueryCostInput) -> Self {
        let mut cost = Self::default();
        selection_cost(&plan.selection, &mut cost);
        filter_cost(plan.filter.as_ref(), &mut cost);
        filter_cost(plan.access_filter.as_ref(), &mut cost);
        cost.sort_specs = plan.sort_specs as u32;
        cost.offset_rows = plan.skip.max(0) as u32;
        cost.page_size = plan.limit.max(0) as u32;
        cost.pagination_cost = pagination_cost(plan.skip, plan.limit);
        cost.score = 1
            + cost.pagination_cost
            + cost.selected_fields
            + cost.sort_specs * 2
            + cost.filter_predicates * 3
            + cost.relationship_edges * 10
            + cost.many_to_many_expansions * 25
            + cost.relationship_depth * 15;
        cost
    }

    pub fn for_aggregate(plan: &RuntimeAggregateCostInput) -> Self {
        let mut cost = Self::default();
        filter_cost(plan.filter.as_ref(), &mut cost);
        filter_cost(plan.access_filter.as_ref(), &mut cost);
        cost.sort_specs = plan.sort_specs as u32;
        cost.aggregate_outputs = (plan.group_by_count + plan.metric_count) as u32;
        cost.offset_rows = plan.skip.max(0) as u32;
        cost.page_size = plan.limit.max(0) as u32;
        cost.pagination_cost = pagination_cost(plan.skip, plan.limit);
        cost.score = 25
            + cost.pagination_cost
            + cost.sort_specs * 3
            + cost.filter_predicates * 4
            + cost.aggregate_outputs * 15
            + (plan.having_predicates as u32 * 8);
        cost
    }
}

fn selection_cost(selection: &RuntimeSelectionCostTree, cost: &mut QueryCost) {
    for node in &selection.fields {
        selection_node_cost(node, 0, cost);
    }
}

fn selection_node_cost(node: &RuntimeSelectionCostNode, depth: u32, cost: &mut QueryCost) {
    cost.selected_fields += 1;
    if node.has_target_entity || node.data_type.is_relationship() {
        let next_depth = depth + 1;
        cost.relationship_depth = cost.relationship_depth.max(next_depth);
        cost.relationship_edges += 1;
        if node.data_type.is_many_to_many() {
            cost.many_to_many_expansions += 1;
        }
        for child in &node.children {
            selection_node_cost(child, next_depth, cost);
        }
    }
}

fn filter_cost(filter: Option<&RuntimeFilterCostNode>, cost: &mut QueryCost) {
    let Some(filter) = filter else {
        return;
    };
    filter_node_cost(filter, 0, cost);
}

fn filter_node_cost(filter: &RuntimeFilterCostNode, depth: u32, cost: &mut QueryCost) {
    match filter {
        RuntimeFilterCostNode::All => {}
        RuntimeFilterCostNode::And(items) | RuntimeFilterCostNode::Or(items) => {
            for item in items {
                filter_node_cost(item, depth, cost);
            }
        }
        RuntimeFilterCostNode::Field { .. } => cost.filter_predicates += 1,
        RuntimeFilterCostNode::Relation { kind, filter } => {
            let next_depth = depth + 1;
            cost.relationship_edges += 1;
            cost.relationship_depth = cost.relationship_depth.max(next_depth);
            if *kind == RuntimeRelationKind::ManyToMany {
                cost.many_to_many_expansions += 1;
            }
            filter_node_cost(filter.as_ref(), next_depth, cost);
        }
    }
}

/// Per-100-rows penalty applied to the OFFSET (`skip`). Deep offsets force Postgres to scan and
/// discard every skipped row on each page, so the penalty is steep enough that very deep offsets
/// (e.g. skip=100000) blow the default query-cost budget (1_000) and are rejected, steering
/// clients toward keyset pagination. Shallow offsets (a few pages) stay cheap.
const OFFSET_COST_PER_HUNDRED_ROWS: u32 = 40;

fn pagination_cost(skip: i32, limit: i32) -> u32 {
    let skip = skip.max(0) as u32;
    let limit = limit.max(0) as u32;
    let offset_cost = (skip / 100) * OFFSET_COST_PER_HUNDRED_ROWS;
    offset_cost + (limit / 100) + 1
}

fn env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_with_many_to_many_selection() -> RuntimeQueryCostInput {
        RuntimeQueryCostInput {
            selection: RuntimeSelectionCostTree {
                fields: vec![
                    RuntimeSelectionCostNode {
                        data_type: RuntimeDataType::Uuid,
                        has_target_entity: false,
                        children: Vec::new(),
                    },
                    RuntimeSelectionCostNode {
                        data_type: RuntimeDataType::ManyToMany,
                        has_target_entity: true,
                        children: vec![RuntimeSelectionCostNode {
                            data_type: RuntimeDataType::Uuid,
                            has_target_entity: false,
                            children: Vec::new(),
                        }],
                    },
                ],
            },
            filter: Some(RuntimeFilterCostNode::Field {
                data_type: RuntimeDataType::String,
            }),
            access_filter: None,
            sort_specs: 1,
            skip: 0,
            limit: 25,
        }
    }

    #[test]
    fn relationship_and_many_to_many_selection_increase_query_cost() {
        let cost = QueryCost::for_query(&query_with_many_to_many_selection());

        assert!(cost.score > 1);
        assert_eq!(cost.relationship_depth, 1);
        assert_eq!(cost.relationship_edges, 1);
        assert_eq!(cost.many_to_many_expansions, 1);
    }

    #[test]
    fn budget_rejects_excessive_many_to_many_expansion() {
        let budget = QueryCostBudget {
            max_many_to_many_expansions: 0,
            ..QueryCostBudget::default()
        };
        let cost = QueryCost {
            many_to_many_expansions: 1,
            ..QueryCost::default()
        };

        let err = budget.enforce_cost(&cost).expect_err("budget should fail");

        assert!(err.to_string().contains("many-to-many expansion count"));
    }

    #[test]
    fn deep_offset_is_budget_rejected_while_shallow_offset_is_cheap() {
        let budget = QueryCostBudget {
            max_offset_rows: 200_000,
            ..QueryCostBudget::default()
        };

        // A shallow offset (first few pages) stays well within budget.
        let shallow = RuntimeQueryCostInput {
            skip: 100,
            limit: 50,
            ..RuntimeQueryCostInput::default()
        };
        budget
            .enforce_query(&shallow)
            .expect("shallow offset should be accepted");

        // A deep offset forces Postgres to scan and discard 100k rows; it must be rejected so
        // clients are steered toward keyset pagination.
        let deep = RuntimeQueryCostInput {
            skip: 100_000,
            limit: 50,
            ..RuntimeQueryCostInput::default()
        };
        let err = budget
            .enforce_query(&deep)
            .expect_err("deep offset should exceed the query cost budget");
        assert!(err.to_string().contains("query cost"));

        // Direct cost comparison: the deep offset dominates the score.
        assert!(QueryCost::for_query(&deep).pagination_cost > DEFAULT_MAX_QUERY_COST);
        assert!(QueryCost::for_query(&shallow).pagination_cost < 100);
    }

    #[test]
    fn budget_rejects_query_plan_when_query_ir_cost_exceeds_limit() {
        let budget = QueryCostBudget {
            max_query_cost: 1,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&query_with_many_to_many_selection())
            .expect_err("budget should reject high-cost query");

        assert!(err.to_string().contains("query cost"));
    }

    #[test]
    fn budget_rejects_excessive_filter_predicates() {
        let plan = RuntimeQueryCostInput {
            filter: Some(RuntimeFilterCostNode::And(vec![
                RuntimeFilterCostNode::Field {
                    data_type: RuntimeDataType::String,
                },
                RuntimeFilterCostNode::Field {
                    data_type: RuntimeDataType::String,
                },
            ])),
            ..RuntimeQueryCostInput::default()
        };
        let budget = QueryCostBudget {
            max_filter_predicates: 1,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&plan)
            .expect_err("filter predicate cap should reject the query");

        assert!(err.to_string().contains("filter predicate count 2"));
    }

    #[test]
    fn budget_rejects_nested_relationship_filter_depth() {
        let plan = RuntimeQueryCostInput {
            filter: Some(RuntimeFilterCostNode::Relation {
                kind: RuntimeRelationKind::NavByForeignKey,
                filter: Box::new(RuntimeFilterCostNode::Relation {
                    kind: RuntimeRelationKind::NavByForeignKey,
                    filter: Box::new(RuntimeFilterCostNode::Field {
                        data_type: RuntimeDataType::String,
                    }),
                }),
            }),
            ..RuntimeQueryCostInput::default()
        };
        let budget = QueryCostBudget {
            max_relationship_depth: 1,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&plan)
            .expect_err("nested relationship filter should exceed depth budget");

        assert!(err.to_string().contains("relationship depth 2"));
    }

    #[test]
    fn budget_rejects_excessive_sort_specs() {
        let plan = RuntimeQueryCostInput {
            sort_specs: 2,
            ..RuntimeQueryCostInput::default()
        };
        let budget = QueryCostBudget {
            max_sort_specs: 1,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&plan)
            .expect_err("sort spec cap should reject the query");

        assert!(err.to_string().contains("sort spec count 2"));
    }

    #[test]
    fn budget_rejects_excessive_selected_fields() {
        let plan = RuntimeQueryCostInput {
            selection: RuntimeSelectionCostTree {
                fields: vec![
                    RuntimeSelectionCostNode {
                        data_type: RuntimeDataType::String,
                        has_target_entity: false,
                        children: Vec::new(),
                    },
                    RuntimeSelectionCostNode {
                        data_type: RuntimeDataType::String,
                        has_target_entity: false,
                        children: Vec::new(),
                    },
                ],
            },
            ..RuntimeQueryCostInput::default()
        };
        let budget = QueryCostBudget {
            max_selected_fields: 1,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&plan)
            .expect_err("selected field cap should reject the query");

        assert!(err.to_string().contains("selected field count 2"));
    }

    #[test]
    fn budget_rejects_excessive_aggregate_outputs() {
        let plan = RuntimeAggregateCostInput {
            group_by_count: 2,
            metric_count: 2,
            ..RuntimeAggregateCostInput::default()
        };
        let budget = QueryCostBudget {
            max_aggregate_outputs: 3,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_aggregate(&plan)
            .expect_err("aggregate output cap should reject the aggregate");

        assert!(err.to_string().contains("aggregate output count 4"));
    }

    #[test]
    fn budget_rejects_offset_rows_before_score_limit() {
        let plan = RuntimeQueryCostInput {
            skip: 101,
            limit: 25,
            ..RuntimeQueryCostInput::default()
        };
        let budget = QueryCostBudget {
            max_query_cost: 1_000,
            max_offset_rows: 100,
            ..QueryCostBudget::default()
        };

        let err = budget
            .enforce_query(&plan)
            .expect_err("offset row cap should reject the query");

        assert!(err.to_string().contains("offset row count 101"));
    }
}
