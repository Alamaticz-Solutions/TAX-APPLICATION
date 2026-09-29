# Eight-Vignette Intelligent Experience Shared Foundation R1

Status: normative contract implemented on current main; historical delivery record superseded

Spec depth: full

> **Convergence note (2026-08-14).** The eight-recipe, package, admission,
> readiness, nonclaim, and evidence contracts in this document remain normative
> inputs to IX Runtime Convergence R1. Its historical branch, base, lane,
> admission, WIP, push, and destination statements are superseded operational
> history and grant no current authority. The one-time local bootstrap is bound
> instead to packet
> `target/appfw/ix-real-vertical-local-bootstrap-20260814.json`, exact base
> `32160570b7d8d5b8e813e95da053918cba878af8`, the reviewed convergence plan
> at `07a364ee82fdba65d38d66bb3afe0038b305e9b8`, one active producer, and a
> stop before push or PR. This reconciliation creates no Product acceptance,
> package publication, native qualification, security approval, release,
> deployment, or production-readiness claim.

For a human-readable explanation of the interaction patterns, authorization
boundary, context flow, model invocation points, and recommended model types,
see [Intelligent Experience Patterns And Model Orchestration](../architecture/intelligent-experience-patterns-and-model-orchestration.md).
This specification remains the normative contract.

## Historical Delivery Record (Superseded)

The following owner, branch, baseline, Product Increment, and Delivery Lane
entries are retained only as provenance for the completed implementation. They
do not describe current work, reopen a lane, or grant current source, review,
push, merge, release, Product, architecture, security, or risk authority.

Owner roles:

- Product Owner / outcome sponsor: Wayne Kempf
- Architect: App Framework Architect assigned to the IX shared-foundation lane
- Program Flow Controller: current Nexus master-plan coordination task
- Implementation owner: App Framework Architect + Framework Coding Agent on
  `feature/ix-eight-vignette-contract-r1`
- Integration owner: assigned by the governing Product Increment plan
- Review owner: independent comprehensive Framework PR Review Agent

Accepted source baseline:
`origin/integrate/nexus-product-tooling-r1@9ea26157a2bed29424e1052a909303985ba466bd`

Product Increment registration:

- Product Increment: `PDS-PI-IX8-SHARED-FOUNDATION-01`
- Delivery Lane: `PDS-IX8-L1-SHARED-FOUNDATION`
- Flow state: `in_progress`; manually admitted by Program Flow against planning
  commit `2100e93ef73563261582c92527ed35afbf1307a9`
- Validated plan amendment
  `cf6dc14f9061aa46c6b4e2f4985ac58edae60940` adds only the two detached and
  clean-checkout package proof scripts required by the accepted package-version
  change
- Latest validated plan amendment:
  `bef9e41effff85f0e131fd6135ee0de6e04aeb27` adds only the Web
  `ProgressiveResponse` source and focused contract test needed to preserve
  editable-region callback semantics, plus the native package self-check needed
  for the accepted package-version change
- Validated plan amendment
  `1548e52fe5d139f9b40334a17e78c1651b39bef0` adds only the PDS Health
  changelog required by the Web package's accepted SemVer change
- Latest validated plan amendment
  `46e5ca7c86ef64892fff40e22ece06675846ef1f` adds only the typed PDS
  catalog version constant required to keep the public Web package's accepted
  SemVer change aligned with its machine-checked catalog contract
- Validated plan amendment
  `c133aab7d7631de97b1b5686319c1f1ebf998f24` adds only the static PDS
  catalog source map and component-checker roots required to register the public
  recipe composition honestly in existing catalog governance
- Latest validated plan amendment
  `f0cb52b29777f29b7767b4b6636e80211ddeb9ab` adds only the existing PDS
  interactive example and copy-ready snippet roots required by that same public
  component registration
- Latest validated mechanical plan correction
  `326105a67542f86f5f5163fa0f815eebfb3afc9c` replaces the mistaken
  `appfw_ui/pds_health/scripts/check-pds-components.mjs` root with the actual
  authorized repository checker path `scripts/check-pds-components.mjs`.
  Scope, WIP, and lane status are unchanged; the immutable plan file SHA-256
  is `bb1af6ac33692e6b06ae1c964cd9a3fdf3079f57fbe25ef699d5f1485c08d848`
