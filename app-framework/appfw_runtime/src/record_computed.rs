use std::collections::HashMap;

use chrono::{DateTime, Utc};
use inflector::cases::camelcase::{is_camel_case, to_camel_case};
use inflector::cases::pascalcase::{is_pascal_case, to_pascal_case};
use inflector::cases::snakecase::{is_snake_case, to_snake_case};
use inflector::cases::tablecase::{is_table_case, to_table_case};
use inflector::cases::titlecase::{is_title_case, to_title_case};
use inflector::string::pluralize::to_plural;
use serde_json::{Map, Value};
use strfmt::strfmt;
use tracing::{debug, warn};

use crate::{MetadataError, RuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeComputedKind {
    Concatenate,
    Format,
    Word,
    Inflection,
    DateTimeNow,
    None,
}

pub fn try_compute_property(
    computed: RuntimeComputedKind,
    property_name: &str,
    meta: Option<&Value>,
    record: &Map<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    match computed {
        RuntimeComputedKind::Concatenate => {
            let function = ConcatenateFunction::init(property_name, meta)?;
            function.eval(record)
        }
        RuntimeComputedKind::Format => {
            let function = FormatFunction::init(property_name, meta)?;
            function.eval(record)
        }
        RuntimeComputedKind::Word => {
            let function = WordFunction::init(property_name, meta)?;
            function.eval(record)
        }
        RuntimeComputedKind::Inflection => {
            let function = InflectionFunction::init(property_name, meta)?;
            function.eval(record)
        }
        RuntimeComputedKind::DateTimeNow => DateTimeNowFunction::init().eval(record),
        RuntimeComputedKind::None => Ok(None),
    }
}

trait Evaluable {
    fn eval(&self, record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError>;
}

fn computed_meta<'a>(
    property_name: &str,
    meta: Option<&'a Value>,
    computed_key: &str,
) -> Result<&'a Value, RuntimeError> {
    meta.and_then(|meta| meta.get(computed_key)).ok_or_else(|| {
        MetadataError::InvalidComputedMetadata {
            property_name: property_name.to_string(),
            message: format!("missing '{}' metadata block", computed_key),
        }
        .into()
    })
}

fn computed_string_array(
    property_name: &str,
    meta: &Value,
    field_name: &str,
) -> Result<Vec<String>, RuntimeError> {
    let values = meta
        .get(field_name)
        .and_then(Value::as_array)
        .ok_or_else(|| MetadataError::InvalidComputedMetadata {
            property_name: property_name.to_string(),
            message: format!("'{}' must be an array", field_name),
        })?;

    values
        .iter()
        .map(|item| {
            item.as_str()
                .map(to_snake_case_lenient)
                .ok_or_else(|| MetadataError::InvalidComputedMetadata {
                    property_name: property_name.to_string(),
                    message: format!("'{}' items must be strings", field_name),
                })
                .map_err(RuntimeError::from)
        })
        .collect()
}

fn computed_string<'a>(
    property_name: &str,
    meta: &'a Value,
    field_name: &str,
) -> Result<&'a str, RuntimeError> {
    meta.get(field_name).and_then(Value::as_str).ok_or_else(|| {
        MetadataError::InvalidComputedMetadata {
            property_name: property_name.to_string(),
            message: format!("'{}' must be a string", field_name),
        }
        .into()
    })
}

pub struct ConcatenateFunction {
    pub prop_names: Vec<String>,
    pub delimiter: Option<String>,
}

impl ConcatenateFunction {
    pub fn init(property_name: &str, meta: Option<&Value>) -> Result<Self, RuntimeError> {
        let fn_meta = computed_meta(property_name, meta, "ConcatenateComputed")?;
        let prop_names = computed_string_array(property_name, fn_meta, "prop_names")?;
        let delimiter = fn_meta
            .get("delimiter")
            .and_then(Value::as_str)
            .map(str::to_string);

        Ok(Self {
            prop_names,
            delimiter,
        })
    }
}

