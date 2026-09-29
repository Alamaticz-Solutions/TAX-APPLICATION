# Dynamic form/workflow engine design: screens, fields, conditions, document types

Status: **design, not yet built.** Written 2026-09-29 against the business spec section 5.5/5.5a
(Tab 2 — Tax Document Collection) and the wider Create New Request flow (sections 5.4-5.8), at the
user's request to make the whole flow — screens, fields, field values, document types, visibility
conditions and workflow stages — database-driven instead of hardcoded, the same way
[[tax-rbac-implementation]] made roles and permissions database-driven. Every design decision below
is a deliberate reuse of a pattern already proven in `project-governance`'s v6 catalogue layer
(`ActivityDataPoint`/`DataPoint`/`Condition`/`ConditionClause`/`FieldValue`) — see section 1.

## 1. What Project Governance already built, and what carries over as-is

Governance's `.appfw/model/schemas/governance/entity_types/` has a complete, working example of
exactly this problem — "let an administrator add a question to a form without a developer" — and
it is the direct precedent for everything below:

| Governance entity | What it does | Tax equivalent |
|---|---|---|
| `DataPoint` | The question catalogue (226 rows): code, label, help text, `answer_shape` (single/list/table/file), `storage_target` (field_store/column/child_table/document/jira/undecided), `promoted_column`, `option_list_id`. | `RequestField` — same shape, same idea: some answers are real table columns, some live in a generic field-value table, and the catalogue row says which. |
| `Activity` | One form/gate (a screen, conceptually). | `RequestScreen` — Client Info, Upload Documents. |
| `ActivityDataPoint` | Which questions appear on which activity's form, at what `requirement` (mandatory/optional/conditional), gated by an optional `condition_id`. `UNIQUE(activity_id, data_point_id, form_stage)`. | `RequestScreenField` — same columns, same constraint shape. |
| `Condition` + `ConditionClause` | A generic rule: `join_kind` (and/or) plus one or more clauses, each `{data_point_id, comparison, value}` with `comparison IN (is, is_not, is_one_of, greater_than, less_than, is_answered)`. Reused for two different purposes in governance (form visibility, and project-type applicability) — "one mechanism, two uses." | Reused unchanged as `Condition`/`ConditionClause`, plus **one genuine extension** (section 4) so a clause can also test a document type's own attributes, not only an answered field. |
| `OptionList` + `OptionValue` | A managed, admin-editable dropdown. | `FieldOptionList` + `FieldOptionValue` — Document Type, Document Year, Entity Type all become option lists instead of the hardcoded arrays in `frontend/src/features/shared/config/documentTypes.ts`. |
| `ProjectType` + `ProjectTypeActivity` | Which activities apply to which project type, mandatory or not, in what order. | `RequestType` + `RequestTypeScreen` — the "different request types" requirement: adding a second request type is a data row, not a code change. |
| `ActivitySubmission` | One filled-in form instance for one gate on one project; `save_stage` custom method turns a JSON payload into individual `FieldValue` rows. | `RequestScreenSubmission` — one filled screen instance per routing record; `save_screen` custom method, same shape. |
| `ProjectActivity` | Where one project stands on one gate: status, current stage, due date, attempt count. | `RequestRecordScreenStatus` — where one routing record stands on one screen. |
| `FieldValue` | The EAV answer store: one row per (project, question, item), with exactly one `value_*` column filled (`ck_field_value_exactly_one` CHECK), versioned, one-current-answer enforced by a partial unique index added in a migration (not the model, because a partial index can't be declared there). | `RequestFieldValue` — identical shape and identical constraints, including the same migration caveat. |

The whole point of copying this pattern rather than inventing a new one is that it is **already
battle-tested against exactly this risk profile** (duplicate answers under concurrent saves,
answers to deleted questions, a condition an administrator can publish that silently does
nothing) — see the comments in `field_value.yaml` and `condition_clause.yaml` for the specific
failures each constraint exists to prevent.

## 1a. Re-checked against section 2.3 and the client lookup (2026-09-29)

Re-read section 2.3 "Data Scoping" directly: it is two sentences, both RBAC — "Standard Users see
only records where they are the assigned operator... Administrators see all records." Nothing in
it concerns screens, fields, document types or conditions. That work is
[[tax-rbac-implementation]], already built and verified (25/25 live checks). There is nothing new
for this design to do there.

Also re-checked the client-name lookup specifically, since the request called it out: section 5.4
says every Tab 1 client field is "Read-only after lookup," sourced from the "Client reference
database." **That table already exists** — `TaxPayerRefData` (`.appfw/model/schemas/tax_routing/
entity_types/tax_payer_ref_data.yaml`), modeled from spec 3.5, with its own RBAC (staff read active
rows only, admin all — [[tax-rbac-implementation]]). Nothing about its *design* is missing or
hardcoded.

**What is still hardcoded is the frontend's client search**: `frontend/src/features/routing/
ClientSearch.tsx` calls `mockClientService`, which searches an in-memory array in
`features/shared/data/clients.ts` — not `TaxPayerRefData` over GraphQL. This is a *frontend wiring*
gap, not a database design gap, and it's the same category of work as section 8 below (the
screen-resolution query): real GraphQL calls replacing mock services. It's included in the build
order (section 9, step 5) alongside the document-type option lists, since both are "stop reading
from a hardcoded/mock array, read from the database instead" changes to the same two screens.

## 2. What "Section 2.3 of the AttachDocument requirements" maps to in this spec

The business spec I have (`TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`) doesn't have a
section literally numbered "2.3 AttachDocument" — its 2.3 is "Data Scoping" (RBAC). Reading the
document's own outline, the document-upload requirements this request describes are:

- **3.3 Document Fields** — the per-document-row field catalogue (Document Type, Document Year,
  Document Name, Entity Type, Entity Name, Entity Type Number, Is Password Protected, Attachment
  File, K-1 Attachments, Form 8308 Attachments, Amended Documents) with a `Required` column already
  marked `Yes` / `Conditional` / `Auto`.
- **3.4 Document Type → Box Folder Routing** — the "available document types and their
  requirements" table the Upload Documents screen shows: 9 document types, each with a business
  name, external/internal folder, and free-text notes that encode requirement flags in prose
  ("External only", "K-1 and Draft K-1 types only").
- **5.5 Tab 2 — Tax Document Collection** — the same fields again, this time with a `Visibility`
  column instead of `Required` (Always visible / When document belongs to an entity / K-1 and
  Draft K-1 types only / Form 8308 type only) — this is the conditional-visibility rule set.
- **5.5a Document Validation Rules** — `IsDocumentAttached`, `ValDocName`/`ValDocNameExtensions`,
  `CheckDuplicateDoc_Info` — validation rules, not visibility rules, but the same "rule an
  administrator should be able to change" category.

If a different section 2.3 exists in a version of the spec I don't have, tell me and I'll re-read
against it — everything below is built from the four sections above, which is the complete set of
document-upload requirements in the copy I have.

## 3. Entity model

New entities in `.appfw/model/schemas/tax_routing/entity_types/`, all `audited`. Table names
below are illustrative snake_case; the model YAML is the source of truth once built.

### Catalogue (admin-edited; changing behaviour is a data edit, never a code change)

**`RequestType`** — `id, code, name, description, is_active`. Seed: one row, `TAX_DOCUMENT_ROUTING`.
Adding a second request type later (the spec never names one, but the user asked for the
capability) is one more row here plus its own `RequestTypeScreen` rows — no other table changes.

**`RequestScreen`** — `id, code, name, sort_order, is_active`. Seed: `client_info`,
`upload_documents` (the two data-entry screens; Processing and Completed are system-driven
displays, not forms, and stay out of the catalogue — see section 6).

**`RequestTypeScreen`** — `id, request_type_id FK, screen_id FK, is_mandatory, sort_order`.
`UNIQUE(request_type_id, screen_id)`. Which screens belong to which request type, and in what
order — the direct analogue of `ProjectTypeActivity`.

**`RequestField`** — the field catalogue. `id, code, label, help_text, answer_shape
('single'|'list'|'file'|'table'), data_type ('string'|'int'|'decimal'|'boolean'|'date'|'uuid'|
'file'|'enum'), option_list_id FK nullable, storage_target ('column'|'field_store'),
promoted_column nullable, is_active`. `UNIQUE(code)`.
- `storage_target = 'column'` rows point (`promoted_column`) at a real column that already exists
  on `TaxRoutingRecord`/`TaxDocument` (e.g. `document_type_route_id`, `document_year`,
  `client_full_name`) — the catalogue describes the *placement and visibility* of a field that is
  still a normal typed column, queryable and indexable exactly as today.
- `storage_target = 'field_store'` rows have no promoted column; their answers live in
  `RequestFieldValue` (section 5). This is for anything added later that doesn't yet warrant a
  schema change — the same escape hatch `field_store` gives governance's 72-of-226 answers.
- Every field the spec lists today (3.3, 5.5) maps to `storage_target = 'column'`, because every
  one of them is already a real column on `TaxRoutingRecord` or `TaxDocument`. Nothing in this
  design forces a rewrite of those tables into pure EAV — see section 7 for why that's
  deliberate.

**`RequestScreenField`** — placement. `id, screen_id FK, field_id FK, requirement
('mandatory'|'optional'|'conditional'|'readonly'), condition_id FK nullable, section (text,
groups fields into the PDS `FieldGroup`s a screen renders — "Client Profile", "Contact Emails",
"Intelligence Routing", "Extracted Documents"), sort_order`.
`UNIQUE(screen_id, field_id, section)`. The direct analogue of `ActivityDataPoint`. A field with
`requirement = 'conditional'` is hidden unless `condition_id`'s condition evaluates true against
the record/document being edited.

**`FieldOptionList`** — `id, name, description`. Seed: `document_type`, `document_year`,
`entity_type`.

**`FieldOptionValue`** — `id, option_list_id FK, code, label, sort_order, is_active`.
`UNIQUE(option_list_id, code)`. Seeds the 9 document types (replacing
`frontend/src/features/shared/config/documentTypes.ts`'s `DOCUMENT_TYPES` array), the year list
(replacing `DOCUMENT_YEARS`), and the 3 entity types (replacing `ENTITY_TYPES`).

**`Condition`** — `id, name, join_kind ('and'|'or'), description`. Reused unchanged from
governance.

**`ConditionClause`** — the one genuine extension over governance's version (justified in section
4). `id, condition_id FK, subject_kind ('field'|'document_type_attribute'), subject_field_id FK
nullable, subject_attribute text nullable, comparison ('is'|'is_not'|'is_one_of'|'is_answered'),
value, sort_order`. Exactly one of `subject_field_id` / `subject_attribute` is set, matching which
`subject_kind` the row declares (enforced the same way `field_value.yaml`'s
`ck_field_value_exactly_one` enforces its own "exactly one" rule — a CHECK constraint added in a
migration).

### Document type requirements (admin-managed today; unchanged model, newly exhaustive)

**`DocumentTypeRoute`** (existing entity) gains the requirement flags the "Document Type
Requirements" table needs to stop being hardcoded: `allows_entity` (bool), `requires_k1_attachments`
(bool), `requires_form_8308_attachments` (bool), `supports_password_protection` (bool),
`is_external_only` (bool). These five booleans are exactly `documentTypes.ts`'s `base` object and
per-row overrides today (`allowsEntity`, `k1Attachments`, `form8308Attachments`,
`supportsPasswordProtection`, `externalOnly`) — moved from a TypeScript array to columns an admin
edits through the existing `document_type_route.*` CRUD permissions built in
[[tax-rbac-implementation]]. `external_folder`/`internal_folder`/`document_type` and the free-text
"Note" column already exist on this entity; nothing about its shape changes except these five
additions.

### Instance data (per routing record; system-written)

**`RequestScreenSubmission`** — `id, routing_record_id FK, screen_id FK, attempt_no, status
('in_progress'|'submitted'), submitted_by_user_id FK, submitted_at, catalogue_version` (a plain
integer or timestamp pinning which version of the field catalogue this submission was filled
against — same purpose as governance's `catalogue_version_id`, simplified to a version stamp since
Tax doesn't (yet) need governance's full `CatalogueVersion` entity). `UNIQUE(routing_record_id,
screen_id, attempt_no)`. Custom method `save_screen(routing_record_id, screen_code, payload:
JSON)`, the analogue of `ActivitySubmission::save_stage`.

**`RequestFieldValue`** — the EAV store, for `storage_target = 'field_store'` fields only (section
7 explains why most fields never need a row here). `id, routing_record_id FK, tax_document_id FK
nullable (set when the value belongs to one document row, not the record), field_id FK, item_no
(int, default 0), version_no (int, default 1), is_current (bool), value_text, value_number,
value_date, value_bool, value_option_id FK nullable, value_json, submission_id FK`. Same
"exactly one value_* column" CHECK and same partial-unique-index-added-in-a-migration caveat as
governance's `field_value.yaml` — copy that file's comments verbatim into this one; they are not
Governance-specific, they document a real framework constraint (a null `value_json` scalar cannot
be sent to the Postgres provider — "Finding P" in governance's `CLAUDE.md`).

## 4. Why `ConditionClause` needs one extension: document-type-conditional visibility

Section 5.5's visibility column says K-1 Attachments shows "K-1 and Draft K-1 types only" and
Form 8308 Attachments shows "Form 8308 type only" and Entity fields show "when document belongs to
an entity." In governance, a `ConditionClause` always tests an *answered question*
(`data_point_id`) — e.g. "risk_level is HIGH." Doing the same here would mean a clause like
"document_type is_one_of [K-1, Draft K-1]", which works, but has a real cost: **adding a tenth
document type that also takes K-1 attachments means editing this condition's clause list**, not
just adding a `FieldOptionValue` row — exactly the kind of change-requires-a-data-edit-in-two-places
problem the whole design exists to avoid.

The fix: let `requires_k1_attachments` (etc.) live on `DocumentTypeRoute` itself (section 3), and
let a `ConditionClause` test that flag directly — `subject_kind = 'document_type_attribute'`,
`subject_attribute = 'requires_k1_attachments'`, `comparison = 'is'`, `value = 'true'` — evaluated
against the row identified by whichever `document_type_route_id` the document being edited
currently has selected. Adding a tenth K-1-like document type is then one `DocumentTypeRoute` row
with `requires_k1_attachments = true`; the condition that shows the K-1 Attachments field never
changes. This is the one place this design diverges from governance's `ConditionClause`, and the
divergence is additive (`subject_kind = 'field'` behaves exactly like governance's version
always did) — nothing about governance's own tables or engine needs to change for Tax to add this.

## 5. Seeded conditions and screen-field rows (from spec 5.5)

| RequestScreenField | requirement | condition |
|---|---|---|
| Document Type, Document Year, Document Name, Is Password Protected, Attachment File | mandatory | none — always visible |
| Entity Type | conditional | clause: `document_type_attribute.allows_entity is true` |
| Entity Name | conditional | clause: `document_type_attribute.allows_entity is true` AND `field(entity_type).is_answered` |
| Entity Type Number | conditional (readonly, auto-populated) | same as Entity Name |
| K-1 Attachments | conditional | clause: `document_type_attribute.requires_k1_attachments is true` |
| Form 8308 Attachments | conditional | clause: `document_type_attribute.requires_form_8308_attachments is true` |

`Is Password Protected`'s control itself is always visible (per 5.5), but whether it does anything
is already gated by `supports_password_protection` at the document-type level — modeled the same
way as the fields above, as a `RequestScreenField` marking the control `enabled_condition` rather
than `visibility_condition` if the generator's forms support that distinction; otherwise the
frontend disables the checkbox when the flag is false, same as `UploadDocumentsStep.tsx` does
today with the mock config.

## 6. What stays out of the catalogue, on purpose

- **Processing and Completed** are system-rendered progress/result displays (`ProcessingStep.tsx`,
  `CompletedStep.tsx`), not data-entry forms. They have no fields to catalogue. Putting them in
  `RequestScreen` would invite someone to add `RequestScreenField` rows to a screen that never
  reads them — a shape governance avoids by only cataloguing activities that are actual forms.
- **Custom methods (`cancel_record`, `reassign_record`)** and their arguments are not catalogued
  fields; they're fixed API surface, same as governance's `start`/`submit`/`skip` on
  `ProjectActivity` are fixed regardless of how many `DataPoint`s exist.
- **RBAC** ([[tax-rbac-implementation]]) is a separate, already-built catalogue layer. The two are
  intentionally not merged: `RequestScreenField`'s `requirement` controls whether a field is asked
  at all; permission grants control who may act on the record it belongs to. A future
  `RequestScreenField`-level permission gate (e.g., a field only Tax Admin may edit) is a natural
  extension — add a `min_permission_code` column — but nothing in the current spec asks for it, so
  it's not in this design.

## 7. What this replaces, what it doesn't, and why

**Replaces (moves from hardcoded TypeScript to database rows):**
- `frontend/src/features/shared/config/documentTypes.ts`'s `DOCUMENT_TYPES`, `DOCUMENT_YEARS`,
  `ENTITY_TYPES`, `K1_ATTACHMENT_OPTIONS`, `FORM_8308_ATTACHMENT_OPTIONS` arrays — become
  `FieldOptionList`/`FieldOptionValue` rows and `DocumentTypeRoute` columns, fetched via GraphQL.
- Every `if (cfg?.allowsEntity)` / `cfg?.supportsPasswordProtection` conditional currently in
  `UploadDocumentsStep.tsx` and `EditDocumentDialog.tsx` — becomes "does this field's
  `RequestScreenField.condition_id` evaluate true for the document's current
  `document_type_route_id`," evaluated once by the backend (a `resolve_screen_fields(screen_code,
  context)` custom query) and consumed by the frontend as a list of `{field_code, visible,
  required}` rather than re-implemented in JS.

**Deliberately does NOT replace:**
- `TaxRoutingRecord` and `TaxDocument`'s existing typed columns. Every field the spec names today
  has `storage_target = 'column'` (section 3) — the catalogue describes their *visibility and
  placement*, not a new place to store their *values*. Full EAV storage for these (moving
  `document_year`, `client_full_name`, etc. into `RequestFieldValue`) would mean losing real SQL
  types, indexes and foreign keys on data that is queried constantly (every list screen, every
  RBAC "own" filter) in exchange for flexibility nothing in the spec asks for. Governance's own
  design makes the identical choice — `promoted_column` and `storage_target = 'column'` exist
  specifically so that "most answers are still real columns" is the default, and `field_store` is
  the exception for the ~32% of answers that don't fit a fixed schema. Tax's field set is smaller
  and entirely spec'd already, so the field-store fraction should be close to zero at launch;
  `RequestFieldValue` exists so a *future* field can be added without a migration, not because
  today's fields need it.

## 8. Frontend

Per the user's instruction: PDS components only, no new custom components, nothing hardcoded.

- A generated GraphQL query, `resolveScreenFields(screenCode, context: JSON) -> [{field_code,
  label, control_type, visible, required, option_list_code}]`, evaluated server-side against the
  `Condition`/`ConditionClause` engine — the frontend never evaluates a condition itself, the same
  way it never evaluates an RBAC grant itself.
- `ClientInfoStep.tsx` and `UploadDocumentsStep.tsx` (and `EditDocumentDialog.tsx`) stop importing
  `DOCUMENT_TYPES`/`ENTITY_TYPES`/etc. from `config/documentTypes.ts` and instead read the option
  lists and the resolved field visibility from this query, feeding the same PDS components already
  in place (`SingleSelectField` for option lists, `CheckboxField`, `FieldGroup` sectioning) —
  **no new frontend components**, only their data source changes.
- This is real backend wiring, which the frontend does not have yet for anything else (screens
  still run on mock data). Building this screen-resolution query is a reasonable place to start
  the real GraphQL wiring, since it's self-contained and needed regardless of when the rest of the
  data layer gets wired.

## 8a. PDS App Framework structure — compliance check before any build starts

Per CLAUDE.md's generated-vs-hand-owned split, confirmed against every piece of this design:

| Rule | This design |
|---|---|
| Entities are modeled in `.appfw/model/schemas/tax_routing/entity_types/*.yaml`, never hand-written SQL | All 11 new entities + `DocumentTypeRoute`'s 5 new columns are model YAML. `database/_pkg/schemas/tax_routing/{tables,seed}.pg.sql` are 100% generated output, exactly like every table added so far (including all of [[tax-rbac-implementation]]'s tables). No file under `database/_pkg/` is hand-edited. |
| RBAC policies are `.rego` bodies under `.appfw/model/schemas/tax_routing/rbac/`, using `input.user.grants` | Every new entity gets its own policy file, same shared grant-rule template as the 20 existing ones — no role literal, ever (this is *forced*, not optional: section 9 step 2 exists because a missing policy silently denies every write that touches the entity, the exact bug already hit once). |
| Custom business logic lives in `backend/src/services/**`, called from thin `handlers/tax_routing/<entity>.rs` overrides | `resolve_screen_fields` and `save_screen` are services, called the same way `services/authz.rs` and `services/routing_record_actions.rs` already are. |
| Frontend consumes PDS components from `@appfw/pds-health-components`; no product-local design-system layer; check the PDS catalogue before writing anything custom | Confirmed in section 8: `SingleSelectField`, `CheckboxField`, `FieldGroup` are reused as-is; this design adds zero new frontend components, only a new data source (GraphQL instead of a hardcoded array) for components already in place. `pds-frontend-guard` gates the change like any other frontend edit. |
| Seed data is `.appfw/model/schemas/tax_routing/seeds/*.yaml`, never inserted by hand | The document-type flags, option lists, and screen/field/condition rows in section 5 are seed files, following the same `entity_type`/`columns`/`records` shape as [[tax-rbac-implementation]]'s 5 seed files. |
| Migrations belong in `database/_pkg/migrations/` only for what the model can't express (CHECK constraints spanning generated logic, partial unique indexes) | Two such migrations are already called out: `ck_field_value_exactly_one`-equivalent CHECK on `RequestFieldValue`, and the one-current-answer partial unique index — both copied verbatim from `field_value.yaml`'s own documented caveat, not new invention. |

Nothing in this design asks for a framework change, a hand-written table, or a bypass of the
existing generate → validate → boundary-check → policy-test loop. It is structurally the same kind
of change as the RBAC build, reusing the same tooling and the same verification gates
(`scripts/appfw product validate/generate/boundary-check/policy-test`, then
`cargo check/test --workspace`, then `npm run test:frontend`).

## 9. Build order (proposed; not started)

1. Model `RequestType`, `RequestScreen`, `RequestTypeScreen`, `RequestField`,
   `RequestScreenField`, `FieldOptionList`, `FieldOptionValue`, `Condition`, `ConditionClause`,
   `RequestScreenSubmission`, `RequestFieldValue`; add the five flags to `DocumentTypeRoute`.
   `product validate` / `generate`.
2. RBAC for the new entities, same pattern as [[tax-rbac-implementation]] bug 1 already forces:
   every new entity needs its own `.rego` file and `permission` rows before any write involving it
   (a foreign key to any of these tables will otherwise silently deny, exactly like `AppUser` did).
3. Seed data: the rows in section 5, plus the document-type flags from spec 3.4/5.5.
4. `services/form_engine.rs`: the condition evaluator (`evaluate_condition(condition_id, context)
   -> bool`) and `resolve_screen_fields` custom query.
5. Frontend: swap `documentTypes.ts`'s static arrays for the live query; verify
   `npm run test:frontend` (the ratchet/scaffold checks won't know the difference, but the
   `documentTypes.test.ts`-style unit tests, if any exist, need updating to mock the query).
6. `RequestScreenSubmission`/`RequestFieldValue` wiring is only needed once a field actually has
   `storage_target = 'field_store'` — defer until the first such field exists, per section 7.
