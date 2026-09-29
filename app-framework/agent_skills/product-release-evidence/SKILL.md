---
name: product-release-evidence
audience: product
phase: release
cli_namespace: product
artifacts: target/appfw/release-check.json,target/appfw/agent-handoff.json
description: Use when preparing production readiness, release checks, provider certification, security/ops/performance evidence, SBOM/supply-chain artifacts, or final handoff.
---

# Product Release Evidence

## Use When

- A change affects production readiness.
- Preparing provider, security, observability, performance, supply-chain, or
  deployment evidence.
- Deciding whether a release blocker is code, docs, live evidence, or risk
  acceptance.

## Procedure

1. Start from the current scorecard in `docs/release/roadmap.md`.
   Use `docs/release/release-gate-ci-cd.md` and
   `docs/release/deployment-reference.md` for deeper release and deployment
   evidence contracts.
2. Run local coherence checks before live release gates.
3. Produce or verify required artifacts:
   validation, generate-check, tests, docs-check, handoff, SBOM, dependency,
   secret, PHI/log lint, provider, security, ops, and performance evidence.
4. For framework/toolchain release work, verify the ProGet distribution
   manifest from `scripts/appfw framework package --json`; product releases
   should consume the approved packaged `appfw` toolchain rather than a local
   framework checkout.
5. Keep MCP release-excluded unless explicitly certified.
6. Separate local proof, release-candidate proof, and production proof.
7. Record skipped live checks with environment requirements and risk.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw framework docs-check --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product release-check --json
scripts/appfw product handoff --json
```

Use strict CI wrappers for final release evidence.

## Guardrails

- Do not call static artifacts live certification.
- Do not promote without provider-backed evidence for selected providers.
- Do not leave stale scorecard/security claims after code posture changes.
