extern crate tera;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::Serialize;
use serde_json::*;
use serde_yaml;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self};
use std::path::{Path, PathBuf};
use tera::*;

// to_plural => "crate" => "crates".to_string(),
use inflector::string::pluralize::to_plural;
// to_pascal_case => "foo" => "Foo".to_string(),
use inflector::cases::pascalcase::{is_pascal_case, to_pascal_case};
// to_snake_case => "fooBar" => "foo_bar".to_string(),
use inflector::cases::snakecase::to_snake_case;
// to_table_case => "fooBar" => "foo_bars".to_string(),
use inflector::cases::tablecase::{is_table_case, to_table_case};
// to_title_case => "fooBar" => "Foo Bar".to_string(),
use inflector::cases::titlecase::{is_title_case, to_title_case};
// to_camel_case => "foo_bar" => "fooBar".to_string(),
use inflector::cases::camelcase::{is_camel_case, to_camel_case};
// to_screaming_snake_case => "foo_bar" => "FOO_BAR".to_string(),
use inflector::cases::screamingsnakecase::{is_screaming_snake_case, to_screaming_snake_case};
// to_kebab_case => "foo_bar" => "foo-bar".to_string(),
use inflector::cases::kebabcase::{is_kebab_case, to_kebab_case};

use crate::app_workspace::AppWorkspace;

use super::{console, files};

