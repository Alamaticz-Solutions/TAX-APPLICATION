# North-Star Wave 0 Contract Freeze

> **Status: active architecture concern.** This document records the shared
> contract seams that must stay stable while the North-Star execution plan fans
> out. The current priority and scorecard live in
> [Roadmap](../../release/roadmap.md); if this page and the Roadmap disagree on
> current state, update this page after reconciling the Roadmap.

Wave 0 exists so parallel agents can work safely. It freezes the shared seams
that later lanes touch: provider capabilities, provider families, app manifest
and config validation, generated UI/mobile contracts, CLI namespaces, PDS design
system evidence, and release-evidence categories.

## Operating Rule

Wave 0 edits are serial and owned by one framework steward. Reconnaissance,
read-only audits, and test runs may happen in parallel, but contract-writing
edits should not be split across lanes. Once a seam is frozen, later lanes may
extend implementation behind that seam, but should not rename or re-shape it
without returning to Wave 0.

## Frozen Seams

| Wave | Seam | Source Of Truth | Freeze Rule | Proof |
| --- | --- | --- | --- | --- |
| W0.1 | Direction | `docs/strategy/product-development-north-star.md`, `docs/release/roadmap.md` | The Roadmap is the current official implementation plan; the North Star explains why. | `scripts/appfw framework docs-check --changed-only --json` |
| W0.2 | Provider capability areas | `appfw_runtime/src/provider_contract_types.rs`, `appfw_runtime/src/provider_capabilities.rs`, `appfw_runtime/src/provider_certification.rs` | SaaS provider writes stay unsupported until G1 evidence exists; the opt-in write gates are explicit runtime areas, not prose-only promises. | `cargo test -p appfw-runtime provider_capabilities --lib` |
| W0.3 | Provider family set | `appfw_runtime/src/provider_keys.rs` | `DATABASE`, `GRAPH_READ`, and `EXTERNAL_API` must be disjoint and cover `FrameworkProvider::ALL`. | `cargo test -p appfw-runtime provider_keys --lib` |
| W0.4 | Product model and manifest | `app_gen/src/app_manifest.rs`, `app_gen/src/config_contract.rs`, `app_gen/src/validation.rs`, `docs/reference/app-manifest.md` | App topology remains manifest-owned; schemas, relationships, seed data, provider environments, and generated config contracts remain model/config-owned. | `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json` |
| W0.5 | Generated UI and mobile contract | `.appfw/model`, `.appfw/manifest.yaml`, ADR 0017, `docs/frontend/mobile-react-native.md` | The frozen target requires Web and mobile TypeScript to be sibling outputs from canonical model/normalized experience inputs; neither channel output may become the other's generator source. The current `scripts/appfw` mobile projector still parses generated Web TypeScript, so it is explicitly nonconforming Prototype debt for M1, not a canonical seam. React Native + Expo remains the reference native target and PWA the responsive-web fallback. | current drift only: `scripts/appfw product generate --target mobile-rn --check --json`; conformance requires the M1 source-owned normalized-contract test; `mobile-test` remains non-authoritative diagnostics |
| W0.6 | CLI namespace and future verbs | `scripts/appfw`, `appfw_cli`, `docs/reference/cli.md` | New product-facing commands belong under `scripts/appfw product`; framework stewardship commands belong under `scripts/appfw framework`; reserved future verbs must answer `--plan --json` until executable evidence exists. | `scripts/appfw framework cli-test --plan --json`; `scripts/appfw framework docs-check --json` |
| W0.7 | PDS design system and evidence registry | `appfw_ui/pds_health`, `docs/frontend/pds-health-design-system.md`, `scripts/check-pds-components.mjs`, `scripts/ci/release-evidence-check.sh` | PDS components/tokens are consumed from source; release-evidence categories are extended in place, not invented as parallel registries. | `scripts/check-pds-components.mjs --json`; `APPFW_RELEASE_ARTIFACT_DIR=/private/tmp/appfw-wave0-release-evidence scripts/ci/release-evidence-check.sh --local-fixture` |
| W0.8 | Needs-spec lanes | [Wave 0 Spec Artifact Contracts](north-star-wave-0-specs.md) | A lane is not implementation-ready until a spec names its schema/artifact and a docs-check or release-evidence assertion can fail when it is missing. | `scripts/appfw framework docs-check --json` |

## SaaS Governed-Write Capability Shape

`SaasReadArea` is read-first by history, but it now explicitly includes the
future governed-write gates required by Archetype 2:

