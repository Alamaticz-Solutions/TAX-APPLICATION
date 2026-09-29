# RBAC design: database-driven roles, permissions and record scope

Status: **implemented and passing local verification 2026-09-29.** Written 2026-09-28 against the
business spec (`.appfw/specs/source/TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`,
sections 2, 9, 10, 12). Section 13 at the end records what changed between the design and the
build, and what is still open.

## 1. What exists today (audit)

| Area | Today | Source |
|---|---|---|
| Users | No user table. The caller is whatever the identity provider (Okta) puts in the token: `user_name`, `tenant_id`, `roles[]`, `scopes[]`. Local dev: no header = admin, or `Bearer appfw-local:user=..;roles=..`. | `app-framework/appfw_runtime/src/auth.rs` |
| Roles | Plain strings from the token (`tax_staff`, `tax_admin`). **No role table.** | same |
| Permissions | **None.** Access is decided by Rego policies that test role names directly. | `.appfw/model/schemas/tax_routing/rbac/*.rego` (10 files, all name `tax_staff`/`tax_admin`) |
| Record scope | Rego adds a row filter `assigned_operator = input.user.user_name` for staff. `assigned_operator` is a free-text user name, not a foreign key. | `tax_routing_record.rego`, `tax_document.rego` |
| Policy input | Only `tenant_id`, `user_name`, `roles`, `scopes`. No user id, no permissions. | `backend/src/config/app_config.rs` `evaluate_user_access` (hand-owned) |
| Frontend | Mock `Role = 'standard' \| 'admin'`; 21 places test the role or call `canCancel`/`canActOn`. **This is exactly the hardcoding the requirement forbids.** | `frontend/src/features/shared/state/TaxRoutingProvider.tsx` and screens |
| Framework RBAC tables | **None in the framework.** The framework's model is roles-as-token-strings plus Rego. | `app-framework` (searched runtime, generator, docs) |
| Precedent to reuse | Project Governance keeps a data-driven access table (`ActivityRoleAccess`: role, may_view, may_edit, may_decide, unique per activity) and resolves the caller to a `users.id` in hand-owned code before evaluating policy. Same pattern is used here. | `project-governance/.appfw/model/.../activity_role_access.yaml`, `backend/src/platform/identity.rs` |

Conclusion: nothing to reuse as tables. What we **can** reuse is the framework's extension point:
`evaluate_user_access` is product-owned, so it may enrich the Rego input with data loaded from the
database. That is the whole trick that makes RBAC database-driven without patching the framework.

## 2. Design principles

1. Roles are **data**. Rego and code never name a role; they ask "does this user hold permission X, with what scope?".
2. The identity provider only authenticates. Who the user is and what they may do come from our tables.
3. One resolver (`services/authz`) is the only place that reads the RBAC tables. Rego policies, custom methods and the
   `myPermissions` API all consume its output, so the three can never disagree.
4. Enforcement is at the backend (Rego row filters + service checks). The frontend only reflects what the backend returns.
5. Every change to roles/permissions/assignments, and every permission-sensitive business action, is audited.

## 3. Database schema (schema `tax_routing`)

Modelled as framework entities (so DDL, GraphQL, audit companions and the UI contract are generated), not hand-written SQL.

```
app_user 1───* user_role *───1 role 1───* role_permission *───1 permission
   │                                                            
   └──1───* tax_routing_record.assigned_user_id (FK, replaces free-text assigned_operator)
```

### `app_user`
| Column | Type | Notes |
|---|---|---|
| id | uuid PK | |
| user_name | text NOT NULL | the token's `user_name`; the join key from authentication |
| tenant_id | text NOT NULL | |
| display_name | text NOT NULL | |
| email | text | |
| is_active | boolean NOT NULL default true | inactive = no permissions, whatever the token says |
| created_at / updated_at | timestamptz | |

Constraints: `UNIQUE (tenant_id, user_name)`. Index: the unique index doubles as the resolver lookup.

### `role`
| Column | Type | Notes |
|---|---|---|
| id | uuid PK | |
| code | text NOT NULL | stable key, e.g. `TAX_STAFF`, `TAX_ADMIN` |
| name | text NOT NULL | display: "Tax Staff", "Tax Admin" |
| description | text | |
| is_system | boolean NOT NULL default false | seeded roles cannot be deleted |
| is_active | boolean NOT NULL default true | |

Constraints: `UNIQUE (code)`.

