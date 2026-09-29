#![allow(dead_code)]

use std::sync::Arc;

use appfw_runtime::{
    json::JsonObj,
    query_cost::{
        RuntimeAggregateCostInput, RuntimeFilterCostNode, RuntimeQueryCostInput,
        RuntimeRelationKind, RuntimeSelectionCostNode, RuntimeSelectionCostTree,
    },
    query_filter::conjunction_token as conjunction,
    query_ir as runtime_query_ir, PolicyAccess, QueryCost, QueryCostBudget,
    RuntimeProviderAggregatePlan, RuntimeProviderMutationPlan, RuntimeProviderQueryPlan,
};
use serde_json::{json, Map, Value};

use crate::{
    config::app_config::AppConfig,
    product_api::{
        runtime_model_metadata, runtime_property_metadata, RuntimeDataType, RuntimeEntityMetadata,
        RuntimeModelMetadata, RuntimePropertyMetadata,
    },
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType, PropertyType},
};

pub use crate::product_api::RuntimeFilterOp as FilterOp;
pub use appfw_runtime::query_ir::{
    RuntimeAggregateFieldDescriptor as AggregateFieldDescriptor,
    RuntimeAggregateFunction as AggregateFunction, RuntimePagination as Pagination,
    RuntimePaginationPolicy as PaginationPolicy, RuntimeSortDirection as SortDirection,
};
pub use appfw_runtime::RuntimeProviderMutationKind as MutationKind;

#[derive(Clone, Debug)]
pub enum FilterAst {
    All,
    And(Vec<FilterAst>),
    Or(Vec<FilterAst>),
    Field(FilterPredicate),
    Relation(RelationFilter),
}

pub type AccessFilterAst = FilterAst;

#[derive(Clone, Debug)]
pub struct FilterPredicate {
    pub prop: Arc<PropertyType>,
    pub runtime_prop: RuntimePropertyMetadata,
    pub op: FilterOp,
    pub value: Value,
}

#[derive(Clone, Debug)]
pub struct RelationFilter {
    pub prop: Arc<PropertyType>,
    pub runtime_prop: RuntimePropertyMetadata,
    pub target_entity_type: Arc<EntityType>,
    pub kind: RelationKind,
    pub filter: Box<FilterAst>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationKind {
    NavByForeignKey,
    ManyToMany,
}

#[derive(Clone, Debug, Default)]
pub struct SortAst {
    pub specs: Vec<SortSpec>,
}

#[derive(Clone, Debug)]
pub struct SortSpec {
    pub prop: Arc<PropertyType>,
    pub runtime_prop: RuntimePropertyMetadata,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct SelectionTree {
    pub name: String,
    pub runtime_entity: RuntimeEntityMetadata,
    pub fields: Vec<SelectionNode>,
}

#[derive(Clone, Debug)]
pub struct SelectionNode {
    pub prop: Arc<PropertyType>,
    pub runtime_prop: RuntimePropertyMetadata,
    pub target_entity_type: Option<Arc<EntityType>>,
    pub children: Vec<SelectionNode>,
}

#[derive(Clone, Debug)]
pub struct QueryPlan {
    pub entity_type: Arc<EntityType>,
    pub selection: SelectionTree,
    pub filter: Option<FilterAst>,
    pub access_filter: Option<AccessFilterAst>,
    pub sort: SortAst,
    pub pagination: Pagination,
}

#[derive(Clone, Debug)]
pub struct AggregatePlan {
    pub entity_type: Arc<EntityType>,
    pub filter: Option<FilterAst>,
    pub access_filter: Option<AccessFilterAst>,
    pub group_by: Vec<GroupBySpec>,
    pub metrics: Vec<AggregateMetric>,
    pub having: Option<AggregateHavingAst>,
    pub sort: AggregateSortAst,
    pub pagination: Pagination,
}

#[derive(Clone, Debug)]
pub struct GroupBySpec {
    pub prop: Arc<PropertyType>,
    pub alias: String,
}

#[derive(Clone, Debug)]
pub struct AggregateMetric {
    pub function: AggregateFunction,
    pub prop: Option<Arc<PropertyType>>,
    pub alias: String,
}

#[derive(Clone, Debug, Default)]
pub struct AggregateHavingAst {
    pub predicates: Vec<AggregateHavingPredicate>,
}

#[derive(Clone, Debug)]
pub struct AggregateHavingPredicate {
    pub metric_alias: String,
    pub op: FilterOp,
    pub value: Value,
}

#[derive(Clone, Debug, Default)]
pub struct AggregateSortAst {
    pub specs: Vec<AggregateSortSpec>,
}

#[derive(Clone, Debug)]
pub struct AggregateSortSpec {
    pub alias: String,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct MutationPlan {
    pub kind: MutationKind,
    pub entity_type: Arc<EntityType>,
    pub selection: Option<SelectionTree>,
    pub input: JsonObj,
    pub access_filter: Option<AccessFilterAst>,
}

impl QueryPlan {
    pub fn new(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        let plan = Self {
            entity_type: entity_type.clone(),
            selection: SelectionTree::parse_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                selections,
            )?,
            filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                filter,
            )?,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type.clone(),
                access.filter.clone(),
            )?,
            sort: SortAst::parse(entity_type, sort)?,
            pagination: Pagination::new(skip, limit)?,
        };
        enforce_query_cost(&plan)?;
        Ok(plan)
    }

