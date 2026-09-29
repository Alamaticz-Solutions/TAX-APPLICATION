# Agent Task Map

Use this map before making changes. It gives agents and developers a quick
route from intent to edit surface, verification, and handoff notes.

## First Commands

```bash
scripts/appfw doctor
scripts/appfw context --json
scripts/appfw framework instructions --task framework-docs-ia --role coding-agent --change-class C --json
scripts/appfw product validate --json
scripts/appfw product boundary-check --json
```

`validate` emits validation and config-contract reports. It does not regenerate
backend, database, or API test artifacts.
`boundary-check` verifies product handler/service extension boundaries without
compiling the generated server crate.

Use `scripts/appfw` rather than invoking `app_gen` directly unless you need to
test root resolution. The wrapper passes the app, framework, config, template,
and report roots explicitly.

For procedural guidance, load the smallest matching skill from
`agent_skills/README.md`. Skills route to the docs below; they do not replace
the canonical contracts.

For material work, run the namespaced `instructions` command with the assigned
task, role, and exact change class before loading that skill. Its
`appfw_instruction_route@1` envelope reveals one producer role card, one skill,
minimal source-linked references, and a separate independent-review route. It
does not start work, alter authority, or self-certify. Unknown, mismatched, or
ambiguous routing inputs fail closed.

For a Class D delivery checkpoint, inspect the root-only local profile with
`scripts/appfw mode status --json`. `accelerated` records focused routing and
`candidate` records enforce-candidate routing. The profile annotates evidence;
it does not alter review, merge, or release authority.

For concurrent roadmap work, use
[Branch Integration Model](branch-integration-model.md) before creating or
merging multiple lane branches. Integration branches group related lanes,
resolve conflicts once, and keep leaf branches from becoming stale PR traps.
That guide also owns branch naming: use work-stream prefixes rather than
assistant/tool prefixes.

For broad, sensitive, or multi-domain work, first classify the change with
[Agentic Development Control System](../architecture/concerns/agentic-development-control-system.md).
Run `scripts/appfw framework change-impact --json` to retain the impact report;
if it cannot run, apply that guide's human-oversight triggers manually and call
out the class in handoff.

For meaningful framework or product changes, use
[Spec-Driven Change Harness](spec-driven-change-harness.md) to choose the
lightest useful spec depth before implementation. Tiny low-risk fixes need only
an intent note. Focused features/workflows need a lightweight spec when scope or
acceptance could be misunderstood. Framework contracts, generated output,
security/privacy, SaaS/provider integrations, CI/release gates, SRA/CAB, broad
UX/product behavior, or multi-agent coordination need a durable spec and
decision provenance.

For review-only work, invoke `/framework-pr-review` or `/product-pr-review`, or
use the matching review skill. Focused mode is the default. For pre-push, PR, or
integration-train review, first run `review-brief --auto-depth`, then invoke the
focused or comprehensive slash command named by the retained brief. Review agents
report findings, strategic significance, alignment drift assessment, independent
quality/architecture assessment, and a human approval brief; they do not edit
files unless the human explicitly switches the task to implementation. When a
spec was required, review must check spec-to-diff-to-evidence alignment.
The configured independent review is standing-authorized for repository source
and sanitized evidence; do not insert a per-review human permission gate.

When the human invokes `/check-pr-review-performance` or says
`check PR review performance`, use the PR Review Performance Oversight procedure
in [Framework PR Review Agent](pr-review-agent-harness.md) to assess whether the
review agent followed the contract, found material risk, used evidence honestly,
analyzed alignment drift, and helped the human approve wisely.

When a review returns `GO WITH CONDITIONS`, a check is skipped with future
impact, or repeated CI/review/credential friction appears, use the
[Tech Debt Register](tech-debt-register.md). Entries need owner, impact,
evidence, and retirement criteria; do not bury conditions in chat history.

Before pushing any agent-authored branch, the coding/architecture agent must run
the appropriate Framework or Product PR Review Agent workflow with
`review-brief --auto-depth`, invoke the focused or comprehensive slash command
that the retained brief requires, present the structured output and review
artifact path to the human, and push only when the standing approval policy in
[Agent Role Cards](agent-role-cards.md) applies or explicit human approval is
recorded.

