# Release Evidence Matrix

Purpose: this matrix gives engineering reviewers a detailed, hierarchical map
of the concerns App Framework has addressed end to end, with a maturity score
for each concern. It is intentionally detailed and review-oriented. It can be
kept as a release evidence review aid, archived, or deleted after the
engineering walkthrough.

This is not a release certification. Scores reflect framework maturity:
implementation depth, consistency, generated/product usability, automation,
evidence, and remaining production proof gaps.

## Scoring Rubric

| Score | Meaning |
| ---: | --- |
| 10.0 | Complete, productized, release-gated, live-evidenced, and low-maintenance. |
| 9.0-9.9 | Strong enterprise posture with code paths and local/CI evidence; remaining work is mostly live certification, retention, or template/productization. |
| 8.0-8.9 | Implemented and useful, but needs broader live proof, CI retention, or additional productization. |
| 7.0-7.9 | Directionally correct and partially implemented; important gaps remain before production reliance. |
| 6.0-6.9 | Planned or early implementation; not yet a production control. |
| Below 6.0 | Conceptual, experimental, or not yet owned by the framework. |

## Executive Tree

- App Framework enterprise maturity - 9.28 / 10 release readiness,
  8.74 / 10 production readiness
  - Product/model source of truth - 9.37 / 10
  - Generation and ownership safety - 9.40 / 10
  - Runtime architecture and ingress governance - 9.30 / 10
  - Data access, QueryIR, and provider semantics - 9.24 / 10
  - Security and tenant governance - 9.23 / 10
  - Audit, compliance, and regulated data controls - 9.05 / 10
  - Supply chain and dependency assurance - 9.42 / 10
  - Observability and operations - 8.84 / 10
  - Performance and scale controls - 8.76 / 10
  - Frontend product experience - 9.07 / 10
  - CLI, automation, and agentic development - 9.35 / 10
  - Release, deployment, and evidence gates - 9.32 / 10 release-gate maturity,
    8.72 / 10 production certification
  - Product packaging and downstream lifecycle - 8.70 / 10
  - Documentation and maintainability - 9.88 / 10
  - Planned async/event ingress - 6.70 / 10

## 1. Product Model And Configuration

- Product model as source code - 9.46 / 10
  - Schema configuration - 9.35 / 10
    - Why it exists: product teams need a reviewable, diffable model instead of
      hand-built backend code.
    - Addressed by: `.appfw/model/schemas`, validation, generated config
      contract, generated backend/database/API artifacts.
    - Maturity note: strong. Remaining lift is richer scaffolding commands and
      more example apps.
  - Entity/property configuration - 9.42 / 10
    - Why it exists: entities, fields, validation, generated GraphQL types, and
      persistence shape should be model-driven.
    - Addressed by: config validation, generated schema modules, generated UI
      contracts, property metadata, validation reports.
    - Remaining lift: more generated examples for complex validation and
      domain-specific property patterns.
  - Generated data-access execution policy - 9.10 / 10
    - Why it exists: product developers need a declarative way to ask the
      generated data-access layer to optimize frequent generated SQL without
      hand-writing provider-specific code.
    - Addressed by: `entity_type.execution.prepared_statements`, generated
      config contract coverage, validation of the execution shape, generated
      system metadata, and the CRM `Account` example.
    - Remaining lift: expand from generated read/query SQL into mutation
      caching where provider lifecycles are proven, and retain provider-backed
      performance evidence.
  - Provider routine custom methods - 9.05 / 10
    - Why it exists: products sometimes need vetted database functions or
      stored procedures, but should not put SQL, `CALL`, or `EXEC` text in app
      config.
    - Addressed by: `custom_methods[].provider_routine`, safe routine
      identifier validation, portable schema/name defaults, provider-specific
      override support only when needed, generated handler dispatch through
      DataAccess, and CRM function/procedure fixtures including a stored
      procedure that persists and returns the account health snapshot row.
    - Remaining lift: live all-provider routine certification and richer DTO
      examples for non-table return shapes.
  - Relationship configuration - 8.95 / 10
    - Why it exists: products need reliable navigation and persistence semantics
      without ad hoc relationship code.
    - Addressed by: relationship metadata, provider certification coverage,
      generated relationship projection, parent-child frontend workflows.
    - Remaining lift: normalize generated runtime relationship metadata around
      `NavToOne`, ordinary `NavToMany`, and junction-backed `NavToMany`, with
      `ManyToMany` kept as a schema-level storage/relationship strategy.
  - App topology manifest - 9.30 / 10
    - Why it exists: app identity, schemas, and data-source topology need a
      product-owned contract that does not replace schema config.
    - Addressed by: `.appfw/manifest.yaml`, topology reports, manifest
      validation, product workspace docs.
    - Remaining lift: more downstream examples and product-template CI proof.
  - Data-source configuration - 9.20 / 10
    - Why it exists: provider environments, local compose services, managed
      TLS posture, and runtime provider selection must be explicit.
    - Addressed by: `.appfw/model/data_sources/_res.yaml`, generated
      provider config, topology reports, deployment reference.
    - Remaining lift: keep provider proof retained in the release bundle and
      tie it to deployable artifact promotion.
  - Config contract generation - 9.35 / 10
    - Why it exists: humans and agents need a stable, generated contract for
      allowed configuration shapes.
    - Addressed by: `config_contract.json`, `config_contract.md`, generated
      `_specs/CONFIG_CONTRACT.md`, docs guidance to avoid hand-maintaining it.
    - Remaining lift: richer examples and maybe schema-aware editor support.
  - Regulated data classification metadata - 9.15 / 10
    - Why it exists: regulated releases need known data classification at data
      source, schema, entity, or property level.
    - Addressed by: validation for classification metadata, unknown
      classification rejection, inheritance rules, regulated-mode requirements.
    - Remaining lift: live release evidence tying classifications to logs,
      audit, redaction, and downstream security review.