    pub fn new_keyset(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<Value>,
        sort: Option<Value>,
        after: Option<String>,
        limit: i32,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let sort = ensure_keyset_sort(app_config.clone(), entity_type.clone(), sort)?;
        let filter = apply_keyset_cursor_filter(filter, &sort, after.as_deref())?;
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        let plan = Self {
            entity_type: entity_type.clone(),
            selection: SelectionTree::parse_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                selections,
            )?,
            filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                filter,
            )?,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type.clone(),
                access.filter.clone(),
            )?,
            sort: SortAst::parse(entity_type, Some(sort))?,
            pagination: Pagination::keyset(after, limit)?,
        };
        enforce_query_cost(&plan)?;
        Ok(plan)
    }

    pub fn filter_json(&self) -> Option<Value> {
        self.filter.as_ref().and_then(FilterAst::to_filter_value)
    }

    pub fn sort_json(&self) -> Option<Value> {
        self.sort.to_sort_value()
    }

    pub fn into_runtime_provider_plan(
        self,
    ) -> RuntimeProviderQueryPlan<Arc<EntityType>, SelectionTree, FilterAst, SortAst, AccessFilterAst>
    {
        let selection_json = self.selection.to_json();
        let filter_json = self.filter.as_ref().and_then(FilterAst::to_filter_value);
        let sort_json = self.sort.to_sort_value();
        let access_filter_json = self
            .access_filter
            .as_ref()
            .and_then(FilterAst::to_filter_value);
        RuntimeProviderQueryPlan::new(
            self.entity_type,
            self.selection,
            self.filter,
            self.access_filter,
            self.sort,
            self.pagination,
            selection_json,
            filter_json,
            sort_json,
            access_filter_json,
        )
    }
}

impl AggregatePlan {
    pub fn new(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        filter: Option<Value>,
        group_by: Option<Value>,
        metrics: Option<Value>,
        having: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let group_by = GroupBySpec::parse_many(entity_type.clone(), group_by)?;
        let metrics = AggregateMetric::parse_many(entity_type.clone(), metrics)?;
        ensure_unique_aggregate_aliases(&group_by, &metrics)?;
        let having = AggregateHavingAst::parse_optional(&metrics, having)?;
        let sort = AggregateSortAst::parse(&group_by, &metrics, sort)?;
        let model_metadata = runtime_model_metadata(app_config.as_ref());

        let plan = Self {
            entity_type: entity_type.clone(),
            filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                filter,
            )?,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type.clone(),
                access.filter.clone(),
            )?,
            group_by,
            metrics,
            having,
            sort,
            pagination: Pagination::new(skip, limit)?,
        };
        enforce_aggregate_cost(&plan)?;
        Ok(plan)
    }

    pub fn into_runtime_provider_plan(
        self,
    ) -> RuntimeProviderAggregatePlan<
        Arc<EntityType>,
        FilterAst,
        AccessFilterAst,
        GroupBySpec,
        AggregateMetric,
        AggregateHavingAst,
        AggregateSortAst,
    > {
        let filter_json = self.filter.as_ref().and_then(FilterAst::to_filter_value);
        let access_filter_json = self
            .access_filter
            .as_ref()
            .and_then(FilterAst::to_filter_value);
        RuntimeProviderAggregatePlan::new(
            self.entity_type,
            self.filter,
            self.access_filter,
            self.group_by,
            self.metrics,
            self.having,
            self.sort,
            self.pagination,
            filter_json,
            access_filter_json,
        )
    }
}