### `permission` (the capability catalogue; seeded, changed only by migration)
| Column | Type | Notes |
|---|---|---|
| id | uuid PK | |
| code | text NOT NULL | e.g. `routing_record.cancel` |
| name / description | text | shown in the admin UI |
| resource | text | entity the permission governs, e.g. `tax_routing_record`; NULL for UI-only capabilities |
| action | text | `create` \| `read` \| `update` \| `delete`; NULL for non-CRUD capabilities |
| owner_field | text | column that ties a row to its owner for `own` scope, e.g. `assigned_user_id` |

Constraints: `UNIQUE (code)`; `CHECK (action IN ('create','read','update','delete') OR action IS NULL)`.

### `role_permission`
| Column | Type | Notes |
|---|---|---|
| id | uuid PK | |
| role_id | uuid FK -> role | ON DELETE CASCADE |
| permission_id | uuid FK -> permission | ON DELETE RESTRICT |
| scope | text NOT NULL default 'all' | `own` = only rows the user owns, `all` = every row |

Constraints: `UNIQUE (role_id, permission_id)`; `CHECK (scope IN ('own','all'))`. Index: `role_id`.

### `user_role`
| Column | Type | Notes |
|---|---|---|
| id | uuid PK | |
| user_id | uuid FK -> app_user | ON DELETE CASCADE |
| role_id | uuid FK -> role | ON DELETE RESTRICT |
| granted_by | uuid FK -> app_user | who assigned it |
| granted_at | timestamptz | |

Constraints: `UNIQUE (user_id, role_id)`. Indexes: `user_id`, `role_id`.

Change to existing entities: `tax_routing_record.assigned_operator` (text) becomes `assigned_user_id` (uuid FK -> app_user),
and the same on `tax_document` and `exception_task`. This makes ownership a real relationship, lets an admin reassign by
picking a user, and removes name-string matching.

All five tables carry the framework `audited` facet, which generates the `*_audit` companions automatically.

## 4. Permission catalogue and matrix (from spec section 2.2)

| Permission code | Spec capability | Resource / action | Tax Staff | Tax Admin |
|---|---|---|---|---|
| `routing_record.create` | Create new routing record | tax_routing_record / create | all | all |
| `routing_record.read` | See records (data scoping, BR-19/20) | tax_routing_record / read | **own** | **all** |
| `routing_record.process` | Process own assigned records | tax_routing_record / update | **own** | all |
| `routing_record.view_assignee` | See who each record is assigned to (BR-20) | none (UI capability) | - | all |
| `routing_record.bulk_resolve` | Bulk-select and resolve (deferred, section 3 of the design doc) | none | all | all |
| `routing_record.cancel` | Cancel / withdraw (BR-18) | none (custom method) | - | all |
| `routing_record.reassign` | Change the assigned operator | none (custom method) | - | all |
| `tax_document.*` (create/read/update/delete) | Documents follow their record | tax_document | own | all |
| `exception_task.read` / `.update` | View exception queue | exception_task | read all, update own | all |
| `reference_data.read` | Taxpayers, entities, routing table | tax_payer_ref_data, tax_entity_list, document_type_route / read | all (active rows) | all |
| `reference_data.manage` | Maintain reference data | same / create, update, delete | - | all |
| `rbac.manage` | Manage users, roles, assignments | rbac tables | - | all |
| `audit.read` | Read audit trails | *_audit / read | - | all |

The matrix is **seed data**, in a migration, never in code. Changing what Tax Staff may do is an `UPDATE role_permission`.

## 5. Record-level access model

- Ownership = `assigned_user_id`. A permission with `scope = own` is enforced as the row filter
  `owner_field = current user's id`; `scope = all` adds no filter. The filter is added by the policy and
  applied in the SQL `WHERE`, so a non-owner can neither list nor fetch by id nor update the row.
- Documents and exception tasks carry the owner id too (denormalised), because a policy sees one row at a time.
- The tenant filter the framework already applies stays in front of all of this.

## 6. Backend authorization flow

```
request + token
  -> framework authenticates -> UserAuth {user_name, tenant_id, ...}
  -> services/authz::resolve(user)            [the ONE reader of the RBAC tables]
       SELECT the user's active roles, their role_permission rows, permission rows
       -> Principal { user_id, grants: [{code, resource, action, scope, owner_field}] }
       cached per (tenant, user_name), ~60 s TTL, evicted when rbac tables change
  -> evaluate_user_access(entity, action, user)
       Rego input.user gains { id, grants }        (hand-owned change in app_config.rs)
       Rego: find a grant where resource = input.entity_type and action = input.action
             scope all  -> {"allow": true, "filter": {}}
             scope own  -> {"allow": true, "filter": {owner_field: {"_eq": input.user.id}}}
             none       -> deny
  -> SQL runs with the filter
custom methods (cancel_record, reassign_record, retry_failed_step)
  -> authz::require(principal, "routing_record.cancel", Some(&record))   same resolver, same grants
```

