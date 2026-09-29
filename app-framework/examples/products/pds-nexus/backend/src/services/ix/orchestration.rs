//! Providerless context-to-recipe orchestration logic for the Nexus
//! "My Work" intelligent experience.
//!
//! These are plain service functions written against the un-gated Framework
//! contract and recipe types. They resolve bounded, tenant-scoped work
//! context, select the PDS-registered recipe for the requested intent, and
//! compose deterministic `pds.ix.presentation@1` payloads and draft plans.
//! No AI provider is consulted anywhere (providerless by contract), and no
//! lifecycle fact is authored here: the chat-gated B_HOST glue converts
//! these product values into runtime drafts (DEFERRED_TO_B_HOST).
//!
//! Unavailability is a first-class posture: when authorized context is
//! absent the resolution and the run plan both say so explicitly instead of
//! fabricating context.

use appfw_runtime::ix::{
    IxArtifactStatus, IxContextFreshness, IxContextSummary, IxContractError, IxPhase,
    IxRecipeRegistration, IxSourceSummary, PDS_IX_PRESENTATION_SCHEMA_VERSION,
    PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION,
};
use serde_json::{json, Value};

/// The PDS-registered intent served by the My Work journey.
pub const NEXUS_MY_WORK_INTENT_KEY: &str = "pds.ix.intent.attention-stewardship@1";
/// Product artifact type registered for the My Work brief.
pub const NEXUS_MY_WORK_ARTIFACT_TYPE: &str = "nexus.ix.my-work-brief@1";
/// PDS renderer fixed by the attention-stewardship recipe.
pub const NEXUS_MY_WORK_RENDERER_KEY: &str = "pds.ix.recipe.attention-stewardship@1";
/// Work items stalled at least this many days need attention.
pub const NEXUS_MY_WORK_STALLED_THRESHOLD_DAYS: u32 = 5;
/// Bounded number of attention regions in one brief revision.
pub const NEXUS_MY_WORK_MAX_REGIONS: usize = 16;

/// Risk posture carried by a product work item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NexusWorkRisk {
    Low,
    Medium,
    High,
}

impl NexusWorkRisk {
    fn label(self) -> &'static str {
        match self {
            Self::Low => "Low risk",
            Self::Medium => "Medium risk",
            Self::High => "High risk",
        }
    }
}

/// One bounded, tenant-scoped work item snapshot supplied by product data
/// access. The orchestration functions never fetch data themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NexusMyWorkItem {
    pub record_locator: String,
    pub title: String,
    pub owner_team: String,
    pub phase: String,
    pub risk: NexusWorkRisk,
    pub days_stalled: u32,
    /// RFC 3339 timestamp of the item's source refresh.
    pub refreshed_at: String,
}

/// Authorized, bounded context resolved for one principal.
#[derive(Clone, Debug, PartialEq)]
pub struct NexusIxResolvedContext {
    pub context_id: String,
    pub summary: IxContextSummary,
    /// Portable JSON payload for the product orchestrator (never logged).
    pub payload: Value,
}

/// Context resolution outcome. Absence is explicit, never fabricated.
#[derive(Clone, Debug, PartialEq)]
pub enum NexusIxContextResolution {
    Available(Box<NexusIxResolvedContext>),
    Unavailable { reason: String },
}

/// Product draft plan for one run. The chat-gated host glue converts these
/// into Framework orchestration drafts at B_HOST.
#[derive(Clone, Debug, PartialEq)]
pub enum NexusIxRunPlan {
    /// The experience is unavailable for this principal/focus; the journey
    /// must present the unavailable posture instead of a fabricated brief.
    Unavailable { reason: String },
    Drafts(Vec<NexusIxRunDraft>),
}

/// One product-authored draft step (draft content only; the Framework
/// lifecycle stamps every canonical event).
#[derive(Clone, Debug, PartialEq)]
pub enum NexusIxRunDraft {
    Phase {
        phase: IxPhase,
        reason: String,
    },
    Artifact {
        artifact_id: String,
        artifact_type: String,
        content_schema_version: String,
        renderer_key: String,
        revision: u64,
        status: IxArtifactStatus,
        changed_region_ids: Vec<String>,
        presentation: Value,
    },
    Completed {
        message: String,
    },
}

