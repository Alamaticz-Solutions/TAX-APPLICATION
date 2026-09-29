use std::collections::BTreeSet;

use appfw_runtime::graph_provider::{RuntimeGraphNamedQuery, RuntimeGraphQueryLimits};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Default parameter name a tenant-scoped graph operation binds to the caller's
/// tenant. The provider always overrides this parameter with the authenticated
/// `tenant_id`, so a caller can never widen scope by supplying it directly.
pub const DEFAULT_TENANT_PARAMETER: &str = "tenant_id";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Neo4jParameter {
    pub name: String,
    pub value: Value,
}

impl Neo4jParameter {
    pub fn new(name: impl Into<String>, value: Value) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Neo4jGraphQueryLimits {
    pub max_depth: u16,
    pub max_results: u32,
    pub timeout_ms: u64,
}

impl From<Neo4jGraphQueryLimits> for RuntimeGraphQueryLimits {
    fn from(value: Neo4jGraphQueryLimits) -> Self {
        RuntimeGraphQueryLimits {
            max_depth: value.max_depth,
            max_results: value.max_results,
            timeout_ms: value.timeout_ms,
        }
    }
}

/// A vetted, named graph read operation.
///
/// Named queries are the only graph access surface: the runtime dispatcher
/// resolves an operation name to one of these definitions and supplies bound
/// parameter values. The cypher text, declared parameters, traversal limits,
/// and tenant/start-node bindings are all owned here, never by the caller.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Neo4jGraphQuery {
    pub name: String,
    pub cypher: String,
    pub parameters: Vec<Neo4jParameter>,
    pub limits: Neo4jGraphQueryLimits,
    /// Parameter the provider binds to the caller's tenant. `Some` (the common
    /// case) makes the operation tenant-scoped; `None` marks a deliberately
    /// tenant-neutral framework/system operation.
    pub tenant_parameter: Option<String>,
    /// Parameter that identifies the traversal start node. Recorded as a
    /// non-redacted locator in the audit trail when present.
    pub start_node_parameter: Option<String>,
}

impl Neo4jGraphQuery {
    /// Build a tenant-scoped named query (binding `tenant_id`) and reject any
    /// cypher that is not a single read-only statement.
    pub fn new(
        name: impl Into<String>,
        cypher: impl Into<String>,
        parameters: Vec<Neo4jParameter>,
        limits: Neo4jGraphQueryLimits,
    ) -> Result<Self, Neo4jGraphQueryError> {
        let query = Self {
            name: name.into(),
            cypher: cypher.into(),
            parameters,
            limits,
            tenant_parameter: Some(DEFAULT_TENANT_PARAMETER.to_string()),
            start_node_parameter: None,
        };
        validate_read_only_cypher(&query.cypher)?;
        validate_bounded_traversal(&query.cypher)?;
        Ok(query)
    }

    /// Override the tenant parameter name (default [`DEFAULT_TENANT_PARAMETER`]).
    pub fn with_tenant_parameter(mut self, name: impl Into<String>) -> Self {
        self.tenant_parameter = Some(name.into());
        self
    }

    /// Mark this operation as deliberately tenant-neutral. Use only for
    /// framework/system graph reads that have no tenant dimension.
    pub fn without_tenant_scope(mut self) -> Self {
        self.tenant_parameter = None;
        self
    }

    /// Declare which parameter carries the traversal start-node locator.
    pub fn with_start_node_parameter(mut self, name: impl Into<String>) -> Self {
        self.start_node_parameter = Some(name.into());
        self
    }

    pub fn is_tenant_scoped(&self) -> bool {
        self.tenant_parameter.is_some()
    }

    /// Names of every parameter the operation declares, including the tenant
    /// parameter. Used to reject callers that supply undeclared parameters.
    pub fn declared_parameter_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self.parameters.iter().map(|p| p.name.clone()).collect();
        if let Some(tenant) = &self.tenant_parameter {
            names.insert(tenant.clone());
        }
        names
    }

    pub fn runtime_query(&self) -> RuntimeGraphNamedQuery {
        RuntimeGraphNamedQuery {
            name: self.name.clone(),
            query_text: self.cypher.clone(),
            parameters: self
                .parameters
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.value.clone()))
                .collect(),
            limits: self.limits.clone().into(),
        }
    }
}

