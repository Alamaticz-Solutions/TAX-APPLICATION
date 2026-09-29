# CRM Activities Signature Experience Slice

## Intent

Improve the human-owned CRM `/activities` route with a CRM-specific,
product-local, fixture-backed work queue and detail composition. It
demonstrates signature-experience construction inside the CRM reference
product without claiming a reusable PDS pattern, selecting a Nexus journey,
or introducing a live provider or product data contract.

## Signature Brief

- **Desired feeling:** calm, exact confidence that the product understands why
  an activity matters and preserves the consequence of a local review without
  pretending to act.
- **Signature object:** one flat `ActivityDecisionSpine` presents the causal
  sequence `Relationship → Evidence → Dependency → Consequence → Receipt`.
  One derived truth model owns partial relationship context, freshness,
  actionability, consequence inspection, and receipt posture.
- **Queue-to-spine moment:** selection changes the subject and the complete
  ordered path together; it does not leave detached readiness or summary
  cards behind.
- **Consequence moment:** `Inspect consequence` reveals the fixture-backed
  consequence and any partial, stale, refreshing, or blocked constraint in the
  same tool region.
- **Receipt moment:** `Record local review receipt` acknowledges only the
  selected item and its current truth posture after inspection, moves focus to
  the receipt, and continues to state `No provider action was sent`. A ready
  receipt may resurface when ready truth returns, but it never authorizes a
  partial or stale posture without a fresh inspection.
- **Structured equivalent:** the visual rail is an ordered list inside the
  region named `Decision path from relationship context to local receipt`.
  Every stage exposes an icon, a text state, and detail; color is never its
  only state channel.
- **Reduced motion and adverse truth:** the receipt acknowledgment is restrained
  to 180 ms and collapses under reduced motion. Loading, empty, offline,
  unauthorized, and error states use confined status/alert semantics;
  unauthorized state masks the decision path and receipt. Partial and stale
  paths remain inspectable while their constraint stays visible.

## Scope

- Human-owned CRM Activities queue/detail composition and deterministic fixtures.
- Local preview and receipt interaction only. No mutation, remote provider, or
  generated route behavior.
- Product-owned Expo continuation route using the same neutral work-item state.
- One narrow framework-introspection rule recognizes product `.appfw/specs/`
  files as editable application source so handoff can retain this spec without
  an unknown-ownership exception.

## Boundaries

- `/data/activities` remains the untouched generated `EntityDataRoute` fallback.
- The slice changes no model, backend, provider, runtime, API-test, shared
  design-system, generator-template, generated-output, or unrelated CLI
  behavior. The ownership classification described above is the sole
  framework/CLI behavior change.
- All data remains deterministic, synthetic, and local to the product surface.

## Current PDS Baseline

The slice consumes the current canonical PDS token package and the shared
commodity `Button` and `FeedbackState` components where their contracts are
accurate. The decision spine and its truth model remain product-local because
their relationship, evidence, dependency, consequence, and receipt semantics
are CRM workflow composition, not a reusable PDS primitive. This slice does not
use process-progress semantics, governed-write audit components, operation
state, or AI-evidence components.

## Proof

- Render the CRM route at `/activities` on desktop and mobile viewports.
- Cover ready, loading, empty, partial, stale, offline, unauthorized, error,
  and recovery states; retained inspectable content for partial/stale states;
  visible `Partial` and `Stale` text states; ordinary list/button keyboard
  behavior; post-transition focus; per-activity receipt isolation;
  reduced-motion-safe rendering; no fetch/XHR or non-GET interaction requests;
  light/dark adaptive contrast; and no serious or critical accessibility
  findings.
- Prove exactly one decision-path region and the ordered labels Relationship,
  Evidence, Dependency, Consequence, and Receipt. Detached decision-lens,
  readiness-card, and readiness-count summaries must be absent.
- Test 320, 390, 834, and 1440 px viewports with the same DOM and ordered
  content. The local experience must add no document-level horizontal
  overflow.

## Non-claims

This is not Nexus activation, a ServiceNow integration, a live action,
provider certification, user-study result, production release, mobile-web
shell certification, reusable PDS component graduation, release-candidate
evidence, or claim of signature-quality acceptance. It remains a
fixture-backed, non-Nexus, non-production local prototype. The current
mobile-shell gap is a separate follow-on and is not resolved or certified by
this slice.
