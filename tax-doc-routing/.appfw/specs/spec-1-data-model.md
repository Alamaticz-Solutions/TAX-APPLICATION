# Spec: Tax Document Routing — Data Model (includes status/exception control flow)

Status: draft

Spec depth: full

Owner roles:

- Product Owner: TBD
- Architect: TBD
- XO: TBD
- Implementation owner: TBD
- Review owner: TBD

## Business Value

Establishes the single source-of-truth schema for the entire Tax Document
Routing application — the primary routing record, its per-document child
rows, the two internal reference tables that drive auto-population, and the
lifecycle-state model that replaces what would otherwise be a generic
workflow engine. Every other spec in this application (screens, Box
integration, PDF processing, notifications) depends on this one.

## Problem

Source document (`TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`,
section 3) defines the record, document, and reference-table shape but in
Pega-specific terms (classes, data pages, report definitions,
`pyUseAlternateDB`). This spec translates that into the App Framework's
schema model and resolves how the described 4-stage lifecycle is actually
implemented — not as a generic state-machine engine, but as a status field
plus an exception-task entity, per the decision below.

## Goals

- Model the primary routing record, its document child rows, and the two
  reference tables (`TaxPayerRefData`, `TaxEntityList`) as `.appfw/model`
  entities.
- Replace the document's original multi-stage workflow description with a
  `status` enum on the primary record plus a dedicated `ExceptionTask`
  entity, since this application's stages are fixed and linear with
  exception-only branching (see Decision Provenance).
- Preserve every field-level behavior called out in the source doc that
  has downstream consequences (write-back, masking, join keys, filename
  construction inputs) even where this spec doesn't implement that
  downstream behavior itself.

## Non-Goals

- Box API connector implementation (separate spec).
- PDF password/encryption implementation (separate spec — this spec only
  models where the password value is stored and how it's protected at
  rest/in transit, not the encryption operation itself).
- Screen implementation (separate specs per tab/stage).
- Live Oracle master-data integration — `OracleID` is modeled as a plain
  foreign-key-shaped field for now; see Decision Provenance for why this
  is deferred rather than built against `appfw_provider_oracle_financials`.

## Scope

`.appfw/model` schema definitions for five entities: `TaxRoutingRecord`,
`TaxDocument`, `TaxPayerRefData`, `TaxEntityList`, `ExceptionTask`. No
provider/connector work, no frontend work — this spec is schema-only.

## Repository Context

Follow the CRM reference example's entity pattern (`account.yaml` /
`generated.rs` / `account.rs` / services split) for every custom method
below. Field-level audit redaction should follow the `email` field's
`meta.audit.redact: true` convention already used in the CRM example.

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-09-22 | Product Owner | Replace the generic multi-stage workflow-engine approach with a `status` enum + `ExceptionTask` entity. | The application has only one human-driven stage (Stage 1 intake); Stages 2–4 are automated/system-driven per the source doc's own "Automated Actions" language, and its exception handling (5.3, 6.3) already describes a queue-and-retry pattern, not branching workflow. A generic state-machine engine would be unused capability. | Revisit if a future stage introduces real multi-actor routing or configurable branching beyond retry-on-exception. |
| 2026-09-22 | Product Owner | `TaxPayerRefData` and `TaxEntityList` are internal reference tables (not external-system integrations). | Confirmed by Product Owner. Modeled as ordinary `app_owned` entities, admin-managed through generated CRUD, not a provider connector. | Revisit if either table is later sourced from a live external system (e.g. if Oracle master data needs to be read live rather than periodically synced by admins). |
| 2026-09-22 | Product Owner (carried forward from Tab 1 spec) | Client fields denormalized onto `TaxRoutingRecord` at lookup time; `Read/Write Password` stored as a functional secret, masked in all UI, excluded from audit/history capture. | Source doc's own security note (section 3.5) independently flags this exact risk in the legacy system: *"TaxPayerRWPassword is stored as plain text... flows to the case clipboard without masking... Ensure the property is masked in all UI sections and excluded from case history capture."* This spec fixes that known issue rather than reproducing it. | N/A |

## Data Model Detail

### `TaxRoutingRecord` (primary entity)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | Uuid (primary key) | Yes | System-generated |
| `status` | Enum: `collecting_info`, `processing`, `cleaning_up`, `resolved`, `cancelled`, `exception` | Yes | Replaces the source doc's implicit workflow-stage concept. Drives which automated step runs next; `exception` means an `ExceptionTask` is open against this record. |
| `created_at` / `updated_at` | DateTime | Yes | Framework-standard audit fields |
| `assigned_operator` | Text (or FK to a `User`/`Operator` entity if one exists elsewhere in the app) | Yes | Staff member currently responsible |
| Client fields (denormalized) | see below | — | Populated from `TaxPayerRefData` at lookup time, per the earlier Tab 1 decision |