- Latest validated plan amendment
  `11c91af5168f5a43fd70f49877ef9bd5dd25a835` adds only
  `appfw_ui/pds_health/native-components/src/ix-recipe-projection.ts`, keeping
  the accepted native `ix-recipes` rendering API intact while giving detached
  non-rendering proof an executable React-Native-free resolver entry. The
  validator and portfolio are green; the amended plan file SHA-256 is
  `e6ce2fa501de296ce3265b1be431033ab87b116b48931a64dffacecd1652ca7b`.
- Destination: `integrate/ix-eight-vignette-foundation-r1`
- Repository: App Framework only

## Business Value

Nexus and future App Fabric applications need to express the same eight
Intelligent Experience (IX) patterns without copying a catalog demo, inventing
different intent names by channel, or letting a product-selected renderer evade
the framework admission boundary. The shared foundation makes each experience
discoverable and consumable as one versioned recipe identity across Web,
native iOS, native Android, and the App Framework lifecycle.

The value of this increment is not eight new screens. It is one trustworthy
contract that lets products compose channel-appropriate experiences while the
framework can prove which intent, presentation schema, renderer recipe, and
required capability set it admitted. Product applications retain ownership of
domain data, language, policy, actions, and journey composition.

## Problem

The current repository has three strong but disconnected assets:

1. The PDS catalog contains all eight interactive reference vignettes, but its
   `pds.ix.reference_playback@1` types and six purpose-shaped fixture payloads
   are intentionally catalog-private.
2. `@appfw/pds-ix-presentation-contract` and the PDS Web/native packages expose
   a shared `pds.ix.presentation@1` envelope and generic presentation anatomy,
   but no public eight-recipe identity or capability contract.
3. `appfw-runtime` validates registered artifact type, presentation schema,
   and renderer tuples, but a caller can currently construct an arbitrary
   `IxRunPolicy` registry. The runtime does not bind a canonical recipe intent
   and its required capability declaration to that tuple.

Consequently, an 8/8 gallery result proves reference behavior only. It does
not prove public package convergence, Web/native recipe parity, or
recipe-bound lifecycle admission.

## Goals

- Publish one React-free, DOM-free, product-free, versioned and closed registry
  of exactly eight IX recipe identities.
- Publish the closed capability vocabulary and exact required capability set
  for every recipe.
- Let a product register its own artifact type while requiring exact agreement
  with the canonical intent, presentation schema, renderer, and capabilities.
- Expose public Web and React Native recipe-resolution and generic presentation
  APIs for all eight identities without promoting catalog fixture payloads or
  eight monolithic components.
- Make App Framework lifecycle admission validate and retain the same complete
  recipe registration before it admits a recipe-bound run.
- Prove exact 8/8 registry parity, Web/native projection parity, package-only
  consumption, PDS-to-runtime projection integrity, and fail-closed runtime
  behavior.
- Preserve honest prototype and native `not-qualified` readiness.

## Non-Goals

- Moving `IxReferenceRecipeArtifact`, recovery-readiness fixtures, timed
  playback, or product copy out of the PDS catalog.
- Standardizing product artifact types, domain schemas, focus kinds, prompts,
  orchestration, calculations, thresholds, actions, permissions, or routes.
- Providing eight large public React or React Native components.
- Claiming that a capability declaration proves its behavior executed or met
  Product acceptance.
- Adding HTTP, SSE, GraphQL, MCP, Kafka, provider, prompt, model, durable task,
  persistence, background worker, cross-device continuation, or product route
  wiring.
- Claiming live Okta, ServiceNow, Workday, Kafka, Grafana, provider, device,
  security, release, SRA, or CAB readiness.
- Qualifying iOS or Android from component or React Native test evidence.
- Editing Nexus product source, generators/templates, central roadmap or
  portfolio files, CI/release controls, provider/chat/ingress source, or root
  package/release manifests in this Delivery Lane.

### Synthetic Reference Exception

The catalog preserves these eight named, sanitized reference recipes so their
interaction behavior stays inspectable and testable:

- Analyze Why
- Contextual Conversation
- Adaptive Composition
- Working Goal Plan
- Adaptive Information Lens
- Situation to Strategy
- Attention Stewardship
- Agents Helping You