pub(crate) fn cost_for_query(plan: &QueryPlan) -> QueryCost {
    QueryCost::for_query(&runtime_query_cost_input(plan))
}

fn enforce_query_cost(plan: &QueryPlan) -> Result<QueryCost, AppError> {
    QueryCostBudget::from_env()
        .enforce_query(&runtime_query_cost_input(plan))
        .map_err(AppError::from)
}

fn enforce_aggregate_cost(plan: &AggregatePlan) -> Result<QueryCost, AppError> {
    QueryCostBudget::from_env()
        .enforce_aggregate(&runtime_aggregate_cost_input(plan))
        .map_err(AppError::from)
}

fn runtime_query_cost_input(plan: &QueryPlan) -> RuntimeQueryCostInput {
    RuntimeQueryCostInput {
        selection: runtime_selection_tree(&plan.selection),
        filter: plan.filter.as_ref().map(runtime_filter_node),
        access_filter: plan.access_filter.as_ref().map(runtime_filter_node),
        sort_specs: plan.sort.specs.len(),
        skip: plan.pagination.skip,
        limit: plan.pagination.limit,
    }
}

fn runtime_aggregate_cost_input(plan: &AggregatePlan) -> RuntimeAggregateCostInput {
    RuntimeAggregateCostInput {
        filter: plan.filter.as_ref().map(runtime_filter_node),
        access_filter: plan.access_filter.as_ref().map(runtime_filter_node),
        sort_specs: plan.sort.specs.len(),
        group_by_count: plan.group_by.len(),
        metric_count: plan.metrics.len(),
        having_predicates: plan
            .having
            .as_ref()
            .map(|having| having.predicates.len())
            .unwrap_or(0),
        skip: plan.pagination.skip,
        limit: plan.pagination.limit,
    }
}

fn runtime_selection_tree(selection: &SelectionTree) -> RuntimeSelectionCostTree {
    RuntimeSelectionCostTree {
        fields: selection
            .fields
            .iter()
            .map(runtime_selection_node)
            .collect(),
    }
}

fn runtime_selection_node(node: &SelectionNode) -> RuntimeSelectionCostNode {
    RuntimeSelectionCostNode {
        data_type: node.runtime_prop.data_type,
        has_target_entity: node.target_entity_type.is_some(),
        children: node.children.iter().map(runtime_selection_node).collect(),
    }
}

fn runtime_filter_node(filter: &FilterAst) -> RuntimeFilterCostNode {
    match filter {
        FilterAst::All => RuntimeFilterCostNode::All,
        FilterAst::And(items) => {
            RuntimeFilterCostNode::And(items.iter().map(runtime_filter_node).collect::<Vec<_>>())
        }
        FilterAst::Or(items) => {
            RuntimeFilterCostNode::Or(items.iter().map(runtime_filter_node).collect::<Vec<_>>())
        }
        FilterAst::Field(predicate) => RuntimeFilterCostNode::Field {
            data_type: predicate.runtime_prop.data_type,
        },
        FilterAst::Relation(relation) => RuntimeFilterCostNode::Relation {
            kind: if relation.runtime_prop.data_type.is_many_to_many() {
                RuntimeRelationKind::ManyToMany
            } else {
                match relation.kind {
                    RelationKind::NavByForeignKey => RuntimeRelationKind::NavByForeignKey,
                    RelationKind::ManyToMany => RuntimeRelationKind::ManyToMany,
                }
            },
            filter: Box::new(runtime_filter_node(relation.filter.as_ref())),
        },
    }
}

