# Database Package

This directory is the CRM product's generated database package surface.

```text
database/_pkg/
```

The product owns the generated package artifacts and any product migration
content under `_pkg`. The database runner, provider clients, and migration
command implementation are framework-owned and are executed by `scripts/appfw`
from the framework checkout.

Run database workflows from the product root:

```bash
scripts/appfw migrate
scripts/appfw migrate doctor
scripts/appfw migrate plan --json
scripts/appfw migrate lint --phase all --json
scripts/appfw migrate rollback-guide --json
scripts/appfw migrate status --json
scripts/appfw migrate apply --json
```

Connection metadata is generated from product data-source config into:

```text
database/_pkg/data_sources.yaml
```

Do not add credentials to this directory. Local credentials belong in local env
files or a secret manager, and deployed credentials belong in approved platform
secret injection.
