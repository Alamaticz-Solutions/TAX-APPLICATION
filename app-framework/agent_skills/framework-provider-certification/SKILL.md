---
name: framework-provider-certification
audience: framework
phase: provider-certification
cli_namespace: framework
artifacts: api_tests/target/provider-parity.json,target/appfw/provider-performance.json
description: Use when changing provider behavior, provider SDK rules, provider certification contracts, QueryIR compilation, or provider performance evidence.
---

# Framework Provider Certification

## Use When

- Changing PostgreSQL, MongoDB, MS SQL Server, Snowflake, QueryIR compilation,
  provider routines, or provider performance certification.

## Procedure

1. Read provider SDK and certification docs.
2. Add/adjust provider-neutral contracts before claiming parity.
3. Run focused tests locally and live provider certification when services are
   available.
4. Record unsupported provider behavior as explicit capability status.

## Proof

```bash
scripts/appfw framework provider-test --all --json
scripts/appfw framework provider-performance --json --all
scripts/appfw framework handoff --json
```

## Guardrails

- Do not mark provider behavior enterprise-ready without executable evidence.
- Do not hide provider limitations in product code.
