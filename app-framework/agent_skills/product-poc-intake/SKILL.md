---
name: product-poc-intake
audience: product
phase: intake
cli_namespace: product
artifacts: .appfw/poc-intake.yaml,.appfw/poc-analysis.yaml,.appfw/model-proposal.yaml,.appfw/manifest.yaml,target/appfw/product-analysis.json,target/appfw/model-proposal.json,target/appfw/model-status.json,.appfw/target/appfw/validation.json
description: Use when converting a citizen-developed workbook, static HTML app, embedded data dashboard, or vibe-coded proof of concept into an enterprise App Framework product.
---

# Product PoC Intake

## Use When

- A business PoC exists outside App Framework.
- Source evidence includes Excel, CSV, HTML, embedded JSON, screenshots, or a
  static app.
- The product model and frontend need to be inferred from business artifacts.

## Procedure

1. Open `.appfw/poc-intake.yaml` or create the product with
   `--profile product-intake`.
   Use `docs/lifecycle/intake-and-discovery.md` for the deeper intake contract.
2. Confirm source artifacts are workspace-local relative paths, preferably
   under `.appfw/source-evidence/`. If evidence came from Downloads, Documents,
   email, a ticket, or a shared drive, copy only the approved/redacted artifact
   or a minimal structured summary into the product workspace first. Do not
   analyze absolute local paths.
3. Run `scripts/appfw product analyze --summary --json` to profile the source
   artifacts and write `target/appfw/product-analysis.json` plus
   `.appfw/poc-analysis.yaml`. Use `--full --json` or omit `--summary` only
   when a tool intentionally needs the full report on stdout.
4. Review `model_clues` in `target/appfw/product-analysis.json` as a queue of
   candidate entities, properties, routines, and frontend views. Workbook
   sheets and named tables may provide entity-owned candidate fields; verify
   every clue against the source artifacts before modeling. Treat workbook
   formulas, pivots, charts, hidden sheets, and data validations as behavior or
   dashboard evidence until reviewed.
5. Run `scripts/appfw product propose-model --summary --json`, then review
   `.appfw/model-proposal.yaml` as a draft, not final config. Keep fields
   already grouped under candidate entities with their evidence unless the
   business model proves they are derived, lookup, DTO-only, or rejected.
   Prefer reviewed semantic labels and suggested domain names over raw workbook
   table IDs when naming final product entities.
6. Run `scripts/appfw product model-status --json` to confirm the conversion is
   in `proposal_ready_for_modeling` before writing schema source. Re-run it
   after modeling, validation, generation, and handoff. Move to
   `proposal_reviewed` only after `review_decisions` has no unresolved entries
   and sign-off includes `model_owner`, `reviewed_at_utc`, and
   `accepted_for_config: true`. After sign-off, use
   `source_authoring_plan` in `target/appfw/model-status.json` as the
   machine-readable queue for schema source root, accepted entity source
   targets, source-file categories, and validation sequence.
7. Optionally run `scripts/appfw product scaffold-model --dry-run --json`
   after signed review to preview starter entity/policy source. Run it without
   `--dry-run` only when accepted entity decisions have explicit `config_kind`
   and `classification`; continue source modeling from evidence afterward.
8. Use the proposal `implementation_plan` as the next work queue: schema source
   root, required source-file categories, entity tasks, relationship tasks,
   service/custom-method tasks, frontend tasks, evidence tasks, and validation
   sequence. Keep it review-only until a human/architect accepts the model.
   Keep ambiguous entries in `needs_review` or `needs_discovery`; move entries
   to `accept`, `reject`, `defer`, `split`, or `merge` only when the review
   reaches a terminal decision.
9. Inspect source artifacts before modeling. Identify sheets, embedded data,
   charts, filters, workflows, calculations, and user-editable fields.
10. Separate product facts:
   - persisted entities;
   - lookup/status values;
   - relationships;
   - derived/report DTOs;
   - custom methods;
   - frontend dashboards/forms.
11. Record classification assumptions for regulated, PHI, financial, internal,
   or synthetic data.
12. Use CRM only as an implementation reference. Do not copy its model into the
   target product.
13. Draft the product schema under `.appfw/model/schemas/<schema>/`.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

Add frontend and live checks when those surfaces are in scope.

## Guardrails

- Never embed raw source data or secrets in docs or handoff.
- Never point intake, analysis, or handoff files at absolute local paths outside
  the product workspace.
- Do not assume every spreadsheet column is a persisted property.
- Preserve ambiguity as open questions instead of guessing regulated semantics.
