---
name: framework-dependency-maintenance
audience: framework
phase: dependency-maintenance
cli_namespace: framework
artifacts: target/appfw/dependency-check.json,target/appfw/dependency-plan-*.json
description: Use when checking dependency upgrade pressure, OSV/CVE findings, dependency plans, or dry-run/apply upgrade automation for framework packages.
---

# Framework Dependency Maintenance

## Use When

- Running supply-chain/dependency checks.
- Planning or applying package upgrades.
- Reviewing OSV/CVE, cargo, npm, or policy findings.

## Procedure

1. Run offline dependency pressure checks first.
2. Generate an upgrade plan for the package being considered.
3. Use dry-run upgrade automation before apply mode.
4. Pair dependency changes with docs, tests, and handoff evidence.

## Proof

```bash
scripts/appfw framework dependency-check --json --offline
scripts/appfw framework dependency-plan --json --offline --package <name>
scripts/appfw framework dependency-upgrade --json --check --package <name>
scripts/appfw framework handoff --json
```

## Guardrails

- Do not upgrade dependencies without retained risk and verification evidence.
- Do not suppress vulnerability findings without owner, expiry, and rationale.
