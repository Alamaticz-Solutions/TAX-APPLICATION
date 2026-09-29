# PDS Nexus Database Package

This directory is the product-owned generated database package surface.

The selected provider is `PostgreSQL` through data source `pg_primary`.
Generated package artifacts and product migrations belong under:

```text
database/_pkg/
```

Run database workflows from the product root after the model exists:

```bash
scripts/appfw product migrate doctor
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate rollback-guide --json
```