// Stricter check than `inflector::is_snake_case`, which incorrectly returns false for
// anything containing digits. We want digit-bearing identifiers (e.g. `t12m_ebitda`,
// `q1_revenue`) authored in YAML to pass through codegen untouched.
fn looks_like_snake_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.starts_with('_') || s.ends_with('_') {
        return false;
    }
    if s.contains("__") {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn register_hash_string(tera: &mut Tera) -> tera::Result<()> {
    tera.register_filter("hash_string", hash_string);
    Ok(())
}

/// Convert a string to pascal case: User, AuditRecord (whether singular or plural)
pub fn hash_string(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("hash_string", "value", String, value);
    // let s = String::from("ProjectStatus::id");

    let res = just_hash(s);

    to_tera_value(res)
}

pub fn just_hash(input: String) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn register_uuid(tera: &mut Tera) -> tera::Result<()> {
    tera.register_filter("uuid", uuid);
    Ok(())
}

/// Create a new uuid
pub fn uuid(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(seed) = value.as_str().filter(|seed| !seed.is_empty()) {
        return to_tera_value(deterministic_uuid(seed));
    }
    let new_uuid = uuid::Uuid::new_v4().to_string();
    to_tera_value(new_uuid)
}

fn deterministic_uuid(seed: &str) -> String {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

pub fn register_seed_literals(tera: &mut Tera) -> tera::Result<()> {
    tera.register_filter("pg_seed_literal", pg_seed_literal);
    tera.register_filter("mssql_seed_literal", mssql_seed_literal);
    tera.register_filter("snowflake_seed_literal", snowflake_seed_literal);
    Ok(())
}

pub fn register_backend_type_plans(tera: &mut Tera) -> tera::Result<()> {
    tera.register_filter("rust_type", rust_type_filter);
    Ok(())
}

pub fn rust_type_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let prop = value
        .as_object()
        .ok_or_else(|| tera::Error::msg("rust_type expects a property object"))?;
    let entity_type = args
        .get("entityType")
        .and_then(Value::as_object)
        .ok_or_else(|| tera::Error::msg("rust_type requires entityType=<entity object>"))?;
    let object_kind = args
        .get("objectKind")
        .and_then(Value::as_str)
        .ok_or_else(|| tera::Error::msg("rust_type requires objectKind=<base|input|projection>"))?;
    let data_type = prop
        .get("data_type")
        .and_then(Value::as_str)
        .ok_or_else(|| tera::Error::msg("rust_type property is missing data_type"))?;

    to_tera_value(rust_type_name(prop, entity_type, object_kind, data_type)?)
}

fn rust_type_name(
    prop: &Map<String, Value>,
    entity_type: &Map<String, Value>,
    object_kind: &str,
    data_type: &str,
) -> tera::Result<String> {
    let scalar = match data_type {
        "Uuid" => Some("String".to_string()),
        "UuidArray" => Some("Vec<String>".to_string()),
        "ObjectId" => Some("bson::oid::ObjectId".to_string()),
        "ObjectIdArray" => Some("Vec<bson::oid::ObjectId>".to_string()),
        "Boolean" => Some("bool".to_string()),
        "String" => Some("String".to_string()),
        "StringArray" => Some("Vec<String>".to_string()),
        "Date" => Some("chrono::NaiveDate".to_string()),
        "DateTime" => Some("chrono::DateTime<chrono::Utc>".to_string()),
        "Time" => Some("chrono::NaiveTime".to_string()),
        "Int8" => Some("i8".to_string()),
        "Int8Array" => Some("Vec<i8>".to_string()),
        "Int16" => Some("i16".to_string()),
        "Int16Array" => Some("Vec<i16>".to_string()),
        "Int32" => Some("i32".to_string()),
        "Int32Array" => Some("Vec<i32>".to_string()),
        "Int64" => Some("i64".to_string()),
        "Int64Array" => Some("Vec<i64>".to_string()),
        "Float32" => Some("f32".to_string()),
        "Float64" => Some("f64".to_string()),
        "Enum" => Some(required_prop_str(prop, "enum_type_name")?.to_string()),
        "EnumArray" => Some(format!(
            "Vec<{}>",
            required_prop_str(prop, "enum_type_name")?
        )),
        "Json" => Some("serde_json::Value".to_string()),
        "JsonArray" => Some("Vec<serde_json::Value>".to_string()),
        _ => None,
    };
    if let Some(scalar) = scalar {
        return Ok(scalar);
    }

    match data_type {
        "Object" => Ok(type_for_kind(
            object_kind,
            required_nested_type(prop)?,
            false,
        )),
        "ObjectArray" => Ok(format!(
            "Vec<{}>",
            type_for_kind(object_kind, required_nested_type(prop)?, false)
        )),
        "NavToOne" => Ok(format!(
            "Box<{}>",
            type_for_kind(object_kind, nav_target_type(prop, entity_type)?, false)
        )),
        "NavToMany" => Ok(format!(
            "Vec<{}>",
            type_for_kind(object_kind, nav_target_type(prop, entity_type)?, false)
        )),
        "ManyToMany" => Ok(format!(
            "Vec<{}>",
            type_for_kind(object_kind, many_to_many_target_type(prop)?, true)
        )),
        other => Err(tera::Error::msg(format!(
            "rust_type does not support data_type `{other}`"
        ))),
    }
}

fn type_for_kind(object_kind: &str, type_name: &str, many_to_many: bool) -> String {
    match object_kind {
        "base" => type_name.to_string(),
        "input" | "create" | "update" if many_to_many => "i64".to_string(),
        "input" | "create" | "update" => format!("Input{type_name}"),
        "projection" => format!("{type_name}Projection"),
        _ => format!("/* unsupported objectKind {object_kind} */ {type_name}"),
    }
}

fn required_prop_str<'a>(prop: &'a Map<String, Value>, field: &str) -> tera::Result<&'a str> {
    prop.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| tera::Error::msg(format!("rust_type property is missing {field}")))
}

fn required_nested_type(prop: &Map<String, Value>) -> tera::Result<&str> {
    prop.get("nested_entity_type")
        .and_then(Value::as_object)
        .and_then(|nested| nested.get("type_name"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            tera::Error::msg("rust_type object property is missing nested_entity_type.type_name")
        })
}

