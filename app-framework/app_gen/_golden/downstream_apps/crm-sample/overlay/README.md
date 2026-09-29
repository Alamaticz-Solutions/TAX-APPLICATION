# App Framework CRM Sample

This application was bootstrapped from the App Framework `crm-sample` golden
profile. Treat `.appfw/model` as the application model, generated backend and
database files as reviewable build output, and human-owned handler/service files
as the durable extension surface.

## First Commands

```bash
scripts/appfw doctor
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product handoff --json
```

Use `scripts/appfw product explain ownership <path>` before editing
generated-looking files, and refresh `appfw.lock` with
`scripts/appfw product lock --write` after an intentional framework upgrade
cascade.