```text
GovernedWriteEnforcement
DelegatedActorContext
TokenStoreIsolation
NamedMutationRegistry
MutationRequestBinding
IdempotencyAndReplayProtection
WritePolicyAndScopeEnforcement
WriteAuditAndEvidence
```

These areas are deliberately `Unsupported` until G1 proves them for a concrete
provider and operation. That means a provider can be read-capable without
implying safe write-back, and a future write-capable provider must prove every
gate independently.

## CLI Evidence Slots

The following slots are owned by the Roadmap. Most now have first executable
report slices while preserving `--plan --json` for metadata compatibility. Do
not reuse these names for unrelated behavior:

| Lane | Reserved Slot | Purpose |
| --- | --- | --- |
| D6 | `scripts/appfw framework docs-check --changed-only --plan --json` plus phase-budget subchecks | Keep developer-loop timing measurable without hiding full-mode quality gates. |
| U2 | `scripts/appfw product harness-check --json` | Validate least-privilege product-agent profiles. |
| U4 | `scripts/appfw framework provider-graduation --json` | Report and certify connector capability graduation evidence. |
| U5 | `scripts/appfw product generate --target mobile-rn --json`; `scripts/appfw product mobile-test --json` | Generate the React Native + Expo mobile target and retain non-authoritative Prototype diagnostics; candidate posture requires the future source-bound mobile candidate checker. |
| U6 | `scripts/appfw framework fork-check --json` | Detect product forks of PDS components/tokens and generated contracts. |
| U7 | `scripts/appfw framework composition-check --json` | Validate manifest-driven runtime/build composition. |
| P6 | `scripts/appfw product compat-verify --json` | Verify product compatibility with packaged framework artifacts. |
| G4 | `scripts/appfw framework governance-check --json` | Validate PHI governance, provenance, and signing evidence decisions. |

## Evidence Registry Freeze

W0.7 freezes the existing evidence registries so later lanes extend the same
release surface:

- `scripts/ci/release-evidence-check.sh` remains the release evidence registry.
- `scripts/ci/security-assurance-decision.sh` remains the DAST, SAST, ASVS,
  release-provenance, and artifact-signing decision model.
- `scripts/check-pds-components.mjs` plus
  `appfw_ui/pds_health/reference/catalog.json` remain the PDS component evidence
  registry.
- `scripts/check-pds-tokens.mjs` plus `appfw_ui/pds_health/tokens` remain the
  PDS token evidence registry.

New G2/G4/U5/U6/U7 evidence should extend these registries in place rather than
creating a parallel checker.

## Needs-Spec Queue

These specs unblock Wave 1/2 work. Each spec should be short, artifact-shaped,
and backed by a failing assertion before implementation claims are made. The
full contract is [Wave 0 Spec Artifact Contracts](north-star-wave-0-specs.md).

| Lane | Spec Needed | Minimum Artifact Contract |
| --- | --- | --- |
| G1 | Delegated/on-behalf-of auth subsystem | Token-store identity key, tenant partition, operation actor context, named mutation registry, policy input, audit event, revocation behavior. |
| G2 | Conversational/action UI primitives | Intent Preview, confirm-before-act, Action Audit, Undo/compensation state, denied-policy state, a11y rules, PDS catalog entries. |
| G3 | Agentic threat model catalog | OWASP Agentic Top 10 mapping, filesystem/network sandbox posture, tool misuse controls, identity escalation controls. |
| G4 | PHI governance/provenance consolidation | Existing evidence categories to consolidate, SLSA/cosign external-CI decision, local fallback posture, release-evidence schema. |
| U2 | Least-privilege product-agent profile | Allowed commands, writable paths, network/live-service policy, review checkpoints, handoff requirements. |
| U4 | Connector graduation | Unsupported-capability inventory schema, graduation evidence schema, live-vs-compiler proof rules. |
| U5 | React Native mobile generator/test | `mobile-rn` target ownership, generated contract, token bridge, native auth, secure storage, push hooks, offline/degraded states, device evidence. |
| U6 | Fork detection | PDS source packages, product consumption evidence, local-copy exception/risk-acceptance format. |
| U7 | Manifest composition | Feature/provider/runtime-ingress selection schema, binary composition evidence, duplicate dependency budget. |

## Agent Guidance

When a task touches a frozen seam:

1. Read the Roadmap row and this page.
2. Change the source of truth, not generated output.
3. Add or update the assertion that would fail without the contract.
4. Run the smallest focused proof first, then the namespace handoff.

If a lane needs to reshape a Wave 0 seam, pause implementation and update the
Roadmap plus this page before touching code.
