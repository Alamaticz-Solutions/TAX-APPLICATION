# Spec: Tax Document Routing — Roles & Permissions / Data Scoping

Status: draft, **superseded 2026-09-29** by database-driven RBAC — see the section at the end of
this file and `docs/architecture/rbac-design.md`. Kept for history; do not implement against the
role-literal design described below.

Spec depth: full

Owner roles:

- Product Owner: TBD
- Architect: TBD
- XO: TBD
- Implementation owner: TBD
- Review owner: TBD

## Business Value

Establishes who can see and do what across the Tax Document Routing app,
before any screen or custom method ships depending on an access model that
doesn't exist yet. Without this, the frontend has nothing real to reflect
(per ADR 0010, the UI must reflect backend authorization, never invent its
own), and admin-only fields already assumed by spec-1 (`TaxPayerRefData`,
`TaxEntityList`) have no enforcement behind them.

## Problem

Two roles exist in the original Pega application per the source spec's own
structure: Standard User (Tax Staff) and Administrator (Tax Admin). Today,
`scripts/appfw product validate` reports no RBAC policies at all for the
`tax_routing` schema (`RBAC directory not found`) — every entity is
currently unrestricted. Spec-1's Handoff Notes already assumed admin-only
write access to the two reference tables; that assumption has never been
encoded as an enforceable policy.

## Goals

- Define exactly two roles: `tax_staff` and `tax_admin`.
- Tax Staff can read and act only on `TaxRoutingRecord` rows assigned to them
  (`assigned_operator` = their user name), and on the `TaxDocument` rows of those
  records (owner denormalised onto the document). Tax Admin reads all. This follows
  business spec BR-19/20 and supersedes the earlier "no per-record filtering" decision.
- The `ExceptionTask` queue is the one shared surface: every Tax Staff member can read
  every exception (business spec 5.3); acting on one is limited to its assignee.
- Tax Admin has every Tax Staff permission, plus exclusive write access to
  `TaxPayerRefData` and `TaxEntityList`, plus exclusive ability to reassign
  `TaxRoutingRecord.assigned_operator`.
- `ExceptionTask.retry_failed_step` is restricted to the parent
  `TaxRoutingRecord`'s current `assigned_operator`, or any Tax Admin —
  this is an action-level ownership check, not a read-visibility filter.
- Frontend role-gated UI (e.g. hiding the reassign control from Tax Staff)
  reflects these backend policies; it does not independently decide access.

## Non-Goals

- A third role, team/office-based scoping, or manager/supervisor roles —
  explicitly ruled out; only `tax_staff` and `tax_admin` exist for this
  app.
- Team/office-based scoping beyond `assigned_operator` — not needed; ownership by
  assigned operator is the only row scope.
- Okta group-to-role mapping mechanics (which Okta `groups` claim values
  map to `tax_staff`/`tax_admin`) — covered by the existing Okta runtime
  contract (`OKTA_AUDIENCE`/`OKTA_ISSUER`/`OKTA_CLIENT_ID`,
  `docs/release/deployment-reference.md`); this spec defines the roles
  themselves, not the identity-provider group mapping, which is an
  operational/deployment configuration concern.
- MCP tool-surface exposure — already decided per-method in spec-1
  (`mcp_enabled` flags); this spec does not revisit those.

## Scope

`.appfw/model/schemas/tax_routing/rbac/*.rego` (new — currently missing
entirely). No entity/field changes; this is a policy-only spec against the
6 entities already defined in spec-1.

## Repository Context

RBAC lives in `.appfw/model/schemas/<schema>/rbac/*.rego`, evaluated by
`regorus` at runtime, separately from business workflow logic (confirmed
pattern from the sibling `project-governance` product: `rbac/*.rego`
policy bodies are hand-authored; the generator adds the package header and
helpers). Policy evaluation happens inside `DataAccess`, before any SQL —
a handler cannot bypass it; every read/write goes through the same
`(user, action) -> {allow, filter}` check.

