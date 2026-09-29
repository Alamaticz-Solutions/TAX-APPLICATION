# Live Environment Work Items

These work items track enterprise-readiness tasks that cannot be completed
inside this repository alone. They require managed infrastructure, CI/CD
ownership, observability systems, security tooling, release authority, or
production-like credentials.

Repo-side code, command, and documentation changes should make these items easy
to run and review. The live environment owns the retained evidence.
Before handing provider/runtime/release-gate work to CI, repo-side changes
should also pass `scripts/appfw framework local-live-preflight --json`. That
local artifact proves branch-level CI readiness; it does not close these live
environment items or replace release-authoritative evidence.

The `LIVE-*` IDs are App Framework checkpoint IDs for release automation. They
are not PDS baseline section numbers. For PDS Security Baseline r4.5 evidence,
use the section names in
[`pds-baseline-evidence-kit.md`](pds-baseline-evidence-kit.md) when collecting
or reviewing proof; the `LIVE-*` IDs only keep the release gate stable.

## P0 Release Blockers

| ID | Work Item | Owner | Required Evidence | Acceptance Criteria |
| --- | --- | --- | --- | --- |
| LIVE-001 | Run provider-backed release certification with one backend URL per provider. | CI/CD + platform | `target/appfw/release-check.json`, `target/appfw/provider-parity.json`, `target/appfw/provider-test-*.log`, `api_tests/target/provider-parity.json`; branch preflight may also retain `target/appfw/local-live-release-preflight.json` | `scripts/appfw framework release-check --json` is `ok:true`; `API_TEST_BASE_URL_POSTGRES`, `API_TEST_BASE_URL_MONGO`, `API_TEST_BASE_URL_MSSQL`, and `API_TEST_BASE_URL_SNOWFLAKE` are populated by the release lane; every `LiveCertified` provider area reports `passed`. Local `local-live-preflight --json` is expected before PR, but only the CI/CD release lane closes LIVE-001. |
| LIVE-002 | Produce live security certification evidence from the provider-backed release run. | Security + CI/CD | `target/appfw/security-certification.json`, provider logs referenced by that artifact | Tenant isolation, locator/IDOR negatives, denied mutation access filters, denied-error normalization, and audit redaction/chain contracts pass for PostgreSQL, MongoDB, MS SQL Server, and Snowflake. |
| LIVE-003 | Run strict supply-chain evidence in an environment with writable advisory/cache paths and network access where required. | CI/CD + security | `target/appfw/dependency-check.json`, SBOM artifacts, `target/appfw/supply-chain-gate.json`, `target/appfw/release-evidence-check.json` | `scripts/ci/supply-chain-gate.sh` and strict dependency check are `ok:true`; `APPFW_DEPENDENCY_CHECK_CARGO_HOME` is either set to a writable cache volume or the artifacts record a writable fallback; cargo-deny, cargo-audit, npm audit, OSV, secret scan, PHI log lint, and SBOM generation are either green or formally risk-accepted. |
| LIVE-004 | Cut the first release candidate through the `v*` tag lane. | Release management + CI/CD | Bitbucket release artifact bundle, tag metadata, release notes, artifact hashes | A `v*` tag build runs the strict production release gate; `main` remains focused provider-backed CI evidence until release-authority artifacts are available. Release notes reference the build and retained artifact bundle. |
| LIVE-015 | Retain PDS Security Baseline r4.5 traceability for production-readiness claims. | Release authority + security | Structured `pds-baseline-decision.json`, [`pds-security-baseline-traceability.md`](pds-security-baseline-traceability.md), [`pds-baseline-evidence-kit.md`](pds-baseline-evidence-kit.md), retained evidence files, risk-acceptance artifacts for deferred controls | Every LIVE-015 through LIVE-021 baseline work item is mapped to retained evidence or to an approved, time-boxed risk exception with owner, approver, scope, rationale, compensating controls, and follow-up. |
| LIVE-016 | Prove PDS IAM, Okta, MFA, and user-lifecycle posture. | IAM + security + platform | Okta OIDC/SAML configuration proof, MFA policy proof, SCIM or lifecycle API evidence, role/group access matrix, admin and vendor access-review evidence | Internet-facing and privileged access use PDS-approved IdP/MFA; shared accounts are prohibited; SCIM or lifecycle APIs support provisioning/update/deprovisioning; admin/vendor access reviews are retained. |
| LIVE-017 | Prove PDS secrets-management posture. | Security + platform | CyberArk or KMS configuration evidence, dynamic secret evidence where applicable, secret access/modification audit logs, Wiz or approved CD scan evidence | Secrets are not written to disk or transmitted cleartext; non-human account secrets are distributed by an approved system; lifecycle and access events are immutable and retained for at least 90 days. |
| LIVE-018 | Prove PDS internet-facing API and gateway posture. | Network + platform + security | WAF/firewall policy evidence, TLS/certificate evidence, API inventory/security-tooling evidence, SAST/DAST artifacts | Internet-facing APIs route through approved gateway/WAF controls; TLS is 1.2+ with approved ciphers; certificates are within policy lifetime; OWASP API Top 10, SAST, and DAST Critical/High findings are remediated before production. |

## P1 Enterprise Evidence