/// The validated PDS recipe registration for the My Work journey. The
/// registration is checked against the Framework-embedded PDS registry, so a
/// drifting recipe id, renderer, schema, or capability set fails here.
pub fn my_work_recipe_registration() -> Result<IxRecipeRegistration, IxContractError> {
    IxRecipeRegistration::new(
        PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION,
        "attention-stewardship",
        NEXUS_MY_WORK_INTENT_KEY,
        NEXUS_MY_WORK_ARTIFACT_TYPE,
        PDS_IX_PRESENTATION_SCHEMA_VERSION,
        NEXUS_MY_WORK_RENDERER_KEY,
        vec![
            "pds.ix.capability.focus-context@1".to_string(),
            "pds.ix.capability.progressive-presentation@1".to_string(),
            "pds.ix.capability.evidence-disclosure@1".to_string(),
            "pds.ix.capability.human-control@1".to_string(),
            "pds.ix.capability.attention-correction@1".to_string(),
        ],
    )
}

/// Context-to-recipe selection: only intents with a product-registered
/// recipe are servable; everything else is explicitly unavailable.
pub fn select_recipe_registration(
    intent_key: &str,
) -> Result<Option<IxRecipeRegistration>, IxContractError> {
    if intent_key == NEXUS_MY_WORK_INTENT_KEY {
        return my_work_recipe_registration().map(Some);
    }
    Ok(None)
}

/// Resolves the bounded My Work context for one tenant-scoped principal.
///
/// `evaluated_at` is the caller-supplied RFC 3339 evaluation instant; item
/// refresh instants newer than it are a caller error surfaced by the
/// Framework summary validation in tests.
pub fn resolve_my_work_context(
    tenant_id: &str,
    subject: &str,
    evaluated_at: &str,
    items: &[NexusMyWorkItem],
) -> NexusIxContextResolution {
    if tenant_id.trim().is_empty() || subject.trim().is_empty() {
        return NexusIxContextResolution::Unavailable {
            reason: "No authorized principal context is present for this request.".to_string(),
        };
    }
    if items.is_empty() {
        return NexusIxContextResolution::Unavailable {
            reason: format!(
                "No work-queue context exists for {subject} in this tenant; \
                 the My Work experience is unavailable rather than fabricated."
            ),
        };
    }
    let stalled = stalled_items(items);
    let latest_refresh = items
        .iter()
        .map(|item| item.refreshed_at.as_str())
        .max()
        .map(ToString::to_string);
    let summary = IxContextSummary {
        focus_label: format!("My Work queue for {subject}"),
        detail: format!(
            "{} tracked work items; {} stalled {} days or longer.",
            items.len(),
            stalled.len(),
            NEXUS_MY_WORK_STALLED_THRESHOLD_DAYS
        ),
        evaluated_at: evaluated_at.to_string(),
        freshness: IxContextFreshness::Current,
        sources: vec![IxSourceSummary {
            source_ref: "nexus.work.de-novo-tracker".to_string(),
            label: "De Novo tracker (local synthetic projection)".to_string(),
            refreshed_at: latest_refresh,
            freshness: IxContextFreshness::Current,
            inspectable: true,
        }],
        gaps: vec![
            "ServiceNow source reconciliation is not connected in the local vertical.".to_string(),
        ],
    };
    let payload = json!({
        "schemaVersion": "nexus.ix.my-work-context@1",
        "tenantId": tenant_id,
        "subject": subject,
        "evaluatedAt": evaluated_at,
        "stalledThresholdDays": NEXUS_MY_WORK_STALLED_THRESHOLD_DAYS,
        "items": items.iter().map(work_item_json).collect::<Vec<_>>(),
    });
    NexusIxContextResolution::Available(Box::new(NexusIxResolvedContext {
        context_id: format!("nexus-ix-my-work-{tenant_id}-{}", key_safe(subject)),
        summary,
        payload,
    }))
}

