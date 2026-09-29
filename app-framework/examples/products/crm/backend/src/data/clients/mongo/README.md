# MongoDB Client Notes

This directory contains the MongoDB provider implementation used by the
generated backend.

## Responsibilities

- Compile the shared query/filter/sort model into MongoDB documents and
  aggregation pipelines.
- Preserve access-control filters when combining user filters with policy
  filters.
- Map generated schema metadata into MongoDB selection, mutation, relationship,
  and pagination behavior.
- Keep behavior aligned with the SQL providers unless a limitation is
  documented.

## Key Files

```text
mongo_client.rs
filter.rs
literal.rs
sort.rs
```

Provider behavior should be verified through focused unit tests where possible,
then through the root framework checks:

```bash
scripts/appfw test
```

MongoDB Rust driver reference:

```text
https://www.mongodb.com/docs/drivers/rust/current/
```
