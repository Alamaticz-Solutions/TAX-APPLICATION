# PoC To Enterprise Product Intake

This Golden Path guide is for the common enterprise pattern where a business
citizen developer has created a useful proof of concept outside App Framework:
an Excel workbook, a static HTML app, a spreadsheet-backed dashboard, a
notebook, or another "vibe-coded" artifact.

The goal is not to copy the PoC into production. The goal is to extract product
intent, data shape, and user experience intent, then create an App Framework
product with a governed schema, generated backend, provider-aware data package,
enterprise frontend, tests, and release evidence.

A hosted, conversation-fronted delivery of this same path is specified as a
held target state in the
[Hosted Product Factory Spec](../specs/hosted-product-factory.md); this
document remains the canonical intake contract either way.

If the source is a real legacy application with code, a live or exported
database, stored procedures, jobs, integrations, and production behavior, use
[Legacy Application Modernization](legacy-modernization.md) instead. That path
adds code analysis, data discovery, stored procedure disposition, migration
evidence, and strangler/refactoring gates before generation.

## Operating Principle

Use the PoC as business evidence. Use the framework as the production chassis.

For this path, the CRM sample is a reference implementation only:

- use the CRM schema as an example of well-formed `.appfw/model`;
- use the CRM frontend as an example of model-driven enterprise UX, data grids,
  forms, dashboards, theming, and generated UI contracts;
- use the CRM docs and release artifacts as examples of evidence depth; and
- do not start by leaving the CRM data model in the target product repo.

## Intake Questions

Before generation, the developer or AI harness must capture an intake record.
The CLI should eventually prompt for these values or accept them as explicit
flags/non-interactive JSON:

| Question | Why It Matters |
| --- | --- |
| What is the app name and display name? | Drives `.appfw/manifest.yaml`, app identity, and generated labels. |
| What is the product schema name? | Creates the product-owned schema namespace, such as `equity`. |
| Which backend provider is needed now? | Choose the provider that owns the product schema. `PostgreSQL` remains the default CRUD scaffold. `FabricSqlAnalytics` is supported for external read-only Fabric reporting schemas and requires Entra auth plus Microsoft ODBC Driver 18 at runtime; the framework provider uses `odbc-api` with vendored unixODBC by default. MongoDB, MS SQL Server, and Snowflake remain framework-certified provider targets for provider-specific product scaffolds. |
| Is an MCP server needed? | Enables or defers the optional agent/tool ingress in `.appfw/manifest.yaml`. |
| Is a Kafka client needed? | Enables or defers event/message ingress and the later event contract work. |
| Is a product UI needed? | Must be `none`, `scaffold`, or `enterprise`; decides whether the AI harness creates frontend screens now. |
| Which artifact contains example data? | Gives the AI harness concrete workbook, CSV, embedded JSON, or static data to inspect. |
| Which artifact contains the PoC UI? | Gives the AI harness the HTML/dashboard/app to interpret as UX intent. |
| Is the PoC data synthetic, internal, PHI, financial, or regulated? | Drives classification metadata, release evidence, and handling constraints. |
| Which users/workflows does the PoC support? | Helps identify dashboards, forms, custom methods, and policies. |

The intake belongs in the product repo under:

```text
.appfw/poc-intake.yaml
```

The intake should name source artifacts by relative path, not embed large files
or secrets. Keep the first reviewable evidence set inside the product workspace,
normally under `.appfw/source-evidence/`, before asking an agent to analyze it.
If the original artifact lives in Downloads, Documents, email, a ticket, or a
shared drive, copy only the approved/redacted artifact or a minimal structured
summary into the product workspace and reference that relative path. Do not use
absolute local paths in intake files; they make agent runs slower, trigger
desktop filesystem prompts, and break CI reproducibility.

## Target CLI Behavior

Use `appfw product new --profile product-intake` for this path, not the CRM sample
profile. It is safe for a human terminal session and for non-interactive
agent/CI use:

```bash
scripts/appfw product new ../operations-insight \
  --from current \
  --profile product-intake \
  --app-name operations-insight \
  --display-name "Operations Insight" \
  --schema ops \
  --provider PostgreSQL \
  --mcp false \
  --kafka false \
  --ui enterprise \
  --poc-source ../operations-insight-poc \
  --data-artifact source-data.xlsx \
  --ui-artifact prototype.html \
  --json
```

Then profile the configured source artifacts:

```bash
scripts/appfw product analyze --summary --json
```

