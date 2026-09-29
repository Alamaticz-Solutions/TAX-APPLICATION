# Agent Handoff Backlog

> **Purpose.** The design/strategy for the current increment is complete and
> committed to the docs below. This page is the *execution index* for agents and humans:
> what to build, where it is specified, whether it is buildable now, and the
> guardrails. It intentionally does **not** restate the specs — it points to
> them. Keep this page current as items land; delete rows when merged.

## How to work from this page

1. Read the cited spec for a row before starting — it carries the done-state.
2. **Lane-sized PRs only** (North Star Part 6 anti-pattern: no multi-lane
   mega-branch, even if every gate passes).
3. Honor the **Wave 0 contract freeze** (`docs/architecture/concerns/north-star-wave-0.md`)
   and the **spec-done rule** (`north-star-wave-0-specs.md`): a lane is not done
   until a `docs-check` / `release-evidence-check.sh` assertion fails without its
   artifact.
4. **Evidence-gate discipline**: never guess a vendor / enterprise / internal-service
   contract; "absent — evidence-gated" with a named blocking artifact.
5. Verify each lane per its done-state, then `scripts/appfw framework handoff --json`.

## Source-of-truth specs

| Domain | Canonical spec |
| --- | --- |
| Direction | `docs/strategy/product-development-north-star.md` |
| Sequenced plan | `docs/release/roadmap.md` (North-Star Execution Plan; Wave 3; Wave 4 value-graduation plan) |
| SaaS providers | `docs/runtime/saas-connectors.md` + `appfw_provider_<vendor>/docs/vendor-contract.md` |
| AI chat & search | `docs/runtime/ai-chat-search.md` (CH1–CH9, held Slice-2 edits, spec contracts) + `docs/frontend/agentic-ux.md` (invoked/ambient UX contract, CH-AMBIENT) |
| Threat litmus | `docs/architecture/concerns/threat-model-litmus.md` (FAIL rows = work) |
| Agentic change control | `docs/architecture/concerns/agentic-development-control-system.md` |
| Wave 0 seams/specs | `docs/architecture/concerns/north-star-wave-0.md`, `…-wave-0-specs.md` |

## The gate that unblocks almost everything

Most rows below edit Rust crates / `scripts/appfw` / shared docs that are
currently **uncommitted in a parallel "lane-2" working tree**. They are blocked
not by design but by the shared tree. **Once lane-2 merges to `main`**, fan out
these lanes from a clean `main` checkout, one lane-sized PR each.

## Priority-Move Coordination Refresh

Current local branch evidence shows many Wave 4 slices exist, but Wave 4 should
not be treated as complete until the integration state is refreshed against the
current governance/review/SRA/CAB branch state. Some older Wave 4 integration
work predates the newer PR review, SRA package, branch naming, CAB TODO, and
agent-governance docs; merging stale integration output can delete or downgrade
those newer control surfaces.

Initial local audit, 2026-07-05: remote refresh was initially unavailable from
this checkout because Bitbucket REST credentials were being retried with
mismatched token/header assumptions. The durable fix is
`docs/start/bitbucket-rest-auth.md`: use `BITBUCKET_API_TOKEN` as an Atlassian
user API token with Basic auth and the Atlassian account email as the username,
then prove the path with `scripts/ci/bitbucket-api-smoke.sh --repo --pipelines`.
The local refs were still useful enough to show that Wave 4 is **not complete**:

- one Wave 4 UI/Nexus integration branch contains many first slices: contract
  freeze, prompt audit, chat SSE shell, answer envelope, chat-eval command, AI
  search provider, AI gateway auth, view registry, Conversation family, Ambient
  family, Nexus pilot/visual/UX evidence, PHI pipeline governance, and AIBOM
  posture;
- that integration branch is stale relative to the newer governance/review/SRA
  package/CAB package direction and must not be merged as-is;
- later Wave 4 leaf branches are not absorbed into that older integration
  branch: CH1 runtime-layer proof, CH2 data-part/transcript contract, CH5 Flow
  token bridge, CH5 markdown/sanitizer decision, CH6 judge evidence gate, CH6
  promptfoo pipeline plan, CH6 wave-status evidence, CH7 gateway decision
  evidence, and SEC prompt-audit posture;
