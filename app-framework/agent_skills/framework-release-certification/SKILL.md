---
name: framework-release-certification
audience: framework
phase: release-certification
cli_namespace: framework
artifacts: target/appfw/release-check.json,target/appfw/security-certification.json,target/appfw/ops-certification.json
description: Use when certifying App Framework release readiness, security posture, observability posture, supply-chain evidence, provenance, or signing decisions.
---

# Framework Release Certification

## Use When

- Preparing framework release evidence or production-readiness scoring.
- Producing provider, security, operations, SBOM, dependency, provenance, or
  signing evidence for the framework.

## Procedure

1. Start from release scorecard and evidence matrix.
2. Separate local proof, release-candidate proof, and production proof.
3. Run strict release gates when provider and platform services are available.
4. Record formal risk acceptance for evidence that is intentionally deferred.

## Proof

```bash
scripts/appfw framework release-check --json
scripts/appfw framework security-certification --json
scripts/appfw framework ops-certification --json
scripts/appfw framework handoff --json
```

## Guardrails

- Do not call static artifacts live certification.
- Do not include MCP in release posture unless dedicated MCP certification is
  complete.
