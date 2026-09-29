# Security Threat Model

This document captures the backend security contract for generated GraphQL APIs,
the model-driven MCP endpoint, provider query compilation, admin diagnostics,
generated policy behavior, and the current formal security assessment.
It is a release-facing companion to `docs/runtime/provider-certification.md`.
Agentic control-flow, tool-use, sandbox, and human-review risks are covered in
the companion [Agentic Threat Model](agentic-threat-model.md).

## Security Goals

- Authentication is required outside local development.
- Authorization is deny-by-default and evaluated before data provider calls.
- Tenant isolation is a framework contract for tenant-scoped entities, not a
  policy authoring convention.
- Provider-native errors are logged server-side but normalized before they reach
  GraphQL/API callers.
- GraphQL requests are bounded by body size, query depth, query complexity, and
  rate limits.
- MCP is opt-in, authenticated, origin-aware, bounded by result size, and routed
  through generated public handler-level operations before reaching
  DataAccess/QueryIR/policy/provider execution.
- Provider compilers produce parameterized SQL/BSON from shared IR and schema
  metadata rather than concatenating user-controlled identifiers or literals.
- Admin diagnostics require an authenticated admin role and troubleshooting is
  disabled unless explicitly enabled.
- Secrets are resolved through the configured secret provider and copied into
  runtime data-source environments before provider clients initialize.

## GraphQL Threats

| Threat | Mitigation |
| --- | --- |
| Deep recursive selection causes high CPU/database cost. | Generated schema routes apply `limit_depth` from `APP_GRAPHQL_MAX_DEPTH`. |
| High-complexity query causes expensive resolver fanout. | Generated schema routes apply `limit_complexity` from `APP_GRAPHQL_MAX_COMPLEXITY`. |
| Oversized body exhausts memory. | Root router applies `DefaultBodyLimit` from `APP_REQUEST_BODY_LIMIT_BYTES`. |
| Request flood degrades the service. | Root router applies keyed governor rate limiting by tenant, auth token hash, or client IP. |
| Unauthenticated access outside local development. | `RuntimeJwtExtractor` rejects missing or invalid credentials outside `ENV_NAME=local`; introspection is separated from normal execution. |
| Provider-native error detail leaks through GraphQL. | Provider errors are mapped to stable `DataStoreError` classes before GraphQL serialization. |

## Provider SQL/BSON Compilation Threats

| Threat | Mitigation |
| --- | --- |
| SQL injection through field names or sort keys. | Query IR resolves fields through entity metadata before provider compilation. |
| SQL/BSON injection through filter values. | PostgreSQL, MS SQL, Snowflake, and MongoDB compilers bind values or build typed BSON documents from validated IR. |
| Semantic drift between providers. | Provider certification runs shared live contracts for filters, sorting, pagination, relationships, access filters, errors, concurrency, and audit. |
| Native constraint errors leak provider details. | `provider_error` maps duplicate key, foreign key, required value, and unknown provider failures to stable messages. |
| Tenant filter omitted by one provider. | Tenant isolation is applied before provider calls as a required access filter for tenant-scoped entities. |

## Tenant Isolation Contract

Tenant isolation is applied centrally by `appfw-runtime::tenant_isolation`.
An entity with a native `tenant_id` property is tenant-scoped. When access is
allowed, the framework conjoins the policy filter with:

```json
{ "tenant_id": { "_eq": "<user tenant id>" } }
```

If a tenant-scoped entity is requested by a user without a tenant id, access is
denied before provider execution. Local policy bypass skips this only for local
development workflows and must not be enabled in deployed environments.

## Admin Diagnostics Threats

| Threat | Mitigation |
| --- | --- |
| Non-admin user reads schema/model diagnostics. | `/admin/model` and troubleshooting routes require an authenticated user with the `admin` role. |
| Troubleshooting reveals policy or audit detail by default. | Troubleshooting endpoints require `APP_ADMIN_TROUBLESHOOTING_ENABLED=true`. |
| Diagnostics expose request identifiers inconsistently. | Admin error payloads include request/correlation context for support without exposing secrets. |
| Query diagnostics leak raw SQL, bind values, or tenant-sensitive filters. | Provider EXPLAIN hooks default to `unsupported`; providers may override only with safe, redacted diagnostics. Query diagnostics expose QueryIR cost and budget metadata instead of raw provider commands. |
| Static admin bundle leaks secrets. | Admin model is generated from runtime metadata and provider capability summaries; credentials are not included. |

## MCP Threats

