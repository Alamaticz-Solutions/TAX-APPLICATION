# ADR 0017: Web/Native Experience Projection Boundary

## Status

Accepted on 2026-08-03 for App Framework Prototype implementation under the
[App Fabric Mobile UX Master Plan](../../specs/app-fabric-mobile-ux-master-plan.md).
Implementation maturity remains `experimental`, `candidate_ready: false`,
`release_ready: false`, and `release_authority: none`.

This ADR clarifies ADR 0015 and ADR 0016. It does not ratify React Native,
Expo, PDS Native, or any mobile packaging and distribution path as an
enterprise standard. It does not admit work, authorize a source lane, qualify
a renderer, or approve candidate or production use.

## Context

[ADR 0015](0015-pds-experience-system-dimensions.md) established one PDS
Experience System, two appearance axes, adaptive platform scale, window-size
classes, and channel-native iOS and Android expression. Its requirement for
identical APIs, DOM meaning, and keyboard behavior is correct within the Web
renderer across Web appearance and adaptation combinations. It cannot mean
that a DOM component tree, browser navigation model, or Web interaction API is
also the native implementation.

[ADR 0016](0016-pds-web-interaction-substrate.md) selected React Aria
Components behind PDS-owned Web APIs. It explicitly kept native iOS and Android
on React Native and channel-native interaction implementations. React Aria is
therefore a `web-dom` implementation detail, not the cross-renderer contract.

Responsive Web and an installed native application can share product intent,
state, authorization, and continuity while requiring different layout,
navigation, lifecycle, accessibility, device, and distribution behavior. A
compact browser viewport proves responsive Web only. Treating it as native
evidence would hide the exact platform work the catalog and promotion model
must make visible.

## Decision

### Canonical Projection Schema

The PDS catalog and the versioned experience contract use this closed
renderer-projection vocabulary:

```text
renderer projection
|-- web-dom
|-- native-ios
`-- native-android

canonical size class
|-- compact
|-- medium
`-- expanded
```

`renderer projection x canonical size class` is the canonical rendering
schema. The following rules apply:

1. `renderer projection` identifies the implementation and semantic runtime,
   not an appearance theme. `web-dom` uses browser and DOM semantics;
   `native-ios` and `native-android` use their platform-native semantic and
   lifecycle environments.
2. A native projection already names its platform. There is no independent
   native-platform axis and therefore no invalid combination such as an iOS
   projection with an Android platform value.
3. A canonical size class describes available application-window composition,
   not a marketed device model and not a claim that layouts are shared. The
   breakpoint values and renderer mappings belong in versioned PDS design data;
   components must not create private size-class definitions.
4. Presets such as `iOS phone`, `iOS tablet`, `Android phone`, and
   `Android tablet` select a projection, size class, evidence target, and input
   environment. Presets are conveniences and evidence fixtures, not semantic
   dimensions.
5. Appearance and environment remain orthogonal. Visual grammar, color mode,
   applicable density, forced-colors or high-contrast posture, text scale,
   reduced motion or transparency, direction, locale, connectivity, policy or
   data state, and assistive-technology or input conditions do not create new
   renderer projections.
6. Not every projection and size-class pair must be implemented. Applicability
   and evidence status make intentional omissions explicit; the schema does not
   manufacture a full cross-product requirement.

Adding, renaming, or removing a canonical projection or size-class identifier
requires a versioned schema change and an ADR amendment. A renderer must not
silently coerce an unknown value to `web-dom`, `compact`, or another convenient
fallback.

### Shared Semantics, Separate Projections

Web and native consume one versioned, channel-neutral experience contract. The
shared contract owns:

- stable journey, task, view, action, field, state, checkpoint, and telemetry
  identifiers;
- purpose, content hierarchy, criticality, action consequences, and product
  terminology;
- typed reads and governed-action intent, validation, policy, tenant,
  provenance, audit, request, and correlation semantics;
- loading, empty, partial, stale, offline, denied, expired, conflict, success,
  and unexpected-error meaning;
- interruption, exact-resumption, deep-link, notification, and attention
  intent;
