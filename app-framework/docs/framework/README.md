# Framework Steward Documentation

Use this path when you are evolving App Framework itself: docs IA, CLI
contracts, generator behavior, runtime ingress, provider behavior, framework
release gates, dependency policy, packaging, or reusable frontend scaffolding.

Framework work owns reusable contracts and certification. Product work consumes
those contracts through `scripts/appfw product ...` and generated artifacts.

Canonical CLI namespace:

```bash
scripts/appfw framework <command>
```

## Framework Stewardship Lifecycle

| Phase | Skill | Primary Docs | Canonical Commands |
| --- | --- | --- | --- |
| Docs IA | `framework-docs-ia` | [Maintainability](../architecture/concerns/maintainability.md), [ADR 0013](../architecture/adr/0013-docs-information-architecture.md) | `scripts/appfw framework docs-check --json` |
| Product intake proof | `framework-docs-ia` | [PoC Intake](../lifecycle/intake-and-discovery.md), [Product Golden Path](../lifecycle/product-golden-path.md) | `scripts/appfw framework intake-proof --json` |
| Frontend design system | `framework-frontend-design-system` | [PDS Health Design System](../frontend/pds-health-design-system.md), [ADR 0008](../architecture/adr/0008-frontend-design-system.md) | `scripts/appfw framework docs-check --json`; `scripts/appfw framework golden-downstream --json` when scaffold behavior changes |
| Generator | `framework-generator` | [Codegen API](../reference/codegen-api.md), [Generated Ownership](../start/generated-ownership.md) | `scripts/appfw framework validate --json`; `scripts/appfw framework generate --check --json` |
| Runtime ingress | `framework-runtime-ingress` | [Architecture](../architecture/overview.md), [MCP](../runtime/mcp.md) | `scripts/appfw framework feature-check --json`; `scripts/appfw framework test --fast --json` |
| Provider certification | `framework-provider-certification` | [Provider Certification](../runtime/provider-certification.md), [Provider SDK](../runtime/provider-sdk.md) | `scripts/appfw framework provider-test --all --json`; `scripts/appfw framework provider-performance --json --all` |
| Release certification | `framework-release-certification` | [Release Gate](../release/release-gate-ci-cd.md), [Deployment Reference](../release/deployment-reference.md) | `scripts/appfw framework local-live-preflight --json`; `scripts/appfw framework release-check --json`; `scripts/appfw framework security-certification --json`; `scripts/appfw framework ops-certification --json` |
| Dependency maintenance | `framework-dependency-maintenance` | [CLI Reference](../reference/cli.md) | `scripts/appfw framework dependency-check --json --offline`; `scripts/appfw framework dependency-plan --json --offline` |
| Handoff | `framework-handoff` | [Agent Task Map](../start/agent-task-map.md) | `scripts/appfw framework handoff --json` |

## Guardrails

- Keep framework stewardship guidance out of product lifecycle docs unless it is
  needed to consume the framework.
- Keep active branch ledgers, extraction scratch notes, and completed plans in
  `docs/archive/`, not live architecture.
- Keep provider, runtime, generator, docs-check, and release claims backed by
  executable checks or explicit release evidence.
- Preserve flat CLI commands as compatibility aliases, but make new docs,
  skills, and CI examples use the `framework` namespace.