**Client fields, denormalized onto this entity** (source doc 3.2, all populated from `TaxPayerRefData` at lookup — see Tab 1 spec):

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `client_first_name` / `client_last_name` / `client_full_name` | Text | Yes | Auto-populated, read-only in UI |
| `office_location` | Text | Yes | Auto-populated |
| `pds_email` / `personal_email` | Email | Yes | Auto-populated |
| `additional_email` | Email | No | Staff-entered, editable |
| `notification_flag` | Boolean | Yes | Staff choice, editable |
| `internal_folder` / `folder_name` | Text | Yes | Auto-populated; Box folder references — see Box connector spec for how these are used |
| `read_write_password` | Text, **`meta.audit.redact: true`**, never returned to frontend as plaintext | Yes | Functional secret for PDF locking (Stage 2). UI renders a status indicator only, per prior decision. |
| `client_reference_id` | Uuid/Text (FK to `TaxPayerRefData.id`) | Yes | Kept even though fields are denormalized, so the source reference record can still be traced |

### `TaxDocument` (child entity, one-to-many under `TaxRoutingRecord`)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | Uuid (primary key) | Yes | |
| `routing_record_id` | Uuid (FK to `TaxRoutingRecord`) | Yes | |
| `document_type` | Enum (see `DocumentTypeRoute` below) | Yes | |
| `document_year` | Int32 | Yes | Tax year |
| `document_name` | Text | Yes | |
| `entity_type` | Enum | Conditional | Required when document belongs to an entity |
| `entity_oracle_id` | Text | Conditional | FK-shaped join key to `TaxEntityList.oracle_id`; required when `entity_type` is selected |
| `entity_name` | Text | Conditional | Resolved from `entity_oracle_id` via `TaxEntityList` lookup (see custom method below) — do not require the user to type this |
| `is_password_protected` | Boolean | Yes | |
| `box_file_url` | Url | Auto (system-set) | Populated after Box upload — out of scope for this spec |
| `is_duplicate` | Boolean | Auto (system-set) | Set by duplicate-check logic — out of scope for this spec |
| `attachment_file` | File reference (PDF) | Yes | |
| `k1_attachments` / `form_8308_attachments` / `amended_documents` | File list references | Conditional/No | Per document-type-specific rules — see Tab 2 spec |

### `DocumentTypeRoute` (small reference entity, source doc 3.4)

Models the Document Type → Box Folder routing table as data, not a hardcoded mapping, since it's the kind of thing that changes (the doc notes "S-Corp... Added Jan 2025" as evidence this table has grown over time).

| Field | Type | Notes |
| --- | --- | --- |
| `document_type` | Text (matches `TaxDocument.document_type` enum values) | |
| `business_name` | Text | |
| `external_folder` | Text | |
| `internal_folder` | Text, nullable | Null/`"External only"` for types that never write to the internal hierarchy (S-Corp Election, S-Corp, Form 8308, Extensions per the source table) |
| `note` | Text | |

### `TaxPayerRefData` (reference entity, admin-managed, source doc 3.5)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | Uuid (primary key) | Yes | Source doc: "Client ID / `ID`" |
| `first_name` / `last_name` / `full_name` | Text | Yes | |
| `personal_email` / `pds_email` | Email | Yes | |
| `external_folder_id` | Text | Yes | Source doc flags this as a **misleadingly named** legacy field (labeled/functions as "Client External Folder ID" despite its underlying name); do not carry the misleading name forward into this schema — call it `external_folder_id`, not `folder_name`. |
| `internal_folder_path` | Text | Yes | |
| `is_active` | Boolean | Yes | Only `true` records are returned to any UI dropdown |
| `read_write_password` | Text, **`meta.audit.redact: true`**, never returned as plaintext to any frontend query | Yes | Per-client PDF password — this is the field the security note in Decision Provenance is about. Treat with the same rigor as the denormalized copy on `TaxRoutingRecord`. |
| `entity_type` | Enum | No | Admin visibility only — explicitly **not** auto-populated onto routing records per the source doc |