impl Evaluable for ConcatenateFunction {
    fn eval(&self, record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError> {
        let mut vars = Vec::new();
        for field_name in self.prop_names.iter() {
            if let Some(Value::String(value)) = record.get(field_name) {
                vars.push(value.replace('"', ""));
            }
        }

        debug!(
            field_count = vars.len(),
            "evaluated concatenate computed property"
        );
        Ok(Some(Value::String(
            vars.join(self.delimiter.as_deref().unwrap_or("")),
        )))
    }
}

pub struct FormatFunction {
    pub prop_names: Vec<String>,
    pub template: String,
}

impl FormatFunction {
    pub fn init(property_name: &str, meta: Option<&Value>) -> Result<Self, RuntimeError> {
        let fn_meta = computed_meta(property_name, meta, "FormatComputed")?;
        let prop_names = computed_string_array(property_name, fn_meta, "propNames")?;
        let template = computed_string(property_name, fn_meta, "template")?.to_string();

        Ok(Self {
            prop_names,
            template,
        })
    }
}

impl Evaluable for FormatFunction {
    fn eval(&self, record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError> {
        let mut vars = HashMap::new();
        for field_name in self.prop_names.iter() {
            if let Some(Value::String(value)) = record.get(field_name) {
                vars.insert(field_name.to_string(), value.replace('"', ""));
            }
        }

        Ok(Some(Value::String(strfmt(&self.template, &vars).map_err(
            |err| RuntimeError::Validation(format!("format computed property failed: {}", err)),
        )?)))
    }
}

pub struct WordFunction {
    pub source_field: String,
}

impl WordFunction {
    pub fn init(property_name: &str, meta: Option<&Value>) -> Result<Self, RuntimeError> {
        let fn_meta = computed_meta(property_name, meta, "WordComputed")?;
        let source_field =
            to_snake_case_lenient(computed_string(property_name, fn_meta, "sourceField")?);

        Ok(Self { source_field })
    }
}

impl Evaluable for WordFunction {
    fn eval(&self, record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError> {
        let value = record
            .get(&self.source_field)
            .and_then(Value::as_str)
            .map(|value| value.replace(' ', "_").to_lowercase())
            .unwrap_or_default();

        Ok(Some(Value::String(value)))
    }
}

pub struct DateTimeNowFunction {}

impl DateTimeNowFunction {
    pub fn init() -> Self {
        Self {}
    }
}

impl Evaluable for DateTimeNowFunction {
    fn eval(&self, _record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError> {
        let now: DateTime<Utc> = Utc::now();
        Ok(Some(Value::String(now.to_rfc3339())))
    }
}

pub struct InflectionFunction {
    pub source_field: String,
    pub name: String,
}

impl InflectionFunction {
    pub fn init(property_name: &str, meta: Option<&Value>) -> Result<Self, RuntimeError> {
        let fn_meta = computed_meta(property_name, meta, "InflectionComputed")?;
        let source_field =
            to_snake_case_lenient(computed_string(property_name, fn_meta, "sourceField")?);
        let name = to_snake_case_lenient(computed_string(property_name, fn_meta, "name")?);

        Ok(Self { source_field, name })
    }
}

impl Evaluable for InflectionFunction {
    fn eval(&self, record: &Map<String, Value>) -> Result<Option<Value>, RuntimeError> {
        let value = record
            .get(&self.source_field)
            .and_then(Value::as_str)
            .map(|value| match self.name.as_str() {
                "pascal_1" => pascal_1(value),
                "pascal_n" => pascal_n(value),
                "snake_1" => snake_1(value),
                "snake_n" => snake_n(value),
                "caption_1" => caption_1(value),
                "caption_n" => caption_n(value),
                "camel" => camel(value),
                "caption" => caption(value),
                _ => {
                    warn!(inflection = %self.name, "unknown inflection computed filter");
                    value.to_string()
                }
            })
            .unwrap_or_default();

        Ok(Some(Value::String(value)))
    }
}

fn pascal_1(value: &str) -> String {
    if is_pascal_case(value) {
        value.to_string()
    } else {
        to_pascal_case(value)
    }
}

fn pascal_n(value: &str) -> String {
    to_plural(&pascal_1(value))
}

fn snake_1(value: &str) -> String {
    if is_snake_case(value) {
        value.to_string()
    } else {
        to_snake_case_lenient(value)
    }
}

fn snake_n(value: &str) -> String {
    if is_table_case(value) {
        value.to_string()
    } else {
        to_table_case(value)
    }
}

fn caption_1(value: &str) -> String {
    if is_title_case(value) {
        value.to_string()
    } else {
        to_title_case(value)
    }
}

fn caption_n(value: &str) -> String {
    to_plural(&caption_1(value))
}

fn camel(value: &str) -> String {
    if is_camel_case(value) {
        value.to_string()
    } else {
        to_camel_case(value)
    }
}

fn caption(value: &str) -> String {
    if is_title_case(value) {
        value.to_string()
    } else {
        to_title_case(value)
    }
}

fn to_snake_case_lenient(value: &str) -> String {
    if looks_like_snake_case(value) {
        value.to_string()
    } else {
        to_snake_case(value)
    }
}

fn looks_like_snake_case(value: &str) -> bool {
    if value.is_empty() || value.starts_with('_') || value.ends_with('_') || value.contains("__") {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use serde_json::json;

    use super::*;

    fn record(value: Value) -> Map<String, Value> {
        value.as_object().expect("record object").clone()
    }

    fn expect_string(result: Option<Value>) -> String {
        result
            .expect("computed value")
            .as_str()
            .expect("computed string")
            .to_string()
    }

    #[test]
    fn computed_none_returns_no_value() {
        let result = try_compute_property(
            RuntimeComputedKind::None,
            "name",
            None,
            &record(json!({ "name": "Acme" })),
        )
        .expect("none computed should succeed");

        assert!(result.is_none());
    }

    #[test]
    fn concatenate_computed_joins_string_fields_with_delimiter() {
        let meta = json!({
            "ConcatenateComputed": {
                "prop_names": ["firstName", "last_name"],
                "delimiter": " "
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Concatenate,
            "full_name",
            Some(&meta),
            &record(json!({
                "first_name": "Ada",
                "last_name": "Lovelace"
            })),
        )
        .expect("concatenate should compute");

        assert_eq!(expect_string(result), "Ada Lovelace");
    }

    #[test]
    fn concatenate_computed_ignores_missing_and_non_string_fields() {
        let meta = json!({
            "ConcatenateComputed": {
                "prop_names": ["name", "missing", "count"],
                "delimiter": "-"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Concatenate,
            "display",
            Some(&meta),
            &record(json!({ "name": "Acme", "count": 12 })),
        )
        .expect("concatenate should skip unusable values");

        assert_eq!(expect_string(result), "Acme");
    }

    #[test]
    fn format_computed_renders_template_with_snake_case_field_names() {
        let meta = json!({
            "FormatComputed": {
                "propNames": ["firstName", "lastName"],
                "template": "{first_name} <{last_name}>"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Format,
            "label",
            Some(&meta),
            &record(json!({
                "first_name": "Ada",
                "last_name": "Lovelace"
            })),
        )
        .expect("format should compute");

        assert_eq!(expect_string(result), "Ada <Lovelace>");
    }

    #[test]
    fn format_computed_fails_when_template_variable_is_missing() {
        let meta = json!({
            "FormatComputed": {
                "propNames": ["firstName"],
                "template": "{first_name} {last_name}"
            }
        });

        let err = try_compute_property(
            RuntimeComputedKind::Format,
            "label",
            Some(&meta),
            &record(json!({ "first_name": "Ada" })),
        )
        .expect_err("missing template variable should fail");

        assert!(err.to_string().contains("format computed property failed"));
    }

    #[test]
    fn word_computed_normalizes_source_field_to_lower_underscore_word() {
        let meta = json!({
            "WordComputed": {
                "sourceField": "displayName"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Word,
            "slug",
            Some(&meta),
            &record(json!({ "display_name": "North America Sales" })),
        )
        .expect("word should compute");

        assert_eq!(expect_string(result), "north_america_sales");
    }

    #[test]
    fn word_computed_returns_empty_string_when_source_field_is_missing() {
        let meta = json!({
            "WordComputed": {
                "sourceField": "displayName"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Word,
            "slug",
            Some(&meta),
            &record(json!({})),
        )
        .expect("missing word source should be tolerated");

        assert_eq!(expect_string(result), "");
    }

    #[test]
    fn inflection_computed_supports_all_declared_inflections() {
        let cases = [
            ("pascal_1", "knowledge article", "KnowledgeArticle"),
            ("pascal_n", "knowledge article", "KnowledgeArticles"),
            ("snake_1", "KnowledgeArticle", "knowledge_article"),
            ("snake_n", "KnowledgeArticle", "knowledge_articles"),
            ("caption_1", "knowledge article", "Knowledge Article"),
            ("caption_n", "knowledge article", "Knowledge Articles"),
            ("camel", "knowledge article", "knowledgeArticle"),
            ("caption", "knowledge article", "Knowledge Article"),
        ];

        for (name, source, expected) in cases {
            let meta = json!({
                "InflectionComputed": {
                    "sourceField": "sourceName",
                    "name": name
                }
            });

            let result = try_compute_property(
                RuntimeComputedKind::Inflection,
                "computed_name",
                Some(&meta),
                &record(json!({ "source_name": source })),
            )
            .unwrap_or_else(|err| panic!("inflection {name} should compute: {err}"));

            assert_eq!(expect_string(result), expected, "inflection {name}");
        }
    }

    #[test]
    fn inflection_computed_returns_source_for_unknown_inflection_name() {
        let meta = json!({
            "InflectionComputed": {
                "sourceField": "sourceName",
                "name": "unknown_shape"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Inflection,
            "computed_name",
            Some(&meta),
            &record(json!({ "source_name": "Knowledge Article" })),
        )
        .expect("unknown inflection should fall back to source value");

        assert_eq!(expect_string(result), "Knowledge Article");
    }

    #[test]
    fn inflection_computed_returns_empty_string_when_source_field_is_missing() {
        let meta = json!({
            "InflectionComputed": {
                "sourceField": "sourceName",
                "name": "snake_1"
            }
        });

        let result = try_compute_property(
            RuntimeComputedKind::Inflection,
            "computed_name",
            Some(&meta),
            &record(json!({})),
        )
        .expect("missing inflection source should be tolerated");

        assert_eq!(expect_string(result), "");
    }

    #[test]
    fn datetime_now_computed_returns_valid_rfc3339_utc_timestamp() {
        let before = Utc::now();
        let result = try_compute_property(
            RuntimeComputedKind::DateTimeNow,
            "updated_at",
            None,
            &record(json!({})),
        )
        .expect("datetime now should compute");
        let after = Utc::now();

        let value = expect_string(result);
        let parsed = DateTime::parse_from_rfc3339(&value)
            .expect("computed datetime should be RFC3339")
            .with_timezone(&Utc);

        assert!(parsed >= before);
        assert!(parsed <= after);
    }

    #[test]
    fn missing_computed_metadata_block_fails_closed() {
        let err = try_compute_property(
            RuntimeComputedKind::Concatenate,
            "full_name",
            None,
            &record(json!({})),
        )
        .expect_err("missing computed metadata should fail");

        assert!(matches!(
            err,
            RuntimeError::Metadata(MetadataError::InvalidComputedMetadata { .. })
        ));
        assert!(err
            .to_string()
            .contains("missing 'ConcatenateComputed' metadata block"));
    }

    #[test]
    fn malformed_string_array_metadata_fails_closed() {
        let meta = json!({
            "ConcatenateComputed": {
                "prop_names": "firstName"
            }
        });

        let err = try_compute_property(
            RuntimeComputedKind::Concatenate,
            "full_name",
            Some(&meta),
            &record(json!({})),
        )
        .expect_err("malformed prop_names should fail");

        assert!(matches!(
            err,
            RuntimeError::Metadata(MetadataError::InvalidComputedMetadata { .. })
        ));
        assert!(err.to_string().contains("'prop_names' must be an array"));
    }

    #[test]
    fn non_string_array_item_metadata_fails_closed() {
        let meta = json!({
            "ConcatenateComputed": {
                "prop_names": ["firstName", 12]
            }
        });

        let err = try_compute_property(
            RuntimeComputedKind::Concatenate,
            "full_name",
            Some(&meta),
            &record(json!({})),
        )
        .expect_err("non-string prop_names item should fail");

        assert!(matches!(
            err,
            RuntimeError::Metadata(MetadataError::InvalidComputedMetadata { .. })
        ));
        assert!(err
            .to_string()
            .contains("'prop_names' items must be strings"));
    }

    #[test]
    fn malformed_string_metadata_fails_closed() {
        let meta = json!({
            "FormatComputed": {
                "propNames": ["firstName"],
                "template": 42
            }
        });

        let err = try_compute_property(
            RuntimeComputedKind::Format,
            "label",
            Some(&meta),
            &record(json!({ "first_name": "Ada" })),
        )
        .expect_err("non-string template should fail");

        assert!(matches!(
            err,
            RuntimeError::Metadata(MetadataError::InvalidComputedMetadata { .. })
        ));
        assert!(err.to_string().contains("'template' must be a string"));
    }
}