impl GroupBySpec {
    fn parse_many(
        entity_type: Arc<EntityType>,
        input: Option<Value>,
    ) -> Result<Vec<Self>, AppError> {
        let Some(items) = normalize_array_input(input, "group_by")? else {
            return Ok(Vec::new());
        };
        let mut groups = Vec::with_capacity(items.len());
        for item in items {
            let (field, alias) = match item {
                Value::String(field) => {
                    let prop = AppConfig::get_prop(entity_type.clone(), &field)?;
                    let alias = prop.name.clone();
                    (field, alias)
                }
                Value::Object(mut obj) => {
                    let field = take_string(&mut obj, "field", "group_by item")?;
                    let alias = obj
                        .remove("alias")
                        .map(|value| expect_string(value, "group_by alias"))
                        .transpose()?;
                    let prop = AppConfig::get_prop(entity_type.clone(), &field)?;
                    let alias = alias.unwrap_or_else(|| prop.name.clone());
                    (field, alias)
                }
                other => {
                    return Err(AppError::Validation(format!(
                        "group_by entries must be field strings or objects, got {}",
                        value_kind(&other)
                    )))
                }
            };
            let prop = AppConfig::get_prop(entity_type.clone(), &field)?;
            let runtime_prop = runtime_property_metadata(prop.as_ref());
            ensure_groupable(&runtime_prop)?;
            validate_output_alias(&alias)?;
            groups.push(Self { prop, alias });
        }
        Ok(groups)
    }
}

impl AggregateMetric {
    fn parse_many(
        entity_type: Arc<EntityType>,
        input: Option<Value>,
    ) -> Result<Vec<Self>, AppError> {
        let Some(items) = normalize_array_input(input, "metrics")? else {
            return Ok(vec![Self {
                function: AggregateFunction::Count,
                prop: None,
                alias: "count".to_string(),
            }]);
        };
        if items.is_empty() {
            return Err(AppError::Validation(
                "metrics must contain at least one aggregate metric".to_string(),
            ));
        }

        let mut metrics = Vec::with_capacity(items.len());
        for item in items {
            let mut obj = item.as_object().cloned().ok_or_else(|| {
                AppError::Validation(format!(
                    "metrics entries must be objects, got {}",
                    value_kind(&item)
                ))
            })?;
            let function = AggregateFunction::from_name(&take_string_any(
                &mut obj,
                &["fn", "function"],
                "metric function",
            )?)?;
            let field = obj
                .remove("field")
                .map(|value| expect_string(value, "metric field"))
                .transpose()?;
            let prop = field
                .as_ref()
                .map(|field| AppConfig::get_prop(entity_type.clone(), field))
                .transpose()?;
            let runtime_prop = prop
                .as_ref()
                .map(|prop| runtime_property_metadata(prop.as_ref()));
            validate_metric_field(function, runtime_prop.as_ref())?;
            let alias = obj
                .remove("alias")
                .map(|value| expect_string(value, "metric alias"))
                .transpose()?
                .unwrap_or_else(|| default_metric_alias(function, prop.as_ref()));
            validate_output_alias(&alias)?;
            metrics.push(Self {
                function,
                prop,
                alias,
            });
        }
        Ok(metrics)
    }

    pub fn value_data_type(&self) -> DataType {
        match self.function {
            AggregateFunction::Count | AggregateFunction::CountDistinct => DataType::Int64,
            AggregateFunction::Avg => DataType::Float64,
            AggregateFunction::Sum => self
                .prop
                .as_ref()
                .map(|prop| prop.data_type)
                .unwrap_or(DataType::Float64),
            AggregateFunction::Min | AggregateFunction::Max => self
                .prop
                .as_ref()
                .map(|prop| prop.data_type)
                .unwrap_or(DataType::Float64),
        }
    }
}

impl AggregateHavingAst {
    fn parse_optional(
        metrics: &[AggregateMetric],
        input: Option<Value>,
    ) -> Result<Option<Self>, AppError> {
        let Some(obj) = normalize_object_input(input, "having")? else {
            return Ok(None);
        };
        let mut predicates = Vec::new();
        for (alias, value) in obj {
            if !metrics.iter().any(|metric| metric.alias == alias) {
                return Err(AppError::Validation(format!(
                    "having references unknown metric alias '{}'",
                    alias
                )));
            }
            match value {
                Value::Object(op_obj) => {
                    if op_obj.is_empty() {
                        return Err(AppError::Validation(format!(
                            "having predicate for '{}' must not be empty",
                            alias
                        )));
                    }
                    for (op, value) in op_obj {
                        let op = FilterOp::from_token(&op)?;
                        ensure_having_op(op)?;
                        predicates.push(AggregateHavingPredicate {
                            metric_alias: alias.clone(),
                            op,
                            value,
                        });
                    }
                }
                value => predicates.push(AggregateHavingPredicate {
                    metric_alias: alias,
                    op: FilterOp::Eq,
                    value,
                }),
            }
        }
        Ok(Some(Self { predicates }))
    }
}