/// A vetted, named, governed graph WRITE operation.
///
/// The write counterpart of [`Neo4jGraphQuery`]: a single relationship-scoped,
/// parameter-bound, tenant-scoped mutation. It is the only write surface — the
/// write provider resolves an operation name to one of these and binds values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Neo4jGraphMutation {
    pub name: String,
    pub cypher: String,
    pub parameters: Vec<Neo4jParameter>,
    pub limits: Neo4jGraphQueryLimits,
    pub tenant_parameter: Option<String>,
    pub start_node_parameter: Option<String>,
}

impl Neo4jGraphMutation {
    /// Build a tenant-scoped governed mutation, rejecting any cypher that is not
    /// a single, relationship-scoped, bounded write statement.
    pub fn new(
        name: impl Into<String>,
        cypher: impl Into<String>,
        parameters: Vec<Neo4jParameter>,
        limits: Neo4jGraphQueryLimits,
    ) -> Result<Self, Neo4jGraphQueryError> {
        let mutation = Self {
            name: name.into(),
            cypher: cypher.into(),
            parameters,
            limits,
            tenant_parameter: Some(DEFAULT_TENANT_PARAMETER.to_string()),
            start_node_parameter: None,
        };
        validate_governed_write_cypher(&mutation.cypher)?;
        validate_bounded_traversal(&mutation.cypher)?;
        Ok(mutation)
    }

    pub fn with_tenant_parameter(mut self, name: impl Into<String>) -> Self {
        self.tenant_parameter = Some(name.into());
        self
    }

    pub fn without_tenant_scope(mut self) -> Self {
        self.tenant_parameter = None;
        self
    }

    pub fn with_start_node_parameter(mut self, name: impl Into<String>) -> Self {
        self.start_node_parameter = Some(name.into());
        self
    }

    pub fn is_tenant_scoped(&self) -> bool {
        self.tenant_parameter.is_some()
    }

