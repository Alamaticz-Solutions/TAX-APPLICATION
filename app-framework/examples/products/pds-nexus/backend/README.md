# PDS Nexus Backend

This backend crate is product-owned. It starts with the selected provider and
ingress feature flags from `.appfw/poc-intake.yaml`.

Generated route, schema, handler, operation, and data-access files should be
created only after the PoC-derived evidence entity model is written under
`.appfw/model/schemas/denovo_workflow`.

Run from the product root after the model exists:

```bash
scripts/appfw product generate
scripts/appfw product serve
```