The analysis command writes `target/appfw/product-analysis.json` and
`.appfw/poc-analysis.yaml`. Treat those as review artifacts, not final model
source. They help identify candidate entities, relationships, lookups, DTOs,
custom methods, frontend routes, classification assumptions, and open
questions before the agent writes `.appfw/model`. The JSON report includes a
`model_clues` section with bounded, heuristic candidates from CSV/TSV headers,
OOXML workbook sheets and named tables, HTML forms/tables, structured exports,
and source artifact names. When workbook table ownership is clear, the
proposal keeps candidate fields under the candidate table entity; report-only
or prototype-only fields remain unassigned for review. Workbook analysis also
surfaces metadata-only complexity signals such as formula counts, hidden sheets,
pivot tables, charts, and data validations; treat those as calculation,
aggregation, dashboard, or open-question evidence, not automatic persisted
entities. Agents must verify those clues against the actual business semantics
before writing model source.
The `--summary` flag keeps terminal and agent output compact while retaining
the full report at `target/appfw/product-analysis.json`.

Then draft a review-only model proposal:

```bash
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
```

The proposal command reads `target/appfw/product-analysis.json` and writes
`target/appfw/model-proposal.json` plus `.appfw/model-proposal.yaml`. It does
not write final `.appfw/model`; it gives humans and agents a compact review
artifact before source modeling begins. Its `implementation_plan` names the
schema source root, source-file categories, entity/relationship/service/frontend
tasks, evidence tasks, and validation sequence. The full model proposal is
retained on disk even when stdout uses summary mode. Its `review_decisions`
ledger gives reviewers and agents a place to mark candidate entities,
properties, routines, frontend views, and integrations as unresolved
(`needs_review`, `needs_discovery`) or terminal (`accept`, `reject`, `defer`,
`split`, `merge`) before source config is written.

`model-status` writes `target/appfw/model-status.json` and reports the current
phase, blockers, source-model file counts, and next commands. Run it after the
proposal, after writing model source, after validation, and after generation so
the AI harness can hand off a clear conversion checkpoint. It reads the editable
`.appfw/model-proposal.yaml` review ledger; `proposal_reviewed` means
`accepted_for_config` is true, `model_owner` and `reviewed_at_utc` are present,
and no `needs_review`, `needs_discovery`, or missing decision entries remain.
Its `source_authoring_plan` then exposes the schema source root, source-file
categories, accepted entity source targets, review decision counts, and
validation sequence the agent should use while writing `.appfw/model`.

Use [Model Proposal To Config](../model/model-proposal-to-config.md) when
turning the reviewed proposal into `.appfw/model`. The guide keeps the
proposal review, persisted model design, DTO/custom-method decisions,
classification, and relationship semantics separate so agents do not copy
heuristic clues into source config blindly.
After signed review, use `scripts/appfw product scaffold-model --dry-run --json`
as a safe preview of starter source files. Running
`scripts/appfw product scaffold-model --json` writes only starter entity and
deny-by-default policy source plus `target/appfw/model-scaffold.json`; the
agent still owns semantic modeling, relationships, properties, UI, services,
and validation.

Interactive mode asks the same questions one at a time:

- application name and display name;
- product schema;
- backend provider, allowed values: `PostgreSQL` for the current runnable
  product-intake scaffold;
- MCP server needed, allowed values: `true`, `false`;
- Kafka client needed, allowed values: `true`, `false`;
- UI mode, allowed values: `none`, `scaffold`, `enterprise`;
- optional PoC source, data artifact, and UI artifact paths.

In JSON or other non-interactive use, omitted required answers fail early and
the error includes the same question contract and allowed values.

The command creates a boilerplate product workspace with product intent: root
`Cargo.toml`, backend, database, API-test, policy-test, selected-provider
podman/compose, optional frontend, `.appfw/poc-intake.yaml`, and
`.appfw/manifest.yaml`. It writes only the product's app/schema/provider/ingress
choices and must not carry CRM entities, CRM route names, CRM docs, CRM frontend
screens, or other sample residue into the target repository. It stops before
pretending the entity model is known. The next actor is the AI harness, which
inspects the PoC artifacts and writes the product model source. Agents may
inspect the CRM sample as a reference, but they must build the target schema and
UI from the PoC intake artifacts.

## AI Harness Contract

When Codex, Claude, or another agent receives a PoC intake, it should follow
this sequence.

