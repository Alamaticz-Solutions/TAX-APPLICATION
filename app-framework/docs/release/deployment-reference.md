# Deployment Reference

This reference describes the enterprise deployment shape for generated App
Framework backends. It is intentionally operational and agent-readable: the goal
is to make container, environment, secrets, CI, and observability expectations
explicit before each application team creates its own deployment repo.

## Deployment Path For Agents

Use this path before changing deployment or release evidence:

| Stage | Purpose | Commands Or Artifacts |
| --- | --- | --- |
| Local handoff | Prove the checkout is coherent before deployment work leaves the branch. | `scripts/appfw validate --json`; `scripts/appfw docs-check --json`; `scripts/appfw generate --check --json`; `scripts/appfw test --fast --json`; `scripts/appfw handoff --json` |
| Release candidate | Prove generated output, tests, provider semantics, policy, supply chain, frontend, performance, and handoff in CI. | `scripts/appfw release-check --json`; provider certification; SBOM/secret/PHI/security-assurance evidence; frontend/performance evidence when in scope |
| Production promotion | Prove live providers, live security, live operations, PDS baseline posture, deployable image evidence, artifact hashes, and rollback/promotion metadata. | strict `release-evidence-check.sh`; `target/appfw/pds-security-baseline.json`; deployable image SBOM; live OTLP/Alertmanager/runbook artifacts; immutable image digest; promotion annotation |

Keep the three stages separate. Local proof can make a branch ready for review;
it cannot replace live provider-backed release evidence. If a release lane
skips frontend, performance, ops, DAST/SAST/ASVS, provenance, signing, or image
SBOM evidence, the release bundle must say whether that category is out of
scope or formally risk-accepted.

For PDS production-readiness claims, also map the release bundle against
[`pds-security-baseline-traceability.md`](pds-security-baseline-traceability.md).
That baseline requires live or release-authority evidence for IdP/MFA,
user-lifecycle, secrets management, SIEM, gateway/WAF/TLS, backup/restore,
lower-environment data, vulnerability SLA, and SAST/DAST categories that cannot
be proven by local repository checks alone.

## Container Model

Build and ship one product backend image by default:

- Backend image: compiled `backend` binary plus the product SPA bundle when
  present, plus the static admin bundle when `APP_ADMIN_UI_ENABLED=true`.
- Database utility image or CI job image: compiled `database` binary plus
  `database/_pkg`.

The backend image is the one in-cluster product serving unit. It can serve the
Rust API routes, the product SPA, and the admin UI from the same process:

```text
/                 product SPA index/fallback when backend/product_dist exists
/assets/*         product SPA static assets
/<schema>         generated GraphQL API routes such as /crm
/admin            backend-hosted admin UI when enabled
/health/ready     readiness probe
/metrics          Prometheus metrics
```

This is the default topology for product teams. A separate nginx/static
frontend deployment is an explicit advanced topology for products that need an
independent UI release cadence, CDN/edge caching, or different frontend scaling
boundaries.

Product UI packaging posture belongs in `.appfw/manifest.yaml`:

```yaml
ui:
  product_spa:
    enabled: true
    packaging: backend-product-dist
  admin_ui:
    enabled: false
    packaging: disabled
```

When `ui.admin_ui.enabled:true`, release evidence should include
`backend/admin_dist/index.html` and the deployment environment should set
`APP_ADMIN_UI_ENABLED=true`. When admin UI is not part of the product image,
leave `APP_ADMIN_UI_ENABLED=false` and record `ui.admin_ui.packaging: disabled`
so composition checks distinguish an intentional product posture from a missing
admin bundle.

The backend image should run only the backend process:

```bash
ENV_NAME=prod API_HOST=0.0.0.0 API_PORT=8080 backend
```

Run migrations as a separate job before backend rollout:

```bash
scripts/appfw migrate plan --json
scripts/appfw migrate lint --phase all --json
scripts/appfw migrate drift --json
scripts/appfw migrate apply --json
```

Contract migrations are never part of normal deploy. They require an approved
window:

```bash
scripts/appfw migrate apply --phase contract --confirm-contract --json
```

## Environment Contract

Required runtime identity:

```text
ENV_NAME
API_HOST
API_PORT
RUST_LOG or LOG_LEVEL
APP_DATA_SOURCE_NAME
```