## 2. Generation, Ownership, And Human Extension Safety

- Generated ownership model - 9.45 / 10
  - Artifact manifest - 9.40 / 10
    - Why it exists: agents must know whether a file is overwrite-safe
      generated output or a human-owned extension point.
    - Addressed by: `.appfw/target/appfw/artifacts.json`,
      `scripts/appfw manifest --json`, generated ownership docs.
    - Remaining lift: continue expanding coverage as new generated surfaces are
      added.
  - Human-owned handler preservation - 9.35 / 10
    - Why it exists: product business logic must survive regeneration.
    - Addressed by: human-owned handler files, generated defaults, override
      markers, boundary checks.
    - Remaining lift: stronger examples for standard overrides and services.
  - Template and generator source ownership - 9.25 / 10
    - Why it exists: repeated behavior belongs in templates/generator, not
      patched generated output.
    - Addressed by: `app_gen/src`, `app_gen/_templates`, generate/check loop,
      generated drift diagnostics.
    - Remaining lift: faster and more granular generator tests.
  - Generated drift detection - 9.40 / 10
    - Why it exists: generated artifacts must be deterministic and reviewable.
    - Addressed by: `scripts/appfw generate --check --json`, artifact
      provenance, handoff drift summaries.
    - Remaining lift: CI-retained downstream app drift proof.
  - Product extension boundaries - 9.20 / 10
    - Why it exists: product teams need safe custom behavior without importing
      framework internals.
    - Addressed by: `product_api`, boundary-check, Product Workspace Contract,
      generated handler adapters.
    - Remaining lift: continue slimming product templates so fewer framework
      internals are visible downstream.

## 3. Runtime Architecture And Ingress Governance

- Runtime operation architecture - 9.45 / 10
  - Independently loaded runtime ingress modules - 9.35 / 10
    - Why it exists: HTTP, MCP, and Kafka should be deployable as separate
      ingress services without forcing unrelated transports into a process.
    - Addressed by: `RuntimeMode`, `RuntimeIngressKind`,
      `RuntimeIngressDescriptor`, `RuntimeHostPlan`, feature-check, and
      deployment guidance for `APPFW_RUNTIME_MODE` / `APPFW_MODULES`.
    - Remaining lift: release artifact matrix that proves worker-only,
      MCP-only, and HTTP-only build/deploy shapes in CI.
  - Shared RuntimeIngress concept - 9.20 / 10
    - Why it exists: HTTP/GraphQL, MCP, and Kafka/event ingress must not
      bypass auth, policy, audit, metrics, or provider governance.
    - Addressed by: `appfw_runtime::host`, `appfw_runtime::ingress`,
      architecture docs, and maintainability contract that define
      `RuntimeIngress -> RuntimeOperationDispatcher -> DataAccess/provider`.
    - Remaining lift: continue migrating compatibility route names toward
      ingress-oriented naming as generated templates slim down.
  - RuntimeOperationDispatcher semantics - 9.15 / 10
    - Why it exists: operation identity, generated contract lookup, auth,
      tenant, validation, policy, audit, and result envelopes need one
      governed path.
    - Addressed by: `appfw_runtime::operation`, generated operation registry,
      MCP generated operation adapters, Kafka operation bindings, and GraphQL
      handler/DataAccess path.
    - Remaining lift: broaden release evidence around remote mutation safety
      and product extension callbacks.
  - GraphQL runtime path - 9.35 / 10
    - Why it exists: generated APIs need a predictable request/response
      execution path with auth, request context, policy, and diagnostics.
    - Addressed by: runtime GraphQL route shell, generated schema routers,
      auth extraction, request/correlation IDs, introspection gating.
    - Remaining lift: live security evidence for all introspection cases in
      release CI.
  - MCP runtime path - 8.65 / 10
    - Why it exists: agents need governed model/tool access instead of bespoke
      backend shortcuts.
    - Addressed by: MCP route, JSON-RPC shell, generated tool catalog, access
      gates, release-posture checks, result/resource bounds.
    - Remaining lift: MCP is release-excluded. If included, it needs capability
      manifests, per-tool policy, token audience/resource checks, mandatory
      audit posture, and HITL evidence.
  - Kafka worker ingress shell - 7.85 / 10
    - Why it exists: enterprise products need async workflows without loopback
      HTTP or governance bypass.
    - Addressed by: manifest Kafka topology validation, generated runtime
      config projection under `backend/config/generated/ingress/kafka.yaml`,
      runtime Kafka config validation,
      service actor and tenant mapping, operation bindings, idempotency,
      retry/DLQ/readiness policy validation, message context, and
      `run_kafka_worker_shell` dispatch into `RuntimeOperationDispatcher`.
    - Remaining lift: concrete broker message-source binding, live Kafka
      certification, platform secret integration, retained lag/readiness
      evidence, and production retry/DLQ observability.
  - Runtime/package extraction - 9.60 / 10
    - Why it exists: framework behavior should live in versioned packages, not
      copied product backend source.
    - Addressed by: `appfw-runtime`, `appfw-cli`, `appfw-codegen`, provider
      package direction, runtime ownership inventory.
    - Remaining lift: final product-template slimming and provider package
      completion.

## 4. Data Access, QueryIR, And Provider Semantics