fn nav_target_type<'a>(
    prop: &'a Map<String, Value>,
    entity_type: &'a Map<String, Value>,
) -> tera::Result<&'a str> {
    let nav = prop
        .get("nav_by_fk_property")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            tera::Error::msg("rust_type navigation property is missing nav_by_fk_property")
        })?;

    nav.get("resolved")
        .and_then(Value::as_object)
        .and_then(|resolved| resolved.get("type_name"))
        .and_then(Value::as_str)
        .or_else(|| nav.get("type_name").and_then(Value::as_str))
        .or_else(|| entity_type.get("pascal_1").and_then(Value::as_str))
        .ok_or_else(|| tera::Error::msg("rust_type navigation property cannot resolve target type"))
}

fn many_to_many_target_type(prop: &Map<String, Value>) -> tera::Result<&str> {
    prop.get("many_to_many_property")
        .and_then(Value::as_object)
        .and_then(|many_to_many| many_to_many.get("target_type"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            tera::Error::msg(
                "rust_type many-to-many property is missing many_to_many_property.target_type",
            )
        })
}

pub fn pg_seed_literal(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    to_tera_value(seed_literal(SqlSeedDialect::Postgres, value))
}

pub fn mssql_seed_literal(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    to_tera_value(seed_literal(SqlSeedDialect::MsSql, value))
}

pub fn snowflake_seed_literal(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    to_tera_value(seed_literal(SqlSeedDialect::Snowflake, value))
}

#[derive(Clone, Copy)]
enum SqlSeedDialect {
    Postgres,
    MsSql,
    Snowflake,
}