/// Composes one deterministic `pds.ix.presentation@1` envelope for the My
/// Work brief. All regions are marked changed on the first revision; later
/// revisions mark only regions whose backing item changed.
pub fn compose_my_work_presentation(
    presentation_id: &str,
    revision: u64,
    context: &NexusIxResolvedContext,
    items: &[NexusMyWorkItem],
    changed_locators: &[&str],
) -> Value {
    let attention = attention_items(items);
    let regions = attention
        .iter()
        .map(|item| {
            let region_id = region_id_for(item);
            let changed = changed_locators.contains(&item.record_locator.as_str());
            json!({
                "id": region_id,
                "status": "ready",
                "label": item.risk.label(),
                "title": item.title,
                "body": format!(
                    "{} owns this {} work item; it has not moved for {} days.",
                    item.owner_team, item.phase, item.days_stalled
                ),
                "whyItMatters": format!(
                    "Stalled {} work blocks the launch sequence; acting now keeps {} on plan.",
                    item.phase, item.owner_team
                ),
                "evidence": {
                    "summary": "Where did this come from?",
                    "items": [{
                        "sourceRef": "nexus.work.de-novo-tracker",
                        "label": "Record locator",
                        "value": item.record_locator,
                    }],
                },
                "editable": false,
                "changed": changed,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "identity": {
            "schemaVersion": PDS_IX_PRESENTATION_SCHEMA_VERSION,
            "presentationId": presentation_id,
            "revision": revision,
        },
        "announcement": format!("My Work brief revision {revision} is ready."),
        "context": {
            "eyebrow": "Working from",
            "title": context.summary.focus_label,
            "detail": context.summary.detail,
            "announcement": format!(
                "Context resolved from {} source with {} known gap.",
                context.summary.sources.len(),
                context.summary.gaps.len()
            ),
            "gaps": context.summary.gaps,
            "evidence": {
                "summary": "View source",
                "items": context.summary.sources.iter().map(|source| json!({
                    "sourceRef": source.source_ref,
                    "label": source.label,
                    "value": source
                        .refreshed_at
                        .clone()
                        .unwrap_or_else(|| "refresh instant unknown".to_string()),
                })).collect::<Vec<_>>(),
            },
            "metaLabel": format!("Evaluated {}", context.summary.evaluated_at),
        },
        "workStatus": {
            "label": "Evidence checked",
            "detail": format!("{} attention items resolved", regions.len()),
            "active": false,
        },
        "response": {
            "eyebrow": "Progressive response",
            "title": "My Work attention brief",
            "metaLabel": format!("Revision {revision}"),
            "announcement": format!("My Work brief revision {revision} is ready."),
            "active": false,
            "regions": regions,
            "emptyState": "Attention items will appear when work needs you.",
            "footerText": "Local synthetic projection; no external source is consulted.",
        },
    })
}

/// Plans the full draft sequence for one My Work run, honoring the
/// unavailable-when-context-absent posture.
pub fn plan_my_work_run(
    presentation_id: &str,
    resolution: &NexusIxContextResolution,
    items: &[NexusMyWorkItem],
) -> Result<NexusIxRunPlan, IxContractError> {
    let context = match resolution {
        NexusIxContextResolution::Unavailable { reason } => {
            return Ok(NexusIxRunPlan::Unavailable {
                reason: reason.clone(),
            });
        }
        NexusIxContextResolution::Available(context) => context,
    };
    let registration = my_work_recipe_registration()?;
    let attention = attention_items(items);
    let changed = attention
        .iter()
        .map(|item| item.record_locator.as_str())
        .collect::<Vec<_>>();
    let presentation =
        compose_my_work_presentation(presentation_id, 1, context, items, &changed);
    let changed_region_ids = attention.iter().map(|item| region_id_for(item)).collect();
    Ok(NexusIxRunPlan::Drafts(vec![
        NexusIxRunDraft::Phase {
            phase: IxPhase::Understanding,
            reason: "Interpreting the resolved My Work queue context.".to_string(),
        },
        NexusIxRunDraft::Artifact {
            artifact_id: presentation_id.to_string(),
            artifact_type: registration.artifact_type().to_string(),
            content_schema_version: registration.content_schema_version().to_string(),
            renderer_key: registration.renderer_key().to_string(),
            revision: 1,
            status: IxArtifactStatus::Ready,
            changed_region_ids,
            presentation,
        },
        NexusIxRunDraft::Completed {
            message: format!(
                "My Work brief is ready with {} attention item(s).",
                attention.len()
            ),
        },
    ]))
}

/// Items needing attention, ordered by risk then stalled days, bounded to
/// the region budget.
pub fn attention_items(items: &[NexusMyWorkItem]) -> Vec<NexusMyWorkItem> {
    let mut attention = items
        .iter()
        .filter(|item| item.days_stalled >= NEXUS_MY_WORK_STALLED_THRESHOLD_DAYS)
        .cloned()
        .collect::<Vec<_>>();
    attention.sort_by(|left, right| {
        right
            .risk
            .cmp(&left.risk)
            .then(right.days_stalled.cmp(&left.days_stalled))
            .then(left.record_locator.cmp(&right.record_locator))
    });
    attention.truncate(NEXUS_MY_WORK_MAX_REGIONS);
    attention
}

fn stalled_items(items: &[NexusMyWorkItem]) -> Vec<&NexusMyWorkItem> {
    items
        .iter()
        .filter(|item| item.days_stalled >= NEXUS_MY_WORK_STALLED_THRESHOLD_DAYS)
        .collect()
}

fn region_id_for(item: &NexusMyWorkItem) -> String {
    format!("work-{}", key_safe(&item.record_locator))
}

fn work_item_json(item: &NexusMyWorkItem) -> Value {
    json!({
        "recordLocator": item.record_locator,
        "title": item.title,
        "ownerTeam": item.owner_team,
        "phase": item.phase,
        "risk": item.risk.label(),
        "daysStalled": item.days_stalled,
        "refreshedAt": item.refreshed_at,
    })
}

fn key_safe(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

/// Deterministic sample queue shared by the unit tests and the adapter
/// proof fixtures.
#[cfg(test)]
pub(crate) fn sample_items() -> Vec<NexusMyWorkItem> {
    vec![
        NexusMyWorkItem {
            record_locator: "rl_11111111111111111111111111111111".to_string(),
            title: "Credentialing packet".to_string(),
            owner_team: "Provider onboarding".to_string(),
            phase: "Licensing".to_string(),
            risk: NexusWorkRisk::High,
            days_stalled: 8,
            refreshed_at: "2026-08-15T00:00:00Z".to_string(),
        },
        NexusMyWorkItem {
            record_locator: "rl_22222222222222222222222222222222".to_string(),
            title: "Equipment approval".to_string(),
            owner_team: "Clinical operations".to_string(),
            phase: "Buildout".to_string(),
            risk: NexusWorkRisk::Medium,
            days_stalled: 6,
            refreshed_at: "2026-08-15T00:00:00Z".to_string(),
        },
        NexusMyWorkItem {
            record_locator: "rl_44444444444444444444444444444444".to_string(),
            title: "Training roster confirmation".to_string(),
            owner_team: "Learning team".to_string(),
            phase: "Launch readiness".to_string(),
            risk: NexusWorkRisk::Low,
            days_stalled: 2,
            refreshed_at: "2026-08-15T00:00:00Z".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use appfw_runtime::ix::IxArtifactRevision;

    use super::*;

    #[test]
    fn recipe_registration_matches_the_embedded_pds_registry() {
        let registration = my_work_recipe_registration().expect("registration validates");
        assert_eq!(registration.recipe_id(), "attention-stewardship");
        assert_eq!(registration.intent_key(), NEXUS_MY_WORK_INTENT_KEY);
        assert_eq!(registration.artifact_type(), NEXUS_MY_WORK_ARTIFACT_TYPE);
        assert_eq!(registration.renderer_key(), NEXUS_MY_WORK_RENDERER_KEY);
    }

    #[test]
    fn recipe_selection_is_closed_over_registered_intents() {
        assert!(select_recipe_registration(NEXUS_MY_WORK_INTENT_KEY)
            .expect("known intent resolves")
            .is_some());
        assert!(
            select_recipe_registration("pds.ix.intent.analyze-why@1")
                .expect("unregistered intent is a clean miss")
                .is_none(),
            "intents without a product recipe registration must not be served"
        );
    }

    #[test]
    fn context_is_unavailable_when_absent_and_never_fabricated() {
        let empty = resolve_my_work_context("tenant_a", "nexus.user", "2026-08-16T00:00:00Z", &[]);
        match empty {
            NexusIxContextResolution::Unavailable { reason } => {
                assert!(reason.contains("unavailable"));
            }
            NexusIxContextResolution::Available(_) => {
                panic!("absent work context must resolve to the unavailable posture")
            }
        }
        let anonymous =
            resolve_my_work_context("", "nexus.user", "2026-08-16T00:00:00Z", &sample_items());
        assert!(matches!(
            anonymous,
            NexusIxContextResolution::Unavailable { .. }
        ));
    }

    #[test]
    fn resolved_context_passes_framework_summary_validation() {
        let resolution = resolve_my_work_context(
            "tenant_a",
            "nexus.user",
            "2026-08-16T00:00:00Z",
            &sample_items(),
        );
        let NexusIxContextResolution::Available(context) = resolution else {
            panic!("context must be available for a populated queue");
        };
        context
            .summary
            .validate()
            .expect("summary satisfies the Framework context contract");
        assert_eq!(context.summary.freshness, IxContextFreshness::Current);
        assert_eq!(
            context.context_id,
            "nexus-ix-my-work-tenant_a-nexus.user"
        );
        assert_eq!(context.payload["items"].as_array().map(Vec::len), Some(3));
    }

    #[test]
    fn composed_presentation_is_lifecycle_grade_for_the_registered_recipe() {
        let items = sample_items();
        let resolution =
            resolve_my_work_context("tenant_a", "nexus.user", "2026-08-16T00:00:00Z", &items);
        let NexusIxContextResolution::Available(context) = resolution else {
            panic!("context must be available");
        };
        let attention = attention_items(&items);
        let changed = attention
            .iter()
            .map(|item| item.record_locator.as_str())
            .collect::<Vec<_>>();
        let presentation =
            compose_my_work_presentation("nexus-my-work-brief-0001", 1, &context, &items, &changed);
        let revision = IxArtifactRevision {
            artifact_id: "nexus-my-work-brief-0001".to_string(),
            artifact_type: NEXUS_MY_WORK_ARTIFACT_TYPE.to_string(),
            content_schema_version: PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string(),
            renderer_key: NEXUS_MY_WORK_RENDERER_KEY.to_string(),
            revision: 1,
            status: IxArtifactStatus::Ready,
            changed_region_ids: attention.iter().map(region_id_for).collect(),
            presentation,
        };
        revision
            .validate()
            .expect("the composed presentation passes the Framework artifact contract");
    }

    #[test]
    fn run_plan_honors_the_unavailable_posture() {
        let plan = plan_my_work_run(
            "nexus-my-work-brief-0001",
            &NexusIxContextResolution::Unavailable {
                reason: "No work-queue context exists.".to_string(),
            },
            &[],
        )
        .expect("plan resolves");
        assert!(matches!(plan, NexusIxRunPlan::Unavailable { .. }));
    }

    #[test]
    fn run_plan_produces_a_deterministic_bounded_draft_sequence() {
        let items = sample_items();
        let resolution =
            resolve_my_work_context("tenant_a", "nexus.user", "2026-08-16T00:00:00Z", &items);
        let plan = plan_my_work_run("nexus-my-work-brief-0001", &resolution, &items)
            .expect("plan resolves");
        let NexusIxRunPlan::Drafts(drafts) = plan else {
            panic!("populated context must produce drafts");
        };
        assert_eq!(drafts.len(), 3);
        assert!(matches!(
            drafts[0],
            NexusIxRunDraft::Phase {
                phase: IxPhase::Understanding,
                ..
            }
        ));
        let NexusIxRunDraft::Artifact {
            revision,
            status,
            changed_region_ids,
            presentation,
            ..
        } = &drafts[1]
        else {
            panic!("second draft must publish the brief artifact");
        };
        assert_eq!(*revision, 1);
        assert_eq!(*status, IxArtifactStatus::Ready);
        assert_eq!(changed_region_ids.len(), 2, "two items are stalled");
        assert_eq!(
            presentation["response"]["regions"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert!(matches!(drafts[2], NexusIxRunDraft::Completed { .. }));
    }
}
