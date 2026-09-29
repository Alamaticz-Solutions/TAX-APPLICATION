# App Framework Agent Skills

This directory is the repo-native skill pack for agents building products with
App Framework and for framework stewards evolving App Framework itself. Skills
are short operating procedures, not replacement docs.
Use one skill for the current workflow, then open only the linked canonical
docs needed for detail.

The information model is progressive disclosure:

1. `agent_skills/README.md` routes the job.
2. A skill `SKILL.md` gives the concise procedure, proof commands, and
   guardrails.
3. Canonical docs and generated artifacts provide deep guidance only when the
   task needs it.

The skill pack is a team-replicable harness surface. See
`docs/start/team-harness-replication.md` and
`docs/start/agent-role-cards.md` before adding local-only prompts,
machine-specific slash-command assumptions, role instructions, or workflows that
cannot be recreated from a fresh checkout plus approved package feeds.

For Class D delivery work, use `scripts/appfw mode status --json` to reveal the
current worktree's local delivery profile before selecting proof depth. The
root-only mode controller's tracked policy and worktree-local ledger, stored
below each worktree's Git directory, are described in
`docs/start/delivery-profiles.md`.

Do not turn skills into another documentation layer. If a skill grows past a
small operating procedure, move the detail back to the canonical doc it links.

## Skill Router

### Product Skills

Canonical CLI namespace: `scripts/appfw product ...`

PoC and legacy intake skills both use `scripts/appfw product analyze --summary --json`,
then `scripts/appfw product propose-model --summary --json` before final modeling.
Run `scripts/appfw product model-status --json` after proposal review and after
each modeling checkpoint. The analysis command retains
`target/appfw/product-analysis.json` and writes the source-specific review YAML
under `.appfw/`; the proposal command writes `target/appfw/model-proposal.json`
and `.appfw/model-proposal.yaml` as review-only model evidence. The status
command writes `target/appfw/model-status.json` with phase, blockers, and next
commands. The proposal `review_decisions` ledger preserves unresolved
(`needs_review`, `needs_discovery`) and terminal (`accept`, `reject`, `defer`,
`split`, `merge`) choices before final model source is written. After signed
review, `scripts/appfw product scaffold-model --dry-run --json` can preview
starter source files from accepted entity decisions; it is a bootstrap helper,
not an LLM reasoning substitute.

| Job | Skill | Primary Docs |
| --- | --- | --- |
| Create a product repo with clean product intent | `product-bootstrap` | `docs/lifecycle/product-golden-path.md`, `docs/start/cli-quickstart.md` |
| Convert a citizen-developed PoC into a product plan | `product-poc-intake` | `docs/lifecycle/intake-and-discovery.md` |
| Modernize a legacy application into a governed product | `product-legacy-modernization` | `docs/lifecycle/legacy-modernization.md` |
| Design the entity model and config source | `product-schema-modeling` | `docs/model/schema-design.md`, `docs/start/generated-ownership.md` |
| Generate artifacts and prove deterministic output | `product-generate-verify` | `docs/start/generated-ownership.md`, `docs/reference/cli.md` |
| Run locally and test with live services | `product-local-run-test` | `docs/lifecycle/application-lifecycle.md`, `docs/release/deployment-reference.md` |
| Build an enterprise product frontend | `product-frontend` | `docs/frontend/product-frontend.md` |
| Convert a mobile HTML mockup into a React Native app | `product-mobile-react-native` | `docs/frontend/mobile-react-native.md`, `docs/frontend/product-frontend.md` |
| Prove least-privilege product-agent scope | `product-agent-harness` | `docs/architecture/concerns/agentic-threat-model.md`, `docs/reference/cli.md` |
| Prepare release evidence and handoff | `product-release-evidence` | `docs/release/release-gate-ci-cd.md`, `docs/release/deployment-reference.md` |
| Prepare PDS Health SRA package evidence | `/pds-sra-package --product`, `pds-sra-package` | `docs/start/pds-sra-package-harness.md` |
| Run the Product PR Review Agent before human approval, with focused review for narrow work and comprehensive/adversarial review for broad or sensitive work, or audit its performance with `/check-pr-review-performance` | `/product-pr-review`, `/product-pr-review --comprehensive`, `product-pr-review` | `docs/start/product-pr-review-agent-harness.md`, `docs/start/agent-task-map.md` |
| Upgrade a product app across framework changes | `product-upgrade` | `docs/lifecycle/application-lifecycle.md` |
| Preserve product continuation state | `product-handoff` | `docs/start/agent-task-map.md` |