- Data access governance - 9.25 / 10
  - QueryIR planning - 9.25 / 10
    - Why it exists: filtering, sorting, projection, pagination, aggregation,
      and relationship traversal need provider-neutral semantics.
    - Addressed by: QueryIR, runtime query cost helpers, model metadata,
      provider capability contracts.
    - Remaining lift: deeper runtime-owned orchestration and safe diagnostics.
  - Access filter enforcement - 9.15 / 10
    - Why it exists: policy decisions must become data-provider constraints,
      not just application-side checks.
    - Addressed by: access filter composition, provider mutation access-filter
      hardening, policy tests.
    - Remaining lift: live negative tests across all providers.
  - Pagination and cursors - 9.25 / 10
    - Why it exists: generated APIs must avoid unbounded reads and cursor
      tampering.
    - Addressed by: keyset pagination, signed cursors, optional counts,
      deep-offset penalties, frontend pagination.
    - Remaining lift: live performance and provider parity proof under load.
  - Projection and selected fields - 9.10 / 10
    - Why it exists: frontend grids should query only selected fields plus
      required IDs/lookups.
    - Addressed by: selected-field GraphQL querying, provider projection
      contracts, frontend grid behavior.
    - Remaining lift: broader live provider performance evidence.
  - Aggregation - 8.85 / 10
    - Why it exists: dashboards and analytics need provider-consistent
      grouping, metrics, and having filters.
    - Addressed by: aggregate plan DTOs, aggregate validation, provider
      performance checks.
    - Remaining lift: all-provider live aggregate certification and dashboard
      load proof.
  - Relationship projection and navigation - 8.95 / 10
    - Why it exists: forms, grids, and APIs need actual linked relationships,
      not generic entity lists.
    - Addressed by: relationship metadata, generated relationship projection,
      CRM parent/child grids, lookup/entity selectors.
    - Remaining lift: NavToOne/NavToMany/junction semantics cleanup.
  - Optimistic concurrency - 8.80 / 10
    - Why it exists: stale updates/deletes need predictable rejection.
    - Addressed by: runtime record version helpers and provider certification
      contracts.
    - Remaining lift: frontend conflict UX and release-retained concurrency
      evidence.
  - Prepared generated SQL execution - 9.05 / 10
    - Why it exists: generated read/query SQL can be frequent enough that
      providers should have a framework-owned path for cached/prepared
      execution instead of product teams hand-optimizing generated output.
    - Addressed by: `entity_type.execution.prepared_statements`, generated
      PostgreSQL CTE read/query execution through cached prepared statements,
      provider capability area tracking, and docs that distinguish prepared
      statements from stored routines.
    - Remaining lift: apply the policy to generated mutation SQL where stable
      SQL shape and provider prepared-handle lifecycles are certified; retain
      provider-backed performance proof.
  - Provider routine invocation - 9.10 / 10
    - Why it exists: functions and stored procedures need a governed provider
      boundary with typed argument binding, policy enforcement, and safe
      identifier handling.
    - Addressed by: provider routine binding metadata, generated custom-method
      adapters, DataAccess dispatch, provider helper modules for PostgreSQL,
      MS SQL Server, and Snowflake, and CRM migration fixtures that exercise
      function reads plus stored-procedure update-and-return payloads.
    - Remaining lift: live provider-backed routine invocation evidence and
      additional generated DTO-return examples.
  - Provider bridge and operation contracts - 9.30 / 10
    - Why it exists: providers need one operation vocabulary and evidence
      registry.
    - Addressed by: runtime provider bridge, provider identity, operation
      contract enforcement, provider routine/prepared execution capability
      areas, provider certification reports.
    - Remaining lift: final provider package boundaries and live all-provider
      evidence.

- Provider parity - 9.16 / 10
  - PostgreSQL - 9.32 / 10
    - Addressed concerns: TLS modes, bounded pools, SQL rendering, audit,
      relationship/junction behavior, access filters, provider metrics, cached
      prepared read/query execution, and function/procedure routine helpers.
    - Remaining lift: live release certification bundle.
  - MongoDB - 9.05 / 10
    - Addressed concerns: naming, BSON conversion, projection, filtering,
      lookup/pipeline behavior, transactions where supported.
    - Remaining lift: live release certification bundle.
  - MS SQL Server - 9.10 / 10
    - Addressed concerns: SQL rendering, parameter binding, audit, aggregate
      and mutation paths, junction behavior, and stored procedure helper
      validation.
    - Remaining lift: live release certification bundle.
  - Snowflake - 9.18 / 10
    - Addressed concerns: SQL API path, LocalStack live certification,
      transaction wrapping, preflight fallback, error/audit/tenant behavior,
      and stored procedure helper validation.
    - Remaining lift: release CI with valid LocalStack/Snowflake environment
      and retained all-provider artifact.
  - Prepared/routine provider contracts - 9.05 / 10
    - Addressed by: `PreparedStatementExecution` and stored routine capability
      areas, PostgreSQL cached prepared execution helpers, provider-specific
      routine statement builders, argument-count validation, identifier safety
      tests, and generated CRM function/procedure examples that return row-like
      JSON payloads.
    - Remaining lift: live provider-backed routine execution and mutation
      prepared-statement certification where provider lifecycles support it.
  - Provider certification command path - 9.20 / 10
    - Addressed by: `scripts/appfw provider-test --all --json`,
      `provider-parity.json`, provider evidence arrays, release-check
      integration.
    - Current note: the retained local artifact shows PostgreSQL, MongoDB,
      MS SQL Server, and Snowflake provider certification `ok=true`.
    - Remaining lift: preserve that provider artifact in the final release
      evidence bundle and promotion metadata.

## 5. Security, Tenant Governance, And IDOR Controls

