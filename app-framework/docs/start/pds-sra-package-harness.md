# PDS SRA Package Harness

This harness prepares Security Risk Assessment evidence for PDS Health work. It
is intentionally report-only: it helps a human and the SRA reviewers see what is
known, missing, and owned, but it does not approve an SRA.

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

Framework scope writes `target/appfw/sra-package.json` and
`target/appfw/sra-package.md`. Product scope writes
`.appfw/target/appfw/sra-package.json` and
`.appfw/target/appfw/sra-package.md` under the selected product app.

## Framework Perspective

Use framework scope when App Framework itself changes or when the human needs a
platform/control package before evaluating downstream apps.

The framework package should explain:

- reusable runtime, generator, provider, release, evidence, and
  agent-governance controls;
- product/framework ownership boundaries and generated-boundary rules;
- validation, docs-check, release, PR-review, security-baseline, provider, and
  operations evidence that exists for the framework;
- platform limitations and product obligations that cannot be answered by the
  framework alone; and
- each product app perspective when `--all-products` is used.

Framework evidence does not satisfy a product SRA by itself. It supplies the
control substrate and shared assertions that each product app can reference.

## Product App Perspective

Use product scope for every app that may enter SRA, including CRM and PDS Nexus.
Each product package must answer the actual business, data, integration, and
deployment questions for that app.

The product package should supply:

- business purpose, project owner, technical owner, and affected
  assets/processes;
- Data Flow Diagram inputs: source/destination systems, data types and
  classification, PHI/PII/business-sensitive flags, transfer mechanisms,
  encryption in transit, interfaces, validation/transformation steps, and
  access-control/authentication flow;
- Infrastructure / Logical Architecture Diagram inputs: legacy architecture,
  cloud architecture, hosting environment, storage, and compute components;
- authentication, authorization, encryption at rest, encryption in transit,
  connectivity security, logging, monitoring, and SIEM posture;
- service accounts, bot accounts, least privilege, secrets management,
  credential rotation, and shared credential controls;
- vendor/SaaS security evidence when applicable, including SOC 2, ISO 27001,
  questionnaires, data processing controls, subprocessors, and vendor posture;
  and
- risk-register entries with title, description, source, likelihood, impact,
  category, owner, current controls, and mitigation actions.

## Human Approval Brief

When presenting an SRA package to the human, state:

- final readiness: ready to submit, ready with explicit gaps, or not ready;
- which perspective was reviewed: framework, product, or all products;
- missing human inputs and diagram gaps;
- product-specific data classification and vendor/security unknowns;
- retained JSON and Markdown narrative artifacts checked; and
- LogicGate/SRA record links or the reason a record is not yet available.

## Source Handling

The ignored local `regulatory/` folder may contain PDS Health source material
such as SRA diagram requirements, risk forms, governance notes, security
handbooks, and completed-review examples. Treat those files as local source
evidence to summarize into retained artifacts. Do not commit raw regulatory
documents unless a human explicitly approves a sanitized documentation strategy.

## Guardrails

- Do not treat `sra-package.json` or `sra-package.md` as approval.
- Do not claim framework controls answer product-specific data classification,
  diagrams, vendors, service accounts, or risks.
- Do not submit a product package while `missing_human_inputs` still contains
  unaccepted gaps.
- Do not hide AI, MCP, Kafka, SaaS write-back, PHI/PII, secrets, logging, or
  vendor questions as generic deployment risk.
