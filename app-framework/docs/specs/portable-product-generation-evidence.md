# Portable Product Generation Evidence

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: Nexus master-plan owner
- Architect: Nexus/App Framework architecture lane
- XO: Nexus master-plan coordinator
- Implementation owner: App Framework Coding Agent on
  `fix/portable-artifact-lock-v2-current-main`
- Review owner: independent Framework PR Review Agent (comprehensive)

## Business Value

Downstream products must be able to consume an exact App Framework Git or
registry dependency without generated local-development files silently
depending on the checkout that happened to run generation. Reviewers must also
be able to trust every generated-artifact digest as a digest of the final bytes
that generation left on disk.

## Problem

The Fabric Operations Console exposed three upstream defects:

1. `podman-compose.yml` always binds `appfw_runtime` and observability
   configuration from the generating Framework checkout, even when the product
   backend declares an exact Git or registry dependency.
2. the artifact ledger hashes an artifact when it is first emitted, but a later
   generator phase can rewrite that path. The resulting digest can describe
   intermediate rather than final bytes. The system-schema
   `entity_types/_res.yaml` path reproduces this defect.
3. lock version 1 hashes raw `artifacts.json` bytes, including checkout-local
   absolute paths and run-local actions, so identical product output has a
   different lock identity after relocation.

These defects make a dependency-based product non-portable and weaken
generated-output provenance.

## Goals

- Derive the compose Framework dependency posture from the product backend
  Cargo manifest.
- Preserve the declared runtime bind for path mode. Include observability only
  from a complete, validated sibling tree; omit an absent sibling with truthful
  evidence and fail closed when a present sibling is incomplete or symlinked.
- Omit adjacent Framework runtime and observability source binds for Git and
  registry dependency modes, with explicit retained evidence.
- Reconcile artifact ledger hashes from final regular-file bytes immediately
  before writing the ledger.
- Fail closed when a recorded artifact is missing or is not a regular file.
- Bind lock version 2 to a closed, configured-root-qualified `app`/`config`
  artifact identity whose recorded content hashes are independently checked
  against current artifact bytes.
- Reject malformed or unknown lock fields and require explicit version 1
  migration rather than silently treating a raw-manifest hash as portable.
- Cover path, Git, registry, ambiguous dependency, final-byte mutation, and
  invalid final artifact cases with adversarial tests.

## Non-Goals

- Publishing Framework crates to ProGet or certifying private Git access in CI.
- Redesigning the local compose backend build or observability stack.
- Changing product-owned source in the Fabric Operations Console repository.
- Claiming deployment, release, SRA, CAB, or production readiness.

## Scope

Framework-owned generator source and tests in `app_gen`, the generated dev
infra report contract, product lock/upgrade evidence, and the smallest
documentation updates needed to make the behavior discoverable.

## Repository Context

- `app_gen/src/dev_infra.rs` owns generated `podman-compose.yml` and
  `dev_infra.json`.
- `app_gen/src/utils/artifacts.rs` owns the generated artifact ledger and
  provenance record.
- `app_gen/src/utils/type_relationships.rs` legitimately rewrites normalized
  entity-type output after its first emission; the ledger must therefore hash
  final filesystem state rather than an intermediate event.
- `docs/reference/product-workspace-contract.md` targets versioned packages or
  approved Git revisions as the downstream dependency boundary.
- `docs/start/generated-ownership.md` forbids downstream products from patching
  generated compose or artifact evidence by hand.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Product backend Cargo manifest | Read `appfw_runtime` source as path, Git, or registry; Cargo-supported path/Git plus publish-fallback `version` metadata retains the active path/Git classification; genuinely ambiguous/missing shapes fail closed | Product bootstrap dependency modes and generated compose |
| `podman-compose.yml` | Path mode retains the runtime bind at the container path Cargo resolves from `/app`; a complete validated sibling tree retains observability, an absent sibling omits it truthfully, and a present incomplete or symlinked tree fails closed; Git/registry modes omit checkout-bound Framework assets | `dev_infra.json`, split-root equivalence, generator tests |
| `dev_infra.json` | Records dependency source, manifest path, bind/service inclusion, and omission rationale | Generated compose bytes and product handoff evidence |
| `artifacts.json` | Each record carries a SHA-256 of final regular-file bytes | Artifact provenance hash, ownership explanations, lock/upgrade evidence |
| `appfw.lock` | Version 2 hashes a closed, configured-root-qualified `app`/`config` artifact identity and validates actual content bytes before writing | Product lock, new-app bootstrap, upgrade/migration diagnostics |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Continue deriving all binds from `framework_root` | No code change | Leaks the generating checkout and contradicts Git/registry product manifests | Rejected |
| Vendor runtime/observability source into every product | Self-contained compose | Copies Framework source across the product boundary and creates upgrade/license drift | Rejected |
| Keep path binds only for path dependencies; omit checkout-bound assets for Git/registry | Matches the declared dependency boundary and preserves current path development | Portable mode initially has no generated observability profile | Selected |
| Hash only at each emit call | Simple event ledger | Records intermediate bytes when later phases rewrite a path | Rejected |
| Reconcile all records immediately before manifest serialization | Binds evidence to final bytes and detects missing/type drift | Adds one final filesystem pass | Selected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-12 | Nexus master-plan coordinator and App Framework architecture lane | Implement manifest-classified path/Git/registry compose behavior and final-byte artifact reconciliation | Exact Fabric generation leaked `/private/tmp/appfw-integrate-nexus-product-tooling-r1`; its system entity ledger digest differed from the final file | A packaged, checkout-free observability/config distribution contract is accepted |

