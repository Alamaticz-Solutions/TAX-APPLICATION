//! Deterministic, network-free provider-neutral intelligence validation.

use std::collections::BTreeSet;

pub const ANSWER_ENVELOPE_VERSION: &str = "answer_envelope@1";
pub const GENERATED_VIEW_METADATA_VERSION: &str = "generated_view_metadata@1";
pub const RECOMMENDATION_VERSION: &str = "recommendation@1";
pub const AI_EVALUATION_VERSION: &str = "ai_evaluation@1";
pub const LOCAL_FIXTURE_MODE: &str = "local-fixture";

#[derive(Clone, Debug, PartialEq)]
pub struct FoundationReference<'a> {
    pub record_locator: &'a str,
    pub source: &'a str,
    pub provenance: &'a str,
    pub freshness_watermark: &'a str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoundationAnswerEnvelope<'a> {
    pub version: &'a str,
    pub references: &'a [FoundationReference<'a>],
    pub citations: &'a [FoundationReference<'a>],
    pub fallback_view: &'a str,
    pub view_hint: Option<&'a str>,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedViewMetadata<'a> {
    pub version: &'a str,
    pub record_locators: &'a [&'a str],
    pub view_hint: Option<&'a str>,
    pub fallback_view: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundationRecommendation<'a> {
    pub version: &'a str,
    pub why_now: &'a str,
    pub safest_permitted_next_step: &'a str,
    pub preview_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundationEvaluation<'a> {
    pub version: &'a str,
    pub mode: &'a str,
    pub provenance: &'a str,
    pub release_ready: bool,
    pub live_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundationViewSelection<'a> {
    pub selected_view: &'a str,
    pub degraded_to_fallback: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FoundationValidationError {
    UnsupportedVersion,
    InvalidRecordLocator,
    MissingSource,
    MissingProvenance,
    MissingFreshness,
    DanglingCitation,
    InvalidFallback,
    InvalidConfidence,
    RecommendationHasAuthority,
    InvalidLocalEvaluationPosture,
}

pub fn validate_answer_envelope<'a>(
    envelope: &FoundationAnswerEnvelope<'a>,
) -> Result<FoundationViewSelection<'a>, FoundationValidationError> {
    if envelope.version != ANSWER_ENVELOPE_VERSION {
        return Err(FoundationValidationError::UnsupportedVersion);
    }
    validate_fallback(envelope.fallback_view)?;
    if envelope
        .confidence
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(FoundationValidationError::InvalidConfidence);
    }
    for reference in envelope.references.iter().chain(envelope.citations.iter()) {
        validate_reference(reference)?;
    }
    let references = envelope
        .references
        .iter()
        .map(|reference| reference.record_locator)
        .collect::<BTreeSet<_>>();
    if envelope
        .citations
        .iter()
        .any(|citation| !references.contains(citation.record_locator))
    {
        return Err(FoundationValidationError::DanglingCitation);
    }
    Ok(select_view(envelope.view_hint, envelope.fallback_view))
}

pub fn validate_generated_view_metadata<'a>(
    metadata: &GeneratedViewMetadata<'a>,
) -> Result<FoundationViewSelection<'a>, FoundationValidationError> {
    if metadata.version != GENERATED_VIEW_METADATA_VERSION {
        return Err(FoundationValidationError::UnsupportedVersion);
    }
    validate_fallback(metadata.fallback_view)?;
    if metadata
        .record_locators
        .iter()
        .any(|value| !is_record_locator(value))
    {
        return Err(FoundationValidationError::InvalidRecordLocator);
    }
    Ok(select_view(metadata.view_hint, metadata.fallback_view))
}

pub fn validate_recommendation(
    recommendation: &FoundationRecommendation<'_>,
) -> Result<(), FoundationValidationError> {
    if recommendation.version != RECOMMENDATION_VERSION {
        return Err(FoundationValidationError::UnsupportedVersion);
    }
    if recommendation.why_now.trim().is_empty()
        || recommendation.safest_permitted_next_step.trim().is_empty()
        || !recommendation.preview_only
    {
        return Err(FoundationValidationError::RecommendationHasAuthority);
    }
    Ok(())
}

pub fn validate_local_evaluation(
    evaluation: &FoundationEvaluation<'_>,
) -> Result<(), FoundationValidationError> {
    if evaluation.version != AI_EVALUATION_VERSION {
        return Err(FoundationValidationError::UnsupportedVersion);
    }
    if evaluation.mode != LOCAL_FIXTURE_MODE
        || evaluation.provenance != "synthetic"
        || evaluation.release_ready
        || evaluation.live_ready
    {
        return Err(FoundationValidationError::InvalidLocalEvaluationPosture);
    }
    Ok(())
}