- completion therefore requires a rebuild from the current approved base,
  explicit discard/rebase decisions for stale leaf work, aggregate review, and
  retained evidence that still matches current docs, skills, CLI, and gates.

Use this refresh before asking for more Wave 4 implementation:

| Step | Owner | Action | Done when |
| --- | --- | --- | --- |
| PM-R0 | Strategist/Product Manager + Product Owner + XO | Produce the current wave refresh: Strategist challenges strategic business themes, Product Owner names which slices are merged, pending, stale, superseded, or blocked by live/external evidence, and XO keeps role ownership and coordination clear. | A human-readable refresh brief names the next merge order and the branches/slices to discard or rebase. |
| PM-R1 | Integration Branch Manager | Rebuild the Wave 4 integration branch from the current approved base, then merge only still-relevant leaf branches in dependency order. | The aggregate diff does not remove current governance/review/SRA/CAB docs, skills, commands, or release gates. |
| PM-R2 | Architect thread | Continue lane implementation only after PM-R0/PM-R1 name the active surfaces. | New branches are lane-sized and use neutral purpose prefixes, not tool/author prefixes. |
| PM-R3 | PR Review Agent / Tech Debt Steward | Review the refreshed integration diff and convert conditions into explicit debt or follow-up lanes. | Review output gives `GO`, `GO WITH CONDITIONS`, `NO-GO`, or `DEFER` with counts and evidence checked. |
| PM-R4 | Product/Nexus proof threads | Supply product evidence for the slices that claim user-facing value. | Nexus/product visual, UX, eval, telemetry, SRA, CAB, or release evidence is retained where claimed. |
| PM-R5 | Product Owner + XO + Integration Branch Manager + Workstream Analyst | Refresh the delivery loop itself: branching, local proof, PR review, CI, merge, main CI, and release evidence. | Repeated CI failures, late release-lite failures, stale branch conflicts, slow opaque checks, and inefficient token/output burn become local preflight, docs-check subchecks, review-brief signals, CI parallelism/caching work, analyst recommendations, or explicit debt. |
| PM-R6 | Product Owner + XO + Framework Steward | Prove clean-machine harness replication before team handoff. | `docs/start/team-harness-replication.md` is current, repo-native skills are linked, slash-command workflows have tool-neutral fallbacks, local-only inputs are documented, and bootstrap evidence can be reproduced from a fresh checkout without private prompt history. |
| PM-R7 | Framework Structure Steward | Review docs IA, skills, CLI contracts, code organization, generated boundaries, and delivery-harness entropy after broad work or repeated friction. | A maintenance brief gives `healthy`, `watch`, or `needs-maintenance`, names top entropy risks, and recommends Product Owner/Strategist backlog items with owner, priority, evidence, and suggested lane. |
| PM-R8 | Product Owner + Strategist/Product Manager + XO + all agent roles | Preserve the human/agent operating model as the default collaboration contract. | `docs/start/agentic-human-operating-model.md` remains linked from onboarding, strategy, and harness docs; role drift becomes Product Owner/Strategist maintenance, not private thread memory. |
| PM-R9 | Architect thread + Integration Branch Manager | Add repo-owned local hook guardrails for pre-push review hygiene. | **First slice implemented in this branch:** `scripts/ci/install-local-git-hooks.sh` configures `scripts/git-hooks/pre-push`, which runs only for branch updates about to be pushed and blocks stale/missing Framework PR Review Agent evidence through `scripts/ci/pre-push-review-guard.sh`. Future slices can add optional conflict-marker/format pre-commit checks without making review run at commit/save time. |

Near-term priority moves should be distributed as:

| Move | Primary owner | Secondary owner |
| --- | --- | --- |
| Continuous product-owner review loop and product signal intake | Product Owner | Integration Branch Manager for branch/debt signals; Strategist/Product Manager for strategic themes |
| Product outcome telemetry and ROI proof | Product/Nexus proof threads | Architect thread for generated telemetry contracts |
| App intelligence, memory/context, and intelligent layout maturity | Architect thread | Product/Nexus proof threads |
| MCP certification harness | Architect thread | PR Review Agent for security/evidence review |
| Integration/data governance friction reduction | Architect thread | Integration Branch Manager |
| Tech Debt Register, CAB package, and SRA package maturity | Product Owner | PR Review Agent / Tech Debt Steward; Strategist/Product Manager when business-risk themes are affected |
| CI/dev-loop acceleration | Workstream Analyst | Architect thread for technical changes; Integration Branch Manager for CI branch evidence |