Their recovery-readiness names, values, and timed transitions are fixture-only.
They are not reusable product copy, production data, domain defaults, or proof
of a live provider. The reference checker validates deterministic catalog
playback and browser behavior only. It does not validate, simulate, or claim an
application intelligence runtime, lifecycle authority, or provider integration.

## Scope

The source lane is limited to:

- this specification;
- `appfw_ui/pds_health/ix-presentation-contract/**`;
- focused public Web recipe adapter, exports, package metadata, documentation,
  and contract tests in `appfw_ui/pds_health/components/**`;
- focused public native recipe adapter, exports, package metadata,
  documentation, detached-consumer proof, and tests in
  `appfw_ui/pds_health/native-components/**`;
- PDS catalog registry/projection parity only when explicitly admitted by the
  Product Increment lane roots;
- `appfw_runtime/src/ix/**` recipe registration and lifecycle admission;
- the generated PDS registry projection and provenance under
  `appfw_runtime/contracts/pds_health/**`; and
- focused PDS/runtime projection checkers and tests.

The final machine-validated Product Increment plan is authoritative if it
narrows these roots. Any new root requires Program Flow and Integration
rerouting; it is not silently absorbed by this branch.

## Repository Context

This document is the current self-contained canonical source for the shared
IX/PDS obligations. It requires, and no implementation may weaken:

- the eight stable display names below and the catalog-private synthetic
  reference exception;
- one Framework-owned, verified-identity lifecycle authority with exact
  artifact revisions, durable owner-bound replay, deterministic cancellation,
  bounded retained context, and `pds.ix.presentation@1` as the presentation
  authority boundary;
- atomic initial persistence, a versioned validated reconstruction record, and
  stable at-least-once audit projection identity without an exactly-once or
  hosted-durability claim;
- one channel-neutral announcement per presentation revision and exactly one
  emission owner in each renderer, while standalone context and work-status
  primitives remain independently announceable;
- unique progressive-response region identifiers and changed-region references
  that resolve within the exact artifact revision;
- exact recipe registration, capability, renderer, audience, tenant, and human
  authorization bindings before Product preparation or lifecycle mutation;
- an IX-only POST/SSE boundary with strict signed-JWT admission, canonical
  resume cursors, replay without re-execution, and cancellation that preserves
  the first accepted command result across retries and restarts;
- [App Fabric Mobile UX master plan](app-fabric-mobile-ux-master-plan.md),
  especially semantic rather than pixel parity, separate channel renderers,
  and explicit `not-qualified` native evidence;
- `@appfw/pds-ix-presentation-contract@0.2.0`, the current React-free
  presentation schema and validator;
- `@appfw/pds-health-components@0.12.0`, whose IX APIs are available only from
  the explicit `intelligence-presentation`, `intelligence-presentation-model`,
  and `ix-recipes` subpaths and require the optional contract peer when used;
- `@appfw/pds-health-native@0.2.0`, the current React Native presentation
  renderer and package-only proof; and
- `appfw-runtime@0.2.0`, whose existing `IxRunPolicy::new` path must remain
  source-compatible during this additive prototype increment.

The eight vignettes are reference recipes rather than eight monolithic
component APIs. This specification defines the public identity/admission layer;
it does not promote the private fixture models.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| PDS channel-neutral package | Add registry, registration, capability, lookup, validation, assertion, and projection APIs | JSON Schemas, canonical registry document, declaration file, package tests, runtime projection |
| PDS Web package | Add an `ix-recipes` subpath, recipe resolver, and generic safe renderer | PDS registration validator, `pds.ix.presentation@1`, Web package/archive tests |
| PDS native package | Add an `ix-recipes` subpath, native projection resolver, and generic safe renderer | Same registry, native renderer, iOS/Android parity tests, detached consumer |
| PDS catalog | Consume or compare only canonical IDs/capabilities/projection readiness | Existing private fixture types and eight interactive recipes remain private and unchanged in authority |
| App Framework runtime | Add public recipe registration and additive recipe-bound run-policy construction/admission | PDS registry projection, request intent, artifact tuple, snapshot/replay validation |
| Package identities | Advance additive candidate package versions when implementation lands | package manifests, package locks, peer ranges, detached-consumer exact versions |

## Closed Canonical Registry