Rego then contains **no role names and no permission names**: one shared rule body reads `input.user.grants`.
Unknown or inactive users resolve to an empty grant list, which is a deny.

## 7. API-level authorization

| Surface | Mechanism |
|---|---|
| Generated CRUD GraphQL | Rego grant rule above (row filter + allow/deny) |
| Custom methods (cancel, reassign, retry) | `authz::require(...)` at the top of the service function; checks the permission and, for `own`, that the record is the caller's |
| Reading the RBAC tables | Policies allow it only with `rbac.manage`; users read their own grants only through `myPermissions` |
| `myPermissions` (new query) | Returns `[{code, scope}]` for the caller, produced by the same resolver |
| Audit entities | `audit.read` |

## 8. Frontend permission handling

- On sign-in the app calls `myPermissions` once and keeps the result in a `PermissionsProvider`.
- API: `usePermission('routing_record.cancel')` -> boolean, and `<Can permission="routing_record.cancel">...</Can>`
  (renders nothing, or a disabled control with a reason, when false).
- Screens never test a role. "Assigned To" column = `view_assignee`; "Cancel / Withdraw" button = `routing_record.cancel`;
  sidebar entries are driven by a nav config that names the permission each entry needs; "All Cases" = `routing_record.read` with scope `all`.
- The role switcher in the header is a mock-only tool; it is replaced by real sign-in.
- Hiding a control is convenience only; the backend rejects the call regardless.

## 9. Audit requirements

1. **Configuration changes:** all five RBAC tables are `audited`, so every grant, revoke, role change and deactivation records who, when, and before/after.
2. **Sensitive actions** are written by the service to an action log (`routing_record_audit` already exists for record changes; add explicit events):
   `record.cancelled` (resolution + mandatory comment, per spec 9.1), `record.reassigned` (from, to), `record.retried`,
   `rbac.role_granted` / `rbac.role_revoked`.
3. **Denials:** `authz::require` failures are logged with user, permission, target id and reason (security event, not shown to end users).
4. Audit rows are readable only with `audit.read`. (Known framework gap: today the audit gate also needs the legacy `admin` role; see spec-2.)

## 10. Migrations

No hand-written migration files exist yet, and none were needed for this pass: the five RBAC
tables, the `assigned_user_id` foreign keys, and every seed row are all first-class framework
constructs —

- `app_user`, `role`, `permission`, `role_permission`, `user_role` are modeled entities
  (`.appfw/model/schemas/tax_routing/entity_types/{app_user,role,permission,role_permission,user_role}.yaml`);
  `product generate` produces their DDL in `database/_pkg/schemas/tax_routing/tables.pg.sql`
  (idempotent `CREATE TABLE` / `ALTER TABLE ADD COLUMN IF NOT EXISTS`).
- `assigned_operator` (String) was changed to `assigned_user_id` (Uuid, FK to `AppUser`) directly
  on `TaxRoutingRecord`, `TaxDocument` and `ExceptionTask` in the model. Because the local dev
  database holds no real rows (every smoke-test row is prefixed `SMOKE-` and cleaned up after
  itself), this was a direct model edit and a fresh `product migrate`, not an expand/backfill/contract
  sequence. **A product with real data must not do this the same way** — see the note below.
- The permission catalogue, the two roles, the full `role_permission` matrix, and three bootstrap
  users are all `.appfw/model/schemas/tax_routing/seeds/*.yaml` (the same first-class seed
  mechanism the `crm` example profile uses), generated into
  `database/_pkg/schemas/tax_routing/seed.pg.sql` as `INSERT ... WHERE NOT EXISTS` statements —
  safe to re-run, and this is how `scripts/appfw product migrate` applies them locally.

**If this repo already held real routing records** when `assigned_operator` → `assigned_user_id`
was cut over, the safe sequence would be the expand/backfill/contract migrations
`database/_pkg/migrations/README.md` describes: add `assigned_user_id` nullable, backfill it from
`assigned_operator` by matching `app_user.user_name`, verify, then drop `assigned_operator` in a
later migration. That sequence was **not** needed or run here because there was no data to
preserve; do not read the model change above as a precedent for a cutover with real data.

Bootstrap: `seeds/04_app_user.yaml` and `05_user_role.yaml` seed three users — `local-dev` (tenant
`local`, Tax Admin, matching the framework's own "no Authorization header = admin" convenience for
`ENV_NAME=local`) and `alex` / `dana` (tenant `t1`, Tax Staff / Tax Admin, matching the frontend's
mock role switcher). Real deployments create the first admin the same way (a seed row with a
known `user_name`) and everyone else through the RBAC admin screens once built (section 13).