## Buildable NOW (new files only; no shared-file edits)

| Pkg | What | Spec | Notes |
| --- | --- | --- | --- |
| CH6-now | chat-eval spec page (`docs/architecture/concerns/chat-eval-spec.md`) + fixtures (`app_gen/_config/chat_evals/…`) + `scripts/check-chat-eval.mjs` runner; then run it green + prove fail-closed | `ai-chat-search.md` §Testing, CH6 | **Implemented in `codex/wave2-governed-write`.** Runner is zero-dependency, network-free, writes `target/appfw/chat-eval.json` with `release_ready:false`, runs green via `scripts/check-chat-eval.mjs --json`, and proves fail-closed via `scripts/check-chat-eval.mjs --json --inject-leak`. Delete this row after merge, then continue with CH6-slice2. |

**Status of the 2026-07-02 background workflows (folded in):**
- Vendor-doc verification **ran** — the six `appfw_provider_*/docs/vendor-contract.md` were spot-checked against crate source and corrected in place. Treat them as verified-as-of-2026-07-02.
- The **threat-model litmus** is authored at `docs/architecture/concerns/threat-model-litmus.md` (79 rows; rollup below).
- **CH3 ruling (implemented as a Wave 4 contract slice):** `codex/wave4-ai-search-provider` adds the plan-only `appfw_provider_ai_search` crate, typed auth metadata, fail-closed named-query registry, vendor contract, provider-graduation compiler evidence, and `provider-test --provider ai_search --plan --json`. Executable AI search calls remain blocked on the PDS AI search API contract/export and the shared SaaS HTTP executor.
- **Mobile direction is decided:** the W4-MOBILE-RESEARCH decision is complete in
  `docs/strategy/product-development-north-star.md` and
  `docs/frontend/mobile-react-native.md`. React Native + Expo is the primary
  mobile target; PWA is a responsive web fallback. Remaining U5 work is
  implementation/evidence: generated mobile emission, device/simulator smoke,
  runtime audit disposition, and store-track evidence.

## Blocked until lane-2 merges → then lane-sized PRs off `main`

### Post-merge, do first (small, unblock downstream)
| Pkg | What | Spec |
| --- | --- | --- |
| PM-1 | Wire the enforced gates as **required** CI checks (`composition-check --enforce`, `governance-check --enforce`, `fork-check`, `wave2-status` report-only until inputs are CI-produced, `docs-check --changed-only --enforce-budget`) | **Implemented first slice in `scripts/ci/wave3-pr-gates.sh`: PRs hard-fail docs budget, U7 composition, and U6 fork checks; G4 governance must emit fail-closed release-gated JSON; Wave 2 status is retained as report-only. Delete this row after merge.** |
| PM-2 | **Implemented in this branch:** prove the release-lite guard has no checked-in fallback approvals and fails closed for provider/runtime/security-sensitive PRs unless secured CI variables carry an approved evidence URL, approver, and reason. The focused proof is `scripts/appfw framework docs-check --subcheck release-lite-guard --json`. | roadmap Wave-3 notes; `scripts/ci/release-lite-guard.sh`; `docs/release/release-gate-ci-cd.md` |
| PM-3 | Insert the held **DP1–DP7**, **CH1–CH9**, and **CH-AMBIENT** rows into `roadmap.md`; fold the North-Star 3(b) chat/agentic-UX appendix cross-refs | `saas-connectors`/`ai-chat-search`/`agentic-ux` held-edits |
| PM-4 | Add report-only `scripts/appfw framework change-impact --json` and retained `target/appfw/change-impact.json`; classify broad/sensitive diffs, ownership domains, recommended checks, human-review requirement, and integration-branch requirement before turning it into an enforcing PR gate. | agentic development control system |
| PM-5 | **Implemented in this branch:** report-only `scripts/appfw framework review-brief --json` / `scripts/appfw product review-brief --json` default to focused mode, with `--comprehensive` retaining whole-branch/PR review contracts for the Framework/Product PR Review Agent and pre-push human approval. | PR review agent harness |
| PM-6 | **First slices implemented in this branch family:** add report-only `/pds-sra-package` guidance plus `scripts/appfw framework sra-package --all-products --json` and `scripts/appfw product sra-package --json` evidence for production, vendor/SaaS, AI/MCP, sensitive-data, or significant-governance-change work. The command also retains a Markdown narrative beside the JSON report. Remaining lift: richer diagram templates and LogicGate-specific packaging once governance confirms format. | PDS Health SRA intake notes from Copilot attachment; ignored local `regulatory/` source folder; notes below |
| PM-7 | **TODO:** create a report-only CAB/change-advisory package flow, analogous to `/pds-sra-package`, for releases or significant changes that need formal change approval. Expected shape: `/pds-cab-package --framework`, `/pds-cab-package --product`, and retained JSON/Markdown artifacts with change summary, business impact, affected products/services, implementation plan, test evidence, risk, rollback, monitoring/validation, communications, blackout/window constraints, approver/CAB linkage, and open questions. Do not treat the generated package as CAB approval. | change-management/CAB package notes below |

