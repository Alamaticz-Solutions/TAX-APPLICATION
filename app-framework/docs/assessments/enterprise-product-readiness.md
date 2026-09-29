# Enterprise Product Readiness Assessment — App Framework

| | |
| --- | --- |
| **Assessment type** | Independent enterprise product readiness audit |
| **Assessment date** | 2026-06-10; PDS baseline addendum 2026-06-11 |
| **Target** | `app-framework` (branch `codex/pds-catalog-readiness`, framework `0.1.0`, git provenance per [`appfw.lock`](../../appfw.lock)) |
| **Method** | Static review + ran a subset of the framework's own tooling (`doctor`, validation/PDS artifacts) + read the repo's self-assessments + extracted and mapped the PDS Health IT Security Baseline Standard r4.5 |
| **Lens** | Independent audit — verify and challenge the repo's own self-reported scores against observed evidence |
| **Not in scope** | Live penetration testing, deployed/managed-environment validation, a full `scripts/appfw test` / `release-check` run, live IdP/SIEM review, third-party legal/licensing review |

> **Release-readiness reassessment (2026-06-12).** After the
> `codex/pds-catalog-readiness` work merged to `main`, the current release
> posture was re-scored in
> [`enterprise-release-readiness-2026-06-12.md`](enterprise-release-readiness-2026-06-12.md).
> Headline result: engineering maturity is strong, but enterprise release
> readiness remains blocked by credentialed live-lane execution, missing PDS
> Security Baseline r4.5 release-authority evidence, non-ready provider/security
> certification evidence, and no tagged/licensed distribution.
> The reassessment separates codebase/gate implementation work from live
> environment evidence collection and governance/productization work.

> **Status update (2026-06-11).** Repo-side remediation has started. The
> framework now has root [`CHANGELOG.md`](../../CHANGELOG.md),
> [`SECURITY.md`](../../SECURITY.md), [`SUPPORT.md`](../../SUPPORT.md),
> [`docs/release/versioning-and-compatibility.md`](../release/versioning-and-compatibility.md),
> and [`docs/release/live-environment-work-items.md`](../release/live-environment-work-items.md).
> These artifacts improve product-developer consumption and release planning,
> but they do not replace a tagged release, approved `LICENSE`, published
> artifacts, live provider/security/ops evidence, or release authority approval.

> **PDS baseline addendum (2026-06-11).** The PDS Health IT Security Baseline
> Standard r4.5 is now readable and has been included in this assessment. It
> adds formal production-readiness requirements for IdP/Okta/MFA, SCIM or user
> lifecycle APIs, CyberArk/KMS secrets, SIEM retention, WAF/firewall/TLS/cert
> posture, lower-environment data controls, backup/restore, vulnerability SLAs,
> and SAST/DAST remediation. Repo traceability now lives in
> [`docs/release/pds-security-baseline-traceability.md`](../release/pds-security-baseline-traceability.md);
> live-environment follow-up is tracked in
> [`docs/release/live-environment-work-items.md`](../release/live-environment-work-items.md).

> **Relationship to existing docs.** The repository already contains a detailed
> *self*-assessment in [`docs/release/roadmap.md`](../release/roadmap.md),
> [`docs/release/evidence-matrix.md`](../release/evidence-matrix.md), and the
> formal security review in
> [`docs/architecture/concerns/threat-model.md`](../architecture/concerns/threat-model.md).
> This document is deliberately **independent**: it treats those scores as
> *claims to be verified*, not as findings. Where this assessment diverges from
> the self-scores, the divergence and its rationale are stated explicitly.

---

## 1. Executive Summary

App Framework is an unusually mature *engineering* effort: a schema-driven,
multi-provider Rust backend generator with a generated/human ownership model, a
governed runtime, deep secure-by-design controls, an agent-first CLI, an
agent-readable design system (PDS), and a CI pipeline that certifies four
databases. The codebase (~113K lines of Rust across 12 workspace crates, ~730
test attributes) is real, and the framework's own checks pass cleanly today
(`doctor` green, validation `0 errors / 0 warnings`, PDS component check
`67/67`).