The PDS package owns a canonical JSON document with schema identity
`pds.ix.recipe_registry@1`. The document is closed at every object boundary,
contains exactly eight entries in ordinal order, and identifies
`pds.ix.presentation@1` as the only content presentation schema in R1.

The stable recipe table is:

| Ordinal | Recipe ID | Display name | Intent key | Renderer key | Distinguishing required capability |
| ---: | --- | --- | --- | --- | --- |
| 1 | `analyze-why` | Analyze Why | `pds.ix.intent.analyze-why@1` | `pds.ix.recipe.analyze-why@1` | `pds.ix.capability.causal-explanation@1` |
| 2 | `contextual-conversation` | Contextual Conversation | `pds.ix.intent.contextual-conversation@1` | `pds.ix.recipe.contextual-conversation@1` | `pds.ix.capability.contextual-follow-up@1` |
| 3 | `adaptive-composition` | Adaptive Composition | `pds.ix.intent.adaptive-composition@1` | `pds.ix.recipe.adaptive-composition@1` | `pds.ix.capability.composition-change-explanation@1` |
| 4 | `working-goal-plan` | Working Goal Plan | `pds.ix.intent.working-goal-plan@1` | `pds.ix.recipe.working-goal-plan@1` | `pds.ix.capability.goal-plan-revision@1` |
| 5 | `adaptive-information-lens` | Adaptive Information Lens | `pds.ix.intent.adaptive-information-lens@1` | `pds.ix.recipe.adaptive-information-lens@1` | `pds.ix.capability.full-record-fallback@1` |
| 6 | `situation-to-strategy` | Situation to Strategy | `pds.ix.intent.situation-to-strategy@1` | `pds.ix.recipe.situation-to-strategy@1` | `pds.ix.capability.strategy-challenge@1` |
| 7 | `attention-stewardship` | Attention Stewardship | `pds.ix.intent.attention-stewardship@1` | `pds.ix.recipe.attention-stewardship@1` | `pds.ix.capability.attention-correction@1` |
| 8 | `ambient-agent-continuity` | Agents Helping You | `pds.ix.intent.ambient-agent-continuity@1` | `pds.ix.recipe.ambient-agent-continuity@1` | `pds.ix.capability.ambient-work-continuity@1` |

Every recipe requires these four cross-cutting capabilities, in this canonical
order, followed by its distinguishing capability:

1. `pds.ix.capability.focus-context@1`
2. `pds.ix.capability.progressive-presentation@1`
3. `pds.ix.capability.evidence-disclosure@1`
4. `pds.ix.capability.human-control@1`
5. the exact distinguishing capability from the table

The capability list is a required compatibility declaration. It states the
behavior a product/channel implementation owes. It is not runtime telemetry,
an authorization grant, device evidence, or proof that the behavior occurred.

Every descriptor has one closed projection record:

```json
{
  "web-dom": { "applicability": "supported", "readiness": "prototype" },
  "native-ios": { "applicability": "supported", "readiness": "not-qualified" },
  "native-android": { "applicability": "supported", "readiness": "not-qualified" }
}
```

`supported` means the public projection API accepts the recipe. It does not
upgrade the readiness field. Native readiness remains `not-qualified` until
separate exact-platform runtime, accessibility, visual, compatibility, and
device evidence is retained under the mobile qualification contract.

## Product-Owned Registration Contract

A product supplies a closed `pds.ix.recipe_registration@1` object:

```json
{
  "schemaVersion": "pds.ix.recipe_registration@1",
  "recipeId": "analyze-why",
  "intentKey": "pds.ix.intent.analyze-why@1",
  "artifactType": "product.owned.artifact_type@1",
  "contentSchemaVersion": "pds.ix.presentation@1",
  "rendererKey": "pds.ix.recipe.analyze-why@1",
  "requiredCapabilities": [
    "pds.ix.capability.focus-context@1",
    "pds.ix.capability.progressive-presentation@1",
    "pds.ix.capability.evidence-disclosure@1",
    "pds.ix.capability.human-control@1",
    "pds.ix.capability.causal-explanation@1"
  ]
}
```

Only `artifactType` is product-defined. It is a bounded stable key, not an
artifact payload or schema authority. Every other field must equal the selected
canonical descriptor exactly, including capability order. Missing, extra,
duplicated, reordered, or unknown capabilities fail. Unknown fields fail.