Use `API_HOST=127.0.0.1` for host-local development. Containerized deployments
usually set `API_HOST=0.0.0.0` inside the container and let the platform,
ingress, or compose port binding control external exposure.

Runtime ingress selection:

```text
APPFW_RUNTIME_MODE
APPFW_MODULES
```

When no runtime mode is set, a binary compiled with HTTP starts the HTTP
listener only. Worker modules such as Kafka must be selected explicitly with
`APPFW_RUNTIME_MODE=consumers` or `APPFW_MODULES=kafka`, unless the binary was
compiled as a worker-only artifact. This keeps an optional worker feature in
`Cargo.toml` from silently starting broker consumers in an API deployment.

Kafka worker ingress is configured separately from topology so service actors,
tenant mapping, operation bindings, and secret references stay product-owned:

```text
APPFW_KAFKA_INGRESS_CONFIG
```

When this variable is omitted, the backend looks for
`config/generated/ingress/kafka.yaml` relative to the process working
directory. App generation creates that file only for enabled Kafka topology
entries, so disabled or excluded Kafka ingress does not introduce a runtime
binding. Selecting Kafka workers with no config, disabled config, or no broker
message source fails fast instead of silently running an idle worker.
The config must be reviewed before production: retry and dead-letter handling
require an `idempotency_key_field`, dead-letter topics must differ from source
topics, and readiness thresholds such as lag and idle time should match the
product's recovery objective. Broker credentials still come from the platform
secret mechanism via the configured auth `secret_ref`.

Security posture:

```text
APP_AUTH_MODE
APP_ADMIN_UI_ENABLED
APP_ADMIN_TROUBLESHOOTING_ENABLED
APP_PRODUCT_UI_ENABLED
APP_ENABLE_LOCAL_TEST_AUTH=false
APP_ALLOW_MISSING_POLICIES_IN_LOCAL=false
APP_BYPASS_POLICIES_IN_LOCAL=false
APP_CHAT_ENABLED=false
APP_CHAT_REQUIRED_ROLES
APP_CHAT_REQUIRED_SCOPES
APP_CHAT_PROMPT_AUDIT_ENABLED=false
APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED=false
APP_CHAT_PROMPT_AUDIT_SINK
APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS=90
APP_CHAT_KILL_SWITCH_ACTIVE=false
APP_MCP_ENABLED=false
APP_MCP_MUTATIONS_ENABLED=false
APP_MCP_ALLOWED_ORIGINS
APP_MCP_MAX_RESULT_BYTES
APP_MCP_MAX_RESOURCE_BYTES
APP_MCP_MAX_BATCH_ITEMS
APP_MCP_REQUIRED_ROLES
APP_MCP_REQUIRED_SCOPES
APP_MCP_PRIVILEGED_TOOL_ROLES
APP_MCP_PRIVILEGED_TOOL_SCOPES
APP_GRAPHQL_INTROSPECTION_ENABLED=false
APP_GRAPHQL_INTROSPECTION_REQUIRED_ROLES
APP_GRAPHQL_INTROSPECTION_REQUIRED_SCOPES
APP_REQUEST_BODY_LIMIT_BYTES
APP_GRAPHQL_MAX_DEPTH
APP_GRAPHQL_MAX_COMPLEXITY
APP_RATE_LIMIT_PER_SECOND
APP_RATE_LIMIT_BURST
```

### Okta/OIDC Login With oauth2-proxy Sidecar

The recommended managed-environment browser flow keeps App Framework as a
resource server. The app validates bearer JWTs and enforces policy; it does not
own the browser authorization-code callback, PKCE verifier, refresh token, or
session cookie.

For a single app behind Istio, the preferred topology is:

```text
Browser
  -> Istio Gateway / VirtualService
  -> Service port 4180
  -> oauth2-proxy sidecar
       owns /oauth2/* login and callback routes
       redirects unauthenticated users to Okta
       redeems the authorization code
       sets the oauth2-proxy session cookie
       injects Authorization: Bearer <access token>
  -> 127.0.0.1:8080 app backend
       validates issuer, audience, and client id
       maps token claims to UserAuth
       enforces Rego policy, tenant isolation, and audit
```