impl AggregateSortAst {
    fn parse(
        group_by: &[GroupBySpec],
        metrics: &[AggregateMetric],
        input: Option<Value>,
    ) -> Result<Self, AppError> {
        let Some(obj) = normalize_object_input(input, "aggregate sort")? else {
            return Ok(Self::default());
        };
        let mut specs = Vec::with_capacity(obj.len());
        for (alias, value) in obj {
            if !aggregate_alias_exists(group_by, metrics, &alias) {
                return Err(AppError::Validation(format!(
                    "aggregate sort references unknown output alias '{}'",
                    alias
                )));
            }
            specs.push(AggregateSortSpec {
                alias,
                direction: SortDirection::from_value(&value),
            });
        }
        Ok(Self { specs })
    }
}

impl MutationPlan {
    pub fn create(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selections: Value,
        input: JsonObj,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        Ok(Self {
            kind: MutationKind::Create,
            entity_type: entity_type.clone(),
            selection: Some(SelectionTree::parse_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                selections,
            )?),
            input,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type,
                access.filter.clone(),
            )?,
        })
    }

    pub fn update(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selections: Value,
        input: JsonObj,
        read_version: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        Ok(Self {
            kind: MutationKind::Update { read_version },
            entity_type: entity_type.clone(),
            selection: Some(SelectionTree::parse_with_metadata(
                &model_metadata,
                app_config.clone(),
                entity_type.clone(),
                selections,
            )?),
            input,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type,
                access.filter.clone(),
            )?,
        })
    }

    pub fn delete(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        read_version: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<Self, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        Ok(Self {
            kind: MutationKind::Delete { read_version },
            entity_type: entity_type.clone(),
            selection: None,
            input,
            access_filter: FilterAst::parse_optional_with_metadata(
                &model_metadata,
                app_config,
                entity_type,
                access.filter.clone(),
            )?,
        })
    }

    pub fn selection_json(&self) -> Value {
        self.selection
            .as_ref()
            .map(SelectionTree::to_json)
            .unwrap_or_else(|| json!({ "name": self.entity_type.snake_n, "selection_set": [] }))
    }

    pub fn access_filter_json(&self) -> Option<Value> {
        self.access_filter
            .as_ref()
            .and_then(FilterAst::to_filter_value)
    }

    pub fn into_runtime_provider_plan(self) -> RuntimeProviderMutationPlan<Arc<EntityType>> {
        let selection = self.selection_json();
        let access_filter = self.access_filter_json();
        RuntimeProviderMutationPlan::new(
            self.kind,
            self.entity_type,
            selection,
            self.input,
            access_filter,
        )
    }
}

impl FilterAst {
    pub fn parse_optional(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: Option<Value>,
    ) -> Result<Option<Self>, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        Self::parse_optional_with_metadata(&model_metadata, app_config, entity_type, input)
    }

    fn parse_optional_with_metadata(
        model_metadata: &RuntimeModelMetadata,
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: Option<Value>,
    ) -> Result<Option<Self>, AppError> {
        let normalized = normalize_object_input(input, "filter")?;
        normalized
            .map(|obj| Self::parse_object(model_metadata, app_config, entity_type, obj))
            .transpose()
    }

    fn parse_object(
        model_metadata: &RuntimeModelMetadata,
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: Map<String, Value>,
    ) -> Result<Self, AppError> {
        if input.is_empty() {
            return Ok(FilterAst::All);
        }

        let mut parts = Vec::with_capacity(input.len());
        for (key, value) in input {
            match key.as_str() {
                conjunction::AND | conjunction::OR => {
                    let normalized =
                        runtime_query_ir::normalize_filter_conjunction_items(&key, value)?;
                    let mut filters = Vec::with_capacity(normalized.len());
                    for obj in normalized {
                        filters.push(Self::parse_object(
                            model_metadata,
                            app_config.clone(),
                            entity_type.clone(),
                            obj,
                        )?);
                    }
                    parts.push(if key == conjunction::AND {
                        FilterAst::And(filters)
                    } else {
                        FilterAst::Or(filters)
                    });
                }
                _ => {
                    let prop = AppConfig::get_prop(entity_type.clone(), &key).map_err(|_| {
                        AppError::Validation(format!(
                            "invalid filter field '{}': property not found on {}",
                            key, entity_type.pascal_1
                        ))
                    })?;
                    parts.push(parse_filter_field(
                        model_metadata,
                        app_config.clone(),
                        entity_type.clone(),
                        prop,
                        value,
                    )?);
                }
            }
        }

        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            FilterAst::And(parts)
        })
    }

    pub fn to_filter_value(&self) -> Option<Value> {
        match self {
            FilterAst::All => None,
            FilterAst::And(items) => Some(json!({
                conjunction::AND: items.iter().filter_map(FilterAst::to_filter_value).collect::<Vec<_>>()
            })),
            FilterAst::Or(items) => Some(json!({
                conjunction::OR: items.iter().filter_map(FilterAst::to_filter_value).collect::<Vec<_>>()
            })),
            FilterAst::Field(predicate) => Some(json!({
                predicate.prop.name.clone(): {
                    predicate.op.as_filter_token(): predicate.value.clone()
                }
            })),
            FilterAst::Relation(relation) => relation.filter.to_filter_value().map(|nested| {
                json!({
                    relation.prop.name.clone(): nested
                })
            }),
        }
    }
}