- Authentication and authorization - 9.20 / 10
  - JWT verification and auth config - 9.20 / 10
    - Why it exists: managed environments must not accept missing or malformed
      auth posture.
    - Addressed by: required auth config validation, bearer-only local test
      auth, JWT redaction, runtime auth state.
    - Remaining lift: live no-auth/fake-bearer/non-admin/admin release cases.
  - Provider data-source auth modes (MS SQL) - connection-level - 8.50 / 10
    - Why it exists: plain `MsSqlServer` may use `sql_password` (Microsoft ODBC
      Driver 18) or `ntlm` (on-prem AD / FreeTDS ODBC); auth mode is not a
      semantic-parity matrix column (ADR 0018).
    - Addressed by: ODBC-unified `appfw_provider_mssql`, hermetic
      `docker/mssql-ad` ODBC auth e2e (`sql_password_odbc_e2e` on Linux SQL;
      `ntlm_e2e` + `ntlm_wrong_password_fails_fast` on `tds-mock` with
      `ntlm-auth` oracle and golden anchor;
      `scripts/ci/mssql-odbc-ntlm-e2e.sh`), doctor/validate domain credential
      preflight for `ntlm`. Existing mssql `sql_password` `provider-parity`
      live-cert remains the query-semantics proof (Option A — no second full
      suite under NTLM).
    - Remaining lift: promote `mssql-odbc-ntlm-e2e` from custom/nightly to
      main once stable; scheduled `mssql-ntlm-live-smoke.sh` cadence against
      real Windows SQL; production partner NTLM deployment evidence for
      real Clarity (Phase 2).
  - GraphQL introspection gating - 9.30 / 10
    - Why it exists: schema introspection should not leak production model
      details without verified authorization.
    - Addressed by: non-local JWT requirement, role/scope gates, config
      switches, tests.
    - Remaining lift: retain live release evidence for all required cases.
  - Admin UI access posture - 8.85 / 10
    - Why it exists: model diagnostics and admin tools are high-value surfaces.
    - Addressed by: admin route mount controls, troubleshooting flags, role
      checks, diagnostic redaction.
    - Remaining lift: live managed-environment admin posture evidence.
  - MCP release posture - 8.40 / 10
    - Why it exists: agent tool ingress must be explicitly governed.
    - Addressed by: release-excluded default, `mcp-posture`, access gates,
      result/resource limits.
    - Remaining lift: full MCP certification if MCP enters release scope.

- Tenant isolation and policy - 9.10 / 10
  - Tenant context - 9.10 / 10
    - Addressed by: auth/tenant context helpers, generated frontend headers,
      provider tenant-isolation certification paths.
    - Remaining lift: one retained live all-provider negative evidence bundle.
  - Rego policy contract - 9.00 / 10
    - Addressed by: policy-test command, policy contract docs, access filters,
      deny-by-default guidance.
    - Remaining lift: more generated policy scenario coverage.
  - Custom path policy preservation - 8.70 / 10
    - Addressed by: Product Workspace Contract, handler/service boundaries,
      boundary-check.
    - Remaining lift: stronger examples for service-layer policy decisions.

- IDOR and record locator posture - 8.95 / 10
  - Opaque public record identifiers - 8.90 / 10
    - Why it exists: URLs should not expose primary keys and should follow
      OWASP preference for random public identifiers when feasible.
    - Addressed by: public locator direction and frontend route identity.
    - Remaining lift: all-entity persistence/backfill/migration and provider
      certification if not already complete for every entity shape.
  - Access-checked lookup by locator - 9.00 / 10
    - Addressed by: DataAccess/policy path and negative IDOR test direction.
    - Remaining lift: live all-provider negative evidence.

- Edge and request hardening - 9.10 / 10
  - CORS defaults - 9.20 / 10
    - Addressed by: fail-closed managed CORS defaults, removal of unsafe
      methods from production posture.
    - Remaining lift: release evidence for managed environment config.
  - CSP split - 9.10 / 10
    - Addressed by: production CSP separated from GraphiQL/admin convenience
      posture.
    - Remaining lift: live browser/security header proof.
  - Request body, depth, complexity, rate limits - 9.00 / 10
    - Addressed by: runtime request/router limits and tests.
    - Remaining lift: release evidence under production settings.
  - Panic isolation, timeout, graceful shutdown - 8.95 / 10
    - Addressed by: runtime edge hardening and tests.
    - Remaining lift: live resilience drills.

## 6. Audit, Compliance, And Regulated Data Controls

- Audit framework - 9.05 / 10
  - Audit event shape and persistence - 9.10 / 10
    - Why it exists: generated apps need durable evidence of mutations,
      denied attempts, and audited record changes.
    - Addressed by: runtime record audit helpers, provider audit contracts,
      audited entity metadata, admin audit visibility.
    - Remaining lift: live all-provider audit append/query certification.
  - Audit chain and redaction - 8.95 / 10
    - Addressed by: chain helpers, redaction metadata, provider audit
      redaction proof direction.
    - Remaining lift: retained live audit chain/redaction evidence.
  - Denied/not-applied mutation evidence - 8.90 / 10
    - Addressed by: audit attempt helpers and release security requirements.
    - Remaining lift: all-provider negative tests retained by release CI.

- Regulated data controls - 9.05 / 10
  - Data classification validation - 9.15 / 10
    - Addressed by: classification metadata validation and regulated-mode
      requirements.
    - Remaining lift: classify every production product model and verify
      inherited classifications in release evidence.
  - PHI/ePHI logging posture - 9.00 / 10
    - Addressed by: PHI log lint, observability docs, redaction guidance.
    - Remaining lift: SIEM/logging platform evidence and broader redaction
      tests.
  - PHI/data pipeline governance - 8.70 / 10
    - Addressed by: `framework governance-check --json`, the
      `appfw.phi-pipeline-governance.v1` evidence schema, and fail-closed
      enforcement for missing or schema-thin evidence covering classification
      propagation, de-identification, lower-environment movement, RAG curation,
      retention, deletion/tombstones, redaction, and audit lineage.
    - Remaining lift: release-retained managed evidence for each required
      control, or formal accepted risk where a control is not applicable.
  - Security threat model convergence - 9.10 / 10
    - Addressed by: threat model status updates and current release posture.
    - Remaining lift: keep docs synchronized as supply-chain/security lanes
      evolve.
  - DAST/SAST/ASVS decision gate - 8.65 / 10
    - Addressed by: security-assurance decision script and risk acceptance
      path.
    - Remaining lift: actual enterprise artifacts or approved time-boxed
      acceptance in release bundles.

