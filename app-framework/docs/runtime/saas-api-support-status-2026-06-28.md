# SaaS API Support Status And Plan (2026-06-28)

> **Status: implementation coordination report.** This report reassesses the
> `codex/saas-api-intel` branch after reviewing the local provider crates,
> sync-worker generator/runtime surfaces, and the external
> `vendor_api_specs_2026-06-26/enrichment` handoff files. The raw vendor corpus
> remains outside this repository and must stay evidence input, not committed
> framework source.

Read this with [SaaS Connectors](saas-connectors.md),
[SaaS Connector Certification](saas-certification.md), and the
[SaaS API Intelligence Snapshot](saas-api-intel-2026-06-26.md).

## Executive Summary

The branch has moved past architecture-only planning. The framework now has:

- External API provider keys for Salesforce, Workday, Anaplan, Oracle
  Financials, ServiceNow, and iCIMS.
- `appfw_saas_core` request-plan, redaction, pagination, transfer, OAuth, and
  rate-limit primitives.
- Compiler-contracted named-operation planning for Salesforce, Workday,
  Anaplan, and Oracle Financials.
- Metadata-only, fail-closed provider skeletons for ServiceNow and iCIMS.
- SaaS certification areas through `SaasReadArea`, with no live-certified SaaS
  claims yet.
- Sync descriptor validation and fail-closed generation of
  `sync_worker_plan.json` plus `backend/config/generated/sync_workers.yaml`.
- A separate `APPFW_RUNTIME_MODE=sync` worker-only process posture that refuses
  to run with HTTP listener modules selected.

The main gap is execution. Providers can safely build network-free request
plans, and generated products can load a disabled sync-worker plan, but the
runtime does not yet perform scheduled SaaS reads, persist checkpoints,
dead-letter poison records, write local projections, or publish freshness
readiness.

## Provider Status

| Provider | Current status | What is strong | What blocks promotion |
| --- | --- | --- | --- |
| Salesforce | Strongest Wave 1 candidate; compiler-contracted fixed read planning | Account SOQL/REST planning, `getUpdated`, `getDeleted`, continuation URLs, limits summaries, metadata summaries, redaction checks | Live tenant auth, org describe, Health Cloud projection allow-list, rate/backoff evidence, query metrics, freshness, sync execution |
| Workday | Compiler-contracted selected HCM/WWS planning | `Get_Workers` page/count shape, worker events, orgs, locations, job profiles, server timestamp parsing | Tenant WWS request/response evidence, PII redaction, auth, rate/backoff, sync execution |
| Anaplan | Compiler-contracted limited metadata/export helpers | Model status, files list, registry-bound view read/page, file chunk planning | Tenant export allow-list, task polling evidence, auth, metrics, freshness, sync execution |
| Oracle Financials | Compiler-contracted tiny Oracle 26B allowlisted slice | Accounting period status LOV list/get, fixed paths, paging, OpenAPI release pinning | Tenant auth, ledger/business-unit scope, broader operation allow-list, rate/redaction/freshness evidence |
| ServiceNow | Metadata-only, unsupported reads | Planned operation names and blockers are explicit | Authenticated instance OpenAPI or dictionary export, table ACL/domain/plugin evidence |
| iCIMS | Metadata-only, unsupported reads | Planned operation names and blockers are explicit | Authenticated developer docs or sandbox export, tenant field matrix, live samples |

No SaaS provider should be marked `LiveCertified` from offline documentation or
the external corpus alone.

## Architecture Decision

Sync workers should run as their own deployment/process, separate from the HTTP
backend. The current templates already enforce this direction:

- `APPFW_RUNTIME_MODE=sync` selects the sync worker host.
- A process that combines sync workers with HTTP listener modules fails closed.
- Generated sync worker config stays `enabled: false` until activation gates
  are satisfied.

This matches the Kubernetes shape we should target: a dedicated sync-worker
Deployment with its own service account, secrets, resource limits, health and
readiness endpoints, and scaling policy. The app backend should read local
projection tables; it should not synchronously proxy arbitrary SaaS API calls.

## Prioritized Plan

### P0: Make Disabled Worker Plans Operationally Clear

Status: implemented on this branch.

Add machine-readable activation gates to generated sync-worker plans and the
runtime shell report. These gates should explain exactly why a generated worker
plan is disabled:

- `provider_live_certification`
- `provider_request_execution`
- `checkpoint_persistence`
- `idempotent_projection_writes`
- `scheduler_readiness_and_freshness`
- `worker_deployment_wiring`

This keeps the fail-closed posture while making CI, reviewers, and operators
see the remaining work without reading scattered prose.

### P1: Salesforce Incremental Read Pilot

Build the first executable worker around Salesforce Account or another
low-risk, non-PHI projection:

- Bind one `.appfw/model/sync/*.yaml` descriptor to fixed Salesforce named
  operations.
- Execute request plans through a runtime-owned transport abstraction.
- Persist checkpoints with idempotent update semantics.
- Upsert rows into an app-owned projection schema using stable source keys.
- Record dead letters with redacted payload summaries.
- Emit freshness and failure counters for readiness.
- Keep Health Cloud PHI projections gated until org describe, field allow-list,
  and redaction evidence exist.

### P2: Certification And Evidence Harness

Add retained evidence before enabling live certification:

- Provider-level mock HTTP contract tests for request execution, pagination,
  retry/backoff, redaction, and error classification.
- Live tenant evidence capture for approved sandboxes only.
- `provider-test` or equivalent SaaS certification reports that map directly to
  `SaasReadArea`.
- Release evidence checks that refuse `LiveCertified` status without retained
  live artifacts.

### P3: Worker Deployment Wiring

Generate deployment surfaces for the separate worker process:

- Local compose profile for sync workers.
- Kubernetes Deployment guidance or generated manifests.
- Service-account and secret-contract expectations.
- Health/readiness endpoints that reflect freshness, checkpoint lag, and
  consecutive failures.

### P4: Expand Provider Slices After Salesforce

Proceed in parallel only where the evidence is strong enough:

- Workday: `Get_Workers` page/count and freshness pilot.
- Anaplan: export task polling plus chunk download pilot after tenant
  allow-list.
- Oracle Financials: expand from the LOV slice only after tenant operation
  scoping is approved.
- ServiceNow/iCIMS: keep blocked until authenticated docs or tenant exports are
  available.

## Safe Parallel Workstreams

The next implementation phase can safely split into four threads:

| Workstream | Ownership | Depends on |
| --- | --- | --- |
| Worker runtime execution | `appfw_runtime::sync_worker`, transport interfaces, checkpoint/dead-letter abstractions | Activation gates and existing request plans |
| Salesforce pilot | `appfw_provider_salesforce`, sync descriptor fixtures, projection mapping | Runtime execution interfaces |
| Certification/evidence | `appfw_runtime::provider_capabilities`, CLI export, provider-test evidence paths | Provider and worker contracts |
| Deployment wiring | generator templates, docs, local/Kubernetes runtime config | Stable worker host contract |

ServiceNow and iCIMS should not consume implementation threads yet beyond
evidence intake review, because public docs do not justify executable
operation contracts.

## Merge Readiness

Push/merge is appropriate when the branch has:

- Passing focused tests for generator sync-worker planning and runtime
  sync-worker config.
- `scripts/appfw framework validate --json`.
- `scripts/appfw framework docs-check --json`.
- `scripts/appfw framework generate --check --json`, or preserved diagnostics
  if generated drift is intentional and documented.
- A framework handoff artifact that records changed surfaces and verification.