- data classification, persistence eligibility, redaction, screenshot,
  clipboard, sharing, notification, and cache constraints;
- accessibility name, role, state, value, order, announcement, text-scale,
  motion, contrast, and alternate-interaction intent; and
- API compatibility range, evidence hooks, and support diagnostics.

Each renderer owns how those semantics become an application. Renderer-local
decisions include component trees and public view APIs, layout and density,
navigation and history/back behavior, focus and gesture mechanics, native
controls and accessibility mappings, safe areas and keyboard avoidance,
lifecycle and restoration, device capabilities, local storage, and update and
distribution behavior.

Cross-renderer parity means equivalent semantic role, state, outcome,
authorization, policy effect, continuity, telemetry, and accessibility intent
using each renderer's native semantics. It does not mean pixel parity,
identical information density, identical component props, identical focus or
gesture mechanics, or a shared layout/navigation tree.

Within `web-dom`, ADR 0015's invariant remains unchanged: visual grammar,
color mode, scale, size adaptation, and density where applicable must not fork
the PDS Web component API, DOM meaning, keyboard behavior, authorization, or
business state. Across native projections, equivalent semantics are required,
but channel-appropriate implementations and platform variants are expected.

Consequently, breakpoints and container queries can turn an expanded Web
experience into a high-quality compact Web experience when the task remains
browser-appropriate. They cannot by themselves produce native navigation,
lifecycle, secure storage, OS accessibility, permissions, hardware behavior,
signed distribution, or native evidence.

### One Manifest, Actual Renderers

PDS uses one framework-owned catalog manifest for shared identity,
documentation, ownership, lifecycle, applicability, readiness, and evidence
links. Until package extraction changes the location, the existing canonical
manifest under `appfw_ui/pds_health/reference/catalog.json` remains the source;
a native catalog must not create a competing semantic manifest.

That manifest feeds two executable catalog applications:

- the actual React DOM Web Catalog, importing the supported PDS Web source and
  running in the declared browser and accessibility environment; and
- the actual React Native Catalog, importing the supported PDS Native source
  and executing separately in iOS and Android native runtimes.

Catalog entries cover semantic tokens, components, patterns, floorplans, and
journey recipes. They may offer side-by-side semantic comparison, but each
render is produced by its declared projection. Initially, only the selected
reference floorplan family is required across its applicable projections.
Broad component or floorplan multiplication is not evidence of readiness.

The Web portal may display retained native images, recordings, results, and
links for review. Those displays are evidence mirrors. A phone-shaped DOM
frame, a compact-browser screenshot, React Native rendered through a Web
target, a component snapshot, or an unbound image is not iOS or Android native
proof. Evidence from one native platform does not qualify the other.

Every renderer-evidence record must identify at least:

- catalog entry and stable semantic identifiers;
- renderer projection, canonical size class, preset or device target, OS and
  runtime environment;
- source SHA, catalog-manifest hash, experience-contract version, PDS package
  and token versions, and native app build/runtime identifier where applicable;
- states, appearance and accessibility environments exercised;
- test or capture kind, result, retained artifact location, timestamp,
  freshness or expiry, and evidence owner; and
- known platform difference, fallback, limitation, and compatibility range.

### Applicability, Maturity, Lifecycle, And Readiness

The catalog keeps four concepts independent.

| Concept | Values | Meaning |
| --- | --- | --- |
| Projection applicability | `supported`, `adapted`, `product-specific`, `not-applicable` | Whether and how the semantic entry belongs in a projection |
| Family maturity | `foundation`, `enterprise-ready`, `release-gated` | Existing aggregate PDS family evidence posture |
| Component lifecycle | `experimental`, `beta`, `stable`, `deprecated` | Existing versioned API stability posture |
| Per-projection readiness | `prototype`, `candidate`, `qualified`, `not-qualified`, `not-applicable` | Source-bound implementation and evidence posture for one projection |

Applicability values have these meanings:

- `supported`: PDS supplies the projection implementation within the declared
  contract.
- `adapted`: PDS supplies an intentionally different, renderer-appropriate
  composition that preserves the shared semantics.
- `product-specific`: PDS defines the shared contract or lower-level building
  blocks, while a product owns the projection composition and its evidence.