### Framework Steward Skills

Canonical CLI namespace: `scripts/appfw framework ...`

| Job | Skill | Primary Docs |
| --- | --- | --- |
| Plan a meaningful framework/product change with the right spec depth and decision provenance | no dedicated skill; use the harness doc | `docs/start/spec-driven-change-harness.md`, `docs/specs/spec-template.md` |
| Maintain docs IA, skills, and command contracts | `framework-docs-ia` | `docs/framework/README.md`, `docs/architecture/concerns/maintainability.md` |
| Review docs/skills/CLI/code organization entropy and recommend Product Owner/Strategist maintenance | `framework-structure-steward` | `docs/architecture/concerns/maintainability.md`, `docs/start/team-harness-replication.md`, `docs/start/branch-integration-model.md` |
| Research external market/industry signals and challenge Strategist/Product Owner guidance | `/framework-research-refresh`, `framework-research-steward` | `docs/start/framework-research-steward-harness.md`, `docs/strategy/app-framework-product-management-strategy.md` |
| Capture or retire review conditions and technical debt | `tech-debt-steward` role guidance | `docs/start/tech-debt-register.md`, `docs/start/agent-role-cards.md` |
| Steward PDS Health frontend design system | `framework-frontend-design-system` | `docs/frontend/pds-health-design-system.md`, `docs/architecture/adr/0008-frontend-design-system.md` |
| Change generator or template behavior | `framework-generator` | `docs/reference/codegen-api.md`, `docs/start/generated-ownership.md` |
| Change runtime ingress or operation dispatch | `framework-runtime-ingress` | `docs/architecture/overview.md`, `docs/runtime/mcp.md` |
| Certify provider semantics and performance | `framework-provider-certification` | `docs/runtime/provider-certification.md`, `docs/runtime/provider-sdk.md` |
| Add or change MS SQL / Fabric `auth_mode` (NTLM, SQL password, Entra) | no dedicated skill; use the shared registry and ADR | `appfw_mssql_auth`, `docs/architecture/adr/0018-provider-auth-mode-plurality.md`, `docs/specs/mssql-odbc-ntlm-authentication.md`, `docs/runtime/provider-sdk.md` |
| Certify framework release readiness | `framework-release-certification` | `docs/release/release-gate-ci-cd.md`, `docs/release/deployment-reference.md` |
| Maintain dependency and vulnerability posture | `framework-dependency-maintenance` | `docs/reference/cli.md` |
| Prepare platform and all-product PDS Health SRA package evidence | `/pds-sra-package --framework`, `/pds-sra-package --all-products`, `pds-sra-package` | `docs/start/pds-sra-package-harness.md` |
| Run the Framework PR Review Agent before human approval, with focused review for narrow work and comprehensive/adversarial review for broad or sensitive work, or audit its performance with `/check-pr-review-performance` | `/framework-pr-review`, `/framework-pr-review --comprehensive`, `framework-pr-review` | `docs/start/pr-review-agent-harness.md`, `docs/architecture/concerns/agentic-development-control-system.md` |
| Preserve framework continuation state | `framework-handoff` | `docs/start/agent-task-map.md` |

## Rules

- Before material work, resolve the current task's bounded instruction set and
  independent review route with `scripts/appfw framework instructions --task
  <route> --role coding-agent --change-class <A|B|C|D> --json`, or the product
  namespace for an explicitly supported `product-*` route. The command selects
  one role card, one skill, and source-linked canonical references; it does not
  start work, change authority, or certify its own route.
- Load the smallest skill that matches the task.
- Choose product or framework audience before choosing a skill.
- Treat `SKILL.md` as the procedure and the linked docs as the reference.
- Keep product-specific decisions in the product repo, not in this skill pack.
- Update `scripts/appfw framework docs-check --json` coverage when adding a
  skill or changing a skill command contract.
- Every slash-command workflow must have a tool-neutral skill, doc, or CLI
  fallback so teams using Codex, Claude, Cursor, Copilot, or another harness can
  reproduce the same behavior.
- Do not add extra files under a skill directory unless they are required
  assets, scripts, or references for that exact skill.
- Prefer links to existing docs over new prose. Deep guidance belongs in the
  canonical docs, generated contracts, or retained CLI artifacts.
