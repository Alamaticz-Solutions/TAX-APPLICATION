use serde::Serialize;
use serde_json::{Map, Value};

use crate::{model_metadata::RuntimeDataType, provider_keys::FrameworkProvider, RuntimeError};

pub mod conjunction_token {
    pub const AND: &str = "_and";
    pub const OR: &str = "_or";
}

pub mod filter_token {
    pub const EQUALS: &str = "_eq";
    pub const NOT_EQUALS: &str = "_ne";

    pub const LESS_THAN: &str = "_lt";
    pub const LESS_THAN_OR_EQUAL: &str = "_lte";
    pub const GREATER_THAN: &str = "_gt";
    pub const GREATER_THAN_OR_EQUAL: &str = "_gte";

    pub const REGEX: &str = "_regex";

    pub const STARTS_WITH: &str = "_starts";
    pub const CONTAINS: &str = "_contains";
    pub const NOT_CONTAINS: &str = "_not_contains";
    pub const ENDS_WITH: &str = "_ends";

    pub const OVERLAPS: &str = "_overlaps";
    pub const NOT_OVERLAPS: &str = "_not_overlaps";

    pub const CONTAINED_BY: &str = "_contained_by";
    pub const NOT_CONTAINED_BY: &str = "_not_contained_by";

    pub const IN: &str = "_in";
    pub const NOT_IN: &str = "_not_in";

    pub const BEFORE: &str = "_before";
    pub const DURING: &str = "_during";
    pub const AFTER: &str = "_after";

    pub const HAS: &str = "has";
}

pub type RuntimeFilterObject = Map<String, Value>;

pub fn normalize_filter_input(
    input: Option<Value>,
) -> Result<Option<RuntimeFilterObject>, RuntimeError> {
    match input {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(object)) if object.is_empty() => Ok(None),
        Some(Value::Object(object)) => Ok(Some(object)),
        Some(Value::String(raw)) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Object(object)) if object.is_empty() => Ok(None),
                Ok(Value::Object(object)) => Ok(Some(object)),
                Ok(other) => Err(RuntimeError::Validation(format!(
                    "filter must be a JSON object or a JSON-encoded object string, got {}",
                    value_kind(&other)
                ))),
                Err(err) => Err(RuntimeError::Validation(format!(
                    "filter string is not valid JSON: {}",
                    err
                ))),
            }
        }
        Some(other) => Err(RuntimeError::Validation(format!(
            "filter must be a JSON object or a JSON-encoded object string, got {}",
            value_kind(&other)
        ))),
    }
}