## Architecture And Implementation Notes

Parse the product `backend/Cargo.toml` using a TOML parser and classify the
`appfw_runtime` dependency. Path mode resolves the declared path relative to
the backend manifest and mounts that runtime at the equivalent container path
Cargo resolves from `/app`. If the runtime has no sibling observability root,
generation omits those services and records why. If the sibling exists, every
required asset must be a regular non-symlink path before the services are
included; incomplete or symlinked trees stop generation. Git and registry modes
generate the backend and data/Kafka services without Framework checkout binds or
checkout-backed observability services. Unsupported, missing, and workspace-only
declarations stop generation. Cargo's supported dual-location publish fallback
is classified by its active `path` or `git` selector when paired with `version`
(and optional `registry` for path); `path` plus `git` and `git` plus `registry`
remain ambiguous and stop generation.

Before `artifacts.json` is serialized, reconcile every record against
`symlink_metadata` and final file bytes. Missing paths, directories, and
symlinks are errors rather than silently hashless evidence.

Before writing lock version 2, parse the manifest with a closed record schema
and build checkout-independent `appfw.artifact_manifest_identity@2` records.
Each record carries an explicit `app` or `config` root qualifier plus a path
relative to that independently canonicalized configured root. This keeps the
identity stable when the product, external config root, or external report root
is relocated without collapsing same-named app/config artifacts into one key.
Read each regular non-symlink artifact with a size bound and observed
file-identity checks, and compare its actual SHA-256 to the ledger. App, config,
report, output, and evidence-path ancestors are checked for symlinks before
access; configured filesystem roots are rejected; and opened leaf identity is
checked again after I/O. These checks fail on observed path or file drift; they
do not claim immunity from a hostile process that can mutate parent directories
between individual operating-system calls.

## Security, Privacy, And Governance

No identity, authorization, tenant, PHI/PII, secret, or live-provider behavior
changes. The correction reduces unintended host-path disclosure in generated
product source. It does not prove private Git credentials, registry access,
observability deployment, release approval, or production controls.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Path mode mounts the runtime at Cargo's resolved container path; complete validated sibling observability is included, an absent sibling is omitted truthfully, and a present incomplete or symlinked tree fails closed | focused `dev_infra` tests, semantic runtime-target assertions, and clean CRM generation/diff | review |
| Exact Git-shaped Fabric manifest emits no adjacent Framework/observability bind | focused `dev_infra` tests and detached Fabric-shaped generation proof | review |
| Registry mode emits no adjacent Framework/observability bind | focused `dev_infra` tests | review |
| Ambiguous or unsupported dependency shapes fail closed | focused `dev_infra` tests | review |
| Ledger hashes final bytes after a later mutation | focused artifact tests and generated Fabric hash audit | review |
| Missing/non-regular final artifacts fail | focused artifact tests | review |
| Tamper after manifest, symlinked evidence paths, and unretained hashes fail closed | focused artifact/lock tests | review |
| Standard and external config/report roots produce root-qualified portable lock identity and a current upgrade result | focused relocation test plus full external-root generation -> lock -> upgrade test | review |
| Lock v2 is relocation-stable; strict parsing and v1 migration remain explicit | focused introspection tests | review |
| Framework generation remains deterministic | `scripts/appfw framework generate`; `scripts/appfw framework generate --check --json` | review |
| Framework checks and handoff are current | validation, docs-check, framework tests, change-impact, handoff | comprehensive review |

## Test And Execution Feedback Plan

Start with focused `appfw-codegen` unit tests, then run the Framework generation
loop. Exercise a disposable product copied from the exact Fabric model and Git
dependency shape. Compare every ledger digest to final bytes and scan compose
for generating-checkout leakage. If path-mode output drifts or a supported
Cargo shape cannot be classified unambiguously, stop and revise this spec or
the implementation instead of adding a fallback bind.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Git/registry compose loses the optional observability profile | Explicit report status and no observability-readiness claim; package its config in a later contract | Product/Architecture | accepted limitation for this slice |
| Manifest parsing accepts a misleading mixed source | Fail closed when source selectors are missing, mixed, or workspace-only | Implementation | required |
| Final reconciliation hides a generator sequencing bug | Preserve all ledger events but replace only their content digest with final bytes; adversarial mutation test | Implementation | required |
| Broader generated drift appears | Preserve diagnostics and route to Integration; do not patch product output | Implementation/XO | required |

## Tech Debt And Follow-Up

Define a versioned, checkout-free observability configuration package before
restoring the generated observability profile for Git/registry consumers.
ProGet publication and private Git CI authentication remain separate release
and integration work.

## Handoff Notes

This is a Class C generated-contract change and requires comprehensive
independent Framework review. The implementation agent has no merge, release,
SRA/CAB, or accepted-risk authority.