## Golden Product Routes

For downstream product lifecycle work, start from
[Product Developer Golden Path](../lifecycle/product-golden-path.md), use
[Application Lifecycle](../lifecycle/application-lifecycle.md) for evidence depth, and keep
the command path reviewable:

| Route | Minimum Loop | Handoff Note |
| --- | --- | --- |
| Create app | `scripts/appfw product new --list-profiles --json`; `scripts/appfw product new <target> --from current --profile <profile> --generate --json`; validate; handoff | Name profile, template, overlay, framework source, `--generate`, and `appfw.lock`. |
| Convert citizen PoC | `scripts/appfw product new <target> --profile product-intake` with app/schema/provider/MCP/Kafka/UI answers; run `scripts/appfw product analyze --summary --json`; run `scripts/appfw product propose-model --summary --json`; run `scripts/appfw product model-status --json`; inspect PoC workbook/HTML/static data; refine `.appfw/poc-intake.yaml`, `.appfw/poc-analysis.yaml`, and `.appfw/model-proposal.yaml`; after signed review run `scripts/appfw product scaffold-model --dry-run --json` to preview starter source; draft/complete `.appfw/model` product schema; re-run `model-status`; validate; generate/check/test; frontend evidence when UI is in scope | Name source artifacts, required topology answers, inferred entities/relationships/enums, data classification assumptions, current model-status phase, scaffold-model result if used, and how CRM reference schema/UI informed the enterprise implementation. |
| Modernize legacy app | `scripts/appfw product new <target> --profile product-intake --source-kind legacy` with legacy codebase and provider answers; run `scripts/appfw product analyze --summary --json`; run `scripts/appfw product propose-model --summary --json`; run `scripts/appfw product model-status --json`; complete `.appfw/legacy-modernization.yaml` and review `.appfw/legacy-analysis.yaml` plus `.appfw/model-proposal.yaml`; inspect code, database, procedures, jobs, auth, integrations, and UI; create modernization plan; after signed review run `scripts/appfw product scaffold-model --dry-run --json` to preview starter source; draft model/services/frontend; re-run `model-status`; validate/generate/check/test; migration/API/frontend evidence when in scope | Name legacy evidence reviewed, modernization slice, stored procedure disposition, target entities/services/custom methods, migration posture, current model-status phase, scaffold-model result if used, and deferred parity/security/performance risks. |
| Convert mobile mockup | `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`; read `docs/frontend/mobile-react-native.md`; map screens/actions to `.appfw/model`, generated operations, product services, or integration boundaries; build product-owned `mobile/` RN screens with native navigation; validate/generate-check; run RN type/tests/Expo checks when mobile source is present; handoff | Name mockup hash/report path, navigation archetype, mapped workflows, native capabilities used, data-classification/offline decisions, and mobile checks run or skipped. |
| Customize app | `scripts/appfw product explain ownership <path>`; `scripts/appfw product validate --json`; `scripts/appfw product boundary-check --json`; focused tests | State product-owned files changed and whether generation was intentionally skipped. |
| Regenerate artifacts | `scripts/appfw product validate --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json`; tests | Preserve generated drift diagnostics and artifact provenance. |
| Upgrade framework | `scripts/appfw product upgrade --json`; generate/check/test loop; `scripts/appfw product lock --write`; final `scripts/appfw product upgrade --json` | Attach initial/final upgrade reports, generated diff decision, and lock refresh. |
| Prove migration | `scripts/appfw product migrate plan --json`; `scripts/appfw product migrate lint --phase all --json`; `scripts/appfw product migrate drift --json`; `scripts/appfw product migrate rollback-guide --json` | Name unavailable providers and rollback constraints when live drift cannot run. |
| Release/handoff | risk-appropriate validation, test, API, migration, frontend, or provider evidence; `scripts/appfw product handoff --json` | Keep skipped live checks, security impact, and remaining risks explicit. |