fn seed_literal(dialect: SqlSeedDialect, value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(value) => match dialect {
            SqlSeedDialect::MsSql => {
                if *value {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            SqlSeedDialect::Postgres | SqlSeedDialect::Snowflake => value.to_string(),
        },
        Value::Number(value) => value.to_string(),
        Value::String(value) => string_literal(dialect, value),
        Value::Array(values) => array_literal(dialect, values),
        Value::Object(_) => json_literal(dialect, value),
    }
}

fn array_literal(dialect: SqlSeedDialect, values: &[Value]) -> String {
    match dialect {
        SqlSeedDialect::Postgres => {
            if values.is_empty() {
                return "ARRAY[]::varchar[]".to_string();
            }
            let items = values
                .iter()
                .map(|value| match value {
                    Value::Null => "NULL".to_string(),
                    Value::Bool(value) => value.to_string(),
                    Value::Number(value) => value.to_string(),
                    Value::String(value) => string_literal(SqlSeedDialect::Postgres, value),
                    Value::Array(_) | Value::Object(_) => {
                        string_literal(SqlSeedDialect::Postgres, &json_string(value))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("ARRAY[{items}]")
        }
        SqlSeedDialect::MsSql => {
            string_literal(dialect, &json_string(&Value::Array(values.to_vec())))
        }
        SqlSeedDialect::Snowflake => {
            format!(
                "PARSE_JSON({})",
                string_literal(dialect, &json_string(&Value::Array(values.to_vec())))
            )
        }
    }
}

fn json_literal(dialect: SqlSeedDialect, value: &Value) -> String {
    match dialect {
        SqlSeedDialect::Postgres => {
            format!("{}::jsonb", string_literal(dialect, &json_string(value)))
        }
        SqlSeedDialect::MsSql => string_literal(dialect, &json_string(value)),
        SqlSeedDialect::Snowflake => {
            format!(
                "PARSE_JSON({})",
                string_literal(dialect, &json_string(value))
            )
        }
    }
}

fn string_literal(dialect: SqlSeedDialect, value: &str) -> String {
    let escaped = value.replace('\'', "''");
    match dialect {
        SqlSeedDialect::MsSql => format!("N'{escaped}'"),
        SqlSeedDialect::Postgres | SqlSeedDialect::Snowflake => format!("'{escaped}'"),
    }
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

pub fn register_facets(tera: &mut Tera, workspace: &AppWorkspace) -> tera::Result<()> {
    tera.register_filter(
        "audit_type",
        AuditTypeFilter {
            src_file: files::get_audit_entity_type_file(workspace),
        },
    );
    tera.register_filter(
        "audit_property",
        AuditPropertyFilter {
            src_file: files::get_audit_property_file(workspace),
        },
    );
    tera.register_filter(
        "soft_deleted_property",
        FacetFileFilter {
            src_file: files::get_soft_deleted_property_file(workspace),
        },
    );
    tera.register_filter(
        "version_property",
        FacetFileFilter {
            src_file: files::get_version_property_file(workspace),
        },
    );
    Ok(())
}

struct AuditTypeFilter {
    src_file: PathBuf,
}

impl Filter for AuditTypeFilter {
    fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        let related_type_value = try_get_value!("audit_type", "value", Value, value);
        let related_type = related_type_value
            .as_object()
            .ok_or_else(|| tera::Error::msg("audit_type expects an object"))?;

        let default = serde_json::Value::Null;
        let mut res: Value = super::context::read_yaml(&self.src_file, default)
            .map_err(|err| tera::Error::msg(err.to_string()))?;

        let item = res.as_object_mut().ok_or_else(|| {
            tera::Error::msg(format!("{} must be an object", self.src_file.display()))
        })?;
        console::verbose(format!("audit_type template: {:?}", item));
        let related_name = related_type
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| tera::Error::msg("audit_type value is missing string field `name`"))?;
        item.insert(
            "name".to_string(),
            serde_json::Value::String(format!("{related_name}Audit")),
        );
        item.insert(
            "snake_1".to_string(),
            serde_json::Value::String(format!("{}_audit", to_snake_case(related_name))),
        );
        item.insert(
            "snake_n".to_string(),
            serde_json::Value::String(format!("{}_audit", to_table_case(related_name))),
        );
        item.insert(
            "meta".to_string(),
            serde_json::json!({
                "generatedAuditEntity": true,
                "sourceEntity": related_name,
            }),
        );
        if let Some(props) = item.get_mut("props").and_then(Value::as_array_mut) {
            for prop in props {
                let Some(prop_obj) = prop.as_object_mut() else {
                    continue;
                };
                if prop_obj.get("name").and_then(Value::as_str) != Some("record_id") {
                    continue;
                }
                if let Some(foreign_key) = prop_obj
                    .get_mut("foreign_key")
                    .and_then(Value::as_object_mut)
                {
                    foreign_key.insert(
                        "type_name".to_string(),
                        serde_json::Value::String(related_name.to_string()),
                    );
                }
            }
        }
        to_tera_value(item)
    }
}

struct AuditPropertyFilter {
    src_file: PathBuf,
}

impl Filter for AuditPropertyFilter {
    fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        let entity_type_value = try_get_value!("audit_property", "value", Value, value);
        let entity_type = entity_type_value
            .as_object()
            .ok_or_else(|| tera::Error::msg("audit_property expects an object"))?;

        let default = serde_json::Value::Null;
        let mut res: Value = super::context::read_yaml(&self.src_file, default)
            .map_err(|err| tera::Error::msg(err.to_string()))?;

        let item = res.as_object_mut().ok_or_else(|| {
            tera::Error::msg(format!("{} must be an object", self.src_file.display()))
        })?;
        console::verbose(format!("audit_property template: {:?}", item));
        let entity_name = entity_type
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                tera::Error::msg("audit_property value is missing string field `name`")
            })?;
        let nav = item
            .get_mut("nav_by_fk_property")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                tera::Error::msg("audit_property facet must contain nav_by_fk_property object")
            })?;
        nav.insert(
            "type_name".to_string(),
            serde_json::Value::String(format!("{entity_name}Audit")),
        );
        to_tera_value(item)
    }
}

struct FacetFileFilter {
    src_file: PathBuf,
}