1. Run product analysis and draft the proposal.
   Run `scripts/appfw product analyze --summary --json`, then review
   `target/appfw/product-analysis.json` and `.appfw/poc-analysis.yaml` as a
   starting profile, not as final model source. Use `model_clues` as a review
   queue for candidate entities, properties, routines, and frontend views.
   Run `scripts/appfw product propose-model --summary --json`, then review
   `.appfw/model-proposal.yaml` and
   [Model Proposal To Config](../model/model-proposal-to-config.md) before
   writing any schema source. Run `scripts/appfw product model-status --json`
   to confirm the conversion is in `proposal_ready_for_modeling`. Complete the
   proposal `review_decisions` ledger enough to make unresolved ambiguity
   visible, then sign the review with `model_owner`, `reviewed_at_utc`, and
   `accepted_for_config: true` only when the status can move to
   `proposal_reviewed`. Use `source_authoring_plan` from
   `target/appfw/model-status.json` as the first source-authoring queue after
   sign-off.
   Run `scripts/appfw product scaffold-model --dry-run --json` only after
   sign-off to preview source starters from accepted entity decisions. Run it
   without `--dry-run` only when accepted entity decisions have explicit
   `config_kind` and `classification`; then continue semantic modeling by
   editing source config from evidence.

2. Inspect source artifacts.
   Read workbook sheets, static HTML, embedded data, screenshots, and reference
   notes. Identify entities, measures, dimensions, relationships, enums,
   calculated values, workflow screens, filters, and charts.

3. Draft the product model.
   Create source YAML under `.appfw/model/schemas/<schema>/`. Prefer durable
   business nouns over UI-only names. Separate persisted table entities from
   DTO/result entities. Add data classification metadata before treating the
   model as release-ready.

4. Validate before generating.
   Run `scripts/appfw product validate --json`. On a fresh intake scaffold this
   may pass as topology sanity before entities exist; product-readiness
   evidence starts after reviewed model source exists. Fix model/config errors
   at the source, not in generated output. Run
   `scripts/appfw product model-status --json` again; it should move from
   `model_source_started` to `model_validated` only after validation evidence
   exists.

5. Generate backend and contracts.
   Run `scripts/appfw product generate`, then review generated diff and
   `scripts/appfw product generate --check --json`. Use
   `scripts/appfw product model-status --json` to confirm the conversion has
   reached `generated` before handoff.

6. Create product services where the PoC has logic.
   Put durable calculations and workflows in product-owned services and handler
   overrides, not in generated defaults. Use custom methods when the UI needs a
   report or command that is not plain CRUD.

7. Rebuild the frontend from framework standards.
   Use the PoC UI for intent and the CRM frontend for implementation standards:
   enterprise layout, generated UI contract, server-side grids, forms,
   validation, dark/light mode, charts, accessibility, and release evidence.

8. Prove the app.
   Run the risk-appropriate loop: validation, generation check, fast tests,
   frontend checks when present, live API/provider checks when services are
   available, and handoff.

## Source Artifact Review

A workbook or embedded HTML data set should be reviewed as a model source:

| Artifact Signal | Model Decision |
| --- | --- |
| Sheet/table with stable row identity | Candidate entity type. |
| Repeated category/status values | Candidate enum or lookup entity. |
| Repeated numeric measures by period | Candidate fact table with date/month dimensions. |
| Calculated report row | Candidate DTO or custom method result. |
| Embedded dashboard filter | Candidate query filter, custom method argument, or UX control. |
| Chart series | Candidate metric, aggregate, or generated report method. |
| User-editable field | Candidate persisted property with validation. |
| Read-only derived value | Candidate service calculation, computed field, or DTO field. |

Do not assume every spreadsheet column should become a persisted field. Some
columns are derived, presentation-only, or historical import artifacts.

## Frontend Conversion

The enterprise frontend should not be a direct HTML copy. The agent should:

- preserve business vocabulary, data groupings, report sections, charts, and
  important visual hierarchy from the PoC;
- replace static embedded data with GraphQL/backend data access;
- use the generated UI contract for entity metadata and field shape;
- use the product frontend scaffold conventions for routing, shell, data grids,
  forms, dashboards, lookup selectors, validation, and theming;
- run frontend typecheck/build and E2E/a11y evidence when the frontend is in
  scope; and
- keep custom visual assets product-owned.

## Verification Loop

Minimum loop after model extraction:

```bash
scripts/appfw product validate --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast
scripts/appfw product handoff --json
```

When a frontend is created:

```bash
cd frontend
npm run appfw:check
npm run typecheck
npm run build
cd ..
scripts/appfw product frontend-test --json
```

When live services are available:

```bash
scripts/appfw product migrate
ENV_NAME=local API_PORT=8080 scripts/appfw product serve
scripts/appfw product api-test
```

For release-relevant provider or security behavior, add provider certification
and release evidence as described in `docs/release/release-gate-ci-cd.md`.

## Handoff Requirements

The handoff should state:

- PoC source artifacts reviewed;
- inferred schema name and entity list;
- source data classification assumptions;
- generated artifacts intentionally updated;
- product services/custom methods added;
- frontend screens created or intentionally deferred;
- commands run and artifacts retained; and
- open questions where the PoC data or UI was ambiguous.

The final product should feel like the business PoC grew up, not like the CRM
sample was renamed.