**Custom methods:**
- `list_active_taxpayers` (Query, `mcp_enabled: true` — read-only, safe to expose) — returns all `is_active = true` records, sorted by `full_name` ascending. Source doc flags a real performance concern worth carrying forward: *"no search filter + paging disabled = full table scan on every new case creation."* Unlike the legacy system, **implement this with a server-side search-as-you-type filter and pagination from day one** rather than reproducing the known problem.
- `update_taxpayer_reference` (Mutation, `mcp_enabled: false` — writes should stay off any remote agent surface, same caution as the CRM example's governed graph writes) — write-back of verified Box folder reference/credentials after Stage 1 verification, per source doc's `UpdatePayerRef_Info`.

### `TaxEntityList` (reference entity, admin-managed, source doc 3.6)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | Uuid (primary key) | Yes | System-generated (source doc: `pzInsKey`, Pega-specific — replaced here) |
| `oracle_id` | Text | Yes | **External join key.** Source doc: *"the primary lookup key across all entity reports and the join between Pega's entity list and the external Oracle master data system."* Modeled as a plain text field per this spec's Non-Goals — no live Oracle integration yet. |
| `entity_name` | Text | Yes | Display name, e.g. "Oakwood Medical Partners LLC" |
| `is_active` | Boolean | Yes | Only `true` records shown in the entity dropdown |

**Custom methods:**
- `list_active_entities` (Query, `mcp_enabled: true`) — all active entities, sorted A-Z. Source doc: max 10,000 records, no paging in the legacy system — same recommendation as above, add pagination now rather than later.
- `find_entity_by_oracle_id` (Query, `mcp_enabled: true`, single-record lookup by `oracle_id`) — replaces the legacy `D_FetchTax_EntityInfo` cached-data-page pattern. No caching semantics need to be reproduced; the framework's own query layer handles this as an ordinary lookup.

### `ExceptionTask` (new entity, folds in the former "state machine" spec)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | Uuid (primary key) | Yes | |
| `routing_record_id` | Uuid (FK to `TaxRoutingRecord`) | Yes | |
| `failure_reason` | Text | Yes | What automated step failed and why |
| `failed_step` | Enum (matches the automated step that failed, e.g. `temp_folder_creation`, `pdf_processing`, `cleanup`) | Yes | |
| `opened_at` | DateTime | Yes | For SLA timing, if the team wants that later |
| `resolved_at` | DateTime, nullable | No | Set when a staff member successfully retries |
| `assigned_operator` | Text | No | Who's working the exception queue item |

**Custom method:**
- `retry_failed_step` (Mutation) — re-invokes the same service function that originally failed for this record, and on success sets `TaxRoutingRecord.status` back to its normal progression and stamps `resolved_at`. This is the entire "workflow" this application needs: a linear status progression, plus one retry action for exceptions.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| `.appfw/model` | 5 new entities | Generated GraphQL schema, generated handlers, DB migrations |
| Audit/redaction policy | Two fields (`read_write_password` on two entities) marked `meta.audit.redact: true` | Audit log output, any admin UI record viewer |

## Security, Privacy, And Governance

- Both `read_write_password` fields (denormalized copy and reference-table
  original) must never be returned as plaintext in any GraphQL query used by
  a frontend screen. Confirm this at the resolver/handler level, not just by
  trusting the frontend not to display it.
- This is the schema half of the full-depth PDF password spec (item #5 in
  the sequencing plan) — that spec owns the actual encryption/at-rest
  protection mechanism; this spec only owns the field's shape and its
  audit/redaction marking.
- `TaxPayerRefData` and `TaxEntityList` are admin-managed — confirm role
  scoping (per spec #2) restricts write access to Administrator role only,
  consistent with the source doc's "maintained by administrators
  independently of the case workflow."

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Schema validates | `scripts/appfw product validate --json` | push |
| Generated code matches model, no drift | `scripts/appfw product generate --check --json` | push |
| `read_write_password` fields never appear in generated query response for non-admin roles | API test scenario | PR |
| `list_active_taxpayers` / `list_active_entities` support pagination and search filter (not full-table-scan) | API test scenario + load test at realistic client volume | PR |

## Test And Execution Feedback Plan

Standard `validate` / `generate` / `generate --check` / `test` loop. Add an
API test scenario specifically asserting the password fields are absent from
query responses — this is a security-relevant behavior worth its own
explicit test rather than incidental coverage.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Reproducing the legacy full-table-scan performance problem on the two reference-table list queries | Implement pagination/search-filter from the start, per Data Model Detail notes | TBD | Open — implementation must address, not defer |
| `OracleID` modeled without live Oracle integration may need rework if entity data drifts from the real Oracle master system | Accepted per Decision Provenance; `appfw_provider_oracle_financials` skeleton exists if this needs revisiting | Product Owner | Accepted |

## Tech Debt And Follow-Up

If `TaxEntityList` data is found to drift meaningfully from the live Oracle
system (per the accepted risk above), evaluate building against
`appfw_provider_oracle_financials` as a follow-up spec rather than continuing
to rely on admin-synced data.

## Handoff Notes

This spec is the foundation for every other spec in the sequence. In
particular: spec #2 (roles/permissions) should reference `assigned_operator`
and the admin-only write access to the two reference tables; specs #6–11
(screens and stage logic) should reference `status` and `ExceptionTask`
directly rather than any workflow-engine concept, since none exists in this
design. Do not let a later spec reintroduce state-machine language — the
control flow is deliberately "read status, do the next automated step,
create an `ExceptionTask` on failure."