impl Filter for FacetFileFilter {
    fn filter(&self, _value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        let default = serde_json::Value::Null;
        let res: Value = super::context::read_yaml(&self.src_file, default)
            .map_err(|err| tera::Error::msg(err.to_string()))?;
        to_tera_value(res)
    }
}

pub fn register_fragment(tera: &mut Tera, path: &PathBuf) -> tera::Result<()> {
    tera.register_filter("fragment", FragmentFilter::init(path)?);
    Ok(())
}

pub fn register_inflectors(tera: &mut Tera) -> tera::Result<()> {
    register_inflector(tera, "pascal")?;
    register_inflector(tera, "pascal_1")?;
    register_inflector(tera, "pascal_n")?;
    register_inflector(tera, "snake_1")?;
    register_inflector(tera, "snake_n")?;
    register_inflector(tera, "caption_1")?;
    register_inflector(tera, "caption_n")?;
    register_inflector(tera, "camel")?;
    register_inflector(tera, "caption")?;
    register_inflector(tera, "screaming_snake")?;
    register_inflector(tera, "kebab")?;
    Ok(())
}

pub fn register_inflector(tera: &mut Tera, name: &str) -> tera::Result<()> {
    match name {
        "pascal" => tera.register_filter("pascal", pascal_filter),
        "pascal_1" => tera.register_filter("pascal_1", pascal_1_filter),
        "pascal_n" => tera.register_filter("pascal_n", pascal_n_filter),
        "snake_1" => tera.register_filter("snake_1", snake_1_filter),
        "snake_n" => tera.register_filter("snake_n", snake_n_filter),
        "caption_1" => tera.register_filter("caption_1", caption_1_filter),
        "caption_n" => tera.register_filter("caption_n", caption_n_filter),
        "camel" => tera.register_filter("camel", camel_filter),
        "caption" => tera.register_filter("caption", caption_filter),
        "screaming_snake" => tera.register_filter("screaming_snake", screaming_snake_filter),
        "kebab" => tera.register_filter("kebab", kebab_filter),
        _ => console::warn(format!("filter not found: {name}")),
    }
    Ok(())
}