| Threat | Mitigation |
| --- | --- |
| Agentic clients gain an unaudited alternate backend path. | `/mcp` is only mounted when `APP_MCP_ENABLED=true`; users must pass the MCP role/scope gate; generated operation tools expose public handler-level operation names and execute through the generated handler dispatcher, `DataAccess`, QueryIR, access policy, tenant isolation, redaction, entity audit for audited entities, metrics, and provider clients. |
| Browser-based MCP client can be driven from an untrusted origin. | MCP requests with an `Origin` header must match `APP_MCP_ALLOWED_ORIGINS`, except loopback origins in `ENV_NAME=local`. |
| MCP tool returns too much data to an agent. | Pagination policy applies through QueryIR, JSON-RPC batches are capped by `APP_MCP_MAX_BATCH_ITEMS`, tool responses are capped by `APP_MCP_MAX_RESULT_BYTES`, and resource responses are capped by `APP_MCP_MAX_RESOURCE_BYTES`. |
| Tool resources leak provider credentials. | MCP resources expose schema/entity summaries only; data-source environments and secrets are omitted. |
| Agent performs destructive writes before certification. | MCP mutation tools are not implemented yet; startup rejects `APP_MCP_MUTATIONS_ENABLED=true` until write-safety certification exists. |

## Generated Policy Threats

| Threat | Mitigation |
| --- | --- |
| Missing policy accidentally allows access. | Missing policies deny by default. Local allow-missing is guarded by `ENV_NAME=local`. |
| Policy filter widens a user filter. | Access filters are conjoined with user filters and certified by live policy-widening tests. |
| Policy author forgets tenant scope. | Tenant isolation is added by the framework for tenant-scoped entities. |
| Denied mutations leave no audit context. | Data access records denied and failed mutation audit events where auditing is configured. |
| CI provider certification cannot exercise policy users in compose environments. | `APP_ENABLE_LOCAL_TEST_AUTH=true` accepts only explicit `Bearer appfw-local:...` test tokens and does not enable missing-auth local admin access. Outside `ENV_NAME=local`, startup validation requires `ENV_NAME=compose` and `APP_PROVIDER_CERTIFICATION_CI=true`. |
| Local policy bypass is accidentally enabled in a deployed environment. | Startup validation rejects `APP_BYPASS_POLICIES_IN_LOCAL=true` and `APP_ALLOW_MISSING_POLICIES_IN_LOCAL=true` unless `ENV_NAME=local`. |

## Formal Security Assessment

Assessment date: June 2, 2026. Reassessed on June 3, 2026 after the
release-security hardening branch.