fn validate_reference(
    reference: &FoundationReference<'_>,
) -> Result<(), FoundationValidationError> {
    if !is_record_locator(reference.record_locator) {
        return Err(FoundationValidationError::InvalidRecordLocator);
    }
    if reference.source.trim().is_empty() {
        return Err(FoundationValidationError::MissingSource);
    }
    if reference.provenance.trim().is_empty() {
        return Err(FoundationValidationError::MissingProvenance);
    }
    if reference.freshness_watermark.trim().is_empty() {
        return Err(FoundationValidationError::MissingFreshness);
    }
    Ok(())
}

fn validate_fallback(value: &str) -> Result<(), FoundationValidationError> {
    if value.trim().is_empty() {
        Err(FoundationValidationError::InvalidFallback)
    } else {
        Ok(())
    }
}

fn select_view<'a>(hint: Option<&'a str>, fallback: &'a str) -> FoundationViewSelection<'a> {
    match hint {
        Some(view) if is_registered_view(view) => FoundationViewSelection {
            selected_view: view,
            degraded_to_fallback: false,
        },
        _ => FoundationViewSelection {
            selected_view: fallback,
            degraded_to_fallback: true,
        },
    }
}

fn is_registered_view(value: &str) -> bool {
    matches!(
        value,
        "list" | "detail" | "timeline" | "card" | "flow-graph"
    )
}

fn is_record_locator(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("rl_")
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCATOR: &str = "rl_11111111111111111111111111111111";
    const OTHER: &str = "rl_22222222222222222222222222222222";

    fn reference(locator: &'static str) -> FoundationReference<'static> {
        FoundationReference {
            record_locator: locator,
            source: "synthetic-fixture-store",
            provenance: "synthetic",
            freshness_watermark: "2026-07-18T00:00:00Z",
        }
    }

    #[test]
    fn validates_envelope_and_deterministically_degrades_view() {
        let references = [reference(LOCATOR)];
        let citations = [reference(LOCATOR)];
        let envelope = FoundationAnswerEnvelope {
            version: ANSWER_ENVELOPE_VERSION,
            references: &references,
            citations: &citations,
            fallback_view: "list",
            view_hint: Some("provider-widget"),
            confidence: Some(0.75),
        };
        assert_eq!(
            validate_answer_envelope(&envelope),
            Ok(FoundationViewSelection {
                selected_view: "list",
                degraded_to_fallback: true
            })
        );
    }

    #[test]
    fn malformed_metadata_fails_closed() {
        let references = [reference(LOCATOR)];
        let citations = [reference(OTHER)];
        let envelope = FoundationAnswerEnvelope {
            version: ANSWER_ENVELOPE_VERSION,
            references: &references,
            citations: &citations,
            fallback_view: "list",
            view_hint: None,
            confidence: Some(0.5),
        };
        assert_eq!(
            validate_answer_envelope(&envelope),
            Err(FoundationValidationError::DanglingCitation)
        );
        let invalid = [FoundationReference {
            provenance: "",
            ..reference(LOCATOR)
        }];
        let envelope = FoundationAnswerEnvelope {
            references: &invalid,
            citations: &[],
            ..envelope
        };
        assert_eq!(
            validate_answer_envelope(&envelope),
            Err(FoundationValidationError::MissingProvenance)
        );
    }

    #[test]
    fn generated_view_recommendation_and_evaluation_preserve_non_authority() {
        assert_eq!(
            validate_generated_view_metadata(&GeneratedViewMetadata {
                version: GENERATED_VIEW_METADATA_VERSION,
                record_locators: &[LOCATOR],
                view_hint: None,
                fallback_view: "list",
            }),
            Ok(FoundationViewSelection {
                selected_view: "list",
                degraded_to_fallback: true
            })
        );
        assert_eq!(
            validate_recommendation(&FoundationRecommendation {
                version: RECOMMENDATION_VERSION,
                why_now: "Synthetic freshness threshold reached.",
                safest_permitted_next_step: "Review fixture evidence.",
                preview_only: true,
            }),
            Ok(())
        );
        assert_eq!(
            validate_local_evaluation(&FoundationEvaluation {
                version: AI_EVALUATION_VERSION,
                mode: LOCAL_FIXTURE_MODE,
                provenance: "synthetic",
                release_ready: false,
                live_ready: false,
            }),
            Ok(())
        );
    }
}
