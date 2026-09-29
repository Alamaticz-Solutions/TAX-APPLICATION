---
name: product-local-run-test
audience: product
phase: run-local
cli_namespace: product
artifacts: .appfw/target/appfw/dev_infra.json,target/appfw/agent-handoff.json
description: Use when running a generated product locally, starting backend/frontend/admin services, running generated API scenarios, or producing live local verification evidence.
---

# Product Local Run And Test

## Use When

- Starting backend, CRM/product frontend, admin UI, or local provider services.
- Running generated API scenarios against a live backend.
- Diagnosing local product behavior with real provider data.

## Procedure

1. Validate the repo before starting services.
   Use `docs/lifecycle/application-lifecycle.md` and
   `docs/release/deployment-reference.md` for deeper local and deployment paths.
2. Start required data sources from generated local infra.
3. Run migrations or generated database package setup.
4. Start backend with explicit local environment and port.
5. Start product frontend/admin UI only when needed for the task.
6. Run generated API tests and focused frontend/browser checks.
7. Record skipped live checks with concrete blockers.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw product migrate
ENV_NAME=local API_PORT=8080 scripts/appfw product serve
scripts/appfw product api-test
scripts/appfw product handoff --json
```

Use framework steward commands such as
`scripts/appfw framework provider-test --provider <provider>` for provider
certification and `scripts/appfw product frontend-test --json` for frontend
evidence.

## Guardrails

- Do not run `api-test` unless backend and providers are actually running.
- Do not commit local tokens, trial licenses, or `.env` files.
- Do not treat static local evidence as production release certification.
