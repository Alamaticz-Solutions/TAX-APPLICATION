---
name: framework-runtime-ingress
audience: framework
phase: runtime-ingress
cli_namespace: framework
artifacts: target/appfw/feature-check.json,target/appfw/agent-handoff.json
description: Use when changing App Framework runtime ingress, HTTP/GraphQL, MCP, Kafka, operation dispatch, policy, audit, metrics, or module loading behavior.
---

# Framework Runtime Ingress

## Use When

- Changing runtime ingress modules or shared operation invocation semantics.
- Ensuring HTTP/GraphQL, MCP, and Kafka stay independently loaded but governed
  by the same runtime path.

## Procedure

1. Read `docs/architecture/overview.md` and runtime ingress sections.
2. Keep transport auth separate from runtime actor/tenant/policy context.
3. Route operations through shared dispatcher semantics.
4. Prove feature-gated module compilation.

## Proof

```bash
scripts/appfw framework feature-check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

## Guardrails

- Do not let a trusted transport bypass policy, tenant isolation, audit, or
  provider certification.
- Do not make worker-only modules start HTTP/admin surfaces implicitly.
