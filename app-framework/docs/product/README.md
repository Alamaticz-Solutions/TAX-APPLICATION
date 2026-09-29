# Product Developer Documentation

Use this path when you are building, running, testing, releasing, upgrading,
modernizing, or handing off a product app that uses App Framework.

Product work owns product intent: app identity, schema, data-source topology,
entity model, product handlers/services, policy fixtures, product frontend,
deployment choices, and release evidence for that app.

For the current flagship workflow-product intelligence brief, see
[PDS Nexus Product Development Brief](pds-nexus-product-development-brief.md).
It captures the ServiceNow/Pega-replacement product context from supplied
meeting/email intelligence and translates it into App Framework prioritization
and maturity demands.

For the platform business-value rationale, capability-reuse thesis, and
ownership boundaries, see
[App Framework Platform Strategy](../strategy/app-framework-platform-strategy.md).
For product-management prioritization across citizen-developed applications,
legacy modernization, greenfield products, and Nexus, see
[App Framework Product Management Strategy](../strategy/app-framework-product-management-strategy.md).

Canonical CLI namespace:

```bash
scripts/appfw product <command>
```

In a packaged product environment, `appfw product <command>` is equivalent. The
approved App Framework ProGet toolchain bundle provides `bin/appfw` plus
prebuilt generator and database runner binaries, so normal product validation,
generation, migration, release evidence, and handoff do not require a local
framework checkout or local framework crate builds.

Product repositories should also consume framework Rust crates from the
approved ProGet Cargo registry in committed `Cargo.toml` files. Do not commit
temporary `path = "../../app-framework/..."` overrides in product backend,
API-test, or policy-test crates. Product CI authenticates to the registry with
the approved Cargo token and should be able to build the product without an
adjacent framework checkout.

## Product Lifecycle

| Phase | Skill | Primary Docs | Canonical Commands |
| --- | --- | --- | --- |
| Intake | `product-poc-intake` | [PoC Intake](../lifecycle/intake-and-discovery.md) | `scripts/appfw product new --profile product-intake`; `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product scaffold-model --dry-run --json`; `scripts/appfw product validate --json` |
| Legacy modernization | `product-legacy-modernization` | [Legacy Modernization](../lifecycle/legacy-modernization.md) | `scripts/appfw product new --profile product-intake --source-kind legacy`; `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product scaffold-model --dry-run --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; `scripts/appfw product handoff --json` |
| Bootstrap | `product-bootstrap` | [Product Golden Path](../lifecycle/product-golden-path.md), [CLI Quickstart](../start/cli-quickstart.md) | `scripts/appfw product new --list-profiles --json`; `scripts/appfw product handoff --json` |
| Model | `product-schema-modeling` | [Model Proposal To Config](../model/model-proposal-to-config.md), [Schema Design](../model/schema-design.md), [Generated Ownership](../start/generated-ownership.md) | `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product scaffold-model --dry-run --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json` |
| Generate and extend | `product-generate-verify` | [Generated Ownership](../start/generated-ownership.md), [Product Extension API](../reference/product-extension-api.md) | `scripts/appfw product generate`; `scripts/appfw product generate --check --json` |
| Run local | `product-local-run-test` | [Application Lifecycle](../lifecycle/application-lifecycle.md), [Deployment Reference](../release/deployment-reference.md) | `scripts/appfw product migrate`; `scripts/appfw product serve`; `scripts/appfw product api-test` |
| Frontend | `product-frontend` | [Product Frontend](../frontend/product-frontend.md) | `scripts/appfw product frontend-test --json` |
| Mobile | `product-mobile-react-native` | [React Native Mobile](../frontend/mobile-react-native.md), [Product Frontend](../frontend/product-frontend.md) | `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`; RN checks when mobile source exists |
| Release | `product-release-evidence` | [Release Gate](../release/release-gate-ci-cd.md), [Deployment Reference](../release/deployment-reference.md) | `scripts/appfw product release-check --json`; `scripts/appfw product handoff --json` |
| Upgrade | `product-upgrade` | [Application Lifecycle](../lifecycle/application-lifecycle.md) | `scripts/appfw product upgrade --json`; `scripts/appfw product generate --check --json` |
| Handoff | `product-handoff` | [Agent Task Map](../start/agent-task-map.md) | `scripts/appfw product handoff --json` |

Intake and legacy modernization retain analysis and proposal evidence in
`target/appfw/product-analysis.json`, `.appfw/model-proposal.yaml`, and
`target/appfw/model-proposal.json` before agents write final model source.
`scripts/appfw product model-status --json` adds the current phase, blockers,
and next commands in `target/appfw/model-status.json`. Use the proposal
`review_decisions` ledger and [Model Proposal To Config](../model/model-proposal-to-config.md)
to turn that review evidence into durable `.appfw/model`. After signed
review, `scripts/appfw product scaffold-model --dry-run --json` previews a
small source bootstrap from accepted entity decisions; running it without
`--dry-run` writes starter entity and deny-by-default policy source plus
`target/appfw/model-scaffold.json`. It does not infer relationships, real
properties, frontend behavior, or business rules.

## Guardrails

- Do not edit framework generator, runtime, provider, or CLI code from a product
  task unless the branch is explicitly framework work.
- Do not copy CRM sample entities, UI screens, docs, or provider fixtures into a
  new product.
- Do not patch generated output as the source of truth; change product config,
  generator code, templates, or human-owned extension points as appropriate.
- Keep release evidence product-specific and retain command JSON/artifacts.