The channel-neutral JavaScript API is:

```text
pdsIxRecipeRegistrySchema
pdsIxRecipeRegistrationSchema
pdsIxRecipeIds
pdsIxRecipeCapabilities
pdsIxRecipeRegistry
getPdsIxRecipe(recipeId)
validatePdsIxRecipeRegistration(value)
assertPdsIxRecipeRegistration(value)
projectPdsIxRecipeRegistration(value, projection)
```

The projection argument is closed to `web-dom`, `native-ios`, and
`native-android`. Projection returns the validated registration, canonical
descriptor identity, and truthful applicability/readiness record; it does not
inject layout, copy, data, callbacks, or authority.

The package adds JSON Schema and canonical registry export subpaths. Schema,
JavaScript validator, TypeScript declaration, canonical registry bytes, and
adversarial tests must agree. The implementation advances the candidate
package to `@appfw/pds-ix-presentation-contract@0.2.0`; this is package
identity bookkeeping, not publication or release authority.

## Web And Native Public APIs

### Web

`@appfw/pds-health-components/ix-recipes` exports:

```text
resolvePdsIxWebRecipe(registration)
PdsIxRecipePresentation
PdsIxRecipePresentationProps
```

The resolver validates a registration and projects it only to `web-dom`.
`PdsIxRecipePresentation` validates both the registration and the caller-owned
`pds.ix.presentation@1` envelope, then composes the existing context, work
status, progressive response, and evidence primitives. Invalid input produces
one deterministic, non-actionable accessible fallback and never calls product
callbacks. The component has no recipe-specific product copy or policy.

The candidate Web package advances to
`@appfw/pds-health-components@0.12.0` and preserves all existing exports.

### Native

`@appfw/pds-health-native/ix-recipes` exports:

```text
resolvePdsIxNativeRecipe(registration, projection)
PdsIxRecipePresentation
PdsIxRecipePresentationProps
```

The resolver accepts only `native-ios` or `native-android`; `web-dom` fails.
The generic component validates registration, then delegates caller-owned
presentation data to the existing native `PdsIxPresentation`. Invalid input
uses the existing deterministic non-actionable fallback. It does not share Web
layout or DOM implementation.

The candidate native package advances to
`@appfw/pds-health-native@0.2.0`. Both native projections remain
`not-qualified`.

`@appfw/pds-health-native/ix-recipe-projection` is the non-rendering executable
entry for admission tooling. It exports the same resolver and native projection
type without loading React Native; it does not replace or narrow the promised
`ix-recipes` rendering API.

Web and native tests cover all eight registrations. That is API and semantic
projection parity, not pixel parity or native device proof.

## App Framework Runtime Admission

The runtime adds a public serializable `IxRecipeRegistration` that matches the
PDS registration JSON exactly and validates against a byte-identical packaged
projection of the canonical PDS registry. The runtime does not import React,
TypeScript, PDS renderer code, catalog source, or product source.

The additive public policy path is:

```text
IxRecipeRegistration::new(...complete registration fields...)
IxRunPolicy::new_for_recipe(intent_label, runner, recipe_registration)
```

The existing `IxRunPolicy::new` constructor remains source-compatible in R1.
It is explicitly an unbound generic artifact-registry path and earns no claim
of eight-recipe conformance. It is not silently reinterpreted or removed from
the current `appfw-runtime@0.2.0` candidate.

Before audit append or run creation, recipe-bound start validates:

- registration schema and known recipe ID;
- exact canonical intent key;
- exact `pds.ix.presentation@1` content schema;
- exact canonical renderer key;
- exact ordered required capability set;
- nonempty bounded product-owned artifact type;
- request intent equality with the registration intent;
- exactly one registered artifact tuple derived from the registration; and
- related-artifact equality with the registered artifact type, presentation
  schema, and renderer when continuation is requested.

The exact registration is retained in the run snapshot. Snapshot projection
and replay validation revalidate it against the packaged canonical registry
and require the retained request intent and registered artifact tuple to match.
Artifact publication continues to enforce the existing exact identity,
revision, presentation, and renderer rules.