Per ADR 0010 (`docs/architecture/adr/0010-frontend-security-and-data-governance.md`):
"Authentication and authorization are default-on in the shell... The UI
reflects backend authorization; it never re-implements or replaces it, and
it fails closed on permission errors." Any frontend work built on top of
this spec (Tab 1, the reassignment control, exception retry) must gate on
policy responses from the backend, not hardcode role checks client-side.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| `.appfw/model/schemas/tax_routing/rbac/*.rego` | New policy files for all 6 entities | Generated policy-check wiring inside `DataAccess`; no schema/entity changes |
| `retry_failed_step` custom method (`ExceptionTask`) | Ownership check: caller must be the parent `TaxRoutingRecord.assigned_operator` or `tax_admin` | Requires the mutation handler to look up the parent record's `assigned_operator` before allowing retry — this is Rust service-layer logic, not expressible as a single-row Rego predicate (parent-row ownership, same pattern as the sibling `project-governance` app's stage-routing checks) |
| `update_taxpayer_reference` / any future `TaxEntityList` write | Restricted to `tax_admin` | Already assumed in spec-1; this spec makes it enforceable |
| Frontend role-gated controls (reassign, exception retry) | Must check backend policy response, not client-side role string | `frontend` screens built against Tab 1 / dashboard specs |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Two roles, no per-record read filtering | Matches the earlier draft's simplicity | Any Tax Staff can see every client's tax documents; contradicts business spec BR-19 | Superseded 2026-09-28 |
| Assigned-operator row filtering on reads | Matches BR-19/20 and the field's intent; enforced in Rego so the UI cannot bypass it | Documents need the owner on the row (denormalised `assigned_operator`) because Rego sees one row | **Selected** (2026-09-28) |
| Express `retry_failed_step` ownership as a pure Rego row filter | Simpler, no Rust service-layer code needed | Rego evaluates a single row; determining "is this the record's *current* assigned_operator" at the `ExceptionTask` row still requires resolving the parent `TaxRoutingRecord`, which the sibling app's own docs identify as beyond single-row Rego (parent-row ownership pattern) | Not selected — implement as Rust service-layer check backed by a role gate in Rego |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-09-24 | Product Owner | Exactly two roles: `tax_staff`, `tax_admin`. No additional roles. | Matches source app's original two-role structure; no stated business need for more. | Revisit if a manager/supervisor or read-only auditor role is later requested. |
| 2026-09-24 | Product Owner | ~~No per-record read visibility filtering.~~ **Superseded.** | Replaced by the 2026-09-28 row below. | - |
| 2026-09-28 | Product Owner | Two roles as RBAC following the business spec: Tax Staff see only records assigned to them; Tax Admin sees all. Exception queue stays shared. | Business spec BR-19/20 and 5.3. Implemented in `.appfw/model/schemas/tax_routing/rbac/*.rego`; verified by `scripts/smoke/rbac_smoke.py`. | Revisit if per-team isolation is ever required. |
| 2026-09-24 | Product Owner | Only `tax_admin` can reassign `TaxRoutingRecord.assigned_operator`. | Consistent with admin-only write access already established for `TaxPayerRefData`/`TaxEntityList` in spec-1. | N/A |
| 2026-09-24 | Product Owner | `retry_failed_step` is restricted to the parent record's current `assigned_operator` OR `tax_admin` — an action-level ownership check, distinct from the no-filtering read policy above. | Exception resolution is treated as an operational action tied to whoever currently owns the case, not open to all Tax Staff. | N/A |

## Architecture And Implementation Notes

Two Rego role gates, applied per entity/action:

- **`tax_staff` gate**: allow, no row filter, on `TaxRoutingRecord`,
  `TaxDocument`, `ExceptionTask` reads and routine writes (create/update
  document rows, advance status via automated steps).
- **`tax_admin` gate**: allow, no row filter, on everything `tax_staff`
  can do, plus exclusive allow on `TaxPayerRefData` writes, `TaxEntityList`
  writes, and `TaxRoutingRecord.assigned_operator` reassignment.

`retry_failed_step` cannot be a pure Rego row filter (per Options
Considered) — implement as: Rego gates the *role* (must be `tax_staff` or
`tax_admin` at minimum, i.e. any authenticated app user), and the Rust
service-layer function backing `retry_failed_step` explicitly loads the
parent `TaxRoutingRecord.assigned_operator` and checks
`caller_id == assigned_operator OR caller_role == tax_admin` before
proceeding — the same parent-row-ownership pattern already documented for
sequential-chain routing in the sibling `project-governance` app.

Frontend implication: the reassignment control and the exception-retry
action button should render based on the policy/permission the backend
actually returns for the current user (e.g. a `can_reassign` /
`can_retry` field on the query response, or a 403 the UI surfaces
gracefully), not a hardcoded `if role === 'tax_admin'` check baked into
the component — per ADR 0010, "fails closed on permission errors."

## Security, Privacy, And Governance

- This is Full spec depth per the spec-driven change harness's explicit
  trigger: changes affecting authorization.
- No per-record read isolation is a **deliberate, accepted risk**, not an
  oversight — flagged explicitly below in Risks And Controls so it isn't
  silently rediscovered later as "a bug."
- `retry_failed_step`'s ownership check must fail closed: if the parent
  `TaxRoutingRecord` cannot be resolved, or `assigned_operator` is unset,
  deny rather than default-allow.
- Okta role/group mapping (which real Okta groups populate `tax_staff` vs
  `tax_admin`) is out of scope here (see Non-Goals) but must be resolved
  before production deployment — flagged in Tech Debt And Follow-Up.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| RBAC policies exist and validate | `scripts/appfw product validate --json` (no longer reports "RBAC directory not found") | push |