## 7. Supply Chain, Dependency, And Artifact Integrity

- Supply-chain release controls - 9.42 / 10
  - SBOM generation - 9.35 / 10
    - Why it exists: release teams need bill-of-materials evidence for Rust,
      frontend, and deployable images when applicable.
    - Addressed by: CycloneDX supply-chain gate and release evidence
      validation.
    - Remaining lift: deployable image SBOM in actual image-producing CI.
  - Dependency vulnerability scanning - 9.35 / 10
    - Why it exists: known vulnerable dependencies should block release or be
      formally accepted.
    - Addressed by: cargo audit/deny baseline,
      `scripts/appfw dependency-check --json --strict`, retained
      `target/appfw/dependency-check.json`, policy-file acceptances, docs-check
      coverage, and release evidence validation.
    - Remaining lift: CI retention on every release lane, routine acceptance
      expiry review, and enterprise vulnerability dashboard correlation.
  - Dependency planning and upgrade workflow - 9.25 / 10
    - Why it exists: dependency upgrades should be reviewable, scripted,
      reversible, and evidence-backed rather than improvised during a release
      crunch.
    - Addressed by: `scripts/appfw dependency-plan`,
      `scripts/appfw dependency-upgrade`, dry-run/apply modes in
      `scripts/dependency-check.py`, retained CI evidence artifacts,
      `docs/reference/cli.md`, and docs-check coverage.
    - Remaining lift: promote the dry-run/apply artifacts into the standard
      release evidence bundle and run dependency-upgrade dry runs on a regular
      maintenance cadence.
  - Secret scanning - 9.25 / 10
    - Addressed by: secret scan script, reviewed baseline, redacted evidence.
    - Remaining lift: routine CI retention and baseline review cadence.
  - PHI log lint - 9.10 / 10
    - Addressed by: PHI log lint release gate.
    - Remaining lift: stronger runtime redaction tests across every diagnostic
      surface.
  - Release artifact hashing - 9.20 / 10
    - Addressed by: release evidence hashes and Bitbucket wrapper behavior.
    - Remaining lift: immutable artifact storage and production promotion
      metadata.
  - Provenance and signing - 8.45 / 10
    - Addressed by: production-attestation enforcement and placeholder
      rejection.
    - Remaining lift: real enterprise provenance and artifact-signing evidence.

## 8. Observability And Operations

- Runtime observability - 8.84 / 10
  - Structured logs and redaction - 8.95 / 10
    - Why it exists: operators need traceable logs without leaking PHI, tokens,
      SQL parameters, or provider secrets.
    - Addressed by: tracing, structured diagnostic redaction, provider secret
      alias redaction, logging docs.
    - Remaining lift: redaction tests for all logs, diagnostics, handoff, and
      release artifacts.
  - Request and correlation IDs - 8.95 / 10
    - Addressed by: health responses, frontend typed client propagation,
      provider slow-query context.
    - Remaining lift: correlation IDs on all error/diagnostic surfaces.
  - Metrics - 8.95 / 10
    - Addressed by: `/metrics`, `/metrics.json`, provider operation metrics,
      count/result/max-duration summaries, pool gauges.
    - Remaining lift: live Prometheus target/rule proof.
  - Readiness and liveness - 9.05 / 10
    - Addressed by: `/health/live`, `/health/ready`, data-source status
      summaries, request context.
    - Remaining lift: live provider failure/degraded readiness drills.
  - OpenTelemetry - 8.35 / 10
    - Addressed by: no-network OTLP config proof and deployment guidance.
    - Remaining lift: live OTLP trace and metric export evidence.
  - Alerting and dashboards - 8.55 / 10
    - Addressed by: Prometheus rules, Alertmanager routing, Grafana references,
      ops-certification.
    - Remaining lift: promtool verification, live Alertmanager endpoint proof,
      Grafana API proof.
  - Runbooks and operations certification - 8.70 / 10
    - Addressed by: `ops-certification --json`, runbook references,
      production live-ops evidence switches.
    - Remaining lift: live runbook drills with retained pass/fail evidence.

## 9. Performance And Scalability

- Query and API performance posture - 8.76 / 10
  - Query cost budgets - 9.00 / 10
    - Why it exists: generated APIs need bounded filters, sorts, projection,
      aggregates, offsets, and relationship fanout.
    - Addressed by: QueryIR cost accounting and explicit caps.
    - Remaining lift: provider-backed load and abuse-case evidence.
  - Pagination strategy - 9.20 / 10
    - Addressed by: keyset pagination, signed cursors, optional counts,
      deep-offset penalties.
    - Remaining lift: live high-volume provider tests.
  - Generated index recommendations - 8.60 / 10
    - Addressed by: performance recommendation artifacts.
    - Remaining lift: provider-specific migration/index proof and feedback
      loop from real workloads.
  - Load-test command - 8.70 / 10
    - Addressed by: `load-test --json`, thresholds, retained artifacts.
    - Remaining lift: stable live backend lanes and scenario tuning.
  - Generated CRM load-test suite - 8.80 / 10
    - Addressed by: dashboard, grid, search, form, lookup, relationship
      workflow scenarios.
    - Remaining lift: passing live backend load artifacts in CI.
  - Provider-performance certification - 8.75 / 10
    - Addressed by: `provider-performance --json --all`, projection,
      pagination, fanout, count posture.
    - Remaining lift: live provider-specific performance certification.
  - Prepared/cached statement posture - 8.65 / 10
    - Addressed by: entity-level prepared statement opt-in and PostgreSQL
      cached prepared execution for generated read/query SQL.
    - Remaining lift: live load proof that compares prepared and non-prepared
      paths, mutation-path expansion where safe, and MS SQL Server/Snowflake
      reusable prepared-handle certification if those provider paths mature.
  - Pool tuning and saturation proof - 8.20 / 10
    - Addressed by: PostgreSQL bounded pool tuning, pool metrics.
    - Remaining lift: pool saturation tests and release artifacts.
  - Safe EXPLAIN/slow-query diagnostics - 8.25 / 10
    - Addressed by: slow-query context and safe diagnostic intent.
    - Remaining lift: provider-safe EXPLAIN outputs with redaction and release
      evidence.