#### PM-7 CAB Package Source Notes

The CAB package should be the change-management companion to the SRA package:
it prepares evidence for human Change Advisory Board review, but it does not
approve the change.

The first report-only slice should capture:

- change title, owner, requested deployment window, environment, and release or
  PR linkage;
- business purpose, user/business impact, affected products, services,
  integrations, data stores, jobs, workers, and SaaS platforms;
- implementation plan, deployment steps, feature flags, migration steps,
  package versions, image digests, and expected configuration changes;
- validation evidence: local/CI checks, release-check, provider certification,
  frontend/mobile/E2E evidence, SRA/security evidence, performance/ops evidence,
  and skipped-check rationale;
- risk assessment: blast radius, sensitive surfaces, PHI/PII/security impact,
  customer/team-member impact, dependencies, and known residual risks;
- rollback/backout plan, data correction plan when applicable, and criteria for
  aborting or rolling forward;
- monitoring and validation window: dashboards, logs, alerts, health checks,
  owner on point, escalation path, and post-deploy verification;
- communications plan, support readiness, runbook links, and release notes; and
- CAB record/linkage, approver names when known, decision status, and open
  questions for the human change manager.

#### PM-6 SRA Package Source Notes

The local `regulatory/` folder contains downloaded PDS Health security,
architecture, SRA, and governance source material for future PM-6 analysis. It
is intentionally ignored by `.gitignore`; do not commit the raw regulatory
documents unless a human explicitly approves a sanitized documentation strategy.

The `/pds-sra-package` flow should continue moving toward a review-ready intake
package with:

- business purpose, project owner, technical owner, and affected assets/processes;
- data-flow diagram inputs: source/destination systems, integrations/interfaces,
  transfer mechanisms such as HTTPS/SFTP/API, validation/transformation steps,
  access-control and authentication flow, encryption in transit, data types and
  classification, and explicit PHI/PII/business-sensitive flags;
- infrastructure/logical architecture diagram inputs: legacy architecture, cloud
  architecture, hosting environment, storage, and compute components;
- core SRA criteria: authentication, authorization, encryption at rest,
  encryption in transit, connectivity security, logging, and monitoring;
- identity/access details: service accounts, bot accounts, least privilege model,
  authentication mechanisms, and authorization model;
- logging and monitoring evidence: SIEM integration, audit logging, monitoring
  controls, and audit-correlation posture;
- secrets management evidence: CyberArk, AWS Secrets Manager, credential
  rotation, shared credential controls, and owner/escalation model;
- vendor-security evidence when applicable: SOC 2, ISO 27001, security
  questionnaires, data processing controls, subprocessors, and vendor posture;
- risk-register material: risk title/description, source of risk, likelihood,
  impact, risk category, owner, current controls, and mitigation actions; and
- LogicGate/SRA record linkage when an actual SRA record exists.

The source notes point to `SRA Diagram Requirements`, `PDS Risk Form`, SRA
review notes, completed SRA review patterns, the Information Security Handbook,
and a LogicGate SRA lead from the Primescan 2 presentation. Treat the downloaded
files as source evidence to summarize into generated package artifacts, not as
repo-owned product documentation.