| ID | Work Item | Owner | Required Evidence | Acceptance Criteria |
| --- | --- | --- | --- | --- |
| LIVE-005 | Collect live operations evidence when production ops proof is in scope. | SRE/platform | `target/appfw/ops-certification.json`, `live-otel-evidence.json`, `health-ready.json`, `metrics.prom`, `metrics.json`, `prometheus-targets.json`, `prometheus-rules.json`, `alertmanager-status.json`, `grafana-dashboard-provisioning.json`, `runbook-drill.json` | Release CI sets `APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true` and `APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true`; ops certification is `ok:true` and `release_ready:true`. |
| LIVE-006 | Make performance evidence blocking for release candidates that claim performance readiness. | Performance + CI/CD | `target/appfw/load-test.json`, `target/appfw/load-tests/*.json`, `target/appfw/provider-performance.json` | Release CI sets `APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true`; load-test thresholds and provider-performance live requirements pass. |
| LIVE-007 | Provide regulated security-assurance evidence or formal risk acceptance. | Security + release authority | `target/appfw/dast-evidence.json`, `target/appfw/sast-evidence.json` or `target/appfw/sast.sarif`, `target/appfw/asvs-traceability.json`, `target/appfw/release-provenance.intoto.jsonl`, `target/appfw/artifact-signing.json`, or `target/appfw/security-risk-acceptance.json` | `scripts/ci/security-assurance-decision.sh` is `ok:true`; production attestations are real artifacts unless an unexpired risk acceptance names the owner, approver, scope, rationale, compensating controls, and follow-up plan. |
| LIVE-008 | Retain deployable image provenance and image SBOM when a release promotes an image. | CI/CD + platform | Immutable image digest, `target/appfw/sbom-deployable-image.cdx.json`, promotion annotations | Image SBOM is not marked `not-applicable` for image-promoting releases; ArgoCD or deployment metadata references the same digest that passed the release gate. |
| LIVE-009 | Execute disposable downstream lifecycle proof in CI. | CI/CD + framework team | `target/appfw/golden-downstream.json`, downstream bootstrap JSON, validation/generate-check/test/handoff artifacts | CI runs `scripts/appfw framework golden-downstream --profile crm-sample --execute --json`, not just the metadata proof. |
| LIVE-010 | Operate PR release-lite approvals for provider/security regressions. | CI/CD + platform + security | `target/appfw/release-lite-guard.json`; provider backend startup logs, scoped `provider-parity.json`, scoped `security-certification.json`, or approved manual `release-check` evidence URL | Pull requests that touch provider/runtime/security-critical paths fail closed unless the PR pipeline carries `APPFW_RELEASE_LITE_EVIDENCE_URL`, `APPFW_RELEASE_LITE_APPROVER`, and `APPFW_RELEASE_LITE_REASON` for an approved manual `release-check` or scoped provider-backed release-lite run. |
| LIVE-019 | Prove lower-environment data, backup, restore, and DR posture. | SRE/platform + data owners + security | Synthetic/de-identified lower-environment data proof or risk exception, encrypted backup evidence, retention policy, RPO/RTO, restore or DR drill artifact | Lower environments do not use live PHI/PII/PCI unless an approved exception applies; backups are encrypted in transit and at rest; restore/DR proof is retained before go-live. |
| LIVE-020 | Prove host, cloud, network, and database hardening posture. | Platform + network + database operations | CIS hardening evidence, XDR/EDR/MDM evidence, patch/EOL report, database service-account proof, firewall/traffic-flow evidence | Hosts, databases, and cloud resources meet PDS patch, EOL, least-privilege, endpoint-protection, and network-restriction requirements; High/Critical vulnerabilities are tracked to the required remediation SLA or risk-accepted. |
| LIVE-021 | Prove PDS SIEM, monitoring, and access-review posture. | Security operations + SRE + IAM | SIEM export evidence, retention proof, alert routing, service-account change monitoring, privileged/admin access reviews | Security events, audit logs, service-account changes, and administrative activity are exported, monitored, retained, and reviewable in the approved PDS security operations system. |

## P2 Follow-Up

| ID | Work Item | Owner | Required Evidence | Acceptance Criteria |
| --- | --- | --- | --- | --- |
| LIVE-011 | Add live backend browser smoke for frontend-in-scope releases. | CI/CD + frontend | Frontend test JSON, Playwright report, accessibility evidence | Product frontend checks run against a live backend when release scope includes product UI behavior. |
| LIVE-012 | Retain additional SIEM/audit export evidence for regulated deployments beyond the PDS baseline minimum. | Security operations | Extended SIEM export proof, audit retention proof, runbook reference, product-specific regulatory mapping | Audit events are exported, redacted, retained, sampled, and reviewable for the product's regulated deployment profile after LIVE-021 baseline monitoring evidence is satisfied. |
| LIVE-013 | Record release authority decisions for deferred categories. | Release management | Risk acceptance artifact and issue tracker references | Every deferred evidence category has an owner, approver, expiry, compensating controls, and follow-up work item. |
| LIVE-014 | Select and approve the framework license for downstream consumers. | Legal + product leadership | `target/appfw/release-identity.json`, approved `LICENSE` text, release notes, package metadata updates, [`release-identity-evidence-kit.md`](release-identity-evidence-kit.md), retained `release-identity-decision.json` | A root `LICENSE` file exists before external distribution; package manifests and support docs reference the approved license consistently; the retained release identity decision declares `ok:true` and `release_ready:true` with owner, approver, timestamp, matching `v*` tag, release notes, and distribution artifacts. |

## Repo Contract

Repository changes should keep these work items executable by preserving:

- `scripts/appfw framework release-check --json`
- `scripts/appfw framework local-live-preflight --json`
- `scripts/appfw framework provider-test --all --json`
- `scripts/appfw framework security-certification --json`
- `scripts/appfw framework ops-certification --json`
- `scripts/appfw framework pds-baseline --json`
- `scripts/appfw product frontend-test --json`
- `scripts/appfw framework golden-downstream --json`
- `scripts/ci/supply-chain-gate.sh`
- `scripts/ci/release-evidence-check.sh`

If one of those commands changes, update this file, the CLI reference, and the
release evidence checks in the same change.
