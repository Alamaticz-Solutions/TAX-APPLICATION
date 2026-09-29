---
name: pds-sra-package
audience: framework,product
phase: security-risk-assessment
cli_namespace: framework,product
artifacts: target/appfw/sra-package.json,target/appfw/sra-package.md,.appfw/target/appfw/sra-package.json,.appfw/target/appfw/sra-package.md
description: Use when preparing a PDS Health Security Risk Assessment package for framework changes, product apps, production deployment, vendor/SaaS integrations, AI/MCP/Kafka surfaces, sensitive data, or significant governance/security changes.
---

# PDS SRA Package

## Use When

- A framework or product change needs Security Risk Assessment preparation.
- A product app may handle PHI, PII, business-sensitive data, SaaS/vendor APIs,
  service accounts, secrets, AI, MCP, Kafka, or production deployment.
- A human needs one package that separates reusable framework controls from
  product-specific business, data-flow, vendor, and risk obligations.

## Invocation

Canonical slash command:

```text
/pds-sra-package --framework
/pds-sra-package --product
/pds-sra-package --all-products
```

CLI evidence:

```bash
scripts/appfw framework sra-package --all-products --json
scripts/appfw product sra-package --json
```

## Procedure

1. Start from `docs/start/pds-sra-package-harness.md`.
2. Choose the lens:
   framework for reusable controls, product for the current app, or
   all-products to inventory each product app in the checkout.
3. Run the matching `sra-package --json` command and keep the retained JSON and
   Markdown narrative artifacts.
4. Read the generated `missing_human_inputs`; do not treat the package as ready
   while the Data Flow Diagram, Infrastructure / Logical Architecture Diagram,
   owners, data classification, vendor posture, secrets, logging, or risk
   entries are still missing.
5. For framework scope, explain what App Framework supplies and what every
   product app must still answer independently.
6. For product scope, identify source/destination systems, PHI/PII/sensitive
   data, authn/authz, service accounts, secrets, logging/SIEM, vendors, and
   mitigations for the actual product deployment.
7. Summarize readiness for the human as: ready to submit, ready with explicit
   gaps, or not ready.

## Proof

```bash
scripts/appfw framework sra-package --all-products --json
scripts/appfw product sra-package --json
scripts/appfw framework handoff --json
scripts/appfw product handoff --json
```

## Guardrails

- The SRA package is preparation evidence, not SRA approval.
- Do not commit raw files from ignored `regulatory/` unless the human approves a
  sanitized documentation strategy.
- Do not let framework controls obscure product-specific owners, data
  classification, vendors, diagrams, secrets, logging, or risks.
- Do not claim PHI/PII/security readiness without product evidence and human
  confirmation.
- Keep LogicGate/SRA links, reviewer questions, and accepted risks explicit.