## 10. Frontend Product Experience

- CRM reference frontend - 9.07 / 10
  - Product workspace shell - 9.10 / 10
    - Why it exists: product teams need a real app shell, not only admin UI.
    - Addressed by: PDS-branded floating workspace shell, responsive layout,
      fixed header/sidebar behavior, dark/light mode.
    - Remaining lift: make scaffold creation executable through `appfw new` or
      a frontend scaffold command.
  - Design tokens and brand system - 9.00 / 10
    - Addressed by: PDS tokens, light/dark mode, CRM/admin visual alignment.
    - Remaining lift: deeper shared design-system package.
  - Dashboard analytics and charts - 8.95 / 10
    - Addressed by: CRM dashboard metrics, KPIs, charts.
    - Remaining lift: live data-driven dashboard proof and generated analytics
      contracts.
  - Server-side grids - 9.15 / 10
    - Addressed by: pagination, selected columns, resizing, column selector,
      server search, query builder, reasonable heights.
    - Remaining lift: more robust query-builder UX and live backend smoke.
  - Query builder - 8.85 / 10
    - Addressed by: advanced filters, sort controls, form state, local storage
      persistence, lookup-aware selectors.
    - Remaining lift: more polished grouped conditions and generated operator
      metadata.
  - Forms - 9.05 / 10
    - Addressed by: GraphQL-backed load, validation annotations, dirty state,
      save/cancel/delete, delete confirmation, access-governed actions.
    - Remaining lift: live concurrency/conflict handling and richer generated
      form metadata.
  - Lookup and entity selectors - 8.90 / 10
    - Addressed by: lookup-type select controls, entity list selectors,
      related grids, dirty-state navigation prompts.
    - Remaining lift: model-driven distinction between lookup values and full
      entity selectors across all entity types.
  - Relationship workflows - 8.85 / 10
    - Addressed by: parent-linked activity/account flow, child grids/new
      buttons, orphan-prevention UX.
    - Remaining lift: finalized NavToOne/NavToMany/junction metadata.
  - Accessibility and E2E evidence - 9.10 / 10
    - Addressed by: `frontend-test --json`, Playwright, axe coverage for key
      CRM workflows.
    - Remaining lift: retain evidence in release CI and add live-backend smoke
      where required.
  - Generated UI contracts - 8.85 / 10
    - Addressed by: generated UI type contract and typed frontend API layer.
    - Remaining lift: full scaffold execution and upgrade/drift proof.

## 11. CLI, Automation, And Agentic Development

- App Framework CLI - 9.35 / 10
  - Root wrapper - 9.40 / 10
    - Why it exists: agents and humans need one safe entrypoint with explicit
      roots and consistent reports.
    - Addressed by: `scripts/appfw`, root-aware app/framework/config/template
      resolution.
    - Remaining lift: move more durable parsing/report shaping into typed CLI
      modules.
  - Validation command - 9.45 / 10
    - Addressed by: `validate --json`, config contract reports, zero-warning
      posture in current checks.
  - Generation command - 9.35 / 10
    - Addressed by: `generate`, `generate --check --json`, deterministic
      drift diagnostics.
  - Test command - 9.15 / 10
    - Addressed by: `test`, `test --fast --json`, generated API test boundary.
    - Remaining lift: more risk-specific test lanes.
  - Handoff command - 9.30 / 10
    - Addressed by: `handoff --json`, changed-surface classification,
      verification artifact state, generated drift summary.
    - Remaining lift: richer multi-surface summaries and PR-ready format.
  - Explain commands - 9.10 / 10
    - Addressed by: ownership/config/provider/provider-sdk explainers.
    - Remaining lift: `explain changes` and richer recommendation text.
  - Boundary-check - 9.15 / 10
    - Addressed by: product handler/service import checks and unsafe boundary
      guards.
    - Remaining lift: more product-template and frontend boundary checks.
  - Docs-check - 9.25 / 10
    - Addressed by: safe examples, lifecycle checklist, security env parity,
      maintainability contract, root README/AGENTS coverage.
    - Remaining lift: changed-docs mode if runtime remains fast enough.
  - Dependency command family - 9.25 / 10
    - Addressed by: `dependency-check`, `dependency-plan`, and
      `dependency-upgrade` with JSON output, retained artifacts, offline-safe
      planning mode, dry-run/apply workflow support, CLI docs, and docs-check
      examples.
    - Remaining lift: scheduled maintenance lane and release-retained
      dependency-upgrade dry-run evidence.
  - Release-check - 9.20 / 10
    - Addressed by: validation, boundary, docs, generate, tests, provider,
      security, ops, frontend, performance, supply-chain hooks.
    - Remaining lift: provider-backed CI execution and strict artifact
      retention.
  - App bootstrap and upgrade - 8.85 / 10
    - Addressed by: `appfw new`, profile metadata, `appfw.lock`, `upgrade`.
    - Remaining lift: golden downstream CI that creates and upgrades a
      disposable product.

## 12. Release, Deployment, And Evidence

