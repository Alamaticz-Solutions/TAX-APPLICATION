# PostgreSQL Client Notes

This directory contains the PostgreSQL provider implementation used by the
generated backend.

## Responsibilities

- Compile the shared query/filter/sort model into PostgreSQL SQL.
- Preserve access-control filters when combining user filters with policy
  filters.
- Map generated schema metadata into SQL selection, mutation, relationship, and
  pagination behavior.
- Keep behavior aligned with the other providers unless a limitation is
  documented.

## Key Files

```text
postgres_client.rs
cte.rs
filter.rs
literal.rs
sort.rs
```

Provider behavior should be verified through focused unit tests where possible,
then through the root framework checks:

```bash
scripts/appfw test
```

Historical scratch notes for this provider are archived in
`docs/archive/POSTGRES_CLIENT_SCRATCH_NOTES.md`.