### SaaS data-path (DP) lanes
| Pkg | What | Spec | Depends |
| --- | --- | --- | --- |
| DP2 | Provenance + echo-loop contract (promote `meta.projection_of`/`source_field`; per-record provenance; self-origin filter) | saas design DP2 | buildable early |
| DP3 | Atlas connection contract (SRV/seed-list/replica-set in `appfw_provider_mongo/src/connection.rs` + `config_contract.rs`) | saas design DP3 | W0.4 freeze |
| DP1 | CDC ingestion mode (broker client behind the empty `kafka` feature; envelope decode; offset checkpointing) | saas design DP1 | Implemented in the data-plane integration lane; live broker/client evidence remains human-gated. |
| DP4 | Unified freshness/lineage report | saas design DP4; first report/enforce slice in `framework saas-lineage --json` | DP1/DP2 |
| DP5 | Vendor-doc **structural** wiring: `required_docs`/`required_tokens` + `saas-vendor-doc-parity` subcheck + `ApiSnapshotMetadata` for anaplan & workday + IA rows | saas plan Slice 3 | Implemented in the data-plane integration lane; delete source branch after merge. |
| DP6 | `appfw_saas_testkit` + `provider-test --area saas-read` plan lane | saas design DP6 | — |
| DP7 | Principal envelope (`principal_type`/`on_behalf_of`/`ingress` in `UserAuth` + Rego input) | saas design DP7 | **before/with W3-B** |
| SC-rewrite | `saas-connectors.md` 10-section kappa/CDC/three-path rewrite | saas plan Slice 2 | — |

