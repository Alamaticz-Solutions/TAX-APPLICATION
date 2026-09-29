# Product Extension API

This page defines the stable API boundary product-owned handler and service code
should use when extending generated behavior.

## Goal

Product code should depend on stable abstractions, not provider internals.
That keeps regeneration safe and makes framework packaging upgrades predictable.

## Product-Owned Runtime Imports

In product-owned handler and service modules, prefer imports from:

```text
crate::product_api
```

Current status: `crate::product_api` is an interim compatibility facade while
runtime and provider internals are extracted into framework-owned packages. Do
not expand this facade with new copied framework internals in product
repositories; move stable APIs into framework crates as extraction progresses.
Stable identity contracts (`UserAuth`, `Claims`), `HandlerResult`,
`JsonValue`, `AccessAction`, `PolicyAccess`, `combine_access_filters`,
`DataStoreError`, `ConfigError`, `MetadataError`, and `QueryBuildError` now
come from `appfw-runtime`.

The extension API currently exposes:

- `DataAccess` for provider-neutral data operations;
- `UserAuth` and `Claims` for auth/identity context;
- `EntityType` for generated entity metadata at runtime;
- `HandlerContext` for generated adapter-to-handler context packaging;
- `AppError` plus `HandlerResult<T>` for stable error/result handling;
- `JsonValue` for JSON payloads;
- `AccessAction` and `PolicyAccess` for access-filter semantics.

Product-owned handlers and services may also import generated product schema
types from `crate::schemas::*`, delegate to generated defaults through local
handler module surfaces such as `super::generated`, and use normal Rust
standard-library or third-party dependencies declared by the product backend.
The CRM sample demonstrates this split with `account_health_impl`: the handler
keeps the generated custom method signature and delegates durable workflow logic
to `backend/src/services/account_health.rs`.

Generated GraphQL glue obtains request identity, product `DataAccess`, and
entity metadata through `crate::product_api` helpers, then packages those
values in `HandlerContext` before invoking product handler methods. Product-owned
handlers and services receive `Option<UserAuth>`, `DataAccess`, `EntityType`,
and selections as explicit arguments and should not parse
`async_graphql::Context`, call `ctx.data_unchecked`, or import
`crate::handlers::auth::*` directly.

## Generated Defaults And Overrides

Each product-owned handler file imports its generated defaults:

```rust
#[allow(unused_imports)]
pub(crate) use super::generated::<entity>::*;
```

If the product does not need custom behavior for a standard operation, leave the
operation out of the product-owned file and let the generated default handle it.
This keeps product code focused on intent and prevents framework-generated CRUD
and query boilerplate from becoming copied product source.

When a product intentionally overrides a standard operation (`find_impl`,
`get_impl`, `query_impl`, `aggregate_impl`, `create_impl`, `update_impl`, or
`delete_impl`), mark the file with `appfw: override-standard` near the override
so reviewers can distinguish an intentional product decision from generated
boilerplate drift. Custom methods remain product-owned extension points and do
not need that marker.

## Prohibited In Product-Owned Handlers/Services

Do not import framework-owned backend internals directly from:

```text
crate::data::...
crate::config::...
crate::routes::...
crate::handlers::auth::...
crate::mcp::...
crate::observability::...
crate::admin_ui::...
crate::app_state::...
crate::data::clients::...
```

These modules remain framework-owned implementation detail surfaces. If product
code needs a capability from one of them, expose a stable contract through
`crate::product_api` or a framework package instead of importing the internal
module directly.

This boundary is enforced by `scripts/appfw boundary-check`, which runs from
the framework CLI without compiling the product server crate. Enforcement
parses Rust source with `syn`, checks grouped and renamed `use` trees, and also
walks direct `crate::...` path references in product-owned handler/service
code. The check also rejects product-owned `async_graphql::Context` imports and
direct `ctx.data_opt`/`ctx.data_unchecked` access, and verifies that standard
generated handler defaults are delegated from `generated.rs` unless a
product-owned file explicitly marks a standard operation override.
It also fails when retired framework implementation files reappear in the CRM
sample or split-root product backend template, which keeps the product-facing
extension API from expanding through copied runtime source.

## Allowed Flow

```text
generated route/schema glue
  -> product handler override
  -> product service (optional)
  -> DataAccess
  -> configured provider implementation
```

## Why This Boundary Exists

- Prevents product logic from coupling to provider-specific query builders.
- Keeps generated and human-owned responsibilities clear.
- Makes runtime/provider extraction into framework packages incremental and safe.
- Preserves a stable extension path as framework internals evolve.