fn parse_filter_field(
    model_metadata: &RuntimeModelMetadata,
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    value: Value,
) -> Result<FilterAst, AppError> {
    let runtime_entity = runtime_entity_descriptor(model_metadata, entity_type.as_ref())?;
    let runtime_prop = model_metadata
        .property(&runtime_entity, &prop.name)?
        .clone();
    match runtime_prop.data_type {
        RuntimeDataType::NavToOne | RuntimeDataType::NavToMany => {
            let obj = runtime_query_ir::expect_relationship_filter_object(
                "navigation",
                &prop.name,
                value,
            )?;
            let (target_entity_type, _) = resolve_relationship_target(
                model_metadata,
                app_config.as_ref(),
                &runtime_entity,
                &runtime_prop,
            )?;
            Ok(FilterAst::Relation(RelationFilter {
                prop,
                runtime_prop,
                target_entity_type: target_entity_type.clone(),
                kind: RelationKind::NavByForeignKey,
                filter: Box::new(FilterAst::parse_object(
                    model_metadata,
                    app_config,
                    target_entity_type,
                    obj,
                )?),
            }))
        }
        RuntimeDataType::ManyToMany => {
            let obj = runtime_query_ir::expect_relationship_filter_object(
                "many-to-many",
                &prop.name,
                value,
            )?;
            let (target_entity_type, _) = resolve_relationship_target(
                model_metadata,
                app_config.as_ref(),
                &runtime_entity,
                &runtime_prop,
            )?;
            Ok(FilterAst::Relation(RelationFilter {
                prop,
                runtime_prop,
                target_entity_type: target_entity_type.clone(),
                kind: RelationKind::ManyToMany,
                filter: Box::new(FilterAst::parse_object(
                    model_metadata,
                    app_config,
                    target_entity_type,
                    obj,
                )?),
            }))
        }
        _ => parse_scalar_filter(prop, runtime_prop, value),
    }
}

fn parse_scalar_filter(
    prop: Arc<PropertyType>,
    runtime_prop: RuntimePropertyMetadata,
    value: Value,
) -> Result<FilterAst, AppError> {
    let runtime_predicates = runtime_query_ir::scalar_filter_predicates(&prop.name, value)?;
    let mut predicates = runtime_predicates
        .into_iter()
        .map(|predicate| {
            FilterAst::Field(FilterPredicate {
                prop: prop.clone(),
                runtime_prop: runtime_prop.clone(),
                op: predicate.op,
                value: predicate.value,
            })
        })
        .collect::<Vec<_>>();

    Ok(if predicates.len() == 1 {
        predicates.remove(0)
    } else {
        FilterAst::And(predicates)
    })
}