/// Convert a string to pascal case: User, AuditRecord (whether singular or plural)
pub fn pascal_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("pascal", "value", String, value);
    let res = if is_pascal_case(&s) {
        s
    } else {
        to_pascal_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to pascal case: User, AuditRecord
pub fn pascal_1_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("pascal_1", "value", String, value);
    let res = if is_pascal_case(&s) {
        s
    } else {
        to_pascal_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to pascal case, plural: Users, AuditRecords
pub fn pascal_n_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("pascal_n", "value", String, value);
    let res = if is_pascal_case(&s) {
        s
    } else {
        to_pascal_case(&s)
    };
    let pl_res = to_plural(res.as_str());
    to_tera_value(pl_res)
}

/// Convert a string to snake case: user, audit_record
/// Note: `inflector::is_snake_case` returns false for strings containing digits, and
/// `to_snake_case("t12m_ebitda")` mangles to `"t_1_2m_ebitda"`. Use a stricter
/// already-snake_case check so author-supplied identifiers with digits pass through.
pub fn snake_1_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("snake_1", "value", String, value);
    let res = if looks_like_snake_case(&s) {
        s
    } else {
        to_snake_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to snake case, plural: users, audit_records
pub fn snake_n_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("snake_n", "value", String, value);
    let res = if looks_like_snake_case(&s) {
        to_plural(&s)
    } else if is_table_case(&s) {
        s
    } else {
        to_table_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to caption case: User, Audit Record
pub fn caption_1_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("caption_1", "value", String, value);
    let res = if is_title_case(&s) {
        s
    } else {
        to_title_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to caption case, plural: Users, Audit Records
pub fn caption_n_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("caption_n", "value", String, value);
    let res = if is_title_case(&s) {
        s
    } else {
        to_title_case(&s)
    };
    let pl_res = to_plural(res.as_str());
    to_tera_value(pl_res)
}

/// Convert a string to camel case: user, auditRecords, userId
pub fn camel_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("camel", "value", String, value);
    let res = if is_camel_case(&s) {
        s
    } else {
        to_camel_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to caption case: User, Audit Record
pub fn caption_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("caption", "value", String, value);
    let res = if is_title_case(&s) {
        s
    } else {
        to_title_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to screaming_snake case: USER, AUDIT_RECORD
pub fn screaming_snake_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("caption", "value", String, value);
    let res = if is_screaming_snake_case(&s) {
        s
    } else {
        to_screaming_snake_case(&s)
    };
    to_tera_value(res)
}

/// Convert a string to kebab case: user, audit-record
pub fn kebab_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("caption", "value", String, value);
    let res = if is_kebab_case(&s) {
        s
    } else {
        to_kebab_case(&s)
    };
    to_tera_value(res)
}

pub struct FragmentFilter {
    fragments: HashMap<String, Value>,
}

impl FragmentFilter {
    pub fn init(fragments_dir: &PathBuf) -> tera::Result<FragmentFilter> {
        let mut fragments = HashMap::<String, Value>::new();
        load_fragments(&mut fragments, fragments_dir)?;
        Ok(FragmentFilter { fragments })
    }
}

impl Filter for FragmentFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let s = try_get_value!("fragment", "value", String, value);
        let Some(res) = self.fragments.get(&s) else {
            console::warn(format!("fragment not found: {s}"));
            return Ok(serde_json::Value::Null);
        };
        to_tera_value(res)
    }

    fn is_safe(&self) -> bool {
        true
    }
}

fn load_fragments(fragments: &mut HashMap<String, Value>, path: &Path) -> tera::Result<()> {
    if path.is_dir() {
        // First we have to ensure the child dirs are processed
        for entry in fs::read_dir(path).map_err(|err| {
            tera::Error::msg(format!(
                "could not read fragments directory {}: {err}",
                path.display()
            ))
        })? {
            let entry_path = entry
                .map_err(|err| {
                    tera::Error::msg(format!(
                        "could not read fragments entry in {}: {err}",
                        path.display()
                    ))
                })?
                .path();
            if entry_path.is_dir() {
                load_fragments(fragments, Path::new(&entry_path))?;
            }
        }
        // Then we handle the child files
        for entry in fs::read_dir(path).map_err(|err| {
            tera::Error::msg(format!(
                "could not read fragments directory {}: {err}",
                path.display()
            ))
        })? {
            let entry_path = entry
                .map_err(|err| {
                    tera::Error::msg(format!(
                        "could not read fragments entry in {}: {err}",
                        path.display()
                    ))
                })?
                .path();
            if entry_path.is_file() {
                load_fragment(fragments, Path::new(&entry_path))?;
            }
        }
    }
    Ok(())
}

fn load_fragment(fragments: &mut HashMap<String, Value>, path: &Path) -> tera::Result<()> {
    let file_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            tera::Error::msg(format!("invalid fragment file name {}", path.display()))
        })?;
    let f = std::fs::File::open(path).map_err(|err| {
        tera::Error::msg(format!(
            "could not open fragment YAML {}: {err}",
            path.display()
        ))
    })?;
    let fragment: Value = serde_yaml::from_reader(f).map_err(|err| {
        tera::Error::msg(format!(
            "could not parse fragment YAML {}: {err}",
            path.display()
        ))
    })?;
    fragments.insert(file_name.to_string(), fragment);
    Ok(())
}

fn to_tera_value<T: Serialize>(value: T) -> tera::Result<Value> {
    serde_json::to_value(value).map_err(|err| tera::Error::msg(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_uuid_is_stable_for_seeded_values() {
        assert_eq!(
            deterministic_uuid("AccountAudit"),
            deterministic_uuid("AccountAudit")
        );
        assert_ne!(
            deterministic_uuid("AccountAudit"),
            deterministic_uuid("ContactAudit")
        );
    }
}
