# Golden Downstream App Templates

These profiles define blessed downstream application starting points for
`scripts/appfw product new`. A profile points at a product template, then applies a
small overlay on top.

The `crm-sample` profile is the current executable reference implementation. It
is appropriate when a developer wants a complete model-driven sample app. It is
not the desired product model for citizen-developer PoC conversions.

When the starting point is a workbook, static HTML app, embedded data set, or
other vibe-coded proof of concept, use
`scripts/appfw product new <target> --profile product-intake` and
`docs/lifecycle/intake-and-discovery.md` as the lifecycle contract. That CLI path
asks for the backend provider, MCP need, Kafka need, and UI mode, writes the
intake files plus product-owned backend, database, API-test, policy-test,
selected-provider compose, and optional frontend starter surfaces, and stops
before generation. Those product files must not contain CRM entities, CRM docs,
or CRM UI screens. The AI harness should inspect the PoC artifacts, create the
target product schema and frontend from those artifacts, and use the CRM
schema/UI only as a reference for App Framework standards.

Each profile must include a `profile.json` file with:

```json
{
  "name": "crm-sample",
  "description": "Short product-facing description.",
  "template": "examples/products/crm",
  "overlay": "overlay",
  "verification": [
    "scripts/appfw product validate --json",
    "scripts/appfw product test"
  ]
}
```

`scripts/appfw product new --list-profiles --json` validates that manifest against the
registered profile and emits the profile path plus verification commands for
agents and CI.

Keep overlays focused on app-facing defaults, onboarding text, and app-owned
configuration. Framework code, generator code, provider runtime behavior, and
security gates should remain in the upstream framework.