pub fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFilterValueShape {
    Scalar,
    List,
    ScalarOrList,
    Period,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeFilterOperatorCapability {
    pub op: &'static str,
    pub label: &'static str,
    pub value_shape: RuntimeFilterValueShape,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeFilterOperatorSupport {
    pub supported: bool,
    pub reason: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeFilterDataTypeCapability<T> {
    pub data_type: T,
    pub operators: Vec<RuntimeFilterOperatorCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeFilterCapabilities<P, T> {
    pub provider: P,
    pub data_types: Vec<RuntimeFilterDataTypeCapability<T>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeFilterOperatorSpec {
    pub op: &'static str,
    pub label: &'static str,
    pub value_shape: RuntimeFilterValueShape,
}

impl RuntimeFilterOperatorSpec {
    pub const fn new(
        op: &'static str,
        label: &'static str,
        value_shape: RuntimeFilterValueShape,
    ) -> Self {
        Self {
            op,
            label,
            value_shape,
        }
    }
}

pub const RUNTIME_FILTER_DATA_TYPES: [RuntimeDataType; 22] = [
    RuntimeDataType::Boolean,
    RuntimeDataType::String,
    RuntimeDataType::Enum,
    RuntimeDataType::Uuid,
    RuntimeDataType::ObjectId,
    RuntimeDataType::Date,
    RuntimeDataType::DateTime,
    RuntimeDataType::Time,
    RuntimeDataType::Int8,
    RuntimeDataType::Int16,
    RuntimeDataType::Int32,
    RuntimeDataType::Int64,
    RuntimeDataType::Float32,
    RuntimeDataType::Float64,
    RuntimeDataType::StringArray,
    RuntimeDataType::EnumArray,
    RuntimeDataType::UuidArray,
    RuntimeDataType::ObjectIdArray,
    RuntimeDataType::Int8Array,
    RuntimeDataType::Int16Array,
    RuntimeDataType::Int32Array,
    RuntimeDataType::Int64Array,
];

pub fn runtime_filter_data_types() -> &'static [RuntimeDataType] {
    &RUNTIME_FILTER_DATA_TYPES
}

pub fn runtime_filter_capabilities_for_provider(
    provider: FrameworkProvider,
) -> RuntimeFilterCapabilities<FrameworkProvider, RuntimeDataType> {
    RuntimeFilterCapabilities {
        provider,
        data_types: runtime_filter_data_types()
            .iter()
            .copied()
            .map(|data_type| RuntimeFilterDataTypeCapability {
                data_type,
                operators: runtime_filter_capabilities_for_data_type(provider, data_type),
            })
            .collect(),
    }
}

pub fn runtime_filter_specs_for_data_type(
    data_type: RuntimeDataType,
) -> Vec<RuntimeFilterOperatorSpec> {
    match data_type {
        RuntimeDataType::Boolean => eq_ne_ops(),
        RuntimeDataType::String | RuntimeDataType::Enum => text_ops(),
        RuntimeDataType::Uuid | RuntimeDataType::ObjectId => id_ops(),
        RuntimeDataType::Date | RuntimeDataType::DateTime => temporal_ops(true),
        RuntimeDataType::Time => temporal_ops(false),
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
        | RuntimeDataType::Float32
        | RuntimeDataType::Float64 => numeric_ops(),
        RuntimeDataType::StringArray
        | RuntimeDataType::EnumArray
        | RuntimeDataType::UuidArray
        | RuntimeDataType::ObjectIdArray
        | RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array => array_ops(),
        _ => Vec::new(),
    }
}

pub fn runtime_filter_capabilities_for_data_type(
    provider: FrameworkProvider,
    data_type: RuntimeDataType,
) -> Vec<RuntimeFilterOperatorCapability> {
    runtime_filter_specs_for_data_type(data_type)
        .into_iter()
        .map(|spec| runtime_filter_operator_capability(provider, data_type, spec))
        .collect()
}

pub fn runtime_filter_operator_capability(
    provider: FrameworkProvider,
    data_type: RuntimeDataType,
    spec: RuntimeFilterOperatorSpec,
) -> RuntimeFilterOperatorCapability {
    let support = runtime_filter_operator_support(provider, data_type, spec.op);
    RuntimeFilterOperatorCapability {
        op: spec.op,
        label: spec.label,
        value_shape: spec.value_shape,
        supported: support.supported,
        unsupported_reason: support.reason,
    }
}

pub fn runtime_filter_operator_support(
    provider: FrameworkProvider,
    data_type: RuntimeDataType,
    op: &str,
) -> RuntimeFilterOperatorSupport {
    if provider.is_external_api_provider() {
        return unsupported(
            "external API filters are declared by SaaS named-operation contracts, not generic CRUD filter metadata",
        );
    }

    if runtime_filter_provider_supports(provider, data_type, op) {
        return supported();
    }

    unsupported(match (provider, data_type, op) {
        (
            FrameworkProvider::Postgres,
            RuntimeDataType::ObjectId | RuntimeDataType::ObjectIdArray,
            _,
        ) => "PostgreSQL filter compiler does not support ObjectId data types",
        (_, RuntimeDataType::String | RuntimeDataType::Enum, filter_token::REGEX) => {
            "provider filter compiler does not implement regex matching"
        }
        (_, RuntimeDataType::String | RuntimeDataType::Enum, filter_token::NOT_CONTAINS) => {
            "provider filter compiler does not implement scalar text negated containment"
        }
        (
            _,
            RuntimeDataType::Float32 | RuntimeDataType::Float64,
            filter_token::IN | filter_token::NOT_IN,
        ) => "provider filter compiler does not support scalar float membership",
        (FrameworkProvider::Mongo, data_type, filter_token::IN | filter_token::NOT_IN)
            if is_numeric_type(data_type) =>
        {
            "MongoDB filter compiler does not support scalar numeric membership"
        }
        (
            FrameworkProvider::Postgres,
            data_type,
            filter_token::EQUALS | filter_token::NOT_EQUALS,
        ) if is_array_type(data_type) => {
            "PostgreSQL filter compiler does not support array equality"
        }
        _ => "provider filter compiler does not support this filter operator for this data type",
    })
}

fn runtime_filter_provider_supports(
    provider: FrameworkProvider,
    data_type: RuntimeDataType,
    op: &str,
) -> bool {
    match data_type {
        RuntimeDataType::Boolean => is_eq_ne(op),
        RuntimeDataType::String | RuntimeDataType::Enum => text_supports(provider, op),
        RuntimeDataType::Uuid => id_supports(op),
        RuntimeDataType::ObjectId => provider != FrameworkProvider::Postgres && id_supports(op),
        RuntimeDataType::Date | RuntimeDataType::DateTime => temporal_supports(true, op),
        RuntimeDataType::Time => temporal_supports(false, op),
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64 => numeric_supports(provider, true, op),
        RuntimeDataType::Float32 | RuntimeDataType::Float64 => {
            numeric_supports(provider, false, op)
        }
        RuntimeDataType::StringArray
        | RuntimeDataType::EnumArray
        | RuntimeDataType::UuidArray
        | RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array => array_supports(provider, op),
        RuntimeDataType::ObjectIdArray => {
            provider != FrameworkProvider::Postgres && array_supports(provider, op)
        }
        _ => false,
    }
}

fn text_supports(provider: FrameworkProvider, op: &str) -> bool {
    match op {
        filter_token::EQUALS
        | filter_token::NOT_EQUALS
        | filter_token::STARTS_WITH
        | filter_token::CONTAINS
        | filter_token::ENDS_WITH
        | filter_token::IN
        | filter_token::NOT_IN => true,
        filter_token::REGEX => {
            matches!(
                provider,
                FrameworkProvider::Postgres | FrameworkProvider::Mongo
            )
        }
        filter_token::NOT_CONTAINS => matches!(
            provider,
            FrameworkProvider::Mssql
                | FrameworkProvider::FabricSqlAnalytics
                | FrameworkProvider::Snowflake
        ),
        _ => false,
    }
}

fn id_supports(op: &str) -> bool {
    matches!(
        op,
        filter_token::EQUALS | filter_token::NOT_EQUALS | filter_token::IN | filter_token::NOT_IN
    )
}

fn temporal_supports(include_period: bool, op: &str) -> bool {
    matches!(
        op,
        filter_token::EQUALS
            | filter_token::NOT_EQUALS
            | filter_token::LESS_THAN
            | filter_token::LESS_THAN_OR_EQUAL
            | filter_token::GREATER_THAN
            | filter_token::GREATER_THAN_OR_EQUAL
    ) || (include_period
        && matches!(
            op,
            filter_token::BEFORE | filter_token::DURING | filter_token::AFTER
        ))
}

fn numeric_supports(provider: FrameworkProvider, is_integer: bool, op: &str) -> bool {
    if matches!(
        op,
        filter_token::EQUALS
            | filter_token::NOT_EQUALS
            | filter_token::LESS_THAN
            | filter_token::LESS_THAN_OR_EQUAL
            | filter_token::GREATER_THAN
            | filter_token::GREATER_THAN_OR_EQUAL
    ) {
        return true;
    }

    if !matches!(op, filter_token::IN | filter_token::NOT_IN) {
        return false;
    }

    match provider {
        FrameworkProvider::Postgres => is_integer,
        FrameworkProvider::Mongo | FrameworkProvider::Neo4j => false,
        FrameworkProvider::Mssql
        | FrameworkProvider::FabricSqlAnalytics
        | FrameworkProvider::Snowflake => true,
        FrameworkProvider::ServiceNow
        | FrameworkProvider::Workday
        | FrameworkProvider::Icims
        | FrameworkProvider::Salesforce
        | FrameworkProvider::Anaplan
        | FrameworkProvider::OracleFinancials
        | FrameworkProvider::AiSearch => false,
    }
}

fn array_supports(provider: FrameworkProvider, op: &str) -> bool {
    match op {
        filter_token::EQUALS | filter_token::NOT_EQUALS => provider != FrameworkProvider::Postgres,
        filter_token::CONTAINS
        | filter_token::NOT_CONTAINS
        | filter_token::OVERLAPS
        | filter_token::NOT_OVERLAPS
        | filter_token::CONTAINED_BY
        | filter_token::NOT_CONTAINED_BY => true,
        _ => false,
    }
}

fn is_eq_ne(op: &str) -> bool {
    matches!(op, filter_token::EQUALS | filter_token::NOT_EQUALS)
}

fn is_numeric_type(data_type: RuntimeDataType) -> bool {
    matches!(
        data_type,
        RuntimeDataType::Int8
            | RuntimeDataType::Int16
            | RuntimeDataType::Int32
            | RuntimeDataType::Int64
            | RuntimeDataType::Float32
            | RuntimeDataType::Float64
    )
}

fn is_array_type(data_type: RuntimeDataType) -> bool {
    matches!(
        data_type,
        RuntimeDataType::StringArray
            | RuntimeDataType::EnumArray
            | RuntimeDataType::UuidArray
            | RuntimeDataType::ObjectIdArray
            | RuntimeDataType::Int8Array
            | RuntimeDataType::Int16Array
            | RuntimeDataType::Int32Array
            | RuntimeDataType::Int64Array
    )
}

fn supported() -> RuntimeFilterOperatorSupport {
    RuntimeFilterOperatorSupport {
        supported: true,
        reason: None,
    }
}

fn unsupported(reason: &'static str) -> RuntimeFilterOperatorSupport {
    RuntimeFilterOperatorSupport {
        supported: false,
        reason: Some(reason),
    }
}

fn spec(
    op: &'static str,
    label: &'static str,
    value_shape: RuntimeFilterValueShape,
) -> RuntimeFilterOperatorSpec {
    RuntimeFilterOperatorSpec::new(op, label, value_shape)
}

fn eq_ne_ops() -> Vec<RuntimeFilterOperatorSpec> {
    vec![
        spec(
            filter_token::EQUALS,
            "equals",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::NOT_EQUALS,
            "does not equal",
            RuntimeFilterValueShape::Scalar,
        ),
    ]
}

fn text_ops() -> Vec<RuntimeFilterOperatorSpec> {
    vec![
        spec(
            filter_token::CONTAINS,
            "contains",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::NOT_CONTAINS,
            "does not contain",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::STARTS_WITH,
            "starts with",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::ENDS_WITH,
            "ends with",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::REGEX,
            "matches regex",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::EQUALS,
            "equals",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::NOT_EQUALS,
            "does not equal",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(filter_token::IN, "is one of", RuntimeFilterValueShape::List),
        spec(
            filter_token::NOT_IN,
            "is not one of",
            RuntimeFilterValueShape::List,
        ),
    ]
}

fn id_ops() -> Vec<RuntimeFilterOperatorSpec> {
    vec![
        spec(
            filter_token::EQUALS,
            "equals",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::NOT_EQUALS,
            "does not equal",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(filter_token::IN, "is one of", RuntimeFilterValueShape::List),
        spec(
            filter_token::NOT_IN,
            "is not one of",
            RuntimeFilterValueShape::List,
        ),
    ]
}

fn temporal_ops(include_period: bool) -> Vec<RuntimeFilterOperatorSpec> {
    let mut operators = scalar_comparison_ops(false);
    if include_period {
        operators.extend([
            spec(
                filter_token::BEFORE,
                "before period",
                RuntimeFilterValueShape::Period,
            ),
            spec(
                filter_token::DURING,
                "during period",
                RuntimeFilterValueShape::Period,
            ),
            spec(
                filter_token::AFTER,
                "after period",
                RuntimeFilterValueShape::Period,
            ),
        ]);
    }
    operators
}

fn numeric_ops() -> Vec<RuntimeFilterOperatorSpec> {
    scalar_comparison_ops(true)
}

fn scalar_comparison_ops(include_list: bool) -> Vec<RuntimeFilterOperatorSpec> {
    let mut operators = vec![
        spec(
            filter_token::EQUALS,
            "equals",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::NOT_EQUALS,
            "does not equal",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::GREATER_THAN,
            "greater than",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::GREATER_THAN_OR_EQUAL,
            "greater than or equal",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::LESS_THAN,
            "less than",
            RuntimeFilterValueShape::Scalar,
        ),
        spec(
            filter_token::LESS_THAN_OR_EQUAL,
            "less than or equal",
            RuntimeFilterValueShape::Scalar,
        ),
    ];
    if include_list {
        operators.extend([
            spec(filter_token::IN, "is one of", RuntimeFilterValueShape::List),
            spec(
                filter_token::NOT_IN,
                "is not one of",
                RuntimeFilterValueShape::List,
            ),
        ]);
    }
    operators
}

fn array_ops() -> Vec<RuntimeFilterOperatorSpec> {
    vec![
        spec(
            filter_token::EQUALS,
            "equals",
            RuntimeFilterValueShape::List,
        ),
        spec(
            filter_token::NOT_EQUALS,
            "does not equal",
            RuntimeFilterValueShape::List,
        ),
        spec(
            filter_token::CONTAINS,
            "contains all",
            RuntimeFilterValueShape::ScalarOrList,
        ),
        spec(
            filter_token::NOT_CONTAINS,
            "does not contain all",
            RuntimeFilterValueShape::ScalarOrList,
        ),
        spec(
            filter_token::OVERLAPS,
            "overlaps",
            RuntimeFilterValueShape::List,
        ),
        spec(
            filter_token::NOT_OVERLAPS,
            "does not overlap",
            RuntimeFilterValueShape::List,
        ),
        spec(
            filter_token::CONTAINED_BY,
            "is contained by",
            RuntimeFilterValueShape::List,
        ),
        spec(
            filter_token::NOT_CONTAINED_BY,
            "is not contained by",
            RuntimeFilterValueShape::List,
        ),
    ]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeFilterOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Regex,
    StartsWith,
    Contains,
    NotContains,
    EndsWith,
    Overlaps,
    NotOverlaps,
    ContainedBy,
    NotContainedBy,
    In,
    NotIn,
    Before,
    During,
    After,
    Has,
}

impl RuntimeFilterOp {
    pub fn from_token(token: &str) -> Result<Self, RuntimeError> {
        match token {
            filter_token::EQUALS | "eq" => Ok(Self::Eq),
            filter_token::NOT_EQUALS | "ne" => Ok(Self::Ne),
            filter_token::LESS_THAN => Ok(Self::Lt),
            filter_token::LESS_THAN_OR_EQUAL => Ok(Self::Lte),
            filter_token::GREATER_THAN => Ok(Self::Gt),
            filter_token::GREATER_THAN_OR_EQUAL => Ok(Self::Gte),
            filter_token::REGEX => Ok(Self::Regex),
            filter_token::STARTS_WITH => Ok(Self::StartsWith),
            filter_token::CONTAINS => Ok(Self::Contains),
            filter_token::NOT_CONTAINS => Ok(Self::NotContains),
            filter_token::ENDS_WITH => Ok(Self::EndsWith),
            filter_token::OVERLAPS => Ok(Self::Overlaps),
            filter_token::NOT_OVERLAPS => Ok(Self::NotOverlaps),
            filter_token::CONTAINED_BY => Ok(Self::ContainedBy),
            filter_token::NOT_CONTAINED_BY => Ok(Self::NotContainedBy),
            filter_token::IN => Ok(Self::In),
            filter_token::NOT_IN => Ok(Self::NotIn),
            filter_token::BEFORE => Ok(Self::Before),
            filter_token::DURING => Ok(Self::During),
            filter_token::AFTER => Ok(Self::After),
            filter_token::HAS => Ok(Self::Has),
            other => Err(RuntimeError::Validation(format!(
                "unknown filter operator '{}'",
                other
            ))),
        }
    }

    pub fn as_filter_token(self) -> &'static str {
        match self {
            Self::Eq => filter_token::EQUALS,
            Self::Ne => filter_token::NOT_EQUALS,
            Self::Lt => filter_token::LESS_THAN,
            Self::Lte => filter_token::LESS_THAN_OR_EQUAL,
            Self::Gt => filter_token::GREATER_THAN,
            Self::Gte => filter_token::GREATER_THAN_OR_EQUAL,
            Self::Regex => filter_token::REGEX,
            Self::StartsWith => filter_token::STARTS_WITH,
            Self::Contains => filter_token::CONTAINS,
            Self::NotContains => filter_token::NOT_CONTAINS,
            Self::EndsWith => filter_token::ENDS_WITH,
            Self::Overlaps => filter_token::OVERLAPS,
            Self::NotOverlaps => filter_token::NOT_OVERLAPS,
            Self::ContainedBy => filter_token::CONTAINED_BY,
            Self::NotContainedBy => filter_token::NOT_CONTAINED_BY,
            Self::In => filter_token::IN,
            Self::NotIn => filter_token::NOT_IN,
            Self::Before => filter_token::BEFORE,
            Self::During => filter_token::DURING,
            Self::After => filter_token::AFTER,
            Self::Has => filter_token::HAS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn filter_ops_parse_and_round_trip_tokens() {
        let op = RuntimeFilterOp::from_token(filter_token::CONTAINS).expect("contains token");
        assert_eq!(op, RuntimeFilterOp::Contains);
        assert_eq!(op.as_filter_token(), filter_token::CONTAINS);
        assert_eq!(
            RuntimeFilterOp::from_token("eq").unwrap(),
            RuntimeFilterOp::Eq
        );
        assert!(RuntimeFilterOp::from_token("_bogus").is_err());
        assert_eq!(conjunction_token::AND, "_and");
        assert_eq!(conjunction_token::OR, "_or");
    }

    #[test]
    fn runtime_filter_specs_define_canonical_text_vocabulary() {
        let operators = operator_tokens(RuntimeDataType::String);

        assert!(operators.contains(&filter_token::CONTAINS));
        assert!(operators.contains(&filter_token::NOT_CONTAINS));
        assert!(operators.contains(&filter_token::REGEX));
        assert!(operators.contains(&filter_token::IN));
        assert!(operators.contains(&filter_token::NOT_IN));
    }

    #[test]
    fn runtime_filter_specs_define_temporal_period_scope() {
        assert!(operator_tokens(RuntimeDataType::Date).contains(&filter_token::DURING));
        assert!(operator_tokens(RuntimeDataType::DateTime).contains(&filter_token::DURING));
        assert!(!operator_tokens(RuntimeDataType::Time).contains(&filter_token::DURING));
    }

    #[test]
    fn runtime_filter_specs_define_array_vocabulary() {
        let operators = operator_tokens(RuntimeDataType::StringArray);

        assert!(operators.contains(&filter_token::EQUALS));
        assert!(operators.contains(&filter_token::CONTAINS));
        assert!(operators.contains(&filter_token::OVERLAPS));
        assert!(operators.contains(&filter_token::CONTAINED_BY));
    }

    #[test]
    fn runtime_filter_specs_do_not_emit_duplicates() {
        for data_type in runtime_filter_data_types() {
            let mut seen = HashSet::new();
            for operator in runtime_filter_specs_for_data_type(*data_type) {
                assert!(
                    seen.insert(operator.op),
                    "{data_type:?} emitted duplicate operator {}",
                    operator.op
                );
            }
        }
    }

    #[test]
    fn runtime_filter_data_types_are_canonical_and_unique() {
        let mut seen = HashSet::new();
        for data_type in runtime_filter_data_types() {
            assert!(seen.insert(*data_type), "duplicate {data_type:?}");
        }

        assert!(seen.contains(&RuntimeDataType::String));
        assert!(seen.contains(&RuntimeDataType::ObjectIdArray));
        assert!(!seen.contains(&RuntimeDataType::Json));
        assert!(!seen.contains(&RuntimeDataType::NavToOne));
    }

    #[test]
    fn runtime_filter_capabilities_cover_canonical_data_types() {
        let capabilities = runtime_filter_capabilities_for_provider(FrameworkProvider::Postgres);

        assert_eq!(capabilities.provider, FrameworkProvider::Postgres);
        assert_eq!(
            capabilities.data_types.len(),
            runtime_filter_data_types().len()
        );
        assert_eq!(
            capabilities
                .data_types
                .iter()
                .map(|capability| capability.data_type)
                .collect::<Vec<_>>(),
            runtime_filter_data_types().to_vec()
        );
    }

    #[test]
    fn runtime_provider_support_is_metadata_not_vocabulary() {
        let postgres = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Postgres,
            RuntimeDataType::String,
        );
        let mssql = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Mssql,
            RuntimeDataType::String,
        );
        let fabric = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::FabricSqlAnalytics,
            RuntimeDataType::String,
        );

        assert!(all_ops_for(&postgres).contains(&filter_token::NOT_CONTAINS));
        assert!(all_ops_for(&mssql).contains(&filter_token::REGEX));
        assert!(!is_supported(&postgres, filter_token::NOT_CONTAINS));
        assert!(is_supported(&postgres, filter_token::REGEX));
        assert!(is_supported(&mssql, filter_token::NOT_CONTAINS));
        assert!(!is_supported(&mssql, filter_token::REGEX));
        assert!(is_supported(&fabric, filter_token::NOT_CONTAINS));
        assert!(!is_supported(&fabric, filter_token::REGEX));
    }

    #[test]
    fn external_api_filter_capabilities_fail_closed_until_named_operations_define_predicates() {
        for provider in [
            FrameworkProvider::Anaplan,
            FrameworkProvider::OracleFinancials,
        ] {
            let capabilities = runtime_filter_capabilities_for_provider(provider);

            assert_eq!(capabilities.provider, provider);
            assert!(
                capabilities
                    .data_types
                    .iter()
                    .flat_map(|capability| capability.operators.iter())
                    .all(|operator| {
                        !operator.supported
                            && operator.unsupported_reason.is_some_and(|reason| {
                                reason.contains("SaaS named-operation contracts")
                            })
                    }),
                "expected all generic filter operators to be unsupported for {provider:?}"
            );
        }
    }

    #[test]
    fn runtime_numeric_membership_support_reflects_provider_and_data_type() {
        let postgres_int = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Postgres,
            RuntimeDataType::Int64,
        );
        let postgres_float = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Postgres,
            RuntimeDataType::Float64,
        );
        let mongo_int = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Mongo,
            RuntimeDataType::Int64,
        );
        let snowflake_float = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Snowflake,
            RuntimeDataType::Float64,
        );
        let fabric_float = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::FabricSqlAnalytics,
            RuntimeDataType::Float64,
        );

        assert!(is_supported(&postgres_int, filter_token::IN));
        assert!(!is_supported(&postgres_float, filter_token::IN));
        assert!(!is_supported(&mongo_int, filter_token::IN));
        assert!(is_supported(&snowflake_float, filter_token::IN));
        assert!(is_supported(&fabric_float, filter_token::IN));
    }

    #[test]
    fn runtime_array_equality_support_reflects_provider() {
        let postgres = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Postgres,
            RuntimeDataType::StringArray,
        );
        let mongo = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Mongo,
            RuntimeDataType::StringArray,
        );

        assert!(all_ops_for(&postgres).contains(&filter_token::EQUALS));
        assert!(!is_supported(&postgres, filter_token::EQUALS));
        assert!(is_supported(&mongo, filter_token::EQUALS));
        assert!(is_supported(&postgres, filter_token::OVERLAPS));
    }

    #[test]
    fn runtime_unsupported_capabilities_explain_why() {
        let postgres_object_id = runtime_filter_capabilities_for_data_type(
            FrameworkProvider::Postgres,
            RuntimeDataType::ObjectId,
        );
        let capability = capability_for(&postgres_object_id, filter_token::EQUALS);

        assert!(!capability.supported);
        assert!(capability.unsupported_reason.is_some());
    }

    #[test]
    fn normalize_filter_input_accepts_empty_object_and_json_string() {
        assert_eq!(normalize_filter_input(None).unwrap(), None);
        assert_eq!(normalize_filter_input(Some(Value::Null)).unwrap(), None);
        assert_eq!(
            normalize_filter_input(Some(serde_json::json!({}))).unwrap(),
            None
        );
        assert_eq!(
            normalize_filter_input(Some(serde_json::json!(""))).unwrap(),
            None
        );

        let object = normalize_filter_input(Some(serde_json::json!({ "name": { "_eq": "Acme" } })))
            .unwrap()
            .expect("object filter");
        assert_eq!(object["name"], serde_json::json!({ "_eq": "Acme" }));

        let parsed =
            normalize_filter_input(Some(serde_json::json!(r#"{ "name": { "_eq": "Acme" } }"#)))
                .unwrap()
                .expect("string filter");
        assert_eq!(parsed["name"], serde_json::json!({ "_eq": "Acme" }));
    }

    #[test]
    fn normalize_filter_input_rejects_invalid_shapes() {
        let err = normalize_filter_input(Some(serde_json::json!(["name"]))).unwrap_err();
        assert!(matches!(err, RuntimeError::Validation(_)));

        let err = normalize_filter_input(Some(serde_json::json!("{bad json"))).unwrap_err();
        assert!(matches!(err, RuntimeError::Validation(_)));
    }

    fn operator_tokens(data_type: RuntimeDataType) -> Vec<&'static str> {
        runtime_filter_specs_for_data_type(data_type)
            .into_iter()
            .map(|operator| operator.op)
            .collect()
    }

    fn all_ops_for(capabilities: &[RuntimeFilterOperatorCapability]) -> Vec<&'static str> {
        capabilities.iter().map(|operator| operator.op).collect()
    }

    fn is_supported(capabilities: &[RuntimeFilterOperatorCapability], op: &str) -> bool {
        capability_for(capabilities, op).supported
    }

    fn capability_for<'a>(
        capabilities: &'a [RuntimeFilterOperatorCapability],
        op: &str,
    ) -> &'a RuntimeFilterOperatorCapability {
        capabilities
            .iter()
            .find(|operator| operator.op == op)
            .expect("operator capability should exist")
    }
}
