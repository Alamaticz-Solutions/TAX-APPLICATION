# Framework issues to report to PDS

Findings in `app-framework/` (vendored; this product does **not** patch it) that PDS
needs to act on. Each finding states what was observed, where, and what we did
instead.

---

## S1 — No `FrameworkProvider::SharePoint` / `DataSourceType` entry for SharePoint (Microsoft Graph)

**Found:** while adding SharePoint document storage as a separate provider
(`docs/architecture/sharepoint-integration-plan.md`).

**Observed**

- `app-framework/appfw_runtime/src/provider_keys.rs` defines `FrameworkProvider`
  with `ServiceNow`, `Salesforce`, and the other vendors (variants at lines 11–14,
  string keys at 62–65, parser at 98–101). There is **no** `SharePoint` (and no
  Microsoft Graph) variant, so `provider_keys`, `provider_capabilities` and the
  capability matrix cannot represent it.
- `app_gen`'s config schema (`app_gen/src/bootstrap_types/data_source.rs`,
  `app_gen/src/config_contract.rs`, `app_gen/src/validation.rs`) has no
  `DataSourceType` entry for it, so a SharePoint data source cannot be declared in
  `.appfw/model/data_sources/`.
- Consequence: the provider crate `appfw_provider_sharepoint` can exist and be
  tested, but the framework cannot select, validate, expose or certify it.

**What we did instead (no framework file changed)**

- Added `app-framework/appfw_provider_sharepoint/` as a new, **network-free** crate
  (own crate, own workspace member) following the Salesforce file layout and the
  ServiceNow evidence-gated registry pattern.
- Put execution in the product module `backend/src/services/sharepoint/`, which the
  framework cannot supply yet.
- Marked every operation's `FrameworkProviderIdentityEvidence` gate as open.

**Ask of PDS**

1. Add `FrameworkProvider::SharePoint` (key `sharepoint`) to `provider_keys.rs` and
   the matching entries in `provider_capabilities.rs`.
2. Add a `SharePoint` value to the `DataSourceType` enum
   (`app_gen/_config/schemas/system/gql_enum_types/data_source_type.yaml`) and the
   config contract / validation in `app_gen`.
3. Adopt or replace `appfw_provider_sharepoint` (its `registry` already builds and
   validates request plans) and decide whether Microsoft Graph is one provider with
   several named-operation families (Teams, SharePoint) or several providers. We
   chose separate providers so a change to one cannot affect the other.
4. Wire docs-check parity (`scripts/appfw framework docs-check`) for
   `appfw_provider_sharepoint/docs/vendor-contract.md`. It was written to the
   template but **has not been run through that gate**.

**Severity:** medium. Does not block local build/test; blocks framework-level
certification and any data-source-driven use.

---

## S2 — No durable store for SaaS write ledger / audit in the product model

**Observed:** the governed-write gate needs an idempotency ledger and an audit
sink. The framework's generated model has no ledger entity for SaaS writes in this
product's schema (Project Governance has a `GraphWriteAttempt` entity; Tax
Document Routing does not).

**What we did:** the gate is written against `WriteLedger` / `WriteAuditSink`
traits with non-durable in-memory implementations only. A durable implementation
needs a new entity in `.appfw/model` (a generated surface).

**Ask of PDS:** either provide a shared, provider-neutral write-attempt entity as a
framework fragment (the Project Governance column `meeting_id` is a generic
subject reference despite its name) or confirm the recommended pattern for products
to model their own.

**Severity:** medium for go-live (a restart loses idempotency state).

---

## S3 — Windows host cannot build/test the workspace without the MSVC toolchain

**Observed:** `cargo test` on a Windows developer machine without Visual Studio
Build Tools fails with `linker 'link.exe' not found`, even for dependency-free
crates. Tests were run under WSL Ubuntu instead.

**Ask of PDS:** document the Windows prerequisite (VS Build Tools + Windows SDK, or
the WSL route) in `docs/start/` so it is not rediscovered per developer.

**Severity:** low.