Unknown recipe, intent substitution, artifact substitution, schema
substitution, renderer substitution, missing/extra/duplicated/reordered
capability, unknown field, and related-artifact mismatch fail before any audit
or run-state mutation. Existing unbound policy behavior remains covered by
compatibility tests.

The runtime registry projection has a provenance record that binds the PDS
package name/version, canonical source path, projected path, source and
projection SHA-256, and reviewed/integrated source commits. The projection
checker requires byte identity and safe repository-relative paths. Commit
identities are populated only at the immutable checkpoint/integration step;
working-tree proof cannot manufacture them.

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Keep all eight identities catalog-private | No package/API work | Cannot support product/runtime convergence; repeats current gap | Rejected |
| Publish eight monolithic components and payload schemas | Easy product screenshots | Freezes demo layout and domain assumptions; fights native adaptation | Rejected |
| Publish only eight string IDs | Small | Does not bind intent, renderer, schema, capabilities, or projection readiness | Rejected |
| Create a separate recipe-contract package | Strong separation | Adds a fourth archive/peer dependency and splits two tightly coupled PDS IX contracts before a second independent consumer proves the need | Deferred |
| Extend the channel-neutral presentation-contract package and add separate channel adapters | Reuses the existing shared waist; one source for Web/native/runtime | Requires careful version/projection parity | Selected |
| Replace `IxRunPolicy::new` immediately | Strongest enforcement | Unproven downstream break against a public 0.2.0 candidate API | Rejected for R1; additive bound path selected |
| Hard-code eight tuples independently in Rust | Simple runtime lookup | Creates a second authority and hidden drift | Rejected; use packaged byte-identical PDS projection |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-12 | Wayne Kempf | The eight IX vignettes must converge into the Design System, be supported by App Framework, and be woven into Web and native UX. | Direct sponsor clarification after an audit showed an 8/8 gallery but incomplete shared/product adoption. | Sponsor changes the intended experience family. |
| 2026-08-12 | IX Architecture | Promote stable identities, required capabilities, and projection metadata; keep fixture payloads and domain language private/product-owned. | Preserves PDS/runtime authority boundaries and avoids eight demo-shaped APIs. | A second product proves a reusable higher-level behavior schema. |
| 2026-08-12 | IX Architecture | Use one exact product registration to bind recipe intent to product artifact type, PDS presentation schema, renderer, and capabilities. | Closes the arbitrary tuple gap without taking domain ownership. | Runtime needs multiple canonical recipes in one run; that requires a separately reviewed contract. |
| 2026-08-12 | IX Architecture / coordinator | Keep runtime construction additive instead of breaking `IxRunPolicy::new`. | Repository search finds only internal consumers, but absence of source is not downstream compatibility proof for the public 0.2.0 candidate. | A major-version or proven no-consumer migration authorizes removal. |
| 2026-08-12 | Program Flow | Do not mutate source until a machine-validated Product Increment lane admits exact roots. | Current PDS Experience portfolio entry is `needs_reconciliation` and explicitly requires a canonical plan before more source work. | Validator-green PI/lane assignment is handed to this branch owner. |
| 2026-08-12 | Program Flow | Admit `PDS-IX8-L1-SHARED-FOUNDATION` against planning commit `2100e93ef73563261582c92527ed35afbf1307a9`. | Portfolio and plan validators report zero errors and warnings; exact branch, base, destination, owner, budget, and write roots are frozen. | A root or contract expansion requires a new validated assignment before mutation. |
| 2026-08-13 | Wayne Kempf | Approve one lineage-only I2 checkpoint recording reviewed source `8e5b33119b84fa23eee1f1d3534481101afed2a9`, integrated assembly `1e919fafe783b68cef4abc65f99d585be482b4e7`, base `9ea26157a2bed29424e1052a909303985ba466bd`, and relation `history_preserving_cherry_pick_assembly`. | The approval permits the narrow provenance truth transition after independent source and I1 reviews; it expressly does not authorize merge, release, deployment, publication, Product acceptance, or accepted risk. | A fresh exact-I2 comprehensive review rejects the recorded identity or relation. |

## Security, Privacy, And Governance

- Recipe and capability metadata are presentation compatibility declarations,
  never permissions, roles, scopes, consent, authorization, or policy verdicts.
- The browser/native caller cannot select a provider, model, credential,
  authoritative context, egress destination, or action permission through this
  contract.
