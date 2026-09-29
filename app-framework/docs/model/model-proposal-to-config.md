# Model Proposal To Config

Use this guide after `scripts/appfw product analyze --summary --json` and
`scripts/appfw product propose-model --summary --json` have produced review artifacts.
The proposal is evidence, not source. The agent or developer still owns the
modeling decision before writing `.appfw/model`.

## Inputs

Start from these product-local artifacts:

```text
.appfw/poc-intake.yaml
.appfw/poc-analysis.yaml
.appfw/model-proposal.yaml
target/appfw/product-analysis.json
target/appfw/model-proposal.json
target/appfw/model-status.json
```

Then inspect the original source artifacts named in the intake: workbooks,
CSV/TSV exports, static HTML, embedded JSON, screenshots, reference docs,
legacy code, SQL, or product notes. OOXML workbooks (`.xlsx`/`.xlsm`) may
surface sheet names, named table ranges, row counts, and headers in the
analysis artifact. They may also surface metadata-only complexity signals such
as formulas, hidden sheets, pivot tables, charts, and data validations. That is
still structure and behavior evidence, not final semantics. Do not infer
regulated or financial semantics from a column name alone.

Run `scripts/appfw product analyze --summary --json` before
`scripts/appfw product propose-model --summary --json`; the proposal command reads the
retained `target/appfw/product-analysis.json` artifact.
Run `scripts/appfw product model-status --json` after proposal review and after
each modeling checkpoint. It should report `proposal_ready_for_modeling` before
review decisions are accepted, `proposal_reviewed` before source modeling
begins, `model_source_started` after reviewed source files exist,
`model_validated` after validation passes, and `generated` after generated
artifacts are retained.

The proposal's `review_decisions` ledger is the handoff surface between
heuristic discovery and authored model source. Use it to mark each candidate as
`needs_review` or `needs_discovery` while unresolved, then as `accept`,
`reject`, `defer`, `split`, or `merge` when the review reaches a terminal
decision. Use `defer` only when the reviewer intentionally leaves work out of
the current slice with follow-up notes. Keep `accepted_for_config` false until
entity, relationship, classification, policy, frontend, and evidence decisions
are explicit enough to write source. `model-status` treats the review as ready
only when `accepted_for_config` is true, `model_owner` and `reviewed_at_utc`
are present, and no decision entries remain in `needs_review`,
`needs_discovery`, or missing state.
After sign-off, use `target/appfw/model-status.json` and its
`source_authoring_plan` as the machine-readable source-writing queue. It
summarizes the schema source root, required source-file categories, accepted
entity source targets, review decision counts, and validation sequence derived
from `.appfw/model-proposal.yaml`.

After sign-off, `scripts/appfw product scaffold-model --dry-run --json`
previews a conservative source bootstrap from accepted entity decisions. It
refuses unresolved entity `config_kind` or `classification` values, skips
existing files unless `--force` is passed, and writes
`target/appfw/model-scaffold.json` only when run without `--dry-run`. The
command creates starter entity source and deny-by-default policy files so an
agent can continue from a reviewed queue; it does not infer real properties,
relationship semantics, DTO boundaries, stored procedure disposition,
frontend behavior, or business rules.

## Decision Order

1. Confirm topology.
   Verify app name, schema, provider, MCP/Kafka/UI choices, and data-source
   names in `.appfw/manifest.yaml` and `.appfw/poc-intake.yaml`.

2. Review evidence.
   Use `target/appfw/product-analysis.json` for machine-readable clues and
   `.appfw/model-proposal.yaml` for the human review queue. Treat raw candidate
   entity names, semantic labels, suggested domain names, entity-owned
   candidate properties, unassigned report/view properties, routines, and
   frontend views as proposed, not accepted.
   Use the proposal's `implementation_plan` as the work queue for source
   modeling: it names the schema source root, source-file categories, entity
   tasks, relationship tasks, service/custom-method tasks, frontend tasks,
   evidence tasks, and validation sequence. The plan is still review-only; it
   does not make the proposal authoritative source.
   Update `review_decisions` as you go so unresolved and terminal decisions are
   visible in handoff.

3. Separate persisted state from report outputs.
   Persist durable business nouns and source-of-truth records. Use DTOs,
   custom methods, or frontend view models for calculated report rows,
   dashboard cards, chart series, and parity snapshots.

4. Classify data before validation.
   Add classification metadata for regulated profiles. Unknown regulated data
   should fail validation or be treated as confidential/PHI until reviewed.

5. Model relationships explicitly.
   Use `NavToOne` for a single parent/reference, `NavToMany` for parent-owned
   child collections, and `NavToMany` with junction semantics for many-to-many
   relationships. Do not use relationship names to hide unclear ownership.

6. Decide lookup versus entity.
   Small stable status/category sets can be enums or lookup entities. Entities
   that need ownership, history, policy, audit, or independent navigation should
   be modeled as entity types.

7. Add product operations.
   Use custom methods for domain reports, commands, calculations, imports, or
   parity checks that are not plain CRUD. Use provider routines only when the
   database routine is an intentional product contract with tests and evidence.

8. Write source config.
   Create or update source files under `.appfw/model/schemas/<schema>/`.
   Do not patch generated backend, database, API-test, or frontend-contract
   output to correct a modeling mistake.
   Optionally use `scripts/appfw product scaffold-model --dry-run --json` to
   preview starter files after signed proposal review, then run
   `scripts/appfw product scaffold-model --json` only when the accepted entity
   decisions have explicit `config_kind` and `classification` values.

## Proposal Review Matrix

| Proposal Clue | Config Decision | Evidence To Check |
| --- | --- | --- |
| Candidate entity from file, sheet, or named table | Entity, DTO, seed, or reject | Workbook sheets/tables, source tables, row identity, business noun |
| Semantic label or suggested domain name | Product-owned entity/DTO name or reject | Reference docs, glossary, user vocabulary, naming conventions |
| Entity-owned candidate property from a table/header | Property, lookup, derived value, or reject | Data dictionary, formulas, editable/read-only behavior |
| Unassigned property from prototype/report evidence | DTO field, dashboard metric, filter input, or reject | HTML/embedded data, report layout, backend data need |
| Candidate routine from SQL/reference doc | Custom method, service, provider routine, or reject | Business workflow, provider portability, release evidence |
| Frontend view from HTML heading | Route, dashboard, form, grid, or defer | User workflow, required backend data, accessibility needs |
| Integration point | MCP/Kafka/HTTP/service boundary or defer | Auth actor, tenant source, policy, audit, idempotency |

## Workbook-Backed PoC Example

For a workbook-backed PoC:

- A source table with durable facts is usually a persisted entity candidate.
- A source table with ownership or history rows is usually a child entity
  candidate linked through explicit relationships.
- A source table with small stable bands, statuses, or categories may be a
  lookup entity, enum, or seed set.
- The embedded HTML data object is useful as seed/parity evidence, but the
  workbook/reference docs remain the source of business semantics.
- The proposal should keep workbook table headers under the candidate table
  entity when ownership is clear; remaining HTML/report-only values should stay
  in `unassigned_properties` until reviewed.
- Markdown reference headings such as ``SOURCE_FACTS`` - Operational Metrics can
  provide semantic labels and suggested domain names. Use those as product
  naming hints; do not keep raw table IDs as final entity names unless the
  business vocabulary really uses them.
- Snapshots, rolling histories, derived calculations, and benchmark statuses
  are often better represented as custom method DTO results than as persisted
  tables.

## Verification Loop

After writing source config:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

Add `scripts/appfw product frontend-test --json`, generated API tests, live
provider certification, and release evidence when the changed surface requires
them.