The Kubernetes Service should point at the oauth2-proxy container port, not the
backend port. The proxy should use a loopback upstream such as
`http://127.0.0.1:8080` so the backend is reachable only inside the pod or
through explicitly approved internal probes. The VirtualService can keep the
normal `/` prefix; oauth2-proxy owns `/oauth2/*` and proxies the remaining app
routes upstream.

Use an Okta **Web/confidential** application for oauth2-proxy. A public SPA
client id is not enough because oauth2-proxy needs a client secret. The Okta
redirect URI must match the proxy callback, for example:

```text
https://<app-host>/oauth2/callback
```

Configure oauth2-proxy as the OAuth/OIDC client:

```text
--provider=oidc
--oidc-issuer-url=https://<okta-domain>/oauth2/<authorization-server-id>
--client-id=<oauth2-proxy-web-client-id>
--client-secret=<secret-from-platform-secret-store>
--redirect-url=https://<app-host>/oauth2/callback
--scope="openid email profile groups offline_access"
--reverse-proxy=true
--cookie-secure=true
--code-challenge-method=S256
```

When using oauth2-proxy alpha configuration for upstream and header injection,
inject the **access token** into the upstream request:

```yaml
injectRequestHeaders:
- name: Authorization
  values:
  - claimSource:
      claim: access_token
      prefix: "Bearer "
```

Do not rely on `pass_authorization_header` for App Framework API calls; that
passes the ID token in oauth2-proxy's legacy flag model. App Framework expects
the Okta access token in `Authorization`. If the proxy instead emits
`X-Forwarded-Access-Token`, add an explicit, trusted edge mapping to
`Authorization: Bearer <access token>` before the request reaches the backend.

The backend must validate the same token the proxy injects:

```text
OKTA_ISSUER=https://<okta-domain>/oauth2/<authorization-server-id>
OKTA_CLIENT_ID=<oauth2-proxy-web-client-id>
OKTA_AUDIENCE=<access-token-audience>
```

`OKTA_AUDIENCE` must be set explicitly for managed environments; otherwise the
runtime default is `api://default`, which may not match a custom Okta
authorization server. App Framework maps the access token `company` claim to
tenant id, `sub` to user name, `groups` to roles, and `scp` to scopes. Okta must
emit a `groups` claim in the access token or authenticated users will arrive
with empty roles and will usually be denied by policy.

Deployment review must verify:

- oauth2-proxy uses PKCE with S256 when required by the IdP/security baseline.
- inbound client-supplied `Authorization` headers cannot spoof the trusted
  proxy-injected bearer token.
- the backend port is not directly exposed through the public Service or
  VirtualService path.
- oauth2-proxy cookies are `Secure`, `HttpOnly`, scoped to the app host/path,
  and have SameSite and lifetime values approved for the environment.
- logout, token revocation expectations, MFA, and session timeout are owned and
  evidenced by the IdP/proxy deployment, not by the app backend.
- release evidence includes a no-cookie redirect check, a post-login backend
  request containing a valid access token, a denied request for a token missing
  required groups/roles, and a token audience/issuer mismatch rejection.

Static UI bundle locations:

```text
APP_PRODUCT_UI_DIST_DIR
APP_ADMIN_UI_DIST_DIR
```

When `APP_PRODUCT_UI_DIST_DIR` is omitted, the backend looks for the product
SPA at `backend/product_dist` relative to the backend package manifest. When
the bundle is present and `APP_PRODUCT_UI_ENABLED=true`, the runtime mounts it
at `/` with `/assets/*` static serving and SPA fallback. The fallback reserves
admin, MCP, health, metrics, and generated schema API prefixes so the product
UI cannot mask backend API or operations paths.

Observability:

```text
APP_OTEL_ENABLED
APP_LOG_JSON
OTEL_EXPORTER_OTLP_ENDPOINT
OTEL_SERVICE_NAME
DATABASE_OTEL_ENABLED
DATABASE_LOG_JSON
```

Provider credentials are data-source specific and should come from the platform
secret mechanism, not checked-in config:

```text
PG_SERVICE_ACCOUNT_NAME
PG_SERVICE_ACCOUNT_PASS
MONGO_SERVICE_ACCOUNT_NAME
MONGO_SERVICE_ACCOUNT_PASS
MSSQL_SERVICE_ACCOUNT_NAME
MSSQL_SERVICE_ACCOUNT_PASS
MSSQL_AUTH_MODE
# When MSSQL_AUTH_MODE=ntlm (or YAML auth_mode), provide domain Windows
# credentials via MSSQL_SERVICE_ACCOUNT_NAME / MSSQL_SERVICE_ACCOUNT_PASS.
# Runtime image needs FreeTDS + unixODBC; sql_password uses Microsoft ODBC
# Driver 18. See docs/specs/mssql-odbc-ntlm-authentication.md.
FABRIC_SQL_AUTH_MODE
FABRIC_TENANT_ID
FABRIC_CLIENT_ID
FABRIC_CLIENT_SECRET
FABRIC_TOKEN_SCOPE
APP_FABRIC_ODBC_DRIVER
SNOWFLAKE_SERVICE_ACCOUNT_NAME
SNOWFLAKE_ACCESS_TOKEN
SNOWFLAKE_OAUTH_TOKEN
SNOWFLAKE_JWT
SNOWFLAKE_SERVICE_ACCOUNT_PASS
SNOWFLAKE_AUTH_TOKEN_TYPE
NEO4J_SERVICE_ACCOUNT_NAME
NEO4J_SERVICE_ACCOUNT_PASS
```

Managed environments must use TLS-enabled data-source environment entries.
Local plaintext exceptions are limited to `ENV_NAME=local` and explicit
provider-certification CI settings.

## Secrets

Do not bake secrets into generated images, generated YAML, app repos, or
Bitbucket variables visible to pull requests. Recommended sources:

- Kubernetes secrets or external secret operators.
- Cloud secret managers with short-lived workload identity.
- CI secured variables scoped to release pipelines only.

Secrets must be redacted from logs, diagnostics, GraphQL errors, provider
commands, migration output, and handoff artifacts.

## CI/CD

Application pull requests should run:

```bash
scripts/appfw validate --json
scripts/appfw docs-check --json
scripts/appfw test
scripts/appfw handoff --json
```

Release candidates should additionally run generated API tests against a
deployed test environment:

```bash
scripts/appfw migrate apply --json
scripts/appfw api-test
```

Framework releases must use the Bitbucket release gate:

```bash
scripts/appfw release-check --json
```

Static dependency evidence is produced by the supply-chain lane, not by the
provider-backed release-check process:

```bash
scripts/appfw dependency-check --json --strict
```

The retained report is `target/appfw/dependency-check.json`. It records Cargo
and npm inputs, duplicate Rust dependency versions, direct dependency upgrade
advice, RustSec/cargo-deny status, npm audit/outdated status, and OSV advisory
matches. The CI supply-chain gate writes this artifact and the strict release
evidence check requires it.
Product mobile npm audit findings remain visible when the mobile workspace
retains U5 diagnostic evidence under `.appfw-mobile/`. M0-05 makes
`scripts/appfw product mobile-test --json` non-authoritative and permanently
fail-closed for candidate/release readiness; it cannot waive or accept those
findings. They must be cleared or formally dispositioned before the future
source-bound mobile candidate checker can consider the app, and release still
requires named human authority.
Set `APPFW_DEPENDENCY_CHECK_CARGO_HOME` in CI when advisory database/cache files
must live on a dedicated writable cache volume. If it is unset, the
supply-chain gate and dependency checker use writable `CARGO_HOME`/`$HOME/.cargo`
when available and otherwise fall back to `target/appfw/cargo-home`; both
artifacts record the selected Cargo home and source.

The Bitbucket release wrapper starts a LocalStack Snowflake emulator for live
Snowflake provider certification. Configure `LOCALSTACK_AUTH_TOKEN` as a
secured CI variable before running that wrapper; the wrapper fails during
prerequisite checks when the token is missing or a placeholder. This is separate
from application Snowflake runtime credentials.

ArgoCD should promote only immutable image tags or digests that already passed
the release gate. Promotion annotations should include the Bitbucket build and
provider certification artifact link. Treat
`target/appfw/bitbucket-release-gate.json` `release_ready:true` as the promotion
signal; top-level `ok:true` only proves the wrapper completed and wrote a valid
evidence file. After the wrapper reaches release-gate execution, failed stages
also retain this artifact with `ok:false`, `release_ready:false`,
`failed_stage`, `exit_status`, and available blocker details for audit.