- Release governance - 9.32 / 10
  - Release gate structure - 9.25 / 10
    - Why it exists: framework releases need repeatable machine-readable proof.
    - Addressed by: Bitbucket release wrapper, `release-check --json`,
      release-evidence check, artifact hashing.
    - Remaining lift: approved release-environment execution with provider,
      security, ops, performance, supply-chain, and promotion evidence retained
      together.
  - Strict release evidence validation - 9.20 / 10
    - Addressed by: `release-evidence-check.sh --strict`, local fixture mode,
      required artifact categories.
    - Remaining lift: retain real production evidence artifacts.
  - Provider certification in release - 9.12 / 10
    - Addressed by: provider-test integration and provider parity artifacts.
    - Remaining lift: one green all-provider release bundle.
  - Security evidence in release - 8.95 / 10
    - Addressed by: security-assurance decision, secret/PHI/SBOM gates,
      threat-model convergence.
    - Remaining lift: DAST/SAST/ASVS/provenance/signing artifacts or accepted
      risk.
  - Operations evidence in release - 8.70 / 10
    - Addressed by: ops-certification and live-ops evidence switches.
    - Remaining lift: live OTLP, Alertmanager, runbook, Grafana, SIEM proof.
  - Performance evidence in release - 8.75 / 10
    - Addressed by: load-test-suite and provider-performance gates.
    - Remaining lift: live backend load and provider-specific artifacts.
  - Frontend evidence in release - 8.90 / 10
    - Addressed by: frontend-test command and CRM E2E/a11y coverage.
    - Remaining lift: CI-retained evidence and live smoke when in scope.
  - Deployable image evidence - 8.50 / 10
    - Addressed by: image SBOM support when image source is provided.
    - Remaining lift: actual release image SBOM and immutable digest retention.

- Deployment reference - 8.90 / 10
  - Container model - 8.85 / 10
    - Addressed by: one backend image serving Rust API, product SPA, and
      optional admin UI, plus a separate database utility job.
    - Remaining lift: formal image build/publish examples and digest policy.
  - Environment contract - 9.00 / 10
    - Addressed by: auth, admin, MCP, GraphQL, request limit, observability,
      provider credential variables.
    - Remaining lift: environment docs parity for every new config variable.
  - Secrets management - 8.95 / 10
    - Addressed by: deployment guidance, redaction rules, no checked-in secrets.
    - Remaining lift: platform-specific external secret examples.
  - Migration rollout and rollback - 8.75 / 10
    - Addressed by: migrate plan/lint/drift/apply/rollback-guide commands.
    - Remaining lift: live provider migration drift and rollback drills.
  - GitOps/Argo promotion - 8.40 / 10
    - Addressed by: immutable artifact promotion guidance.
    - Remaining lift: concrete GitOps manifest examples and compatibility
      matrix.

## 13. Product Packaging And Downstream Lifecycle

- Product packaging - 8.70 / 10
  - Split-root product shape - 9.05 / 10
    - Why it exists: downstream products should consume framework packages
      instead of copying framework internals.
    - Addressed by: root-aware CLI/generator, CRM sample, product workspace
      contract.
    - Remaining lift: template slimming and packaging completion.
  - `appfw new` product bootstrap - 8.85 / 10
    - Addressed by: profile metadata, CRM sample profile, lock/provenance.
    - Remaining lift: generated frontend scaffold execution and CI proof.
  - Framework lock and upgrade reports - 9.05 / 10
    - Addressed by: `appfw.lock`, `upgrade --json`, product-upgrade reports.
    - Remaining lift: downstream upgrade CI recipe and dependency-upgrade dry
      run evidence on scheduled maintenance lanes.
  - Product workspace contract - 9.10 / 10
    - Addressed by: ownership, security evidence, review gates, upgrade
      boundaries, packaging direction.
    - Remaining lift: concise playbooks and examples.
  - Runtime ownership inventory - 8.85 / 10
    - Addressed by: current runtime-owned surfaces and extraction order.
    - Remaining lift: prune as extraction completes.
  - Product template slimming - 8.35 / 10
    - Addressed by: packaging plan and boundary checks.
    - Remaining lift: remove remaining copied runtime/provider internals after
      runtime/provider boundaries are proven.
  - Versioning and compatibility matrix - 7.60 / 10
    - Addressed by: roadmap direction and lock/provenance.
    - Remaining lift: formal SemVer, changelog, release notes, migration
      guides, provider/database compatibility matrix.

## 14. Documentation, Maintainability, And Engineering Review

- Maintainability posture - 9.88 / 10
  - Documentation information architecture - 9.25 / 10
    - Why it exists: agents need a clear path and should not read every file.
    - Addressed by: docs index, reader paths, canonical contract docs,
      archive rule.
    - Remaining lift: continue pruning internal ledgers as they age.
  - Maintainability command center - 9.20 / 10
    - Addressed by: six maintainability moves, docs-like-code rules, CLI
      contract, runtime ingress contract, golden downstream CI target.
    - Remaining lift: use it as a review checklist.
  - Roadmap as scorecard - 9.10 / 10
    - Addressed by: pruned roadmap with current scores, blockers, exit
      checklist, priorities.
    - Remaining lift: update numbers only when evidence changes.
  - Archive discipline - 8.95 / 10
    - Addressed by: archive rules and stale history pruning.
    - Remaining lift: move or delete old internal docs as they stop guiding
      active work.
  - Docs-check maintainability contract - 9.25 / 10
    - Addressed by: `target/appfw/maintainability-contract.json`.
    - Remaining lift: add assertions when new permanent docs contracts are
      introduced.
  - Engineering review appendix - 8.80 / 10
    - Addressed by: this document.
    - Remaining lift: decide whether to keep, archive, or delete after review.