## 11. Build order (as executed)

1. Model the five entities plus `assigned_user_id`, the permission catalogue and the
   `role_permission` matrix as seed data; `product validate` / `product generate`.
2. `backend/src/services/authz.rs`: the `AuthzCache` resolver — connects to `pg_primary`,
   loads every active user's grants into memory, refreshes on a 30s timer, and answers
   `resolve(tenant_id, user_name) -> Principal` synchronously from that cache. Wired into
   `AppConfig` (`config/app_config.rs`), which injects `id` and `grants` into the Rego input
   inside `evaluate_user_access`.
3. All ten `.appfw/model/schemas/tax_routing/rbac/*.rego` files rewritten to one shared,
   generated-per-file grant rule (`best_scope` / `access_for_scope` / `own_owner_field`) that
   reads only `input.user.grants` — no role name appears in any of them.
4. Custom methods `cancel_record` and `reassign_record` on `TaxRoutingRecord`
   (`services/routing_record_actions.rs`), and `myPermissions` on `AppUser`
   (`handlers/tax_routing/app_user.rs`), all gated through `authz::require`.
5. `scripts/smoke/rbac_smoke.py` rewritten to seed real `app_user`/`user_role` rows (referencing
   the real seeded role ids) instead of passing a role in the bearer token, and to poll for the
   authz cache's refresh before asserting.
6. **Not done in this pass:** the frontend `PermissionsProvider` / `usePermission()` / `<Can>`
   from section 8, and the RBAC admin screens from section 13. The frontend still runs entirely
   on mock data (tracked separately: "wire frontend screens to GraphQL"), so there is nothing yet
   for a frontend permissions layer to call. `retry_failed_step` also remains an unimplemented
   stub, unchanged by this work.

## 12a. A correction to the original finding (spec-2's gap 3)

Spec-2 originally claimed "`tax_admin` alone cannot read `*_audit` entities; the framework audit
gate appears to need the `admin` role as well." Rebuilding `scripts/smoke/rbac_smoke.py`'s audit
check against this implementation traced that claim to the test itself, not the framework: the
old script selected `{ items { __typename } }` on a query whose custom selection-set builder
(`handlers/selections.rs::get_entity_selections`) resolves each requested field against the
entity's own props and has no case for a GraphQL meta-field like `__typename` — so the request
never reached policy evaluation at all (confirmed with `LOG_LEVEL=debug`: no "querying items" or
policy-evaluation trace appears for that call). Selecting a real column (`audit_id`) reads the
audit trail successfully. There never was a framework-level "needs the literal `admin` role" gate;
audit access is exactly what `*_audit.read` permissions say it is, the same as every other entity.

## 12. What was not fully resolved

1. **`retry_failed_step`'s parent-record ownership check** (section 6, `exception_task.rego`'s
   comment) is unaffected by this pass: the method is still a stub (`custom method
   \`retry_failed_step\` is not implemented yet`), predating RBAC work and out of this scope.
2. **Reassignment does not cascade to documents.** `reassign_record` updates
   `TaxRoutingRecord.assigned_user_id` only. `TaxDocument.assigned_user_id` (and
   `ExceptionTask.assigned_user_id`) are denormalised copies that a reassignment leaves stale
   until the routing engine service (still unbuilt) keeps them in step. A staff member's "own"
   scope on documents can therefore lag an admin's reassignment. Flagged in
   `services/routing_record_actions.rs`, not hidden.
3. **`cancel_record`'s `resolution_status` / `comments` are not persisted.** `TaxRoutingRecord`
   has no matching columns yet (tracked in the design doc's model-work backlog). The custom
   method accepts both, logs them via `tracing::info!`, and returns them in its response, but a
   `queryTaxRoutingRecords` read today cannot show why a record was cancelled.
4. **The authz cache's 30-second refresh** means a grant change (or a brand-new user) takes up to
   30s to take effect for requests already past the auth boundary. `scripts/appfw` migrations and
   the smoke test both account for this by polling.
5. **The plaintext Read/Write password exposure** (spec-2's original finding 1) is unrelated to
   RBAC and still open.

## 13. Frontend and admin-screen work (still to do)

Section 8's `PermissionsProvider`, `usePermission()` and `<Can>` wrapper, and the RBAC admin
screens (manage users, roles, the permission matrix) from the original design, are not built.
Both need the frontend to be wired to the live GraphQL API first — today every screen runs on
`frontend/src/features/shared/services/mock*.ts`. This is tracked as existing, separate follow-up
work, not new scope introduced here.