- `artifactType` is an opaque stable key. Registration contains no prompt,
  domain record, context body, artifact body, tenant data, PHI/PII, credential,
  provider body, or private reasoning.
- Existing identity-bound policy, context authority, egress, audit,
  cancellation, and replay controls remain authoritative.
- The contract and packages must remain free of Okta, ServiceNow, Workday,
  Kafka, Grafana, Nexus, recovery-readiness fixture, and provider-specific
  vocabulary.
- A declaration of `human-control` does not grant or prove a control. Products
  and the runtime must implement and test the applicable control separately.
- Local tests, SSR, RNTL, package archives, and simulator-shaped fixtures do
  not establish iOS/Android device, enterprise identity, production security,
  SRA/CAB, publish, or release readiness.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Canonical document contains exactly the eight ordered IDs, unique intents, unique renderers, exact capability sets, and closed projections | Contract package tests plus JSON Schema/JavaScript parity tests | review |
| Unknown fields, IDs, capabilities, duplicates, reordering, and tuple substitutions fail | Adversarial contract package tests | review |
| Contract implementation is React/DOM/provider/product/fixture free | Source and packed-archive boundary tests | review |
| All eight registrations resolve through public Web API | Web package contract/SSR tests | review |
| Web invalid registration/presentation renders a deterministic non-action fallback | Web component test | review |
| All eight registrations resolve for iOS and Android with exact parity and truthful `not-qualified` readiness | Native contract tests covering 16 projections | review |
| Native invalid registration uses a non-action fallback | React Native Testing Library test | review |
| Public contract, Web, and native packages install and typecheck without adjacent framework source | Offline detached multi-archive consumer proof | review |
| Catalog uses or checks the canonical eight identities without importing private playback types into public packages | Catalog parity checker and package-boundary tests, if catalog roots are admitted | integration |
| Runtime registry is byte-identical to PDS source and provenance-bound | PDS IX runtime projection checker and adversarial checker tests | review |
| Recipe-bound runtime admits every canonical recipe | Focused Rust table test covering 8/8 | review |
| Runtime rejects every intent/artifact/schema/renderer/capability/related-artifact mismatch before audit and state mutation | Focused Rust negative matrix with audit/run-count assertions | review |
| Recipe registration survives snapshot/replay verification and tampering fails closed | Focused Rust snapshot/replay tests | review |
| Existing `IxRunPolicy::new` behavior remains source-compatible | Existing IX suite plus additive compatibility test | review |
| Default and no-default runtime profiles pass | `cargo test -p appfw-runtime ix:: --lib` and `--no-default-features` | review |
| Changed runtime compiles warning-free | default and no-default `cargo clippy ... -D warnings` | review |
| Actual diff remains inside admitted Delivery Lane roots | Product Increment plan checker with `--current-diff <lane-id> --json` | checkpoint |
| Framework generated/docs/test state has no unexplained regression | risk-appropriate App Framework validate, docs-check, generate-check, test, and handoff artifacts | checkpoint |
| Independent reviewer finds no blocker/critical issue | comprehensive Framework PR Review artifact bound to exact local commit | local handoff |

## Test And Execution Feedback Plan

Use the narrowest falsifying loop first:

1. Contract package tests: canonical registry, schema/runtime parity, closed
   registration, exact 8/8 table, malformed inputs, and package contents.
2. Web package: build, public subpath, resolver table, SSR/generic fallback,
   package boundary, and archive plan.
3. Native package: build/typecheck, 16-case projection table, RNTL behavior,
   package boundary, and detached consumer.
4. Runtime: byte-identity projection checker and adversarial checker; focused
   default/no-default IX tests and clippy.
5. Catalog parity only if that root is admitted.
6. Delivery Lane current-diff validation, `git diff --check`, risk-appropriate
   framework proof, retained handoff, local commit, and independent
   comprehensive review.

If execution reveals that generic `pds.ix.presentation@1` cannot represent a
required recipe without product-private executable UI or browser-authoritative
data, stop. Update the accepted contract or create a later product-owned
composition contract; do not widen this registry into a universal UI schema.

