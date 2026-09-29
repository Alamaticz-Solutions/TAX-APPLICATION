//! Package-resolvable schema-name → GraphQL HTTP route-segment converter.
//!
//! `appfw_introspect` (this crate's bin) and `appfw-test` both depend on this
//! crate via Cargo and call [`schema_route_segment`]. Do not add a path
//! include or a second kebab algorithm.

use inflector::cases::kebabcase::{is_kebab_case, to_kebab_case};

/// Official schema-name → GraphQL HTTP route-segment converter.
///
/// Generated backends mount schemas at `/{{ schemaName | kebab }}` via the
/// `app_gen` Tera kebab filter (Inflector 0.11 `is_kebab_case` /
/// `to_kebab_case`). This helper applies the same conversion after the
/// historical leading-slash trim so `{base}/{segment}` matches
/// `runtime_graphql_schema_routes`.
pub fn schema_route_segment(schema_name: &str) -> String {
    let trimmed = schema_name.trim_start_matches('/');
    if is_kebab_case(trimmed) {
        trimmed.to_string()
    } else {
        to_kebab_case(trimmed)
    }
}