- `not-applicable`: the semantic entry intentionally does not belong in that
  projection.

Readiness values have these meanings:

- `prototype`: an actual renderer implementation has bounded exploratory
  evidence, with no candidate or release implication.
- `candidate`: the projection has current, source-bound candidate evidence for
  a declared scope, but has not completed qualification.
- `qualified`: the applicable authority and evidence contract have accepted
  the projection for the stated scope and compatibility range. It is not a
  blanket product-release claim.
- `not-qualified`: the projection is applicable but implementation or required
  evidence is absent, failed, stale, placeholder-only, or outside its declared
  compatibility range.
- `not-applicable`: mirrors an applicability decision and carries no readiness
  claim.

The status invariants are:

1. `not-applicable` applicability requires `not-applicable` readiness. Every
   other applicability value requires a non-`not-applicable` readiness value.
2. Missing implementation or evidence is `not-qualified`, never
   `not-applicable`.
3. Applicability does not imply readiness; lifecycle does not imply renderer
   readiness; family maturity does not qualify every projection.
4. Readiness never inherits from another projection, size class, package,
   product, simulator, browser, or older source SHA.
5. Stale, incompatible, or superseded evidence demotes the affected claim to
   `not-qualified` until current evidence is retained.
6. A cross-renderer claim must enumerate its applicable projections and their
   evidence. It fails closed when any required projection is not qualified for
   the claimed stage.

The existing PDS SemVer, CHANGELOG, component lifecycle, deprecation, family
maturity, and release-certification contracts remain in force. In particular,
`release-gated` describes a family evidence posture; it does not mean that a
native projection or product release is ready.

### Canonical Source And Generation Boundary

The common waist is a normalized intermediate representation derived from
canonical sources and consumed by independent Web and native emitters.

Canonical sources are:

- `.appfw/model` and validated product-owned experience inputs for product
  semantics, state, actions, policy, data classification, and journey intent;
- `.appfw/manifest.yaml` for opt-in product topology and capability declaration;
- the actual API schema and operation contract for variables, identifiers,
  pagination, responses, and errors;
- PDS canonical semantic token data and the shared catalog manifest; and
- versioned compatibility and evidence schemas.

Framework-generated, overwrite-safe outputs may include the validated
experience and mobile capability contract, typed operation SDK and error
models, route and action registries, PDS Native token adapter, safe recipe or
screen shells, ownership metadata, and drift, compatibility, and evidence
manifests.

Product-owned, preserved source includes journey composition, product-specific
navigation and information architecture, copy, composites, device
integrations, offline and conflict policy within governed constraints,
notification and deep-link handlers, store content, and product tests.

Generation must preserve these rules:

- Web and native emitters consume the normalized canonical contract, not each
  other's output.
- Generated Web TypeScript, generated mobile files, rendered catalogs,
  screenshots, and previous build artifacts are never canonical generator
  inputs.
- Framework generation contains no product-specific or CRM fallback and does
  not invent API shapes.
- Generated output is edited through its canonical source, generator, or
  template. Human-owned workflow source is never overwritten.
- Ownership and source hashes travel with generated artifacts so drift and
  compatibility checks can fail closed.

The logical package boundaries `@pds/design-tokens`, `@pds/web`,
`@pds/native`, `@pds/experience-contracts`, and `@pds/catalog` remain target
architecture. This ADR requires their source and import boundaries, not
immediate publication as five packages.

### Compatibility And Versioning

Compatibility is explicit across four independently moving surfaces:

1. experience-contract and catalog-manifest schema;
2. PDS semantic-token, Web, and native package APIs;
3. Web deployment or native binary/runtime/update identity; and
4. backend API and persisted-data compatibility range.

Every build and evidence record binds the exact compatible tuple rather than
claiming compatibility from a branch name or latest version. Web and native
package versions may advance independently, but a parity claim names the
experience-contract version and semantic identifiers both consume.

The existing PDS package SemVer policy applies to renderer packages and shared
contracts:

- removing or changing a stable semantic identifier, state meaning, action
  consequence, accessibility contract, token meaning, or required field is a
  breaking major change;