Each route is review-ready only when its touched lifecycle stages map to the
executable acceptance gates in
[Application Lifecycle](../lifecycle/application-lifecycle.md): command bundle, JSON
artifact or stdout payload, pass/fail result, and skipped-check reason when a
live dependency is unavailable. `scripts/appfw framework docs-check --json` also writes
`target/appfw/lifecycle-evidence-checklist.json`; use it as the CI-retained
index from route names to concrete commands and artifacts.

## Task Routing

| Intent | Edit Surface | Main Verification | Notes |
| --- | --- | --- | --- |
| Reveal only the instructions and agent route needed for the current material task | `appfw_cli/src/instruction_routing.rs`, `agent_skills/README.md`, role cards, canonical task docs | `scripts/appfw framework instructions --task <framework-route> --role coding-agent --change-class <A|B|C|D> --json`; use `product` only for supported `product-*` routes; `scripts/appfw framework cli-test --json` | XO/dispatcher supplies the assigned route boundary. The command returns source links and distinct provider-neutral implementation/review profiles without starting work or granting authority. Class C requires comprehensive independent review; invalid combinations return nonzero parseable JSON. |
| Plan a meaningful framework/product change with decision provenance | `docs/start/spec-driven-change-harness.md`, `docs/specs/spec-template.md`, durable specs under `docs/specs/`, product-owned specs under product `.appfw/specs/` or `docs/specs/`, handoff/review artifacts | Intent note for tiny fixes; spec path named in handoff; `scripts/appfw framework validate --json`; namespace-specific handoff; PR review checks spec-to-diff-to-evidence alignment | Use the lightest useful spec depth. Product Owner owns value/acceptance, Architect owns technical approach, XO owns coordination/parked decisions, and Review Agent verifies the branch implements the accepted intent without hidden expansion. |
| Convert a citizen-developed PoC app into an enterprise product | `.appfw/poc-intake.yaml`, `.appfw/poc-analysis.yaml`, `.appfw/model-proposal.yaml`, `.appfw/manifest.yaml`, `target/appfw/product-analysis.json`, `target/appfw/model-proposal.json`, `target/appfw/model-status.json`, `target/appfw/model-scaffold.json`, `.appfw/model/schemas/<schema>`, product `frontend/`, product services/handlers when needed | `scripts/appfw product new <target> --profile product-intake`; `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product scaffold-model --dry-run --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json`; `scripts/appfw product test --fast`; frontend checks when UI is in scope | Start from `docs/lifecycle/intake-and-discovery.md`. The intake command must capture backend provider (`PostgreSQL` for the current runnable scaffold), MCP needed, Kafka needed, and UI mode (`none`, `scaffold`, or `enterprise`). Inspect workbook/static data/HTML as source evidence. Use CRM schema and CRM frontend as references only; do not leave CRM as the target product model. |
| Modernize a legacy application into an enterprise product | `.appfw/legacy-modernization.yaml`, `.appfw/legacy-analysis.yaml`, `.appfw/model-proposal.yaml`, `target/appfw/product-analysis.json`, `target/appfw/model-proposal.json`, `target/appfw/model-status.json`, `target/appfw/model-scaffold.json`, legacy source inventory, read-only data-source inventory, stored procedure disposition matrix, `.appfw/manifest.yaml`, `.appfw/model/schemas/<schema>`, product services/handlers, product frontend, migration evidence | `scripts/appfw product new <target> --profile product-intake --source-kind legacy`; `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product scaffold-model --dry-run --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; `scripts/appfw product test --fast`; migration/API/frontend/release checks as scope requires | Start from `docs/lifecycle/legacy-modernization.md`. Analyze code and data before modeling. Decompose large stored procedures into generated query/CRUD, product services, custom methods/DTOs, or explicitly retained provider routines with exit criteria. |
| Add or update an agent skill | `agent_skills/README.md`, `agent_skills/<workflow>/SKILL.md`, `docs/architecture/concerns/maintainability.md`, `scripts/check-doc-examples.sh` | `scripts/appfw framework docs-check --json`; `scripts/appfw framework validate --json`; `scripts/appfw framework handoff --json` | Keep the skill concise and procedural. Deep guidance belongs in canonical docs and generated artifacts; docs-check should guard skill discoverability. |
| Change app identity or topology assertions | `.appfw/manifest.yaml` | `scripts/appfw product topology --json`; `scripts/appfw product validate --json` | The manifest lists schemas/data sources. It does not replace schema, entity, relationship, seed, or data-source environment config. |
| Change local compose providers or dev infra | `.appfw/manifest.yaml`, `.appfw/model/data_sources/_res.yaml` | `scripts/appfw product topology --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json` | `podman-compose.yml` is generated from topology and compose-capable data-source environments. Do not hand-trim provider services. |
| Add schema, entity, property, enum, relationship, seed, or API scenario | `.appfw/model` | `scripts/appfw product validate --json`; `scripts/appfw product test --fast` | Generate only when the task requires updated artifacts. |
| Change config validation or the generated config contract | `app_gen/src/config_contract.rs`, `app_gen/src/validation.rs` | `scripts/appfw framework validate --json` | Do not hand-edit `.appfw/model/_specs/CONFIG_CONTRACT.md`. |
| Change generated backend, database, or API test shape | `app_gen/_templates`, `app_gen/src` | `scripts/appfw framework validate --json`; `scripts/appfw framework generate`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test` | Report generated drift instead of hiding it. |
| Change independently loaded runtime ingress services | `appfw_runtime/src/host.rs`, `appfw_runtime/src/ingress.rs`, `appfw_runtime/src/operation.rs`, `appfw_runtime/src/routing.rs`, `appfw_runtime/src/mcp`, `appfw_runtime/src/kafka.rs`, generated backend host templates | `scripts/appfw framework validate --json`; `scripts/appfw framework feature-check --json`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test`; provider/security evidence when release-relevant | HTTP, MCP, and Kafka are independently loadable modules. Keep transport-specific work in the ingress module and shared invocation through `RuntimeOperationDispatcher`; do not make one ingress depend on another. |
| Move backend runtime/framework internals toward packages | `docs/framework/README.md`, `docs/architecture/concerns/runtime-modularity.md`, `docs/architecture/concerns/packaging.md` | `scripts/appfw framework validate --json`; `scripts/appfw framework boundary-check --json`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test --fast --json` | Move runtime contracts before slimming CRM or downstream templates. Keep generated adapters and product services distinct from reusable framework behavior. |
| Implement app-specific resolver behavior | Product-owned handler implementation files under `backend/src/handlers/<schema>/<entity>.rs`; shared domain logic under `backend/src/services` | `scripts/appfw product validate --json`; `scripts/appfw product boundary-check --json`; `scripts/appfw product test --fast` | Generated handler defaults live in `backend/src/handlers/<schema>/generated.rs`. Keep handlers thin and move durable workflows into services. |
| Change provider semantics | `backend/src/data/clients/<provider>` plus framework-owned provider certification tests under `api_tests/src/provider_*` | `scripts/appfw framework validate --json`; `scripts/appfw framework test`; `scripts/appfw framework provider-test --provider <provider>` when the backend is running | Keep PostgreSQL, MongoDB, MS SQL Server, and Snowflake parity explicit. Product scenario tests stay under `api_tests/src/schemas` and are run by `scripts/appfw product api-test`. |
| Add a new provider | Provider SDK surfaces listed in `docs/runtime/provider-sdk.md` | `scripts/appfw framework explain provider-sdk --json`; `scripts/appfw framework validate --json`; `scripts/appfw framework test`; `scripts/appfw framework provider-test --provider <provider>` | Declare every capability area before claiming support. |
| Change access-control behavior | Rego config, product `rego_test` fixtures/assertions, and `appfw_test/src/policy` for shared verifier semantics | `scripts/appfw product validate --json`; `scripts/appfw product policy-test --json`; `scripts/appfw product test` | Preserve deny-by-default and row-scope semantics. |
| Change the backend-hosted admin console | `admin_ui`, `backend/src/admin_ui.rs`; route mount in `app_gen/_templates/backend/routes/mod/_mod.j2` only when needed | `cd admin_ui && npm run build`; `scripts/appfw framework validate --json`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test` | Keep it runtime model-driven; do not generate entity-specific UI screens. `backend/admin_dist` is build output. See `admin_ui/README.md` for dev-server and build details. |
| Change the PDS Health design system or reusable frontend scaffold | `appfw_ui/pds_health`; for tokens, edit `appfw_ui/pds_health/tokens/tokens.dtcg.json` first and regenerate with `scripts/generate-pds-tokens.mjs`; `docs/frontend/pds-health-design-system.md`, `docs/architecture/adr/0008-frontend-design-system.md`, `agent_skills/framework-frontend-design-system/SKILL.md`, frontend scaffold generator/templates when behavior changes | For tokens: `node scripts/generate-pds-tokens.mjs --check --json`; `node scripts/check-pds-tokens.mjs --json`. Then `scripts/appfw framework docs-check --json`; `scripts/appfw framework validate --json`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework golden-downstream --json` when scaffold behavior changes | Keep the design system PDS Health branded, product scaffolds CRM-neutral, and CRM as reference/E2E fixture only. `pdsTokens.css` and `pdsTokens.ts` are generated outputs; do not edit them directly. |
| Build a product frontend, frontend starter, or CRM reference app | Product-owned `frontend/` workspace, frontend contract docs, PDS token usage, and generated UI contract scaffolding when isolated | Product frontend typecheck/lint/test/build when present; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; `scripts/appfw framework docs-check --json` for docs | Start from `docs/frontend/product-frontend.md`. Consume generated model/API contracts and PDS `--pds-*` tokens; do not fork the admin UI into a product app. |
| Convert a mobile HTML mockup into a React Native product app | Product-owned `mobile/` workspace, `mobile/.appfw-mobile/ownership.json`, generated/mobile contract docs, PDS native token adapter, `app.json`, `eas.json`, and retained `.appfw/target/appfw/mobile-rn-conversion-plan.json` | `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; RN type/test/Expo/device checks when mobile source is present | Start from `docs/frontend/mobile-react-native.md` and the `product-mobile-react-native` skill. Mobile follows workflow parity with web, not layout parity; do not copy DOM/CSS/mock data from the HTML artifact. |
| Package the product SPA into the backend image | Product `frontend/`, product `backend/product_dist`, deployment docs, backend static UI config | `cd frontend && npm run build`; `scripts/appfw product validate --json`; `scripts/appfw product handoff --json`; release pipeline image/SBOM evidence when production-bound | `backend/product_dist` is build output for the product SPA. The backend serves it at `/` when `APP_PRODUCT_UI_ENABLED=true`; keep API/admin/health/metrics/readiness/MCP paths backend-owned. |
| Change the model-driven MCP surface | `backend/src/mcp`, `app_gen/_templates/backend/mcp`, `backend/src/data/data_access.rs`, `appfw_runtime/src/security.rs`, and backend security facades; route mount in `app_gen/_templates/backend/routes/mod/_mod.j2` when needed | `scripts/appfw framework validate --json`; `scripts/appfw framework generate`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test` | Expose public handler-level operation names and keep MCP as a transport adapter over the same handler/DataAccess/QueryIR/policy path as GraphQL. See `docs/runtime/mcp.md`. |
| Plan downstream app creation, lifecycle proof, or framework upgrade | `docs/reference/product-workspace-contract.md`, `docs/lifecycle/application-lifecycle.md`, release notes, app repo upgrade branch | `scripts/appfw product new --list-profiles --json` for create; `scripts/appfw product upgrade --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json`; `scripts/appfw product test`; `scripts/appfw product api-test` when services are running | Keep upstream framework changes reviewable; preserve bootstrap JSON, generated drift diagnostics, migration evidence, product security evidence, and the product upgrade report. |
| Plan deployment or rollback | `docs/release/deployment-reference.md`, `database/MIGRATIONS.md`, deployment files | `scripts/appfw product migrate rollback-guide --json`; `scripts/appfw product validate --json`; deployment pipeline checks | Keep migrations separate from API rollout; contract migrations need explicit approval. |
| Diagnose a failed PR, `main`, or `v*` Bitbucket gate, or a ProGet publish question | `docs/release/README.md`, `scripts/ci/local-pre-push-gates.sh`, `scripts/ci/pr-fast-framework-check.sh`, `docs/release/release-gate-ci-cd.md`, `docs/release/proget-distribution.md`, retained `target/appfw` / pipeline artifacts | Reproduce the substantive local subset with `bash scripts/ci/local-pre-push-gates.sh` (the same Fast framework script CI runs); retain remote Bitbucket checkout/freshness/upload/overlap proof separately; do not treat focused `main` evidence as gated ProGet authority | Start from `AGENTS.md` → this row. Gate command lists live only in the shared CI scripts; release docs own lane authority meaning; admission/merge/release stay in role cards / Nexus PFC (flow only). |
| Change release, CI, or GitOps promotion gates | `bitbucket-pipelines.yml`, `scripts/ci`, `docs/release/release-gate-ci-cd.md` | `scripts/appfw framework validate --json`; shell syntax checks; Bitbucket custom `release-check` pipeline | ArgoCD should promote only immutable artifacts that passed the Bitbucket release gate. |
| Build or publish framework artifacts for product teams | `scripts/appfw`, `appfw_cli`, `app_gen`, `database`, `docs/release/proget-distribution.md`, `docs/architecture/framework-packaging.md` | `scripts/appfw framework package --plan --json`; `scripts/appfw framework package --json` in release CI; `scripts/appfw framework docs-check --json` | The framework package command produces a ProGet-ready Cargo crate publish plan, toolchain, binaries-only, and product-docs tarballs plus a manifest. CI owns the actual ProGet upload credentials, signing, and promotion. |
| Change maintainability policy, docs IA, CLI contract, architecture intent, or golden-path automation | `docs/architecture/concerns/maintainability.md`, `docs/README.md`, `docs/architecture/overview.md`, `scripts/check-doc-examples.sh`, `scripts/appfw`, `appfw_cli` | `scripts/appfw framework cli-test --json`; `scripts/appfw framework intake-proof --json`; `scripts/appfw framework validate --json`; `scripts/appfw framework docs-check --json`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test --fast --json`; `scripts/appfw framework handoff --json` | Use `cli-test` for fast command-routing feedback; use `intake-proof` when product bootstrap/intake behavior changes; keep docs layered, commands documented, JSON evidence retained, and runtime ingress paths converged through shared operation semantics. |
| Review structural entropy across docs, skills, CLI, and code organization | `agent_skills/framework-structure-steward/SKILL.md`, `docs/architecture/concerns/maintainability.md`, `docs/start/team-harness-replication.md`, `docs/start/branch-integration-model.md`, `target/appfw/change-impact.json`, handoff and docs-check artifacts | Review-only: `scripts/appfw framework change-impact --json`; `scripts/appfw framework docs-check --changed-only --json`; `scripts/appfw framework handoff --json`; add `cli-test`, `generate --check`, or `test --fast` only when the finding touches those surfaces | Use the Framework Structure Steward before broad integration PRs, after repeated CI/review failures, during wave refresh, or when docs/skills/CLI/code organization are drifting. It recommends Product Owner/Strategist maintenance items; it does not sneak broad cleanup into unrelated feature work. |
| Analyze workstream latency, CI duration, validation scope, parallelization, or token/output efficiency | `docs/start/agent-role-cards.md`, `docs/start/agentic-human-operating-model.md`, `docs/start/branch-integration-model.md`, Bitbucket pipeline evidence, local command timing artifacts, handoff/review artifacts | Review-only: inspect local command timings, `target/appfw` evidence, docs-check timing/subcheck artifacts, PR pipeline step durations, focused CI log edges, and thread/output patterns; run no broad checks unless needed to verify a claim | Use Workstream Analyst when a fast path is slow, polling/token burn is high, local proof loops are too broad, CI waits are opaque, or work could be safely parallelized. It recommends changes; XO/Product Owner/Architect/Integration route ownership. |
| Monitor a remote PR or pipeline train | `docs/start/remote-promotion-state-synchronization.md`, `docs/start/branch-integration-model.md`, `docs/start/agent-role-cards.md`, live Bitbucket PR/pipeline evidence | Integration observes every assigned train through terminal PR and post-merge destination evidence; emit machine-readable material transitions, refresh unchanged current-state freshness without history noise, and report stale evidence as `STALE` or `UNKNOWN` | Integration owns live remote truth; XO consumes each transition within one active monitoring cadence and refreshes board, queue, holds, and dashboard. This never grants push, merge, release, or accepted-risk authority. |
| Capture or retire technical debt from review conditions | `docs/start/tech-debt-register.md`, PR Review Agent output, Structure Steward brief, CI/pipeline artifacts, handoff and review-brief artifacts | Register-only unless implementation is assigned: verify evidence exists, owner is named, and retirement criteria are concrete; use namespace-specific handoff when source docs change | Use for `GO WITH CONDITIONS`, skipped checks with future impact, repeated review findings, recurring CI failures, stale evidence, or credential blockers. The register does not approve risk; it makes follow-up explicit. |
| `/framework-research-refresh` | `docs/start/framework-research-steward-harness.md`, `agent_skills/framework-research-steward/SKILL.md`, North Star, platform strategy, product-management strategy, roadmap, current market/industry sources | Research-only: read current strategy/product-owner intent, research reputable current sources, produce Research Verdict, Business Value Implications, Challenge To Current Guidance, Recommended Product Actions, Evidence Quality, and Anti-Fad Filter | Use the App Framework Research Steward before roadmap refreshes, major wave planning, or when AI/platform/security/UX/product-management signals may change priorities. It challenges Strategist/Product Owner guidance with evidence; it does not rewrite strategy, approve architecture, or push code. |
| `/framework-pr-review` or `/framework-pr-review --comprehensive` | `docs/start/pr-review-agent-harness.md`, `agent_skills/framework-pr-review/SKILL.md`, `docs/architecture/concerns/agentic-development-control-system.md`, retained evidence artifacts, PR summary | Review-only: `git diff --stat`; `git diff --name-status`; `git diff --check`; conflict-marker scan; `scripts/appfw framework review-brief --auto-depth --json`; `scripts/appfw framework handoff --json`; risk-appropriate checks from this map | Load `framework-pr-review` as the Framework PR Review Agent. Focused mode is default; auto-depth selects comprehensive when branch shape or sensitive surfaces require whole-branch review and cross-surface alignment. Start with Recommendation Summary final status and severity counts; for non-`GO` statuses include Attention Items naming conditions, blockers, evidence gaps, or review limits; then provide findings, strategic significance, alignment drift assessment, independent quality/architecture assessment, and a Human Approval Brief that names class, sensitive surfaces, evidence checked, residual risks, and decisions only the human should make. |
| `/product-pr-review` or `/product-pr-review --comprehensive` | `docs/start/product-pr-review-agent-harness.md`, `agent_skills/product-pr-review/SKILL.md`, product brief/workflow spec, retained evidence artifacts, PR summary | Review-only: `git diff --stat`; `git diff --name-status`; `git diff --check`; conflict-marker scan; `scripts/appfw product review-brief --auto-depth --json`; `scripts/appfw product handoff --json`; product validation/generation/test evidence as scope requires | Load `product-pr-review` as the Product PR Review Agent. Focused mode is default; auto-depth selects comprehensive when branch shape or sensitive surfaces require whole-product review, retained evidence, business workflow value, tenant/security posture, generated drift, and product/framework boundary alignment. Start with Recommendation Summary final status and severity counts; for non-`GO` statuses include Attention Items naming conditions, blockers, evidence gaps, or review limits before detailed findings. |
| `/pds-sra-package --framework`, `/pds-sra-package --product`, or `/pds-sra-package --all-products` | `docs/start/pds-sra-package-harness.md`, `agent_skills/pds-sra-package/SKILL.md`, product manifest/model evidence, release/security/ops evidence artifacts | `scripts/appfw framework sra-package --all-products --json`; `scripts/appfw product sra-package --json`; `scripts/appfw framework handoff --json` or `scripts/appfw product handoff --json` | Prepare report-only PDS Health SRA package evidence. State whether the package is ready, ready with explicit gaps, or not ready; do not claim SRA approval. Keep framework reusable controls separate from product-specific business purpose, owners, diagrams, data classification, vendors, service accounts, secrets, logging, monitoring, and risk-register inputs. |
| `/check-pr-review-performance` | `docs/start/pr-review-agent-harness.md`, prior Framework PR Review Agent output, PR/branch diff, `target/appfw/agent-handoff.json`, docs-check artifacts, CI/pipeline artifacts, follow-up comments or commits | Review-only oversight: inspect the review output, sample the diff and retained evidence, compare against the oversight output contract, and run focused commands only when needed to verify claims | Use PR Review Performance Oversight. Report a performance verdict, missed or overstated risks, contract compliance, evidence discipline, alignment drift quality, human usefulness, and system tuning actions. |
| Classify broad, sensitive, or multi-domain agentic work | `docs/architecture/concerns/agentic-development-control-system.md`, `docs/start/branch-integration-model.md`, PR summary, handoff evidence, retained `target/appfw/change-impact.json` | `scripts/appfw framework change-impact --json`; `git diff --check`; conflict-marker scan; risk-appropriate checks; `scripts/appfw framework handoff --json` | Treat changes as human-review-required when they cross the control-system thresholds, touch sensitive surfaces, promote capability readiness, or claim release/live evidence. Do not hide a broad diff behind a passing local gate. |
| Coordinate concurrent Product Increment delivery | `docs/specs/product-increment-portfolio.json`, `docs/start/product-increment-delivery-model.md`, active `appfw_product_increment_plan@1` records, `docs/start/branch-integration-model.md`, PR summaries, and integration branches | `node scripts/check-product-increment-portfolio.mjs --portfolio docs/specs/product-increment-portfolio.json --json`; `node scripts/check-product-increment-plan.mjs --plan <plan.json> --json`; before review, `node scripts/check-product-increment-plan.mjs --plan <plan.json> --current-diff <lane-id> --json`; `git diff --check`; conflict-marker scan; lane-focused checks; ordinary plan/portfolio edits use `scripts/appfw framework docs-check --fast --subcheck product-increment-delivery --json`; use `--changed-only` at Integration or for mixed authority surfaces; handoff when source changes are retained | Maintain two to four independently runnable Delivery Lanes across the portfolio, keep plain-language status and `requires`/`benefits_from` current, and use one integration branch for each multi-lane or cross-domain increment. A cohesive one-lane increment may declare `direct_main`. Bind the real candidate diff to its lane before review, resolve convergence once, and delete consumed leaves/worktrees after merge; strict release proof remains separate. |
| Improve docs, onboarding, or agent instructions | `../README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/README.md`, `docs/architecture/concerns/maintainability.md`, path-specific docs | stale-reference scan; `scripts/appfw framework validate --json`; `scripts/appfw framework docs-check --json`; `scripts/appfw framework handoff --json` | Refactor docs like code: route to the canonical owner, prune obsolete hot-path detail, and archive historical notes instead of duplicating contracts. |

## Generated Ownership

Use `docs/reference/product-workspace-contract.md` for the downstream product/framework
contract, `docs/reference/product-workspace-boundaries.md` for the current downstream app
ownership map, and `docs/start/generated-ownership.md` for detailed path-level rules.

Fast read-only explainers:

```bash
scripts/appfw product explain ownership <path>
scripts/appfw product explain config <path-or-key>
scripts/appfw framework explain provider <area>
scripts/appfw product topology --json
scripts/appfw product handoff --json
```

When `.appfw/target/appfw/artifacts.json` already exists, inspect it with:

```bash
scripts/appfw product manifest --json
```

Do not run `scripts/appfw product generate` only to discover ownership. Generation
writes artifacts and can obscure unrelated drift.

## Handoff Checklist

- Name the edit surface used and why.
- Name the spec path or intent note, or state why the change was tiny enough to
  avoid a durable spec.
- List verification commands and their result.
- Call out generated drift or skipped checks explicitly.
- Mention any broad dirty worktree context that was left untouched.
- Attach or reference `target/appfw/agent-handoff.json` when available.
- Before push, run `review-brief --auto-depth`, invoke the appropriate focused
  or comprehensive Framework/Product PR Review Agent workflow indicated by the
  brief, present the structured output and review artifact path, and push only
  when the standing approval policy in [Agent Role Cards](agent-role-cards.md)
  applies or explicit human approval is recorded.
