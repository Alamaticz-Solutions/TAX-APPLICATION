# Specs index

Source of truth for requirements: `source/TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`
(business spec v1.0). Feature specs below translate it into the App Framework model.

| # | Spec | Status | Notes |
|---|---|---|---|
| 1 | `spec-1-data-model.md`: entities, status + `ExceptionTask` control flow | draft | Foundation for every other spec. |
| 2 | `spec-2-roles-permissions.md`: roles, RBAC, data scoping | draft, implemented | Revised 2026-09-28: staff see only records assigned to them (BR-19/20). Policies live in `.appfw/model/schemas/tax_routing/rbac/`; verified by `scripts/smoke/rbac_smoke.py`. Three known gaps listed at the end of the spec (plaintext password exposure is the important one). |
| 2b | `../../docs/architecture/rbac-design.md`: database-driven RBAC (users, roles, permissions, scope) | implemented | Replaces the role-named Rego policies with `input.user.grants`, resolved from `app_user`/`role`/`permission`/`role_permission`/`user_role` by `backend/src/services/authz.rs`. Supersedes spec-2. Frontend permission wiring and RBAC admin screens are the remaining follow-up (see the design doc section 13). |
| 5.4 | `spec-5.4-taxpayer-details-tab.md`: Tab 1, taxpayer details | draft | Read/Write password is a functional secret; never returned to the browser. |
| 5.5b | `../../docs/architecture/dynamic-form-engine-design.md`: database-driven screens, fields, conditions and document-type requirements | design, not built | Section 5.5/5.5a's field visibility rules (K-1/Form 8308 attachments, entity fields) modeled as a reusable catalogue, following project-governance's `DataPoint`/`Condition`/`FieldValue` v6 pattern. Also moves `documentTypes.ts`'s hardcoded arrays into the database. Awaiting go-ahead to build. |
| - | `signature-experience-activity-queue.md` | unreviewed | Added to this folder by someone else; not yet reconciled with the rest. |

## Decisions recorded outside the specs

- Two roles as RBAC: `tax_staff` (own records) and `tax_admin` (all records, reassign, cancel).
- Storage: Box in the source spec, SharePoint later, behind a `DocumentStore` boundary.
- The 5-minute wait timers, "3,000+" batch size, "Resolve Selected Cases" and "Change Stage" are
  deliberately deferred; see `docs/architecture/tax-document-routing-design.md` section 3.