- an additive optional semantic, component, projection implementation, or
  backward-compatible field is a minor change, provided older consumers either
  ignore it safely or reject it explicitly;
- a behavior-preserving correction is a patch; and
- deprecated stable contracts declare a replacement and removal version,
  remain functional for at least one minor release, and are removed only in a
  major release.

Changing size-class names or meaning is breaking. Tuning versioned breakpoint
values is at least a documented minor design-data change and requires affected
layout, accessibility, and journey evidence; if it invalidates a supported
consumer contract, it is breaking. Adding a projection does not qualify it or
change existing projection evidence.

Renderers negotiate or validate supported schema, package, API, binary/runtime,
and persisted-data ranges at build and startup boundaries. An unsupported
combination fails explicitly or uses a documented product fallback carrying
its own applicability and evidence; it must never masquerade as the requested
projection. Stable checkpoint and resumption identifiers require a migration
path across every supported compatibility window.

### Prototype And Candidate Nonclaims

This decision is an architecture seam for accelerated, pre-candidate work. It
creates vocabulary and boundaries, not implementation or promotion evidence.

At the current stage:

- React Native plus Expo remains the leading bounded reference hypothesis, not
  a ratified enterprise mobile standard;
- a native entry may reach `prototype` only after actual iOS or Android
  renderer evidence exists; otherwise it remains `not-qualified`;
- no entry may claim `candidate` or `qualified` from this ADR, package presence,
  a Web breakpoint, generated route count, local snapshot, or one-platform
  proof;
- existing Web family maturity, component stability, or release certification
  does not transfer to native;
- no store, MDM, signing, production OTA, live identity, PHI or regulated
  offline-data, SRA, CAB, risk-acceptance, or release authority is granted; and
- candidate promotion still requires the source-bound mobile candidate gate,
  applicable authority decisions, zero unresolved candidate blockers, current
  independent review, and explicit human approval for the assembled SHA.

## Consequences

- Products can share journey meaning, policy, continuity, tokens, telemetry,
  and compatibility without forcing Web layout or mechanics onto native.
- The Design System makes Web versus native applicability, intentional
  differences, and evidence gaps visible at component, pattern, floorplan, and
  journey levels.
- Native support costs more than responsive Web because it requires separate
  implementations, runtime evidence, accessibility proof, lifecycle handling,
  and compatibility management.
- The shared manifest and normalized contract reduce semantic drift, while
  independent renderers preserve platform conventions and performance.
- Aggregate claims become more conservative: compact Web, iOS, and Android
  evidence remain distinct and stale or missing proof fails closed.

## Alternatives Considered

- **Treat Web/mobile as a theme or breakpoint. Rejected.** It confuses
  appearance with runtime behavior and cannot prove native lifecycle,
  accessibility, device, security, update, or distribution requirements.
- **Use one component and layout tree for every renderer. Rejected.** It makes
  the shared contract depend on DOM or lowest-common-denominator mechanics and
  erases platform conventions.
- **Add separate renderer and native-platform axes. Rejected.** It creates
  impossible combinations and duplicates information already carried by
  `native-ios` and `native-android`.
- **Create unrelated Web and native catalogs. Rejected.** It permits semantic,
  lifecycle, and evidence drift and prevents one parity ledger.
- **Require every entry on every projection immediately. Rejected.** Channel
  applicability is product- and task-dependent; explicit `not-applicable` and
  `not-qualified` states are more truthful than mechanical projection.

## References

- [App Fabric Mobile UX Master Plan](../../specs/app-fabric-mobile-ux-master-plan.md)
- [PDS UX Design Strategy](../../frontend/ux-design-strategy.md)
- [PDS Experience System Architecture And Coverage](../../specs/pds-experience-system-architecture-and-coverage.md)
- [PDS Experience Dimensions And Catalog Plan](../../specs/pds-experience-dimensions-and-catalog-plan.md)
- [PDS Health Design System](../../frontend/pds-health-design-system.md)
- [React Native Mobile App Contract](../../frontend/mobile-react-native.md)
- [Generated Ownership](../../start/generated-ownership.md)