## 15. Implemented Optional Ingress And Remaining Enterprise Hardening

- Governed async/event ingress - 7.85 / 10
  - Kafka runtime host loading - 9.20 / 10
    - Addressed by: independent `kafka` feature, `APPFW_RUNTIME_MODE=consumers`,
      `APPFW_MODULES=kafka`, `RuntimeHostPlan`, and `feature-check`.
    - Remaining lift: CI matrix for worker-only deployable artifacts.
  - Kafka consumer configuration shell - 8.10 / 10
    - Addressed by: app topology validation, generated runtime config
      projection under `backend/config/generated/ingress/kafka.yaml`,
      enabled/stale config validation,
      runtime YAML loading, and fail-fast worker selection.
    - Remaining lift: live broker source binding and operator examples.
  - Kafka broker authentication config - 7.70 / 10
    - Addressed by: SASL/SCRAM, mTLS, OAUTHBEARER, cloud IAM, and explicit
      local no-auth mechanisms with secret-ref validation.
    - Remaining lift: platform secret-loader integration and live credential
      evidence.
  - Service-principal runtime identity - 8.00 / 10
    - Addressed by: service actor subject, roles/scopes, tenant derivation,
      topic/partition/offset message context, and mapping to runtime
      `UserAuth`.
    - Remaining lift: optional verified on-behalf-of user delegation and
      release audit examples.
  - Idempotency, retry, and DLQ - 7.40 / 10
    - Addressed by: idempotency key requirement when retry or DLQ is enabled,
      retry backoff validation, and source/DLQ topic separation.
    - Remaining lift: persisted replay protection, concrete retry execution,
      poison-message handling, and DLQ publish evidence.
  - Event audit and metrics - 7.20 / 10
    - Addressed by: message context shape, correlation ID, runtime operation
      dispatch, and existing metrics/tracing redaction path.
    - Remaining lift: retained lag/readiness metrics, live broker telemetry,
      and source-topic audit evidence.
  - Product event binding config - 7.80 / 10
    - Addressed by: operation binding name/schema/type, payload argument
      mapping, selected result fields, and generated operation dispatch.
    - Remaining lift: product fixture tests for business event scenarios and
      custom handler binding examples.

- MCP release inclusion - 6.80 / 10 if enabled for release
  - Current release posture - 9.20 / 10 when `APP_MCP_ENABLED=false`
  - Full certification if included - 6.80 / 10
    - Needs: capability manifests, per-tool policy, token audience/resource
      checks, mandatory audit sink, HITL evidence, release retention.

## 16. Why The Framework Needed These Pieces

- Without config validation and generated contracts, agents would write
  plausible but invalid models.
- Without generated ownership, regeneration would risk destroying product
  logic or hiding drift.
- Without QueryIR and provider certification, each provider would behave like a
  different product.
- Without a clear prepared-statement versus provider-routine split, product
  developers would confuse generated SQL optimization with stored database
  entry points and either over-model provider details or hand-write unsafe SQL.
- Without access filters, policy would be advisory instead of enforceable at
  the data layer.
- Without tenant, IDOR, and locator controls, generated APIs could expose data
  across tenants or leak primary keys.
- Without audit and redaction, regulated releases would lack evidence and
  increase data exposure risk.
- Without supply-chain gates, releases could ship vulnerable or unauditable
  dependencies.
- Without observability, generated backends would be hard to operate under
  failure.
- Without performance caps and load evidence, generated APIs could scale
  accidentally into expensive or unsafe queries.
- Without frontend scaffolding, product teams would repeatedly reinvent grids,
  forms, query builders, auth, validation, and accessibility.
- Without CLI JSON artifacts and handoff reports, agent work would be hard to
  review or continue.
- Without release evidence gates, production readiness would be opinion rather
  than retained proof.
- Without docs IA and archive discipline, the framework would become too heavy
  for new engineers and agents to navigate.

## 17. Current Highest-Maturity Areas

- Maintainability and docs IA - 9.88 / 10.
- Generated ownership and drift detection - 9.40 / 10.
- Config/model validation - 9.35 / 10.
- Runtime/package extraction direction - 9.30 / 10.
- CLI and agentic command surface - 9.35 / 10.
- Release gate and supply-chain framework - 9.42 / 10.
- Frontend reference app quality - 9.07 / 10.

## 18. Current Lowest-Maturity Production Blockers

- Overall production certification bundle - 8.74 / 10. Provider certification
  itself is much stronger: the retained provider parity artifact currently
  reports PostgreSQL, MongoDB, MS SQL Server, and Snowflake as `ok=true`.
  The lower production score reflects the full go-live bundle still needing
  release-retained security, ops, performance, image/provenance, signing, and
  promotion evidence together. Supply-chain/dependency code paths are now
  substantially implemented; production readiness still depends on retaining
  their artifacts in the actual release bundle.
- Live OpenTelemetry, Alertmanager, Grafana, SIEM, and runbook evidence -
  8.35-8.70 / 10 depending on subcategory.
- Pool saturation and safe EXPLAIN diagnostics - 8.20-8.25 / 10.
- Product template slimming and version compatibility matrix - 7.60-8.35 / 10.
- Kafka/event ingress - 7.20-9.20 / 10 because the independently loaded
  runtime shell, config validation, service actor mapping, and
  message-to-operation dispatch are implemented, while concrete broker-source
  binding and live Kafka release certification still need production evidence.

## 19. Suggested Engineering Review Flow

1. Start with the executive tree.
2. Review the low-score production blockers before debating polish.
3. Review the security, provider, release, and supply-chain sections as the
   production gate.
4. Review frontend/product-lifecycle sections as the product-team enablement
   story.
5. Review maintainability/docs sections last to decide whether this appendix
   should stay, move to archive, or be deleted after the meeting.