    pub fn declared_parameter_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self.parameters.iter().map(|p| p.name.clone()).collect();
        if let Some(tenant) = &self.tenant_parameter {
            names.insert(tenant.clone());
        }
        names
    }

    pub fn runtime_query(&self) -> RuntimeGraphNamedQuery {
        RuntimeGraphNamedQuery {
            name: self.name.clone(),
            query_text: self.cypher.clone(),
            parameters: self
                .parameters
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.value.clone()))
                .collect(),
            limits: self.limits.clone().into(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Neo4jGraphQueryError {
    #[error("Neo4j graph queries must be a single read-only Cypher statement")]
    MultipleStatements,
    #[error("Neo4j graph queries must start with MATCH, OPTIONAL MATCH, WITH, RETURN, or CALL")]
    UnsupportedStart,
    #[error("Neo4j graph query contains write clause `{0}`")]
    WriteClause(String),
    #[error("Neo4j graph query uses an unbounded variable-length traversal")]
    UnboundedTraversal,
    #[error("Neo4j graph query traversal depth {found} exceeds the maximum of {max}")]
    TraversalDepthExceeded { found: u16, max: u16 },
    #[error("Neo4j governed write must contain a MERGE, CREATE, or DELETE clause")]
    NotAGovernedWrite,
    #[error("Neo4j governed write contains forbidden clause `{0}`")]
    ForbiddenWriteClause(String),
    #[error("Neo4j governed write must target relationships, not nodes: {0}")]
    NonRelationshipMutation(String),
}

pub fn validate_read_only_cypher(cypher: &str) -> Result<(), Neo4jGraphQueryError> {
    let trimmed = cypher.trim();
    if trimmed.trim_end_matches(';').contains(';') {
        return Err(Neo4jGraphQueryError::MultipleStatements);
    }

    let normalized = normalize_cypher(trimmed);
    if !starts_with_read_clause(&normalized) {
        return Err(Neo4jGraphQueryError::UnsupportedStart);
    }

    for clause in [
        "CREATE",
        "DELETE",
        "DETACH DELETE",
        "DROP",
        "LOAD CSV",
        "MERGE",
        "REMOVE",
        "SET",
    ] {
        if contains_clause(&normalized, clause) {
            return Err(Neo4jGraphQueryError::WriteClause(clause.to_string()));
        }
    }

    Ok(())
}

/// Reject queries whose relationship patterns can traverse without an upper
/// bound (`[*]`, `[*..]`, `[*2..]`). An unbounded traversal can walk the entire
/// graph regardless of the configured result cap, so it is never allowed.
pub fn validate_bounded_traversal(cypher: &str) -> Result<(), Neo4jGraphQueryError> {
    for depth in traversal_depths(cypher) {
        depth?;
    }
    Ok(())
}

/// Reject queries whose maximum traversal depth exceeds `max_depth`, and reject
/// unbounded traversals. Depth is read from the relationship var-length
/// quantifiers (`[:REL*1..3]`); a single-hop pattern has depth 1.
pub fn validate_traversal_depth(cypher: &str, max_depth: u16) -> Result<(), Neo4jGraphQueryError> {
    for depth in traversal_depths(cypher) {
        let depth = depth?;
        if depth > max_depth {
            return Err(Neo4jGraphQueryError::TraversalDepthExceeded {
                found: depth,
                max: max_depth,
            });
        }
    }
    Ok(())
}

/// Whether `cypher` references the bound parameter `$name` as a whole token.
pub fn cypher_references_parameter(cypher: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let needle = format!("${name}");
    let bytes = cypher.as_bytes();
    let mut from = 0;
    while let Some(pos) = cypher[from..].find(&needle) {
        let start = from + pos;
        let after = start + needle.len();
        let next_is_ident = bytes
            .get(after)
            .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .unwrap_or(false);
        if !next_is_ident {
            return true;
        }
        from = after;
    }
    false
}

/// Clause keywords used to segment a normalized statement. Multi-word variants
/// (`OPTIONAL MATCH`, `DETACH DELETE`) precede their single-word prefixes so the
/// longest keyword wins at a boundary.
const CLAUSE_KEYWORDS: [&str; 14] = [
    "OPTIONAL MATCH",
    "DETACH DELETE",
    "MATCH",
    "MERGE",
    "CREATE",
    "DELETE",
    "WITH",
    "RETURN",
    "WHERE",
    "SET",
    "REMOVE",
    "UNWIND",
    "CALL",
    "FOREACH",
];

/// Validate a single, governed graph WRITE statement.
///
/// Governed writes are vetted, named, parameter-bound relationship mutations.
/// This is a defense-in-depth guardrail (not a full Cypher parser): it anchors
/// writes on a `MATCH`, allows only relationship-scoped `MERGE`/`CREATE`/`DELETE`,
/// rejects node-detach deletes, schema/label mutations, bulk load, and (for now)
/// property `SET`, and requires a relationship pattern. Tenant binding and
/// bound-parameter enforcement happen in the write provider, mirroring reads.
pub fn validate_governed_write_cypher(cypher: &str) -> Result<(), Neo4jGraphQueryError> {
    let trimmed = cypher.trim();
    if trimmed.trim_end_matches(';').contains(';') {
        return Err(Neo4jGraphQueryError::MultipleStatements);
    }

    let normalized = normalize_cypher(trimmed);
    // Anchor on a MATCH so writes only touch already-located nodes.
    if !(normalized.starts_with("MATCH ") || normalized.starts_with("OPTIONAL MATCH ")) {
        return Err(Neo4jGraphQueryError::UnsupportedStart);
    }

    // Forbidden: node-detach deletes, schema/label mutation, bulk load, and the
    // (deferred) property SET. `DETACH DELETE` is checked before `DELETE`.
    for clause in ["DETACH DELETE", "DROP", "REMOVE", "LOAD CSV", "SET"] {
        if contains_clause(&normalized, clause) {
            return Err(Neo4jGraphQueryError::ForbiddenWriteClause(
                clause.to_string(),
            ));
        }
    }

    // Must contain at least one allowlisted write clause.
    let has_write = ["MERGE", "CREATE", "DELETE"]
        .iter()
        .any(|clause| contains_clause(&normalized, clause));
    if !has_write {
        return Err(Neo4jGraphQueryError::NotAGovernedWrite);
    }

    // The statement must involve a relationship pattern at all.
    if !contains_relationship_pattern(&normalized) {
        return Err(Neo4jGraphQueryError::NonRelationshipMutation(
            "no relationship pattern present".to_string(),
        ));
    }

    // Every CREATE/MERGE clause must write a relationship, never a bare node.
    for (keyword, segment) in clause_segments(&normalized) {
        if (keyword == "CREATE" || keyword == "MERGE") && !segment.contains("-[") {
            return Err(Neo4jGraphQueryError::NonRelationshipMutation(format!(
                "{keyword} clause must target a relationship"
            )));
        }
    }

    // CREATE/MERGE may only connect nodes already bound by a MATCH. This blocks
    // implicit node creation through a relationship pattern with an unbound or
    // labeled/propertied endpoint (e.g. `MERGE (x:Spam)-[r:REFERRED]->(a)`),
    // which would otherwise create the `Spam` node despite the relationship.
    ensure_write_nodes_are_matched(&normalized)?;

    Ok(())
}

/// Reject governed writes whose `MERGE`/`CREATE` clauses reference any node
/// endpoint not bound by a preceding `MATCH` (anonymous nodes and labeled/
/// propertied endpoints in a write clause both imply node creation).
fn ensure_write_nodes_are_matched(normalized: &str) -> Result<(), Neo4jGraphQueryError> {
    let segments = clause_segments(normalized);
    let mut matched: BTreeSet<String> = BTreeSet::new();
    for (keyword, segment) in &segments {
        if *keyword == "MATCH" || *keyword == "OPTIONAL MATCH" {
            for variable in node_variables(segment) {
                if !variable.is_empty() {
                    matched.insert(variable);
                }
            }
        }
    }
    for (keyword, segment) in &segments {
        if *keyword == "MERGE" || *keyword == "CREATE" {
            for variable in node_variables(segment) {
                if variable.is_empty() || !matched.contains(&variable) {
                    return Err(Neo4jGraphQueryError::NonRelationshipMutation(format!(
                        "{keyword} may only connect nodes bound by a preceding MATCH"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Extract the leading identifier of each top-level `( … )` node pattern in a
/// clause segment, skipping nested `{ … }` property maps. Anonymous nodes yield
/// an empty string. The segment is already uppercased/normalized.
fn node_variables(segment: &str) -> Vec<String> {
    let chars: Vec<char> = segment.chars().collect();
    let mut variables = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '(' {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        let mut brace_depth: i32 = 0;
        let mut identifier = String::new();
        let mut reading_identifier = true;
        while cursor < chars.len() {
            let current = chars[cursor];
            if current == '{' {
                brace_depth += 1;
                reading_identifier = false;
            } else if current == '}' {
                brace_depth -= 1;
            } else if brace_depth == 0 {
                if current == ')' {
                    break;
                }
                if reading_identifier {
                    if current.is_ascii_alphanumeric() || current == '_' {
                        identifier.push(current);
                    } else {
                        reading_identifier = false;
                    }
                }
            }
            cursor += 1;
        }
        variables.push(identifier);
        index = cursor + 1;
    }
    variables
}

/// Whether the (normalized) statement contains a relationship detail pattern.
fn contains_relationship_pattern(normalized: &str) -> bool {
    normalized.contains("-[")
}

/// Split a normalized statement into `(clause_keyword, segment_text)` pairs,
/// where `segment_text` is everything up to the next clause keyword.
fn clause_segments(normalized: &str) -> Vec<(&'static str, &str)> {
    let mut markers: Vec<(usize, usize, &'static str)> = Vec::new();
    let mut prev_was_space = true; // start of string counts as a word boundary
    let mut skip_until = 0usize;
    for (idx, ch) in normalized.char_indices() {
        if idx < skip_until {
            prev_was_space = ch == ' ';
            continue;
        }
        if prev_was_space {
            if let Some(keyword) = CLAUSE_KEYWORDS.iter().copied().find(|keyword| {
                normalized[idx..].starts_with(keyword) && {
                    let after = idx + keyword.len();
                    normalized[after..]
                        .chars()
                        .next()
                        .is_none_or(|c| c == ' ' || c == '(')
                }
            }) {
                let end = idx + keyword.len();
                markers.push((idx, end, keyword));
                skip_until = end;
                prev_was_space = false;
                continue;
            }
        }
        prev_was_space = ch == ' ';
    }

    let mut segments = Vec::new();
    for (index, (_, end, keyword)) in markers.iter().enumerate() {
        let segment_end = markers
            .get(index + 1)
            .map(|(start, _, _)| *start)
            .unwrap_or(normalized.len());
        segments.push((*keyword, normalized[*end..segment_end].trim()));
    }
    segments
}

fn normalize_cypher(cypher: &str) -> String {
    cypher
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn starts_with_read_clause(cypher: &str) -> bool {
    ["MATCH ", "OPTIONAL MATCH ", "WITH ", "RETURN ", "CALL "]
        .iter()
        .any(|prefix| cypher.starts_with(prefix) || cypher == prefix.trim_end())
}

fn contains_clause(cypher: &str, clause: &str) -> bool {
    let clause = clause.to_ascii_uppercase();
    cypher == clause
        || cypher.starts_with(&format!("{clause} "))
        || cypher.ends_with(&format!(" {clause}"))
        || cypher.contains(&format!(" {clause} "))
}

/// Yield the resolved traversal depth of every relationship var-length
/// quantifier in `cypher`. A var-length quantifier (`*`) is only considered
/// inside a relationship-detail bracket — a `[` whose preceding non-whitespace
/// character is `-`. This deliberately ignores `count(*)` and list literals.
fn traversal_depths(cypher: &str) -> Vec<Result<u16, Neo4jGraphQueryError>> {
    let chars: Vec<char> = cypher.chars().collect();
    let mut depths = Vec::new();
    let mut prev_non_ws: Option<char> = None;
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        if current == '[' && prev_non_ws == Some('-') {
            // Relationship-detail bracket: scan to the matching `]`.
            let mut end = index + 1;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            let inner: String = chars[index + 1..end].iter().collect();
            if let Some(depth) = relationship_quantifier_depth(&inner) {
                depths.push(depth);
            }
            // Continue after the closing bracket; treat `]` as the last seen
            // non-whitespace character.
            prev_non_ws = Some(']');
            index = end + 1;
            continue;
        }

        if !current.is_whitespace() {
            prev_non_ws = Some(current);
        }
        index += 1;
    }

    depths
}

/// Parse the var-length quantifier inside a relationship-detail bracket.
/// Returns `None` when the bracket has no `*` quantifier (a fixed single hop).
fn relationship_quantifier_depth(inner: &str) -> Option<Result<u16, Neo4jGraphQueryError>> {
    let star = inner.find('*')?;
    let after = inner[star + 1..].trim_start();
    let (min_digits, rest) = split_leading_digits(after);
    let rest = rest.trim_start();

    if let Some(after_dots) = rest.strip_prefix("..") {
        let (max_digits, _) = split_leading_digits(after_dots.trim_start());
        if max_digits.is_empty() {
            // `*..` or `*n..`: no upper bound.
            return Some(Err(Neo4jGraphQueryError::UnboundedTraversal));
        }
        return Some(parse_depth(max_digits));
    }

    if min_digits.is_empty() {
        // `*` with no bounds at all.
        return Some(Err(Neo4jGraphQueryError::UnboundedTraversal));
    }

    // `*n`: an exact-length traversal of depth n.
    Some(parse_depth(min_digits))
}

fn split_leading_digits(s: &str) -> (&str, &str) {
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    s.split_at(end)
}

fn parse_depth(digits: &str) -> Result<u16, Neo4jGraphQueryError> {
    // A depth that does not fit in u16 is far past any sane traversal cap.
    digits
        .parse::<u16>()
        .map_err(|_| Neo4jGraphQueryError::TraversalDepthExceeded {
            found: u16::MAX,
            max: 0,
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn read_only_named_query_maps_to_runtime_query() {
        let query = Neo4jGraphQuery::new(
            "account_relationship_graph",
            "MATCH (a:Account {record_locator: $account}) RETURN a LIMIT 25",
            vec![Neo4jParameter {
                name: "account".to_string(),
                value: json!("rec_123"),
            }],
            Neo4jGraphQueryLimits {
                max_depth: 2,
                max_results: 25,
                timeout_ms: 1_000,
            },
        )
        .expect("read-only query");

        let runtime = query.runtime_query();
        assert_eq!(runtime.name, "account_relationship_graph");
        assert_eq!(runtime.parameters["account"], json!("rec_123"));
        assert_eq!(runtime.limits.max_depth, 2);
        assert!(query.is_tenant_scoped());
        assert_eq!(query.tenant_parameter.as_deref(), Some("tenant_id"));
    }

    #[test]
    fn declared_parameter_names_include_tenant_parameter() {
        let query = Neo4jGraphQuery::new(
            "op",
            "MATCH (a {tenant_id: $tenant_id, id: $account}) RETURN a",
            vec![Neo4jParameter::new("account", json!("rec_1"))],
            limits(),
        )
        .expect("query");

        let names = query.declared_parameter_names();
        assert!(names.contains("account"));
        assert!(names.contains("tenant_id"));
    }

    #[test]
    fn write_clauses_are_rejected() {
        let err = validate_read_only_cypher(
            "MATCH (a:Account {record_locator: $account}) SET a.name = 'bad' RETURN a",
        )
        .expect_err("write should be rejected");

        assert_eq!(err, Neo4jGraphQueryError::WriteClause("SET".to_string()));
    }

    #[test]
    fn multiple_statements_are_rejected() {
        let err = validate_read_only_cypher("MATCH (a) RETURN a; MATCH (b) RETURN b")
            .expect_err("multiple statements should be rejected");

        assert_eq!(err, Neo4jGraphQueryError::MultipleStatements);
    }

    #[test]
    fn unbounded_traversal_is_rejected_at_construction() {
        let err = Neo4jGraphQuery::new(
            "op",
            "MATCH (a {tenant_id: $tenant_id})-[:OWNS*]->(b) RETURN b",
            vec![],
            limits(),
        )
        .expect_err("unbounded traversal should be rejected");
        assert_eq!(err, Neo4jGraphQueryError::UnboundedTraversal);

        assert_eq!(
            validate_bounded_traversal("MATCH (a)-[:OWNS*..]->(b) RETURN b"),
            Err(Neo4jGraphQueryError::UnboundedTraversal)
        );
        assert_eq!(
            validate_bounded_traversal("MATCH (a)-[:OWNS*2..]->(b) RETURN b"),
            Err(Neo4jGraphQueryError::UnboundedTraversal)
        );
    }

    #[test]
    fn bounded_traversal_depth_is_resolved() {
        assert_eq!(
            validate_traversal_depth("MATCH (a)-[:R*1..3]->(b) RETURN b", 3),
            Ok(())
        );
        assert_eq!(
            validate_traversal_depth("MATCH (a)-[:R*1..4]->(b) RETURN b", 3),
            Err(Neo4jGraphQueryError::TraversalDepthExceeded { found: 4, max: 3 })
        );
        // Exact-length quantifier.
        assert_eq!(
            validate_traversal_depth("MATCH (a)-[:R*5]->(b) RETURN b", 3),
            Err(Neo4jGraphQueryError::TraversalDepthExceeded { found: 5, max: 3 })
        );
        // `*..m` upper bound only.
        assert_eq!(
            validate_traversal_depth("MATCH (a)-[:R*..2]->(b) RETURN b", 3),
            Ok(())
        );
    }

    #[test]
    fn count_star_is_not_treated_as_traversal() {
        // `count(*)` and list literals must not be parsed as var-length depth.
        assert_eq!(
            validate_traversal_depth("MATCH (a {tenant_id: $tenant_id}) RETURN count(*)", 3),
            Ok(())
        );
        assert_eq!(
            validate_traversal_depth("MATCH (a) WHERE a.id IN [1, 2, 3] RETURN a", 1),
            Ok(())
        );
    }

    #[test]
    fn parameter_reference_matches_whole_token() {
        assert!(cypher_references_parameter(
            "MATCH (a {tenant_id: $tenant_id}) RETURN a",
            "tenant_id"
        ));
        // A longer parameter that merely starts with the name must not match.
        assert!(!cypher_references_parameter(
            "MATCH (a {tenant: $tenant_other}) RETURN a",
            "tenant"
        ));
        assert!(!cypher_references_parameter(
            "MATCH (a) RETURN a",
            "tenant_id"
        ));
    }

    #[test]
    fn governed_write_accepts_relationship_merge() {
        assert_eq!(
            validate_governed_write_cypher(
                "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $from}), \
                 (b:Account {tenant_id: $tenant_id, record_locator: $to}) \
                 MERGE (a)-[r:REFERRED {tenant_id: $tenant_id}]->(b) RETURN r"
            ),
            Ok(())
        );
    }

    #[test]
    fn governed_write_accepts_relationship_delete() {
        assert_eq!(
            validate_governed_write_cypher(
                "MATCH (a:Account {tenant_id: $tenant_id})-[r:REFERRED]->(b:Account {tenant_id: $tenant_id}) DELETE r"
            ),
            Ok(())
        );
    }

    #[test]
    fn governed_write_rejects_detach_delete_and_schema_clauses() {
        assert_eq!(
            validate_governed_write_cypher(
                "MATCH (a {tenant_id: $tenant_id})-[r]->(b) DETACH DELETE a"
            ),
            Err(Neo4jGraphQueryError::ForbiddenWriteClause(
                "DETACH DELETE".to_string()
            ))
        );
        for (cypher, clause) in [
            (
                "MATCH (a {tenant_id: $tenant_id})-[r:X]->(b) DROP CONSTRAINT foo",
                "DROP",
            ),
            (
                "MATCH (a {tenant_id: $tenant_id})-[r:X]->(b) REMOVE a:Label",
                "REMOVE",
            ),
            (
                "MATCH (a {tenant_id: $tenant_id})-[r:X]->(b) SET r.flag = true",
                "SET",
            ),
        ] {
            assert_eq!(
                validate_governed_write_cypher(cypher),
                Err(Neo4jGraphQueryError::ForbiddenWriteClause(
                    clause.to_string()
                )),
                "expected {clause} to be forbidden"
            );
        }
    }

    #[test]
    fn governed_write_rejects_node_only_mutations() {
        // CREATE of a bare node alongside a relationship match.
        assert!(matches!(
            validate_governed_write_cypher(
                "MATCH (a {tenant_id: $tenant_id})-[:KNOWS]->(b) CREATE (n:Spam) RETURN n"
            ),
            Err(Neo4jGraphQueryError::NonRelationshipMutation(_))
        ));
        // DELETE of a node with no relationship pattern present.
        assert!(matches!(
            validate_governed_write_cypher("MATCH (a {tenant_id: $tenant_id}) DELETE a"),
            Err(Neo4jGraphQueryError::NonRelationshipMutation(_))
        ));
    }

    #[test]
    fn governed_write_rejects_implicit_node_creation_via_merge() {
        // Relationship pattern is present, but the MERGE endpoint `x:Spam` is not
        // MATCH-bound, so Cypher would implicitly create the Spam node.
        assert!(matches!(
            validate_governed_write_cypher(
                "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $account}) \
                 MERGE (x:Spam)-[r:REFERRED {tenant_id: $tenant_id}]->(a) RETURN x"
            ),
            Err(Neo4jGraphQueryError::NonRelationshipMutation(_))
        ));
        // An anonymous endpoint is likewise rejected.
        assert!(matches!(
            validate_governed_write_cypher(
                "MATCH (a:Account {tenant_id: $tenant_id}) MERGE (a)-[r:REFERRED]->() RETURN a"
            ),
            Err(Neo4jGraphQueryError::NonRelationshipMutation(_))
        ));
    }

    #[test]
    fn governed_write_requires_a_write_clause() {
        assert_eq!(
            validate_governed_write_cypher("MATCH (a {tenant_id: $tenant_id})-[r]->(b) RETURN r"),
            Err(Neo4jGraphQueryError::NotAGovernedWrite)
        );
    }

    #[test]
    fn governed_mutation_constructs_and_is_tenant_scoped() {
        let mutation = Neo4jGraphMutation::new(
            "link_account_referral",
            "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $from}), \
             (b:Account {tenant_id: $tenant_id, record_locator: $to}) \
             MERGE (a)-[r:REFERRED {tenant_id: $tenant_id}]->(b) RETURN r",
            vec![
                Neo4jParameter::new("from", json!("rl_a")),
                Neo4jParameter::new("to", json!("rl_b")),
            ],
            limits(),
        )
        .expect("governed mutation");
        assert!(mutation.is_tenant_scoped());
        let names = mutation.declared_parameter_names();
        assert!(names.contains("from") && names.contains("to") && names.contains("tenant_id"));
        assert_eq!(mutation.runtime_query().name, "link_account_referral");
    }

    fn limits() -> Neo4jGraphQueryLimits {
        Neo4jGraphQueryLimits {
            max_depth: 3,
            max_results: 100,
            timeout_ms: 1_000,
        }
    }
}