impl SortAst {
    pub fn parse(entity_type: Arc<EntityType>, input: Option<Value>) -> Result<Self, AppError> {
        let runtime_specs = runtime_query_ir::parse_sort_specs(input)?;
        let mut specs = Vec::with_capacity(runtime_specs.len());
        for runtime_spec in runtime_specs {
            let prop = AppConfig::get_prop(entity_type.clone(), &runtime_spec.field)?;
            let runtime_prop = runtime_property_metadata(prop.as_ref());
            specs.push(SortSpec {
                prop,
                runtime_prop,
                direction: runtime_spec.direction,
            });
        }
        Ok(Self { specs })
    }

    pub fn to_sort_value(&self) -> Option<Value> {
        if self.specs.is_empty() {
            return None;
        }

        let mut obj = Map::new();
        for spec in &self.specs {
            obj.insert(
                spec.runtime_prop.name.clone(),
                Value::String(spec.direction.as_str().to_string()),
            );
        }
        Some(Value::Object(obj))
    }
}

impl SelectionTree {
    pub fn parse(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: Value,
    ) -> Result<Self, AppError> {
        let model_metadata = runtime_model_metadata(app_config.as_ref());
        Self::parse_with_metadata(&model_metadata, app_config, entity_type, input)
    }

    fn parse_with_metadata(
        model_metadata: &RuntimeModelMetadata,
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: Value,
    ) -> Result<Self, AppError> {
        let runtime_entity = runtime_entity_descriptor(model_metadata, entity_type.as_ref())?;
        let name = input
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&entity_type.snake_n)
            .to_string();
        let selection_set = input
            .get("selection_set")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection_set must be an array".to_string())
            })?;

        let mut fields = Vec::with_capacity(selection_set.len());
        for selection in selection_set {
            fields.push(SelectionNode::parse(
                model_metadata,
                app_config.clone(),
                entity_type.clone(),
                selection,
            )?);
        }

        Ok(Self {
            name,
            runtime_entity,
            fields,
        })
    }

    pub fn to_json(&self) -> Value {
        let name = if self.name.is_empty() {
            self.runtime_entity.snake_n.clone()
        } else {
            self.name.clone()
        };
        json!({
            "name": name,
            "selection_set": self.fields.iter().map(SelectionNode::to_json).collect::<Vec<_>>()
        })
    }
}

impl SelectionNode {
    fn parse(
        model_metadata: &RuntimeModelMetadata,
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        input: &Value,
    ) -> Result<Self, AppError> {
        let selection_name = input
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection name must be a string".to_string())
            })?
            .to_string();
        let prop = AppConfig::get_prop(entity_type.clone(), &selection_name)?;
        let runtime_entity = runtime_entity_descriptor(model_metadata, entity_type.as_ref())?;
        let runtime_prop = model_metadata
            .property(&runtime_entity, &prop.name)?
            .clone();
        let selection_set = input
            .get("selection_set")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection_set must be an array".to_string())
            })?;

        let target_entity_type = match runtime_prop.data_type {
            RuntimeDataType::NavToOne | RuntimeDataType::NavToMany => {
                let (entity_type, _) = resolve_relationship_target(
                    model_metadata,
                    app_config.as_ref(),
                    &runtime_entity,
                    &runtime_prop,
                )?;
                Some(entity_type)
            }
            RuntimeDataType::ManyToMany => {
                let (entity_type, _) = resolve_relationship_target(
                    model_metadata,
                    app_config.as_ref(),
                    &runtime_entity,
                    &runtime_prop,
                )?;
                Some(entity_type)
            }
            _ => None,
        };

        let mut children = Vec::new();
        if let Some(target_entity_type) = &target_entity_type {
            children.reserve(selection_set.len());
            for child in selection_set {
                children.push(Self::parse(
                    model_metadata,
                    app_config.clone(),
                    target_entity_type.clone(),
                    child,
                )?);
            }
        }

        Ok(Self {
            prop,
            runtime_prop,
            target_entity_type,
            children,
        })
    }

    fn to_json(&self) -> Value {
        json!({
            "name": self.prop.name,
            "selection_set": self.children.iter().map(SelectionNode::to_json).collect::<Vec<_>>()
        })
    }
}

fn resolve_relationship_target(
    model_metadata: &RuntimeModelMetadata,
    app_config: &AppConfig,
    runtime_entity: &RuntimeEntityMetadata,
    runtime_prop: &RuntimePropertyMetadata,
) -> Result<(Arc<EntityType>, RuntimeEntityMetadata), AppError> {
    let target = model_metadata
        .relationship_target(runtime_entity, runtime_prop)?
        .clone();
    let target_entity_type =
        app_config.get_entity_type_by_name(&target.schema_name, &target.pascal_1)?;
    Ok((target_entity_type, target))
}

