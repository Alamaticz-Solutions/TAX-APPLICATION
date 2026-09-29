# PDS Web Interaction Substrate Decision Evidence

Date: 2026-07-17

Decision outcome: roadmap and architecture changed

## Research Trace

- Execution: Codex primary synthesis with official-source web discovery; no
  model-routed parallel retrieval worker was available in this continuation.
- Effort: high cross-source architectural synthesis; retrieval was not
  repeated after evidence normalization.
- Elapsed analysis window: approximately 25 minutes across repository review,
  official-source verification, contradiction analysis, and contract design.
- Repository sources examined: ADRs 0014/0015, UX design strategy, experience
  architecture/coverage, Nexus brief, roadmap, PDS component package and
  checker, catalog package, current primitives, and Connected Fabric contract.
- External sources examined: React Aria getting started, Base UI overview,
  Material UI overview and v9 notes, and the existing July 2026 design-system
  benchmark evidence in this repository.

## Signals And Contradictions

- React Aria explicitly positions itself as unstyled interaction and
  accessibility infrastructure for custom design systems. That aligns with
  PDS ownership of semantics and visual grammar.
- Base UI is a credible unstyled alternative, but operating two general
  substrates would increase test and upgrade cost without product value.
- Material UI is production-capable but its current visual implementation is
  Material Design 2, so adopting it wholesale would not solve the authentic
  PDS Material-like requirement.
- Full Spectrum, Fluent, or Material visual-system adoption would accelerate
  generic controls while importing another system's visual and API grammar.
- The existing PDS library has differentiated enterprise/intelligence
  semantics worth preserving, but local commodity mechanics are costly and
  uneven. Component count overstates product readiness.

## Human Correction

The Product Owner explicitly rejected building a general-purpose component
engine, ratified ADR 0015, directed preservation of Connected Fabric as it
currently behaves, and prioritized the PDS experience system and intelligent
application capabilities required for Nexus.

## Decision

Adopt React Aria Components behind PDS-owned web APIs, retain Base UI as a
single named fallback, keep specialized engines behind separate adapters, and
retain channel-native React Native implementations. The decision changes the
roadmap from broad local-control refinement to bounded Nexus-shaped deliverables that
migrate commodity mechanics while proving PDS-owned work, trust,
intelligence, notification, journey, and signature capabilities.