Regulated release lanes must also retain the security-assurance decision
artifact from `scripts/ci/security-assurance-decision.sh`. Provide DAST,
SAST/ASVS traceability, release provenance, and artifact-signing evidence before
promotion, or retain a time-boxed `security-risk-acceptance.json` with owner,
approver, release scope, rationale, compensating controls, and follow-up for
each missing category. The default maximum acceptance window is 90 days unless
`APPFW_SECURITY_RISK_ACCEPTANCE_MAX_DAYS` is set by an approved release policy.
Image-based releases should set
`APPFW_RELEASE_REQUIRE_IMAGE_SBOM=true` and provide exactly one release image
source so `scripts/ci/supply-chain-gate.sh` can produce the deployable image
SBOM. If `syft` is not already available, the supply-chain gate installs the
pinned scanner release and verifies the downloaded archive checksum before
generating `target/appfw/sbom-deployable-image.cdx.json`.
Strict dependency evidence is produced by
`scripts/appfw dependency-check --json --strict` inside the supply-chain gate.
Temporary OSV acceptances must be declared in `dependency-check.toml`; the
retained `target/appfw/dependency-check.json` records accepted findings and
fails strict mode when an acceptance expires or the policy file is invalid.
Mobile npm audit findings that are explicitly release-gated by retained U5
mobile evidence are recorded separately from blocking findings; they do not
turn a static mobile scaffold into release evidence.

Release candidates that need production operations proof should also retain
`target/appfw/ops-certification.json` from
`scripts/appfw ops-certification --json`. Set
`APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true` and
`APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true` when live OTLP, readiness,
metrics, alert/routing, dashboard provisioning, and runbook-drill evidence are
required for promotion. Provide `APPFW_ALERTMANAGER_URL` to
`scripts/ci/live-ops-evidence.sh` so `alertmanager-status.json` proves a live
Alertmanager status API endpoint instead of only retained static routing config.
In that artifact, `ok:true` only proves the configured checks passed;
`release_ready:true` is the promotion signal. When `release_ready:false`, the
artifact retains `release_blockers` and remediation work items.
When `APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true`, the Bitbucket release
wrapper collects this evidence automatically before `release-check`, requires
`ops-certification`, and verifies the same posture in the strict release
evidence check. Direct `release-evidence-check` usage also treats live-ops
evidence as requiring a release-ready `ops-certification.json` generated with
`live_evidence_required:true`. Set `APPFW_OPS_BASE_URL` if the operations proof
should target a runtime other than the wrapper's PostgreSQL-backed instance.

PDS production-readiness proof should also retain
`target/appfw/pds-security-baseline.json` from
`scripts/appfw pds-baseline --json`. The Bitbucket release wrapper defaults
`APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true`; the strict release evidence
check then requires `release_ready:true` and the baseline decision artifact
referenced by `APPFW_PDS_BASELINE_DECISION_FILE`. The retained decision must
include release-authority owner and approver fields plus an ISO-8601 approval
timestamp with timezone that is not in the future. Set
`APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=false` only for an explicit
non-production dry run that does not claim PDS production readiness.

Release candidates that need performance proof should retain
`target/appfw/load-test.json`, `target/appfw/load-tests/*.json`, and
`target/appfw/provider-performance.json` from
`scripts/appfw load-test-suite --json` and
`scripts/appfw provider-performance --json --all`. The Bitbucket release wrapper
defaults `APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true` and sets
`APPFW_LOAD_TEST_URL` to the PostgreSQL-backed CRM API it starts. This makes
`scripts/appfw release-check --json` run the load suite and provider-performance
certification, then fail promotion when HTTP failures,
`APPFW_LOAD_TEST_MAX_ERROR_RATE`, `APPFW_LOAD_TEST_MAX_P95_MS`,
`APPFW_LOAD_TEST_MAX_MAX_MS`, or provider performance contracts are violated.
Strict performance release evidence requires the generated load-test suite and
live-certified provider-performance entries for PostgreSQL, MongoDB, MS SQL
Server, and Snowflake; a single `load-test` artifact does not satisfy
enterprise promotion.
Set `APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=false` only for an explicit
non-production dry run that does not start a backend and produce provider parity
evidence before `release-check`.

