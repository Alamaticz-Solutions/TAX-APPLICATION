# ADR 0001: Query IR

## Status

Accepted.

## Context

The backend supports PostgreSQL, MongoDB, MS SQL Server, and Snowflake. Early
provider implementations compiled GraphQL-shaped JSON directly, which made it
easy for filter, sort, projection, relationship, pagination, and aggregation
semantics to drift by provider.

## Decision

DataAccess constructs shared typed plans before provider compilation:

- `FilterAst`
- `SortAst`
- `SelectionTree`
- `MutationPlan`
- `QueryPlan`
- `AggregatePlan`
- `Pagination`

Providers compile these plans into SQL, BSON, or provider-native API calls.
Legacy JSON bridge methods exist only for compatibility while provider code is
migrated.

## Consequences

- Canonical validation happens before provider compilation.
- Security controls such as access filters and query cost budgets are applied
  once at the shared boundary.
- Provider certification can assert semantics against shared plan behavior.
- Provider compilers still own dialect-specific optimization and parameter
  binding.