| Rego policy contract tests pass | `cargo test -p rego_test` (or this app's equivalent policy-test harness) | PR |
| `tax_staff` cannot write `TaxPayerRefData`/`TaxEntityList` | Rego/API test asserting deny | PR |
| `tax_staff` cannot reassign `assigned_operator` | Rego/API test asserting deny | PR |
| Non-owning `tax_staff` cannot call `retry_failed_step` on another operator's record | Service-layer unit test | PR |
| Owning `tax_staff` and any `tax_admin` CAN call `retry_failed_step` | Service-layer unit test | PR |
| Tax Staff read only their own `TaxRoutingRecord`/`TaxDocument` rows; Tax Admin reads all; exception queue is shared | `python scripts/smoke/rbac_smoke.py` (passes: 20 checks) | PR |

## Test And Execution Feedback Plan

Rego policy contract tests per entity/action (allow/deny pairs), a
service-layer unit test for the `retry_failed_step` ownership check
(covering: owner allowed, admin allowed, non-owner denied, unset
`assigned_operator` denied), and an API-level test confirming read
visibility is NOT filtered by `assigned_operator`. If execution reveals a
need for finer-grained permissions (e.g. a "view but not edit" state),
update this spec before improvising undocumented access levels.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Any Tax Staff can see every client's tax documents (no per-record isolation) | Accepted, explicit product decision — not a gap to silently fix later | Product Owner | Accepted |
| `retry_failed_step` ownership check implemented as ad hoc Rust logic instead of declarative policy | Follows established sibling-app pattern for parent-row ownership; covered by dedicated unit tests | TBD | Mitigated by design |
| Okta group-to-role mapping undefined | Must be resolved before production deployment | TBD | Open — tracked in Tech Debt |
| Frontend re-implements role checks instead of reflecting backend policy | ADR 0010 constraint stated explicitly in this spec's Architecture notes; enforce via code review | TBD | Open — verify at implementation |

## Tech Debt And Follow-Up

- Okta group → `tax_staff`/`tax_admin` mapping is not defined by this spec
  and must be resolved before any real (non-placeholder) Okta tenant is
  wired up.
- If per-office or per-team data isolation is ever required, this spec's
  "no read filtering" decision must be revisited — it is not a
  placeholder for future scoping, it is a considered choice that would
  need to be explicitly superseded.

## Handoff Notes

This spec unblocks: (1) the frontend reassignment control on whatever
screen surfaces `TaxRoutingRecord.assigned_operator`, (2) the
exception-retry action button, and (3) enforcement of the admin-only
writes on `TaxPayerRefData`/`TaxEntityList` already assumed by spec-1.
Implement the `.rego` policies and the `retry_failed_step` service-layer
ownership check together — the latter depends on the former's role gate
existing first. Do not build role-gated frontend UI before these policies
exist and are enforced; there would be nothing real for the UI to reflect.

## Implementation status (2026-09-28)

Policies for all 10 `tax_routing` entities are in `.appfw/model/schemas/tax_routing/rbac/`
and verified live by `scripts/smoke/rbac_smoke.py`. Model change: `TaxDocument.assigned_operator`
added so document rows can be filtered by owner.

Known gaps found while verifying (tracked in the design doc):
1. **Plaintext password exposure.** `TaxPayerRefData.read_write_password` (and the denormalised copy
   on `TaxRoutingRecord`) is returned in clear by GraphQL to any Tax Staff user. Rego cannot hide a
   column and the framework has no field-level read protection. Needs a design decision:
   encrypt at rest (GraphQL returns only ciphertext) and/or move the secret to a service-only entity.
2. **Column-level limits.** Rego cannot stop a staff `update` from changing `assigned_operator`
   (reassignment is admin-only). Enforce in the routing service until a custom method owns updates.
3. **`tax_admin` alone cannot read `*_audit` entities**: the framework audit gate appears to need the
   `admin` role as well. Until resolved, map Tax Admin users to both roles.

## Superseded by database-driven RBAC (2026-09-29)

Everything above describes the **role-literal** implementation (Rego policies naming `tax_staff` /
`tax_admin` directly, and `assigned_operator` as a free-text username column). That implementation
has been replaced:

- Roles, permissions and the permission matrix are now rows in `role`, `permission` and
  `role_permission` (seed data, not code); a user's grants come from `user_role`.
- `assigned_operator` (String) is now `assigned_user_id` (a real foreign key to `app_user`) on
  `TaxRoutingRecord`, `TaxDocument` and `ExceptionTask`.
- Every `.rego` policy in `.appfw/model/schemas/tax_routing/rbac/` reads only
  `input.user.grants` — no role name appears in any policy file any more.
- Gap 3 below (`tax_admin` alone could not read `*_audit` entities) is resolved: audit reads now
  require the `*.read` audit permission, granted only to Tax Admin, with no dependency on the
  framework's built-in `admin` role.
- Gaps 1 (plaintext password) and 2 (column-level reassignment limit) are addressed differently:
  gap 2 is now `cancel_record` / `reassign_record` custom methods gated by
  `services::authz::require`, not a service-layer username comparison. Gap 1 is unchanged.

See `docs/architecture/rbac-design.md` for the current design, and `scripts/smoke/rbac_smoke.py`
for the live verification (rewritten to seed real `app_user`/`user_role` rows instead of passing
a role in the bearer token).