## Observability

Every deployed app should expose or export:

- `/health/live` for process liveness.
- `/health/ready` for provider readiness.
- structured logs with request and correlation IDs.
- OpenTelemetry traces and metrics when enabled.
- provider timing metrics, slow-query events, query counts, and result counts.

Minimum retained operations evidence for production review:

- a live OTLP trace and metric sample observed by the collector,
- `/health/ready` output for each provider lane,
- `/metrics` and `/metrics.json` snapshots,
- Prometheus target and rule API output,
- Alertmanager receiver/routing status,
- Grafana dashboard provisioning or immutable dashboard links,
- a runbook drill record with owner, timestamp, scenario, outcome, and follow-up.

When logs can contain PHI, treat the observability platform as a regulated data
system. Minimize at source first: do not log request bodies, tenant records, raw
SQL parameters, provider result payloads, credentials, tokens, or PHI. Collector
redaction is defense in depth, not the primary control.

The recommended Grafana stack is:

- Alloy or an OpenTelemetry collector deployed by the platform, scoped to the
  app namespace or workload identity.
- Loki with tenant isolation enabled and `X-Scope-OrgID` supplied by an
  authenticating reverse proxy or gateway.
- Grafana behind SSO, MFA, RBAC, and audit logging.
- TLS or mTLS between collectors, gateways, stores, and Grafana.
- Encrypted-at-rest object storage or volumes with explicit retention and delete
  policy.
- Low-cardinality Loki labels only; request IDs, correlation IDs, and user or
  tenant identifiers must not become indexed labels.

Admin troubleshooting endpoints must stay disabled unless an environment has an
approved operational need and redaction tests are passing. GraphQL
introspection is enabled by default only for `ENV_NAME=local`; managed
environments should keep `APP_GRAPHQL_INTROSPECTION_ENABLED=false` unless an
approved admin/developer role or scope gate is configured and release evidence
covers that posture.

Chat and AI answer surfaces are also opt-in. Keep `APP_CHAT_ENABLED=false`
until the Wave 4 chat route, answer-envelope, AI-search, prompt-audit, and
evaluation evidence lanes have merged. The first chat route shell is an SSE
bootstrap stream and still requires product wiring to install a
`ChatPromptAuditSink`; environment flags alone are not enough to process chat
traffic. If `APP_CHAT_ENABLED=true`, runtime
safety validation fails closed unless at least one chat role or scope gate is
configured, prompt audit is enabled, SIEM export posture is enabled, a
non-secret audit sink name is configured, prompt/audit retention is at least
90 days, and `APP_CHAT_KILL_SWITCH_ACTIVE=false`. Setting
`APP_CHAT_KILL_SWITCH_ACTIVE=true` disables chat startup even when the rest of
the chat configuration is present; use it as the emergency operations stop
while retaining the full audit posture in managed environments.

MCP is also opt-in and is excluded from the current release posture. Keep
`APP_MCP_ENABLED=false` in release pipelines until MCP capability manifests,
per-tool authorization, token audience/resource checks, mandatory audit
evidence, and HITL evidence are release-gated. Enable `APP_MCP_ENABLED=true`
only for non-release environments where agentic clients are approved,
authenticated, covered by origin controls, and authorized through
`APP_MCP_REQUIRED_ROLES` or `APP_MCP_REQUIRED_SCOPES`. The current MCP surface
is read-only and exposes generated public handler operations directly as tools.
It still runs through the same access policy, tenant isolation, QueryIR limits,
redaction, entity audit for audited entities, and provider metrics as GraphQL.
Keep `APP_MCP_MAX_BATCH_ITEMS`, `APP_MCP_MAX_RESULT_BYTES`, and
`APP_MCP_MAX_RESOURCE_BYTES` aligned with the environment's agent risk profile.

## Deployment Handoff

Every release handoff should include:

```bash
scripts/appfw handoff --json
scripts/appfw upgrade --json
scripts/appfw migrate rollback-guide --json
```

The handoff should identify generated drift, app-owned changes, migration phase
selection, skipped live tests, and any provider-specific limitations.