fn runtime_entity_descriptor(
    model_metadata: &RuntimeModelMetadata,
    entity_type: &EntityType,
) -> Result<RuntimeEntityMetadata, AppError> {
    Ok(model_metadata
        .entity(&entity_type.schema_name, &entity_type.pascal_1)?
        .clone())
}

fn normalize_object_input(
    input: Option<Value>,
    label: &str,
) -> Result<Option<Map<String, Value>>, AppError> {
    runtime_query_ir::normalize_object_input(input, label).map_err(AppError::from)
}

fn normalize_array_input(
    input: Option<Value>,
    label: &str,
) -> Result<Option<Vec<Value>>, AppError> {
    runtime_query_ir::normalize_array_input(input, label).map_err(AppError::from)
}

fn take_string(obj: &mut Map<String, Value>, key: &str, label: &str) -> Result<String, AppError> {
    runtime_query_ir::take_string(obj, key, label).map_err(AppError::from)
}

fn take_string_any(
    obj: &mut Map<String, Value>,
    keys: &[&str],
    label: &str,
) -> Result<String, AppError> {
    runtime_query_ir::take_string_any(obj, keys, label).map_err(AppError::from)
}

fn expect_string(value: Value, label: &str) -> Result<String, AppError> {
    runtime_query_ir::expect_string(value, label).map_err(AppError::from)
}

fn validate_output_alias(alias: &str) -> Result<(), AppError> {
    runtime_query_ir::validate_aggregate_output_alias(alias).map_err(AppError::from)
}

fn ensure_unique_aggregate_aliases(
    group_by: &[GroupBySpec],
    metrics: &[AggregateMetric],
) -> Result<(), AppError> {
    runtime_query_ir::validate_unique_aggregate_aliases(
        group_by
            .iter()
            .map(|group| group.alias.as_str())
            .chain(metrics.iter().map(|metric| metric.alias.as_str())),
    )
    .map_err(AppError::from)
}

fn aggregate_alias_exists(
    group_by: &[GroupBySpec],
    metrics: &[AggregateMetric],
    alias: &str,
) -> bool {
    runtime_query_ir::aggregate_alias_exists(
        group_by
            .iter()
            .map(|group| group.alias.as_str())
            .chain(metrics.iter().map(|metric| metric.alias.as_str())),
        alias,
    )
}

fn ensure_groupable(prop: &RuntimePropertyMetadata) -> Result<(), AppError> {
    runtime_query_ir::ensure_aggregate_groupable(&AggregateFieldDescriptor::from(prop))
        .map_err(AppError::from)
}

fn validate_metric_field(
    function: AggregateFunction,
    prop: Option<&RuntimePropertyMetadata>,
) -> Result<(), AppError> {
    let field = prop.map(AggregateFieldDescriptor::from);
    runtime_query_ir::validate_aggregate_metric_field(function, field.as_ref())
        .map_err(AppError::from)
}

fn default_metric_alias(function: AggregateFunction, prop: Option<&Arc<PropertyType>>) -> String {
    runtime_query_ir::default_aggregate_metric_alias(function, prop.map(|prop| prop.name.as_str()))
}

fn ensure_having_op(op: FilterOp) -> Result<(), AppError> {
    runtime_query_ir::ensure_aggregate_having_op(op).map_err(AppError::from)
}

fn ensure_keyset_sort(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    sort: Option<Value>,
) -> Result<Value, AppError> {
    let primary_key_name = if sort.is_none() {
        app_config.get_primary_key_name(entity_type)?
    } else {
        String::new()
    };
    runtime_query_ir::ensure_keyset_sort(&primary_key_name, sort).map_err(AppError::from)
}

fn apply_keyset_cursor_filter(
    filter: Option<Value>,
    sort: &Value,
    after: Option<&str>,
) -> Result<Option<Value>, AppError> {
    runtime_query_ir::apply_keyset_cursor_filter(filter, sort, after).map_err(AppError::from)
}

fn value_kind(v: &Value) -> &'static str {
    runtime_query_ir::value_kind(v)
}
