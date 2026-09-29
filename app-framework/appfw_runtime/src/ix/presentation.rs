//! Canonical PDS Health IX presentation validation.
//!
//! The schema embedded here is a byte-identical generated projection of the
//! PDS-owned source. IX adds artifact identity and lifecycle rules only after
//! this closed presentation-data contract passes.

use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;

use super::contract::IxContractError;

const PDS_IX_PRESENTATION_SCHEMA_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/pds_health/pds.ix.presentation.v1.schema.json"
));

fn build_validator(schema_bytes: &[u8]) -> Result<Validator, IxContractError> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|_| IxContractError::InvalidPresentation)?;
    jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|_| IxContractError::InvalidPresentation)
}

fn validator() -> Result<&'static Validator, IxContractError> {
    static VALIDATOR: OnceLock<Result<Validator, ()>> = OnceLock::new();
    VALIDATOR
        .get_or_init(|| build_validator(PDS_IX_PRESENTATION_SCHEMA_BYTES).map_err(|_| ()))
        .as_ref()
        .map_err(|_| IxContractError::InvalidPresentation)
}

pub(super) fn validate_pds_ix_presentation(value: &Value) -> Result<(), IxContractError> {
    if validator()?.is_valid(value) {
        Ok(())
    } else {
        Err(IxContractError::InvalidPresentation)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn golden() -> Value {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/contracts/pds_health/working-brief.presentation.json"
        )))
        .expect("packaged PDS golden presentation must be valid JSON")
    }

    #[test]
    fn canonical_golden_and_optional_sections_validate() {
        let value = golden();
        validate_pds_ix_presentation(&value).expect("canonical golden is valid");

        let mut minimal = value;
        let object = minimal
            .as_object_mut()
            .expect("canonical presentation is an object");
        object.remove("context");
        object.remove("workStatus");
        validate_pds_ix_presentation(&minimal).expect("optional sections may be absent");
    }

    #[test]
    fn canonical_schema_rejects_unknown_missing_wrong_and_oversized_values() {
        let mut candidates = Vec::new();

        let mut unknown_root = golden();
        unknown_root["secretContent"] = json!("must not be accepted");
        candidates.push(unknown_root);

        let mut missing_response = golden();
        missing_response
            .as_object_mut()
            .expect("golden is an object")
            .remove("response");
        candidates.push(missing_response);

        let mut wrong_announcement = golden();
        wrong_announcement["announcement"] = json!(42);
        candidates.push(wrong_announcement);

        let mut invalid_status = golden();
        invalid_status["response"]["regions"][0]["status"] = json!("completed");
        candidates.push(invalid_status);

        let mut duplicate_region = golden();
        let region = duplicate_region["response"]["regions"][0].clone();
        duplicate_region["response"]["regions"] = json!([region.clone(), region]);
        candidates.push(duplicate_region);

        let mut too_many_regions = golden();
        let region = too_many_regions["response"]["regions"][0].clone();
        too_many_regions["response"]["regions"] =
            Value::Array((0..65).map(|_| region.clone()).collect());
        candidates.push(too_many_regions);

        let mut oversized_identifier = golden();
        oversized_identifier["identity"]["presentationId"] = json!("x".repeat(129));
        candidates.push(oversized_identifier);

        let mut oversized_unicode_text = golden();
        oversized_unicode_text["announcement"] = json!("😀".repeat(4097));
        candidates.push(oversized_unicode_text);

        for candidate in candidates {
            assert_eq!(
                validate_pds_ix_presentation(&candidate),
                Err(IxContractError::InvalidPresentation)
            );
        }
    }

    #[test]
    fn canonical_schema_accepts_unicode_code_point_boundaries() {
        let mut identifier_boundary = golden();
        identifier_boundary["identity"]["presentationId"] = json!("😀".repeat(128));
        validate_pds_ix_presentation(&identifier_boundary)
            .expect("128 Unicode code points are valid for identifiers");

        let mut text_boundary = golden();
        text_boundary["announcement"] = json!("😀".repeat(4096));
        validate_pds_ix_presentation(&text_boundary)
            .expect("4096 Unicode code points are valid for text");
    }

    #[test]
    fn malformed_packaged_schema_fails_closed_without_panicking() {
        assert!(matches!(
            build_validator(br#"{"type":"not-a-valid-json-schema-type"}"#),
            Err(IxContractError::InvalidPresentation)
        ));
        assert!(matches!(
            build_validator(b"not-json"),
            Err(IxContractError::InvalidPresentation)
        ));
    }
}