If two correction cycles fail to produce new semantic proof, return to
Architecture/Program Flow for scope and contract review before a third cycle.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Registry becomes a second product-domain schema | Only stable identity, capability obligation, projection readiness, and PDS schema metadata are public | PDS / Architect | designed |
| Capability declaration is mistaken for executed behavior | Explicit declaration-only semantics and separate behavioral/product evidence | Product / reviewer | designed |
| Web implementation is reused as native | Separate adapters and renderer implementations; shared contract only | PDS Web/native owners | designed |
| `supported` is mistaken for native qualification | Mandatory per-projection readiness; native fixed to `not-qualified` in R1 | Mobile Architect / reviewer | designed |
| Runtime copies and drifts from PDS authority | Byte-identical packaged projection, provenance, and adversarial checker | App Framework | designed |
| Runtime change breaks public 0.2.0 consumers | Additive constructor; retain and test legacy constructor | App Framework | designed |
| Product-selected tuple bypasses recipe contract | Exact registration and request/related-artifact validation before audit/mutation | App Framework | designed |
| Package source works only inside monorepo | Archive verification and offline detached consumer | PDS package owners | designed |
| Lane starts outside Product Increment governance | Source mutation began only after green plan, durable planning commit, exact roots, and explicit Program Flow admission | Program Flow / branch owner | controlled |

## Tech Debt And Follow-Up

- Nexus Web and native product route adoption is a separate product Delivery
  Lane consuming the released/accepted package contract. This foundation does
  not claim that adoption.
- Native iOS/Android qualification remains a later Mobile/PDS evidence lane.
- Durable recipe-bound task persistence, provider adapters, route wiring, and
  cross-device continuity remain separate App Framework increments.
- The unbound `IxRunPolicy::new` path can be deprecated or removed only after
  downstream consumer evidence and an appropriate compatibility/version
  decision.
- A separate recipe-contract package should be reconsidered only when an
  independent non-presentation consumer demonstrates a real packaging need.

## Historical Handoff Notes (Superseded)

- Spec status is `accepted-for-implementation`; source execution was admitted
  for the exact machine-valid `PDS-IX8-L1-SHARED-FOUNDATION` lane at planning
  commit `2100e93ef73563261582c92527ed35afbf1307a9`.
- The gallery's `pds.ix.reference_playback@1` contract and purpose-shaped
  fixture union remain catalog-private.
- The stable public ambient ID is `ambient-agent-continuity`; the human-facing
  display name remains **Agents Helping You**.
- A locally green branch earns prototype implementation evidence only. Push,
  PR, merge, package publication, Product acceptance, device qualification,
  security approval, and release remain separate authorities.
- The human-approved lineage-only I2 transition records
  `lineage_status=reviewed_source_assembled`, reviewed source
  `8e5b33119b84fa23eee1f1d3534481101afed2a9`, integrated assembly
  `1e919fafe783b68cef4abc65f99d585be482b4e7`, integration base
  `9ea26157a2bed29424e1052a909303985ba466bd`, and relation
  `history_preserving_cherry_pick_assembly`. The reviewed source is
  intentionally not an ancestor of I1: Integration preserved its three source
  patches after the eleven planning commits, and the exact registry source and
  projection blobs remain identical at both checkpoints.
- This lineage truth records only reviewed-source-to-aggregate assembly. It
  does not claim PR, merge or `main` inclusion, package publication, Nexus
  Product adoption or acceptance, device qualification, live providers or
  data, security/SRA/CAB approval, deployment, release, production readiness,
  or accepted risk. I2 requires its own independent comprehensive review before
  a non-main push, and Product consumption may bind only the final reviewed,
  pushed, and live-verified I2 SHA.

## Historical Role Card Check (Superseded)

- **Card used:** Architect Agent and Coding Agent.
- **Within role:** durable technical contract, bounded source design, additive
  compatibility decision, implementation/proof plan, and later focused source
  changes within one assigned branch.
- **Not assumed:** Product Increment admission, WIP change, Integration
  ownership, independent review judgment, push, PR, merge, Product acceptance,
  package publication, risk acceptance, SRA/CAB, device qualification, or
  release authority.
- **Routed:** Product Increment/portfolio registration and exact lane roots to
  the goal-binding owner; later convergence to Integration; product adoption to
  Nexus Web/mobile owners; independent judgment to the Framework PR Review
  Agent.
- **Drift signal:** `none` — source work is active only inside the admitted
  lane and its exact write roots.
