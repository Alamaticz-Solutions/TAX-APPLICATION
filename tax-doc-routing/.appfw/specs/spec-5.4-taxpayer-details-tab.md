# Spec: Tax Document Routing — Form Tab 1: Taxpayer Details

Status: draft

Spec depth: lightweight

Owner roles:

- Product Owner: TBD
- Architect: TBD
- XO: TBD
- Implementation owner: TBD
- Review owner: TBD

## Business Value

Lets a staff member start a tax document routing record by pulling an
existing client's identity and contact details directly from the client
reference database, eliminating manual re-entry of client information that
already exists elsewhere and the transcription errors that come with it.

## Problem

Today, taxpayer/client details for a routing record would otherwise need to
be typed in by hand. Source spec (`TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`,
section 5.4) specifies this must instead be a lookup-and-auto-populate flow:
"The staff member searches for a client from the reference database. All
fields auto-populate from reference data when a client is selected — no
manual typing of client details."

## Goals

- Staff member can search/select a client from the reference database.
- All client-sourced fields auto-populate and render read-only after lookup.
- The two staff-controlled fields (Additional Email, Send Notification) remain
  editable regardless of lookup state.
- No manual entry is possible for any field sourced from the reference
  database.

## Non-Goals

- Editing or correcting client reference data itself (that belongs to the
  client reference database's own system of record, not this form).
- Client reference database search/matching algorithm design (out of scope
  for this screen spec; assumed to be an existing lookup service).
- Tabs 2+ of this form (Tax Document Collection, etc. — separate specs).

## Scope

Product frontend, Tab 1 of the routing-record creation form. No backend
schema changes are anticipated beyond what's needed to store the fields
listed below against the routing record entity — confirm during
implementation whether a `Taxpayer`/`Client` reference is a foreign key to
an existing entity or a denormalized snapshot captured at lookup time
(see Architecture And Implementation Notes).

## Repository Context

This screen follows the **`generated-entity-form`** recipe in
`appfw_ui/pds_health/reference/catalog.json` (`agentDecisionGuide`), with the
client-lookup behavior specifically drawn from that recipe's `LookupSelect`
component — read `componentSource` (`appfw_ui/pds_health/components/src`) for
its real API before wiring this screen. Read `visibleCatalog` /
`interactiveCatalog` for live states (loading, read-only, validation) before
implementation.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| `.appfw/model` entity for the routing record | Add/confirm fields listed below | Generated GraphQL schema, generated handlers |
| Frontend form screen | New Tab 1 implementation | PDS component catalog manifest (no product-local field wrappers) |

## Field-Level Contract

| Field | Control Type | Editable | Source | Component |
| --- | --- | --- | --- | --- |
| First Name | Text | Read-only after lookup | Client reference database | `TextField` (readOnly) |
| Last Name | Text | Read-only after lookup | Client reference database | `TextField` (readOnly) |
| Full Name | Text | Read-only after lookup | Client reference database | `TextField` (readOnly) |
| Office Location | Dropdown | Read-only after lookup | Client reference database | `SelectField` (readOnly/disabled) |
| PDS Email | Email | Read-only after lookup | Client reference database | `TextField` (type=email, readOnly) |
| Personal Email | Email | Read-only after lookup | Client reference database | `TextField` (type=email, readOnly) |
| Additional Email | Email | **Editable** | Staff-entered | `TextField` (type=email) |
| Send Notification | Yes / No | **Editable** | Staff choice | `SelectField` or toggle control per catalog convention for boolean choice |
| Internal Box Folder | Text | Read-only after lookup | Client reference database | `TextField` (readOnly) |
| Folder Name | Text | Read-only after lookup | Client reference database | `TextField` (readOnly) |
| Read/Write Password | Masked status indicator (not plaintext) | Read-only after lookup, populated server-side | Client reference database | `TextField` variant with masked/secret display (e.g. "Password on file" + a reveal-suppressed indicator), **not** a plain read-only `TextField` — see Security section; the raw value must not round-trip to the browser |

Client search/select control (not in the source table but required by the
narrative): `LookupSelect`, bound to the client reference database search
endpoint. Selecting a result triggers population of all read-only fields
above in a single bound update — do not implement as N separate manual
field-population calls.

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Denormalize client fields onto the routing record at lookup time | Record is self-contained; survives client record changes later | Risk of staleness if client data changes after routing record creation | **Selected** |
| Store only a client reference (foreign key) and resolve fields live on every read | Always current | Extra read dependency on every load; must handle "client since deleted/changed" case | Not selected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-09-22 | Product Owner | Denormalize all client-sourced fields (including Read/Write Password) onto the routing record at lookup time. | Record must remain self-contained and usable even if the client reference changes later; password is needed downstream to lock generated PDF files, independent of the client record's current state. | Revisit if client password rotation needs to propagate to already-created routing records. |
| 2026-09-22 | Product Owner | Read/Write Password is a functional secret (used server-side to encrypt/lock PDF output for this routing record), not a display-only field. | Confirmed use case: PDF files generated for this record must be password-locked using this value. | N/A — this is now a hard security requirement, not a UI convenience decision. |

## Architecture And Implementation Notes

Per the `generated-entity-form` recipe: product owns generated field metadata,
validation rules, submit behavior, save conflict handling, and
workflow-specific copy. Do not fork field wrappers or a custom read-only
rendering path — the shared `TextField`/`SelectField` components already
support a read-only/disabled state; use that rather than a bespoke
"display-only" component.

The client-lookup-then-populate interaction should use `LookupSelect`'s
existing selection-callback contract to bind all dependent fields at once,
consistent with the recipe's guidance to bind generated model metadata
through component props rather than ad hoc per-field state management.

"Internal Box Folder", "Folder Name", and "Read/Write Password" are
denormalized onto the routing record per the decision above. Confirm during
implementation whether they still warrant a separate related entity
(e.g. `ClientBoxFolder`) purely for data-modeling clarity, given section 5.3's
description of a related but distinct temp-folder-creation flow with its own
exception path — this is a modeling detail, not a storage-location decision
(both approaches keep the data denormalized/self-contained per the decision
above).

**Read/Write Password handling, specifically:**
- Store the password value encrypted at rest, not as plaintext, following
  whatever secret-handling mechanism this application already uses for other
  credentials (see `docs/release/deployment-reference.md` secrets handling
  rules — do not invent a new, one-off encryption approach for this field).
- The backend must **never return the raw password value** to the frontend
  in any query response used by this screen. The frontend only needs to know
  "a password is on file" (boolean/status), not the value itself.
- The PDF-locking operation (downstream, out of scope for this screen but
  the reason this field is stored) must read the encrypted value server-side
  and use it to lock the generated PDF without the value ever transiting to
  a browser.
- Mark this field with the framework's audit-redaction convention
  (`meta.audit.redact: true` in the `.appfw/model` entity definition, the
  same pattern used for `email` on `Account` in the CRM reference example)
  so it never appears in plaintext in audit logs.

## Security, Privacy, And Governance

- **Read/Write Password is a functional secret, confirmed necessary for
  downstream PDF-locking** (see Decision Provenance). This changes its
  handling from "sensitive display field" to "credential requiring the same
  rigor as any other stored secret":
  - Encrypted at rest; never logged, never returned to the frontend as
    plaintext, never rendered in the UI as a visible value.
  - The UI element on this screen is a **status indicator**, not a
    `TextField` displaying the value — per the updated Field-Level Contract.
  - `meta.audit.redact: true` on the model field.
  - This elevates the field from a UI/UX decision to a **security-review
    item**: flag this screen for security sign-off before release, given a
    functional credential is now part of the persisted record.
- Client reference data (name, email, office location) is PII — confirm
  field-level audit/redaction requirements consistent with the framework's
  existing audit model before this ships.
- This screen has no destructive or policy-sensitive *action* of its own
  (no `governed-action` recipe needed for the form itself) — but the
  password-storage decision means the **backend/data-model work behind this
  screen** should go through the framework's spec-driven change harness at
  **full spec depth**, not lightweight, per its trigger list ("the change
  affects security, authentication, authorization, secrets, encryption,
  tenant isolation... audit"). This screen-level spec should be treated as a
  companion to that full spec, not a substitute for it.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| No component drift from PDS catalog | `scripts/check-pds-components.mjs --json` | push |
| Catalog visual/a11y evidence unaffected | `scripts/check-pds-catalog-evidence.mjs --json` | PR |
| Read-only fields cannot be edited via keyboard/DOM manipulation | Frontend component test | PR |
| Client selection populates all 9 read-only fields in one bound update | Frontend integration test | PR |
| Additional Email and Send Notification remain editable pre- and post-lookup | Frontend component test | PR |
| Password field data-classification decision documented | Security sign-off note in PR | merge |

## Test And Execution Feedback Plan

Unit/component tests for the form's read-only vs. editable field behavior;
an integration test simulating a client search-and-select round trip;
`scripts/appfw product validate --json` / `generate --check --json` if any
model field changes are made. If execution reveals the read-only fields need
richer states (e.g., "no client selected yet" placeholder vs. "client
selected, field populated"), update this spec's field-level contract before
continuing rather than improvising undocumented states.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Read/Write Password exposed via API response or DOM | Encrypt at rest; never return plaintext to frontend; UI shows status indicator only, not the value; audit-redact the field | TBD | Mitigated by design — verify at implementation and security review |
| Client data staleness since denormalized at lookup time | Accepted tradeoff per Decision Provenance; revisit if password rotation must propagate to existing records | Product Owner | Accepted |
| Password field decision was made at screen-spec level but has data-model/security-architecture implications | Route the underlying storage/encryption/PDF-locking design through a full-depth spec per the spec-driven change harness, not just this screen spec | TBD | Open — full spec still needed |

## Tech Debt And Follow-Up

None identified yet — flag here if the Options Considered decision above is
deferred with a "denormalize for now, revisit" acceptance.

## Handoff Notes

This spec covers Tab 1 only. Tabs 2 (Tax Document Collection) and the
document validation rules (5.5, 5.5a) are separate, larger specs — Tab 2 in
particular involves a repeating document list with duplicate-check logic and
K-1-specific handling, which does not fit the `generated-entity-form` recipe
cleanly and will need its own recipe mapping (likely closer to
`generated-entity-workspace` for the document list, with governed-action
elements for validation/duplicate handling — assess before scoping that
spec). The two storage/handling decisions (denormalize client data; store the
password as a functional secret for later PDF-locking) are now made — see
Decision Provenance. What remains open before implementation: a **full-depth
spec** (per the spec-driven change harness) covering the password's
encryption approach, where/how the PDF-locking operation consumes it, and
security sign-off — this screen-level spec intentionally does not attempt to
own that design. Do not let a coding agent implement the password storage
mechanism from this screen spec alone; point it at the full spec once it
exists.