### AI chat (CH) lanes
| Pkg | What | Spec | Depends |
| --- | --- | --- | --- |
| CH1 | Feature-gated chat module + SSE. Runtime-layer proof is added by `wave4/ch1-chat-runtime-layer-proof`: chat transport routes now have shared rate-limit, timeout, and panic-isolation coverage through `apply_runtime_layers`. Closeout is added by `wave4/ch1-chat-runtime-closeout`: long-running SSE body soak, server-side cancel signal, no-buffering header, and explicit orchestrator-disabled proof while live AI/search execution remains out of scope. | `ai-chat-search.md` CH1 | — |
| CH2 | `answer_envelope@1` schema + metadata keys. Data-part/transcript contract slice is added by `wave4/ch2-data-part-transcript-contract`: runtime exports `AiSdkAnswerEnvelopeDataPart`, `ChatTranscriptJsonlEvent`, `data-answer-envelope`, and `chat_transcript_jsonl@1`, with a matching transcript JSONL event schema. Closeout is added by `wave4/ch2-answer-envelope-contract`: schema-artifact pinning, malformed transcript/data-part rejection, and disabled SSE posture without answer-envelope emission. Live SSE orchestrator emission remains future CH3+ work once provider/search execution exists. | CH2 | CH1 |
| CH3 | `appfw_provider_ai_search` crate (plan-only) | CH3 | **Implemented in `codex/wave4-ai-search-provider`; delete after merge.** Executable request planning remains W3/CH9 gated. |
| CH4 | Generated addressing + `viewRegistry` emission (`frontend.rs`) | CH4 | **Implemented in `codex/wave4-view-registry` as the first contract slice:** generated `entity.addressing`, route-safe record locator id metadata, locator resolve op names, `viewRegistry.supports.idKinds`, and CRM scaffold consumption. Delete after merge; richer FlowGraph/timeline registrations and filtered-list URL helpers remain future CH4/CH8 work. |
| CH5 | PDS **Conversation** family + `FlowGraphShell` (`@xyflow/react`) — **first component-family slice implemented in `codex/wave4-conversation-family`**: PDS exports/catalog/checker/static/interactive coverage for MessageThread, Message, MessageComposer, StreamingText, ToolCallStatus, EntityRefCard, CitationList, ConfidenceSignal, AgentTimeline, and FlowGraphShell; **`AgentTimeline`, never "activity"**. First static consumer-wiring proof is in `codex/wave4-nexus-chat-pilot`. Component-family Playwright/axe evidence is implemented in `codex/wave4-ai-a11y`. Markdown/sanitizer decision evidence is added by `wave4/ch5-markdown-sanitizer`: `StreamingText` will use `react-markdown` + `rehype-sanitize`, with `conversation_markdown_sanitizer.live_ready:false` until implementation and release evidence exist. FlowGraph token bridge evidence is added by `wave4/ch5-flow-token-bridge`: `.pds-flow-graph-shell__viewport` publishes PDS-backed `--xy-*` variables and `flow_graph_token_bridge.adapter_ready:false`. Remaining work: assistant-ui/AI SDK adapter, React Flow adapter proof, product-level visual/a11y evidence, and live runtime wiring. | CH5 | CH2 |
| CH6-slice2 | Register `product chat-eval` cmd + docs-check assertions + release-evidence category | CH6 | **Implemented in `wave4/chat-eval-command`; wave-status consumption is implemented in `wave4/ch6-wave-status`; promptfoo PIPELINE plan evidence is added by `wave4/ch6-promptfoo-pipeline` via `scripts/appfw product chat-eval --json --pipeline-plan`; managed judge/live evidence is now fail-closed behind `APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE` / `APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE`. The promptfoo plan remains offline posture evidence; remaining work is producing real judge/live evidence after a certified AI gateway/search provider exists.** |
| CH7 | LiteLLM gateway + AI auth — **first contract slice implemented in `codex/wave4-ai-gateway-auth`**: `GatewayAuthEvidence`, in-perimeter LiteLLM-preferred posture, server-side secret-ref custody, client-credentials token-cache key shape, prompt-audit/SIEM/retention dependency, egress allowlist, policy re-resolution, and provider-test plan evidence. The decision-evidence slice adds `framework ai-gateway-decision`, retaining the `appfw.ai-gateway-decision.v1` managed evidence contract and fail-closed enforce mode for gateway/model selection, LiteLLM-or-equivalent license posture, credential custody, egress approval, prompt-audit dependency, policy-boundary ownership, and owner approvals. Remaining: live gateway/backend swap demo, stakeholder-provided decision evidence, and secret-scan evidence. | CH7 | CH1, CH3 |
| CH8 | Nexus W3-C stalled-tasks pilot — **first static slice implemented in `codex/wave4-nexus-chat-pilot`**: `examples/products/pds-nexus` uses synthetic 176-task/20-team De Novo source evidence, renders the stalled-task answer with PDS Conversation components, resolves record-locator refs into `EntityRefCard`s, opens a `GeneratedViewShell`/`FlowGraphShell`, records evidence/citation/freshness/attribution surfaces, and keeps write-shaped UX preview-only. Component-family visual/a11y evidence is implemented in `codex/wave4-ai-a11y`; Nexus product-level visual/a11y evidence is implemented in `codex/wave4-nexus-a11y` via `frontend npm run appfw:evidence`; local UX metric baselines are implemented in `codex/wave4-nexus-ux-metrics` via `frontend npm run appfw:ux-metrics`. Remaining: live chat/gateway/search wiring and live product evidence. | CH8 | CH1–CH7 |
| CH-AMBIENT | Ambient AI PDS component family — **first component-family slice implemented in `codex/wave4-ambient-ai-family`**: `GeneratedViewShell`, `SuggestedAction`/`RecommendationCard`, `EvidenceSummary`/`InsightSummary`, `FreshnessIndicator`, `AttentionMarker`, and `AiAttributionAffordance`; **autonomy/memory slice implemented in `codex/wave4-ambient-memory-controls`**: `AssistLevelControl` and `MemoryChip`; catalog/checker/static/interactive coverage plus grounding/preview-gating/attribution/autonomy/memory evidence. First static consumer-wiring proof is in `codex/wave4-nexus-chat-pilot`; component-family Playwright/axe evidence is implemented in `codex/wave4-ai-a11y`; product-level visual/a11y evidence is implemented in `codex/wave4-nexus-a11y`. Remaining: live eval proof. | `agentic-ux.md` | CH2, CH4, CH5 |

### Wave 3 (strategic) — full set in `roadmap.md` North-Star Execution Plan
G1 delegated-auth + governed-write substrate, G3/U2 security-by-design + harness
profile, G2 primitives, U5 mobile, U3 PoC/legacy, U6 PDS-from-source, U7 build
modularity, G4 provenance. **Wave 0 must be settled first.**

### Threat-litmus remediations (`docs/architecture/concerns/threat-model-litmus.md`)
Rollup 2026-07-02: **14 PASS / 26 PARTIAL / 39 FAIL** across S1–S10. Every
FAIL/PARTIAL row is a work item with a re-runnable assertion; move a row to PASS
only with a retained evidence artifact. Surface posture:

| Surface | Net | Ties to |
| --- | --- | --- |
| S7 core multi-tenant backend | 4P/3PART/0F — strongest | live negative tests (SA-01..09) |
| S8 supply chain | 3P/2PART/2F | AIBOM report/enforce slice, provenance/signing |
| S9 PHI/classification | 2P/2PART/3F | DP-governance, DATA-AI-STD-*; first PHI-pipeline evidence contract in `codex/wave4-phi-pipeline-governance` |
| S10 observability/audit | 2P/1PART/4F | SIEM export, prompt-log, kill-switch |
| S1 governed write/deleg. auth | 0P/2PART/4F | **W3-B, G1** |
| S3 kappa/CDC | 0P/3PART/3F | DP1–DP4 |
| S4 mobile | 0P/3PART/3F | U5 device evidence; research direction is settled |
| S6 agentic build | 0P/4PART/3F | AIBOM report/enforce slice, prompt-audit report/enforce slice, attribution |
| S5 AI-service egress | 0P/1PART/4F | CH7 gateway/DLP |
| S2 AI chat + prompt injection | 0P/0PART/7F — all FAIL by design | **CH1–CH9** |

**Top-6 highest-leverage gaps (must-close-before-graduation), each already a lane above:**
1. **W3-B delegated/on-behalf-of auth** — gates S1, S2-writes, all "AI acts for a user" (→ W3-B; human-sequenced).
2. **SaaS executor is substrate-only** — `RuntimeHttpSaasRequestExecutor` exists, but provider live bindings, governed-write request conversion, and sync-worker projection execution remain gated (→ W3-A/DP/SaaS executor lane, CH-adjacent).
3. **AIBOM + agent-SBOM + PR-trailer attribution partly executable**:
   `scripts/appfw framework aibom-check --json` now retains local
   SEC-AIBOM posture and `--enforce` fails closed without a release
   attestation; managed CI still owes AIBOM generation and PR-trailer
   validation (→ SEC-AIBOM; AI-STD-003/004, AGENT-STD-005; ties S6/S8).
4. **PHI-pipeline governance live evidence still absent** — first in-repo evidence schema and fail-closed `governance-check` contract lands in `codex/wave4-phi-pipeline-governance`; DP4 now adds a local freshness/lineage posture report, but managed release still owes de-id, lower-environment movement, RAG curation, retention, deletion/tombstone, redaction, and audit-lineage evidence (→ DP-governance / G4; DATA-AI-STD-001..004).
5. **Prompt-audit logging/retention + SIEM export + agent kill-switch first local gate exists; managed evidence absent** — `framework prompt-audit-check --json` now reports SEC-PROMPTAUDIT posture and `--enforce` fails closed until managed prompt-audit/SIEM/retention/kill-switch evidence exists (→ S10 lane; AI-STD-005, AGENT-STD-006).
6. **Prompt-injection defenses forward-only** — assertions fixed now, enforce when CH1 lands (→ CH1–CH5).

Gap 3 now has the first executable SEC-AIBOM report/enforce slice, and Gap 5
now has the first executable SEC-PROMPTAUDIT report/enforce slice. Managed-CI
SEC-AIBOM production and managed prompt-audit/SIEM release evidence remain
external release-authority work.

## NOT Agent Work — Owed By Humans (external/decision)

- Internal PDS AI search service API spec/export (unblocks CH3/CH9).
- ServiceNow dev instance + authenticated export (Unit A → G1 chain, live reads).
- Managed provider backends + `LOCALSTACK_AUTH_TOKEN` + release authority (P1–P4).
- Model/gateway selection sign-off; LiteLLM license verification.
- HITL risk-tier thresholds (AGENT-STD-007), prompt-audit platform (AI-STD-005 /
  CODE-TPC-004), managed AIBOM tooling and PR-trailer enforcement
  (AI-STD-004 / CODE-TPC-005), PHI de-identification sign-off
  (DATA-AI-STD-002), and any other litmus "owed-by-humans" gaps.
- W3-B delegated/on-behalf-of auth is a framework lane, but it is human-sequenced
  and gates every chat-initiated write.

## Reserve Human/Reviewer Tokens For
Design decisions not yet made · adversarial review of agent PRs (fresh-context
reviewer per PR) · the architecture artifact (parked) · reconciling the two
existing threat models with the litmus.