**However, "enterprise product readiness" is not the same as "engineering
maturity," and the two diverge sharply here.** The repository self-reports
**9.27/10 release readiness** and **8.72/10 production readiness**. Those numbers
accurately describe *engineering completeness and gate coverage*, but they
overstate readiness to **adopt, deploy, operate, support, and depend on this as
a versioned enterprise product**, for four structural reasons:

1. **It is a `0.1.0` with no release.** Repo-side versioning docs now exist, but
   there are still **no git tags, approved `LICENSE`, published packages, or
   production-certified release notes.** A downstream team cannot today pin to,
   upgrade between, or be supported on an approved *version* of this framework.
   The framework's own evidence matrix scores this lowest
   (versioning/compatibility 7.60/10); independently it remains the single
   biggest product gap and rates lower still.
2. **Nearly all "production" evidence is local or CI-synthetic.** The roadmap is
   candid about this ("live release evidence still needs to be produced and
   retained from an approved provider-backed release environment"). Verified
   examples: the retained `dast-evidence.json` is a self-described *"local HTTP
   probe suite"* (`zap_available: false`); the CI release gate runs against
   **ephemeral containers** (`postgres:14`, `mongo:7`, mssql, `localstack/snowflake`),
   not a managed/regulated environment; and the strict release-evidence check is
   exercised in CI with `--local-fixture`.
3. **The downstream-product story rests on a single reference app (CRM).** The
   entire packaging/lifecycle/upgrade narrative is proven by one example; no
   second, independently-shaped product has been created via `appfw new` and run
   through generate → test → upgrade in CI.
4. **The assessor and the assessed are the same team.** The 9+ scores, the
   "release-gated" maturity labels in the PDS catalog, and the security
   dispositions are all self-authored against self-authored evidence. That is
   normal for an internal scorecard, but it means the scores carry objectivity
   risk and should not be read as independent certification.
5. **PDS Security Baseline r4.5 adds mandatory production evidence outside the
   repo.** Code-level controls are strong, but PDS production readiness also
   requires proof of Okta/MFA, SCIM or lifecycle automation, CyberArk/KMS
   secrets, SIEM retention, WAF/firewall/TLS/cert posture, backup/restore,
   lower-environment data controls, vulnerability SLAs, and enterprise SAST/DAST
   remediation.

### Independent headline scores

I separate the two questions the self-scores conflate:

| Composite | Independent score | Repo self-score (nearest) | One-line basis |
| --- | ---: | ---: | --- |
| **Engineering & framework maturity** | **8.2 / 10** | 9.27 (release readiness) | Architecture, generation safety, secure-by-design, docs, and CI are genuinely strong and largely verifiable. |
| **Enterprise *product* readiness** | **5.8 / 10** | 8.72 (production readiness) | Early-stage *product*: no tagged distribution, no live/managed-env evidence, single reference product, no approved support/SLA/license model, and no retained PDS baseline evidence bundle. |

**Bottom line:** this is a **high-quality framework that is not yet a shippable
enterprise product.** It is an excellent foundation and is appropriate for
*controlled internal pilots*. It is **not ready to be offered, versioned,
supported, or depended upon as a product** until it cuts a real release, proves
itself in a managed environment, and is exercised by more than one downstream
team. The good news: the gaps are well-understood (the team has already named
most of them) and are about *evidence, packaging, and productization* rather
than missing architecture.

---

## 2. Scoring Methodology

Scores are 0–10. Because this is a *product* readiness audit, two principles
differ from the repo's internal maturity rubric:

- **Live, retained, managed-environment evidence outweighs code paths and
  synthetic fixtures.** A wired gate that has only ever run against CI containers
  or local fixtures is scored as "implemented, unproven in production," not as
  "certified."
- **"Can a customer adopt/operate/support a *version* of this?" is weighted
  heavily.** Versioning, packaging, distribution, support model, and multi-tenant
  proof of adoption count as much as runtime quality.

Each dimension lists my **independent score**, the repo's nearest **self-score**,
what I **verified**, and the **residual gap**.

---

## 3. Independent Scorecard

| # | Dimension | Independent | Repo self-score | Confidence |
| ---: | --- | ---: | ---: | --- |
| 1 | Architecture & runtime modularity | 8.5 | 9.30 | High |
| 2 | Code quality & test depth | 8.0 | 9.38 | Medium |
| 3 | Generation & ownership safety | 9.0 | 9.40 | High |
| 4 | Secure-by-design controls (code) | 8.5 | 9.23 | High |
| 5 | Provider parity & data correctness | 7.5 | 9.16 | Medium |
| 6 | Observability & operations | 6.5 | 8.84 | Medium |
| 7 | Performance & scalability | 6.5 | 8.76 | Medium |
| 8 | CI/CD & release engineering | 8.0 | 9.32 | High |
| 9 | Versioning, packaging & distribution | 4.5 | 7.60–8.70 | High |
| 10 | Documentation & agent/dev experience | 9.0 | 9.88 | High |
| 11 | Frontend / PDS design system | 7.0 | 9.07 (codex 7.7) | High |
| 12 | Downstream adoption & lifecycle proof | 5.5 | 8.70 | High |
| 13 | Live / managed-environment evidence | 4.0 | 8.72 (prod cert) | High |
| 14 | Productization: support, SLA, licensing, cadence | 4.5 | — (not scored) | High |
| 15 | PDS Security Baseline r4.5 alignment evidence | 3.5 | — (not scored) | High |

### 3.1 Architecture & runtime modularity — 8.5
**Verified.** Clean workspace split (`appfw_runtime`, `app_gen`, `appfw_cli`,
per-provider crates, `appfw_test`); a governed runtime path
(`RuntimeIngress → RuntimeOperationDispatcher → DataAccess/provider`) that HTTP,
MCP, and Kafka all route through; root-aware generator with explicit roots.
`doctor` resolves all roots correctly.
**Gap.** "Runtime/package extraction" is self-admittedly incomplete — product
templates still carry copied runtime/provider internals (template slimming
8.35/10). Until that lands, the modular boundary is a design intent more than a
shipped package boundary.

### 3.2 Code quality & test depth — 8.0
**Verified.** ~730 `#[test]`/`#[tokio::test]` attributes; `validate` returns
`0 errors / 0 warnings`; the team claims a zero-warning posture.
**Gap.** I did **not** run the full suite or measure coverage, so depth-by-area
is taken on partial trust. There is no published coverage metric, and "730 test
attributes" says nothing about branch coverage of the security-critical paths
(access filters, tenant isolation, IDOR) that matter most for an enterprise
product. *Recommend a coverage gate with a published number.*

### 3.3 Generation & ownership safety — 9.0
**Verified (standout).** Artifact manifest (`artifacts.json`) distinguishing
`generated` vs. `human_owned`, deterministic `generate --check` drift detection,
config-as-source contract. This is the framework's most differentiated and most
convincing capability, and it is verifiable locally.
**Gap.** Drift proof is local; no CI-retained downstream-app drift evidence yet.

### 3.4 Secure-by-design controls (code) — 8.5
**Verified.** Deny-by-default policy, central tenant isolation, JWT-gated
non-local introspection, parameterized provider compilation, bounded
requests/depth/complexity/rate limits, hardened CORS/CSP, audit hash chains,
PHI log lint, SBOM + `cargo audit`/`cargo deny`, regulated classification
validation. The threat model is genuine and OWASP-2025-mapped.
**Gap (scored separately in #13).** Every *live* security proof — tenant/IDOR
negatives across providers, denied-mutation audit, DAST/SAST/ASVS, provenance &
signing — is either synthetic (`dast-evidence.json` = local probe;
`artifact-signing.json`, `asvs-traceability.json` = local placeholders) or
deferred. Production attestation mode explicitly *rejects* the local
provenance/signing placeholders, which is the right call but confirms none of it
is production-grade yet. MFA / PHI session timeout (SA-07) are delegated to the
IdP and unproven.

### 3.5 Provider parity & data correctness — 7.5
**Verified.** Shared QueryIR, provider certification framework, and a CI release
gate that stands up all four databases. Strong, and partially live (CI
containers).
**Gaps.**
- **Neo4j release posture (independent finding):** a fifth crate,
  [`appfw_provider_neo4j`](../../appfw_provider_neo4j/Cargo.toml) (`0.1.0`,
  "Neo4j graph read provider support"), is a **workspace member**. Runtime docs
  now state that Neo4j is outside CRUD semantic parity, but release artifacts
  must keep that scope explicit: the four CRUD providers are release-certified
  by `provider-test --all`; Neo4j remains a graph-read/governed-write foundation
  with live certification pending. See §5.
- Snowflake "live" certification is via **LocalStack**, not real Snowflake.
- No retained *managed-environment* all-provider bundle (CI-ephemeral only).

### 3.6 Observability & operations — 6.5
**Verified.** `/health/live`, `/health/ready`, `/metrics`, `/metrics.json`,
structured logs with redaction, ops-certification command, Alertmanager probe
hooks, OTLP config.
**Gap.** Self-admitted and confirmed by artifacts: OTLP export, Alertmanager
endpoint, Grafana, SIEM export/retention, and runbook drills are **not live-
proven** (OTLP 8.35/10, alerting 8.55/10). `alertmanager-status.json` exists but
static config ≠ live proof, as the team itself notes. For an enterprise product
that handles regulated data, *operability under failure is unproven*, which is
why I score this well below the self-score.

### 3.7 Performance & scalability — 6.5
**Verified.** QueryIR cost budgets, keyset/signed pagination, load-test harness,
provider-performance command, index recommendation artifacts.
**Gap.** All guardrails, no live load. No provider-backed load artifacts, no
pool-saturation proof (8.20/10 self-score), no real EXPLAIN/slow-query evidence.
Scalability is *bounded by design* but *unproven at volume*.

### 3.8 CI/CD & release engineering — 8.0
**Verified (strong).** [`bitbucket-pipelines.yml`](../../bitbucket-pipelines.yml)
+ a deep [`scripts/ci/`](../../scripts/ci) suite (supply-chain gate, secret
scan, PHI lint, release-evidence check, security-assurance decision, live-ops
evidence). Triggers are sensible: PRs run fast-check/supply-chain/secret-scan;
`main`, `v*` tags, and manual `release-check` run the provider-backed gate.
**Gaps (independent).**
- **PRs do not run the release gate or provider certification** — only `main`/
  tags do. A regression in provider parity or live security is caught *after*
  merge, not at review.
- **The `v*` tag lane has never fired** (no tags exist), so the release path is
  wired but never exercised end to end.
- **Bitbucket-only**; no portability/abstraction if distribution moves.

### 3.9 Versioning, packaging & distribution — 4.5  *(largest product gap)*
**Verified.** `framework_version = "0.1.0"`; **no git tags; no approved release
notes; no approved `LICENSE`; no published crate/npm artifacts.** Draft
changelog and compatibility policy now exist, but PDS package and all provider
crates are still `publish = false`. Split-root consumption still resolves
framework code from a *checkout*, not a versioned package.
**Why this dominates the product score.** Enterprises adopt *versions*. Without
an approved tag, release notes, license, support line, and consumable artifacts,
a downstream team cannot pin, upgrade deterministically, or receive support tied
to a release. The framework has excellent *internal* provenance (`appfw.lock`,
hashes) and now has draft release policy, but no *external* product versioning.
This is the gate between "good framework" and "enterprise product."

### 3.10 Documentation & agent/dev experience — 9.0
**Verified (standout).** ~15.7K lines of well-structured docs with reader-job
IA, 13 ADRs, an agent task map, generated-ownership guides, a skills pack
(`agent_skills/`), and JSON handoff. Genuinely best-in-class for agent
operability. I withhold the self-assigned 9.88 only because docs maintainability
should not out-score the product it documents, and because some internal ledgers
still need pruning (the team agrees).

### 3.11 Frontend / PDS design system — 7.0
**Verified.** Agent-readable catalog ([`catalog.json`](../../appfw_ui/pds_health/reference/catalog.json)):
8 families, 67 components, all exports present, component check `ok: true`
(`67/67`, 20 checks passed). Token-backed CSS, accessibility patterns, an
interactive catalog app, and a CRM reference consumer. Strong structure and a
disciplined readiness contract.
**Gaps.** Most families are self-labelled `release-gated`, but the catalog's own
rule — *"Do not claim a family is enterprise-ready without retained evidence"* —
is the right standard and is only partially met: evidence is local, the package
is `0.1.0`/unpublished, and adoption is proven by one product (CRM). A prior
internal review scored PDS 7.7/10; independently I land near 7.0 for *product*
readiness (excellent foundation, thin live adoption/distribution).

### 3.12 Downstream adoption & lifecycle proof — 5.5
**Verified.** `appfw new`, profile metadata, `appfw.lock`, `upgrade` reports,
and a complete CRM reference (`examples/products/crm`) consuming the split-root
contract.
**Gap.** **One** product. The create → customize → generate → test → upgrade →
ship loop, and the "frontend scaffold execution," are not proven by a second,
differently-shaped product, and not exercised in CI. The product-team enablement
story is asserted more than demonstrated.

### 3.13 Live / managed-environment evidence — 4.0  *(the production gate)*
**Verified.** Local/CI evidence is genuinely extensive and honestly labelled.
**Gap.** Nothing has been proven in an approved, provider-backed, managed
environment with retained immutable artifacts: no real DAST/pentest, no live
OTLP/SIEM, no production header proof, no managed-DB provider bundle, no tagged
release with promotion metadata. This is *the* line between the self-reported
8.72 production score and true production certification, and the team says as
much. As an independent reviewer I cannot credit production readiness on
synthetic evidence.

### 3.14 Productization: support, SLA, licensing, cadence — 4.5
**Independent finding (not in the self-scores).** Repo-side support,
security-response, changelog, and compatibility docs now exist, but no approved
release line, external support target, SLA, license, release cadence, or
ownership/RACI has been approved for the framework as a *product*. These remain
table stakes for enterprise adoption and require release authority, legal, and
support ownership outside the local checkout.

### 3.15 PDS Security Baseline r4.5 alignment evidence — 3.5
**Verified.** The baseline is now readable and mapped in
[`docs/release/pds-security-baseline-traceability.md`](../release/pds-security-baseline-traceability.md).
Several repo controls align well: deny-by-default authorization, tenant
isolation, TLS mode validation, redacted logging, secret-scan, PHI log lint,
security-assurance decision gates, runtime health/metrics, and deployment docs
that keep IdP, SIEM, secret manager, WAF, backup, and host hardening in the
right ownership layer.

**Gap.** Alignment is mostly architectural and documentary. The baseline
requires retained production evidence for Okta/SSO/MFA, SCIM or lifecycle APIs,
CyberArk/KMS secrets, non-human identity rotation and audit, SIEM export and
retention, WAF/firewall/TLS/certificate posture, API inventory/security tooling,
lower-environment data controls, encrypted backup/restore, host/cloud/database
hardening, vulnerability-remediation SLAs, and SAST/DAST Critical/High
remediation. Those are not provable inside this repo alone and now have explicit
live-environment work items: LIVE-015 through LIVE-021.

---

## 4. What Is Genuinely Strong (credit where due)

These are real, verifiable, and differentiating — do not let the critique
obscure them:

- **Generated/human ownership model + deterministic drift detection** (§3.3) —
  the framework's signature capability.
- **Config-as-source with a generated, hash-locked contract** — agents and
  humans share one validated source of truth.
- **Secure-by-design depth** (§3.4) — deny-by-default, central tenant isolation,
  parameterized compilation, OWASP-2025-mapped threat model.
- **Agent operability** — task maps, skills, JSON handoff, generated-ownership
  guides. This is among the best agent-facing repos I have reviewed.
- **Supply-chain rigor** — SBOM, `cargo audit`/`deny`, secret scan, PHI lint,
  dependency-plan/upgrade workflow.
- **Intellectual honesty** — the roadmap and threat model do not hide the
  live-evidence gap. That candor is itself a maturity signal.

---

## 5. Key Independent Findings

| ID | Sev | Finding | Evidence | Recommendation |
| --- | --- | --- | --- | --- |
| F-1 | **P0 (product)** | Not yet a released product: `0.1.0`, no tags, no approved `LICENSE`, no published artifacts, and no production-certified release notes. Draft changelog and compatibility policy now exist. | `appfw.lock`, `git tag` empty, crates `publish=false`, [`CHANGELOG.md`](../../CHANGELOG.md), [`docs/release/versioning-and-compatibility.md`](../release/versioning-and-compatibility.md). | Cut a tagged `0.x`/`1.0.0-rc` through the release lane; approve license; publish internally consumable artifacts; attach retained release notes and evidence bundle. |
| F-2 | **P0 (production)** | No live/managed-environment evidence; production claims rest on local/CI-synthetic proof. | `dast-evidence.json` (local probe, `zap_available:false`); CI gate uses ephemeral containers + `--local-fixture`; `artifact-signing.json`/`asvs-traceability.json` are local placeholders. | Execute `release-check` + `provider-test --all` + security/ops/perf evidence in an approved managed environment; retain immutable artifacts; cut the first `v*` tag through CI. |
| F-10 | **P0 (PDS production)** | PDS Security Baseline r4.5 production alignment is not evidenced. The repo has strong app-security architecture, but no retained evidence bundle for IdP/Okta/MFA, SCIM or lifecycle APIs, CyberArk/KMS secrets, SIEM retention, WAF/firewall/TLS/cert posture, backup/restore, lower-environment data controls, vulnerability SLAs, or SAST/DAST Critical/High remediation. | Source baseline addendum; [`docs/release/pds-security-baseline-traceability.md`](../release/pds-security-baseline-traceability.md); [`docs/release/live-environment-work-items.md`](../release/live-environment-work-items.md). | Make PDS baseline traceability a release-authority gate for production claims. Retain evidence or approved risk exceptions for every baseline category before promotion. |
| F-3 | **P1** | Neo4j is in the workspace but not release-certified as a CRUD provider; its graph-read/governed-write posture must remain explicit in release artifacts. | [`appfw_provider_neo4j/Cargo.toml`](../../appfw_provider_neo4j/Cargo.toml); [`docs/runtime/graph-read-providers.md`](../runtime/graph-read-providers.md); `release-check.json.provider_scope`. | Keep Neo4j outside CRUD release claims until live graph certification exists, or add a separate Neo4j graph certification lane and retained evidence before claiming support. |
| F-4 | **P1** | Single downstream product (CRM) carries the entire adoption/lifecycle/packaging story. | `examples/products/` contains only `crm`. | Create a second, differently-shaped product via `appfw new`; run create→generate→test→upgrade in CI as golden-downstream proof. |
| F-5 | **P1** | PR pipeline now includes a fail-closed release-lite guard for provider/runtime/security-sensitive paths, but the live approval lane still has to be operated. | `bitbucket-pipelines.yml` `pull-requests` runs `scripts/ci/release-lite-guard.sh`; guard evidence is retained at `target/appfw/release-lite-guard.json`. | For sensitive PRs, populate `APPFW_RELEASE_LITE_EVIDENCE_URL`, `APPFW_RELEASE_LITE_APPROVER`, and `APPFW_RELEASE_LITE_REASON` with an approved manual `release-check` or scoped provider-backed release-lite run before merge. |
| F-6 | **P2** | No published test-coverage metric for security-critical paths (access filters, tenant isolation, IDOR). | ~730 test attributes but no coverage gate/number. | Add a coverage gate with a published figure, weighted to security/data paths. |
| F-7 | **P2** | Productization wrapper is started but not approved: support/security docs exist, while SLA, external disclosure channel, license, release cadence, and ownership/RACI remain pending. | [`SECURITY.md`](../../SECURITY.md), [`SUPPORT.md`](../../SUPPORT.md), [`docs/release/live-environment-work-items.md`](../release/live-environment-work-items.md). | Approve the support owner, response targets, external disclosure path, license, release cadence, and RACI before external or regulated production adoption. |
| F-8 | **P2** | Self-assessment objectivity: 9+ scores and "release-gated" PDS labels are self-authored against self-authored evidence. | `roadmap.md`, `catalog.json` maturity labels. | Commission a periodic independent review; downgrade "release-gated" claims to "enterprise-ready (pending live evidence)" until retained external evidence exists. |
| F-9 | **P3** | Working-tree hygiene: untracked screenshots and a `catalog-app/` build are present at repo root / under `appfw_ui`. | `git status` shows `catalog-*.png`, `.playwright-mcp/`, `catalog-app/`. | `.gitignore` build/preview artifacts; keep generated catalog output out of source. |

---

## 6. The Core Gap: Engineering Maturity ≠ Product Readiness

The repository's self-scores are a faithful measure of **how complete the
engineering is** (gates wired, controls coded, docs written). They are *not* a
measure of **whether this can be adopted, run, and supported as a product**,
which depends on three things the framework has deliberately deferred:

| Question an enterprise asks | Current answer | Score impact |
| --- | --- | --- |
| "Which *version* do we adopt, and how do we upgrade safely?" | No approved release — `0.1.0`, no tags, no license, no published artifacts; draft changelog and matrix exist. | §3.9 → 4.5 |
| "Has it survived a real (managed, regulated) environment?" | No — local/CI-synthetic only. | §3.13 → 4.0 |
| "Does it meet the PDS production security baseline?" | Not evidenced — traceability exists, but IdP, secrets, SIEM, WAF/TLS, backup, data, vulnerability, and SAST/DAST evidence are live-environment work. | §3.15 → 3.5 |
| "Has anyone besides the authors built and shipped on it?" | No — one in-repo reference product. | §3.12 → 5.5 |
| "Who supports it, on what SLA, under what license?" | Draft support/security docs exist; approved SLA, owner, and license are still undefined. | §3.14 → 4.5 |

The framework is, in effect, a **release candidate's worth of engineering inside
a pre-release product shell.** Closing the four items above — none of which
require new architecture — would legitimately move the *product* composite from
~5.8 toward the ~8.7 the team already reports for the *engineering*.

---

## 7. Recommended Path to a Defensible 1.0

Ordered by leverage (each unblocks the next):

1. **Cut a real release (F-1).** SemVer, first `v*` tag through the existing CI
   release lane, CHANGELOG, compatibility/upgrade matrix. *This alone moves the
   product from "framework" to "versioned product."*
2. **Make PDS baseline traceability a release-authority gate (F-10).**
   Use [`docs/release/pds-security-baseline-traceability.md`](../release/pds-security-baseline-traceability.md)
   and LIVE-015 through LIVE-021 to require evidence or approved exceptions for
   IdP/MFA, lifecycle, secrets, SIEM, gateway/WAF/TLS, backup/restore,
   lower-environment data, vulnerability SLAs, and SAST/DAST.
3. **Produce one retained managed-environment evidence bundle (F-2).**
   `release-check` + `provider-test --all` + live security/ops/perf evidence from
   an approved environment, with immutable artifacts. Converts §3.4–3.7 and §3.13
   from "coded" to "certified."
4. **Resolve the provider story (F-3)** and **prove a second downstream product
   (F-4)** — ideally the second product is the thing that *consumes the first
   tagged release*, proving F-1 and F-4 together.
5. **Tighten the PR gate (F-5)** and **publish a coverage number (F-6).**
6. **Author the productization wrapper (F-7)** — support/SLA/license/cadence —
   and **commission an independent re-score (F-8).**

A reasonable re-assessment trigger: once F-1, F-2, F-4, and F-10 are done with
retained evidence, the enterprise-product composite should be re-scored
(expected range 8.0–8.7).

---

## 8. Verification Log (what this assessment actually ran/read)

**Ran:** `scripts/appfw doctor` (all required tools `ok`); read fresh
`validation.json` (`valid:true`, `0/0`); `node scripts/check-pds-components.mjs
--json` (`ok:true`, 67/67, 8 families); inspected `git log`, `git tag` (empty),
`Cargo.toml` workspace, `appfw.lock`, `bitbucket-pipelines.yml`, `scripts/ci/`,
and retained `target/appfw/` artifacts.

**Read (self-assessments treated as claims):** `docs/release/roadmap.md`,
`docs/release/evidence-matrix.md`, `docs/architecture/concerns/threat-model.md`,
`README.md`, `AGENTS.md`, `appfw_ui/pds_health/reference/catalog.json`.

**Read (external baseline source):** PDS Health IT Security Baseline Standard
r4.5, provided locally as a PDF during the assessment. The source was extracted
locally for review and mapped into
[`docs/release/pds-security-baseline-traceability.md`](../release/pds-security-baseline-traceability.md).

**Did not run (taken on partial trust):** full `scripts/appfw test`,
`release-check`, `provider-test --all`, live load/DAST, live IdP, live SIEM,
WAF/gateway, CyberArk/KMS, backup/restore, or platform hardening validation. No
deployed environment was available to this review.

**Standing caveat:** scores reflect the working tree on 2026-06-10 plus the PDS
baseline addendum on 2026-06-11, and will move as live evidence is produced.
This is an engineering-judgment audit, not a formal compliance certification.