Scope: source, generated artifacts, CI/release documentation, and security
architecture review of `app_framework`. This assessment reviewed the supplied
Application and Platform Validation Process SOP, Information Security Handbook,
Software Development Life Cycle Policy, EAC application security, data
classification, data security, and agent standards, plus the current
[OWASP Top Ten 2025](https://owasp.org/Top10/2025/) release. It did not include
dynamic penetration testing, deployed-environment validation, access-review
sampling, SIEM evidence review, or live IdP configuration review.

Disposition: the framework is strongly aligned with secure-by-design
development and provider certification. The original P0 code blockers in this
assessment have been reduced to release-evidence requirements: GraphQL
introspection authorization is now gated, SBOM generation and release gating are
wired, regulated data classification is validated, and CORS/CSP defaults have
been hardened. The framework is still not production-certified for regulated PHI
until live provider/security certification, deployable image evidence, retained
release artifacts, and any required operational security evidence are produced
or formally risk accepted.

### Release Scope Decisions

These decisions apply to the current release posture:

- SA-01, SA-02, SA-03, and SA-06 are no longer open code blockers. They are
  implemented controls with residual release-evidence requirements.
- DAST, SAST-equivalent security tooling, OWASP ASVS traceability, release
  provenance, and artifact-signing evidence are in scope for regulated
  production. `scripts/ci/security-assurance-decision.sh` requires those
  categories by default. A lower-risk lane or deferred control must retain a
  time-boxed, approved `security-risk-acceptance.json` with owner, approver,
  expiry, rationale, compensating controls, and follow-up for each missing
  category. Production release gates also set
  `APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS=true`, so local provenance and
  OpenSSL signing artifacts cannot satisfy production provenance/signing
  categories.
- Live operations evidence is release-gated when
  `APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true`. In that mode,
  `ops-certification` requires semantic `ok=true` JSON evidence, live OTLP
  exporter configuration with service identity, protocol, and endpoint,
  Alertmanager proof from `APPFW_ALERTMANAGER_URL`, and passing runbook-drill
  steps. Retained static Alertmanager config is useful local evidence, but it
  does not satisfy live operations proof by itself.
- Deployable image SBOM evidence is required whenever this repository builds or
  promotes a release image, a release image ref/tar/OCI artifact is retained, or
  release policy sets `APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM=true` or
  `APPFW_RELEASE_REQUIRE_IMAGE_SBOM=true`. If the release image is built outside
  this repository, the image-build lane must produce and retain the image SBOM
  and pass the artifact link/hash into the release evidence bundle; this repo's
  gate may mark image SBOM as `not-applicable` only when no deployable image is
  part of the release handled here.
- MCP is release-excluded unless a dedicated MCP release lane certifies signed
  capability manifests, per-action policy authorization, token
  audience/resource enforcement, mandatory audit evidence, HITL records,
  replay tests, and kill-switch behavior. The practical current posture is
  `APP_MCP_ENABLED=false`.
- Release readiness is not final until provider-backed `release-check`,
  `provider-test --all`, `security-certification`, supply-chain/SBOM gates,
  secret scan, PHI log lint, strict release-evidence check, and handoff evidence
  are retained from a clean release checkout.

### Assessment Basis

Positive evidence found:

- Managed-environment API access uses centralized JWT validation for normal
  requests, with local/test auth constrained by startup safety checks.
- Authorization is evaluated server-side, missing policies deny by default, and
  tenant isolation is conjoined centrally for tenant-scoped entities.
- GraphQL routes apply body, rate, depth, complexity, timeout, and panic
  isolation controls.
- GraphQL introspection is disabled by default outside local development unless
  explicitly enabled, and non-local introspection requires a verified JWT plus
  an approved admin/developer role or scope.
- Provider compilers use metadata/IR and bound values rather than accepting
  user-supplied SQL/BSON structure directly.
- Audit events redact sensitive fields before diffing and providers finalize
  audit hash chains before persistence.
- Release CI includes format/lint, RustSec vulnerability scanning, cargo-deny
  advisories/licenses/bans/sources, gitleaks secret scanning, PHI/PII log lint,
  CycloneDX SBOM generation for Rust/frontend dependencies, conditional
  deployable image SBOM evidence, provider certification, and retained release
  evidence.
- Live operations evidence collection now probes backend readiness/metrics and
  Alertmanager when `APPFW_ALERTMANAGER_URL` is provided, redacts retained URLs,
  and separates static observability bundle evidence from production live proof.
- Regulated profiles require effective, known classification metadata for data
  sources, schemas, entities, and properties before validation passes.
- Managed CORS excludes `TRACE` and `CONNECT`; production CSP is split from the
  local/GraphiQL/admin convenience posture.
- MCP is disabled by default, role/scope-gated when enabled, origin-aware for
  browser clients, bounded by result/resource sizes, and mutation support is
  blocked until write-safety certification is implemented.

### Prioritization Model

| Priority | Meaning | Target |
| --- | --- | --- |
| P0 | Release blocker for regulated/PHI production or a critical control failure. | Close before promotion; 30-day critical remediation target. |
| P1 | High-risk gap requiring remediation or formal risk acceptance with compensating controls. | 60-day high remediation target. |
| P2 | Medium-risk hardening, evidence, or documentation gap. | 90-day medium remediation target. |
| P3 | Low-risk improvement or backlog hygiene. | 180-day low remediation target. |

### Prioritization Matrix

| ID | Current Priority | Current Disposition | Finding / Residual Risk | OWASP 2025 Mapping | Internal Driver | Release Action |
| --- | --- | --- | --- | --- | --- | --- |
| SA-01 | P2 residual | Code-resolved; release evidence required. | Non-local GraphQL introspection is disabled by default unless explicitly enabled, and enabled non-local introspection requires a verified JWT plus an admin/developer role or scope. Residual risk is stale or missing release evidence for no-auth, fake-bearer, non-admin, and privileged-user cases. | A01 Broken Access Control, A02 Security Misconfiguration, A07 Authentication Failures | APP-SEC-STD-001 secure defaults, centralized identity, least privilege; SDLC auth requirements | Retain green `scripts/appfw product security-certification --json` evidence for the introspection auth regression cases before release. |
| SA-02 | P2 residual | Gate-resolved; CI artifacts required. | Release scripts generate and validate SBOM evidence for Rust and frontend dependencies, dependency-check advisory evidence, plus deployable image SBOM evidence when an image source/artifact exists or the release requires it. Residual risk is cutting a release without retained CI SBOM/dependency artifacts, with hidden vulnerability exceptions, or without image SBOM proof when applicable. | A03 Software Supply Chain Failures, A08 Software or Data Integrity Failures | APP-SEC-STD-003 SBOM; SDLC supply-chain evidence | Retain `target/appfw/dependency-check.json`, `target/appfw/sbom-manifest.json`, Rust/frontend SBOM artifacts, and deployable image SBOM artifacts when applicable; require any OSV acceptance to be source-controlled in `dependency-check.toml`, time-boxed, and echoed in release evidence. |
| SA-03 | P2 residual | Validation-resolved; product metadata evidence required. | Regulated validation now requires effective, known classification metadata for data sources, schemas, entities, and properties. Residual risk is product data-quality drift or release artifacts that do not preserve the validated contract. | A02 Security Misconfiguration, A04 Cryptographic Failures, A06 Insecure Design, A09 Security Logging and Alerting Failures | DATA-CLASS-STD-001, DATA-CLASS-STD-002, DATA-SEC standards; SDLC data handling requirements | Retain green `scripts/appfw product validate --json` and config-contract evidence for the regulated product profile; document any approved classification exception. |
| SA-04 | P1 | Release-excluded unless separately certified. | MCP controls are strong for a disabled/default-read surface, but regulated agent use is not yet aligned to EAC agent standards: signed/versioned capability manifests, action-layer OPA/Rego policy evaluation before tool execution, RFC 8707 resource/audience enforcement, mandatory denial audit, and HITL evidence are not complete. | A01 Broken Access Control, A06 Insecure Design, A09 Security Logging and Alerting Failures | AGENT-STD-001, AGENT-STD-002, AGENT-STD-006, AGENT-STD-007 | Keep `APP_MCP_ENABLED=false` for this release, or create a dedicated MCP certification lane with capability manifests, per-action policy evaluation, token audience/resource checks, mandatory audit, replay tests, kill-switch evidence, and HITL records. |
| SA-05 | P1 | Gate-hardened operational evidence gap. | Application audit chains exist and live-ops release mode now rejects parse-only evidence unless OTLP, Alertmanager, and runbook-drill artifacts are semantically green. The collector now probes `APPFW_ALERTMANAGER_URL` and records live endpoint status instead of treating static routing config as live proof. Production validation evidence still needs SIEM export, retention, alerting, access-review sampling, and proof that security events are monitored. | A09 Security Logging and Alerting Failures, A10 Mishandling of Exceptional Conditions | Information Security Handbook logging/monitoring requirements; Validation SOP access/security evidence | Add release/deployment evidence for audit log export, at least 90-day operational retention where applicable, live OTLP export, live Alertmanager/Grafana proof, alert thresholds/playbooks, access-control failure monitoring, and DAST/abuse-test alert verification. |
| SA-06 | P2 residual | Code-resolved; deployed-header proof required. | Managed CORS no longer advertises `CONNECT` or `TRACE`, and production CSP is split from local/admin convenience CSP. Residual risk is deployment overlay drift or unverified managed headers. | A01 Broken Access Control, A02 Security Misconfiguration | APP-SEC-STD-001 secure defaults; OWASP hardening guidance | Retain production header/CORS/CSP evidence from a deployed or release-equivalent environment. |
| SA-07 | P2 | Open deployment evidence gap. | MFA, PHI session inactivity timeout, and SSO session lifecycle controls appear delegated to the IdP/frontend but are not yet release-evidenced by the framework. | A07 Authentication Failures | APP-SEC-STD-001 MFA and PHI session requirements; Information Security Handbook IAM requirements | Add deployment validation evidence for MFA, audience/issuer/scope enforcement, logout/token revocation expectations, and PHI inactivity timeout of 15 minutes or less where the generated app handles session state. |
| SA-08 | P3 residual | Reduced by docs-check parity. | Deployment hardening documentation now has security-env parity checks, but runtime/docs parity should continue expanding as new security env vars and startup safety checks are added. | A02 Security Misconfiguration, A10 Mishandling of Exceptional Conditions | Validation SOP evidence-based configuration; SDLC secure configuration requirements | Keep `scripts/appfw framework docs-check --json` green and extend parity assertions when security configuration surfaces change. |
| SA-09 | P2 residual | Production attestation gate added. | CI has strong SCA/secret/log/SBOM gates, the release wrapper requires a formal decision for DAST, SAST-equivalent security tooling, OWASP ASVS traceability, release provenance, and artifact signing, and production attestation mode rejects local provenance/signing placeholders. Residual risk remains until regulated release lanes produce enterprise artifacts or approved, time-boxed risk acceptance. | A05 Injection, A06 Insecure Design, A08 Software or Data Integrity Failures, A10 Mishandling of Exceptional Conditions | APP-SEC-STD-004 SSDLC; SDLC threat modeling and security verification requirements | For regulated production, provide the required security-assurance artifacts and enterprise provenance/signing attestations. For non-regulated or deferred scopes, retain `security-risk-acceptance.json` with owner, approver, expiry, rationale, compensating controls, and follow-up for each missing category. |

### OWASP Coverage Summary

| OWASP 2025 Category | Current Posture |
| --- | --- |
| A01 Broken Access Control | Strong server-side Rego policy, deny-by-default behavior, tenant isolation, admin role checks, verified non-local introspection auth, and MCP front-door gates. Remaining gaps are live provider/security certification and regulated MCP action governance if MCP is included. |
| A02 Security Misconfiguration | Security headers, hardened CORS allow-listing, production/local CSP separation, runtime safety checks, bounded requests, and security-env docs parity are present. Remaining work is deployed-header proof, IdP/session evidence, and ongoing config parity. |
| A03 Software Supply Chain Failures | Cargo audit, cargo-deny, dependency-check evidence, secret scanning, retained release evidence, OSV/CVE correlation, npm audit coverage, and CycloneDX SBOM gates are present. Remaining work is CI retention, deployable image SBOM proof when applicable, and vulnerability/provenance correlation against enterprise attestations. |
| A04 Cryptographic Failures | JWT issuer/audience validation and signed cursors exist; TLS/encryption-at-rest evidence is deployment/provider dependent and must be validated per environment and data classification. |
| A05 Injection | Provider compilers use validated IR and bound values. DAST/fuzzing and ASVS traceability should be added as release evidence. |
| A06 Insecure Design | The framework has a documented threat model, secure patterns, regulated classification validation, and release posture gates. Remaining design work is concentrated in agent/MCP standards and DAST/SAST/ASVS traceability. |
| A07 Authentication Failures | Centralized JWT auth and verified introspection authorization are implemented outside local development. MFA and PHI session lifecycle evidence remain deployment responsibilities. |
| A08 Software or Data Integrity Failures | Release artifacts, audit hash chains, SBOM gates, and release artifact hashes support integrity. Production release gates now reject local provenance/signing placeholders; enterprise provenance/signing evidence or approved risk acceptance remains required. |
| A09 Security Logging and Alerting Failures | Redaction, request context, audit events, PHI log lint, semantic live-ops evidence checks, and live Alertmanager endpoint probing are stronger. SIEM export, retention, alerting, live OTLP/export proof, Grafana API proof, and access-review evidence remain open. |
| A10 Mishandling of Exceptional Conditions | Timeouts, rate limits, panic isolation, redacted errors, and config/docs parity checks are present. Alerting on abuse/DAST scenarios should be added or risk accepted. |

## Release Checks

Before a framework release, run:

```bash
scripts/appfw product release-check --json
```

This command runs validation, MCP release-posture checks, boundary checks,
generated drift checks, local tests, policy evidence, the live provider
certification matrix, security certification, and handoff evidence. It requires
provider-backed backend instances to be running for release-grade provider and
security certification. Bitbucket release gates should also run the supply-chain
gate, dependency-check evidence, secret scan, PHI log lint, SBOM generation, and
strict release-evidence check. ArgoCD should promote only artifacts that already
retained passing
evidence for:

- `target/appfw/release-check.json`
- `target/appfw/provider-parity.json`
- `target/appfw/security-certification.json`
- `target/appfw/release-mcp-posture.json`
- `target/appfw/dependency-check.json`
- `target/appfw/sbom-manifest.json`
- `target/appfw/release-evidence-check.json`
- `target/appfw/agent-handoff.json`

For regulated production, include DAST, SAST-equivalent tooling, OWASP ASVS
traceability, provenance, and artifact-signing artifacts in the retained release
evidence. For non-regulated lanes or formally deferred controls, record an
explicit, time-boxed risk acceptance with compensating controls and follow-up.
Dependency-specific OSV acceptances are source-controlled in
`dependency-check.toml` and echoed into `target/appfw/dependency-check.json`
with package/version, advisory aliases, owner, rationale, and expiry; expired
or invalid acceptances fail strict dependency evidence.
