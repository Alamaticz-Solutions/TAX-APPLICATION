# PDS Baseline Evidence Kit

This guide converts the PDS Security Baseline r4.5 blocker into a retained
release evidence bundle. It is intentionally separate from the baseline
traceability page: traceability proves the repo knows what must be shown, while
this kit describes how release authority supplies the live decision and proof.

Do not commit live evidence, screenshots, tenant data, secrets, tokens, or
environment-specific access exports. Retain the filled bundle in the release
artifact directory or the approved CI artifact store.

## Bundle Contract

Start from the fail-safe template:

```text
docs/release/templates/pds-baseline-decision.template.json
```

Copy it into the release artifact directory as:

```text
target/appfw/pds-baseline-decision.json
```

The template is deliberately not release-ready. It keeps `ok:false`,
`release_ready:false`, placeholder approval metadata, and `status:"todo"` so it
cannot accidentally satisfy the production gate. Release authority must replace
those placeholders before running the live-required gate.

## About the `LIVE-*` IDs

`LIVE-*` IDs are App Framework release-gate checkpoint IDs. They are not section
numbers from the PDS Health IT Security Baseline Standard.

The prefix means "this item needs live-environment or release-authority proof."
The number gives automation a stable handle for failures, remediation lists,
and release evidence checks. PDS reviewers should read the bundle by the
baseline section names in `baseline_sections`; the release gate uses the
`LIVE-*` IDs to make sure every required proof bucket is covered.

Each PDS checkpoint entry, `LIVE-015` through `LIVE-021`, must be changed to one
of these forms:

```json
{
  "id": "LIVE-016",
  "baseline_sections": [
    "Human IAM and SSO",
    "User lifecycle management",
    "Authorization / least privilege",
    "Non-human identities and service accounts",
    "API security",
    "Cloud controls"
  ],
  "status": "evidence",
  "evidence_files": [
    "target/appfw/pds-baseline/live-016-iam-okta-mfa-lifecycle.json"
  ]
}
```

or:

```json
{
  "id": "LIVE-016",
  "baseline_sections": [
    "Human IAM and SSO",
    "User lifecycle management",
    "Authorization / least privilege",
    "Non-human identities and service accounts",
    "API security",
    "Cloud controls"
  ],
  "status": "risk-accepted",
  "risk_acceptance": {
    "owner": "risk owner",
    "approver": "risk approver",
    "scope": "affected systems and controls",
    "expires_on": "2026-12-31",
    "rationale": "why the control is deferred",
    "compensating_controls": "temporary controls in force",
    "follow_up": "tracked follow-up work item"
  }
}
```

Risk acceptances must be unexpired and within the configured maximum window,
which defaults to 365 days.

## Baseline Section Crosswalk

Use this table when collecting evidence from PDS security, IAM, platform,
network, SRE, database operations, and release authority. The `LIVE-*` IDs are
the release-gate work items; the baseline section names are the reviewer-facing
labels from the PDS Health IT Security Baseline Standard r4.5 traceability map.

| PDS baseline section | Decision item | Evidence bundle file |
| --- | --- | --- |
| Exceptions and risk acceptance | LIVE-015 | `live-015-release-authority.json` |
| Human IAM and SSO | LIVE-016 | `live-016-iam-okta-mfa-lifecycle.json` |
| User lifecycle management | LIVE-016 | `live-016-iam-okta-mfa-lifecycle.json` |
| Authorization / least privilege | LIVE-016 | `live-016-iam-okta-mfa-lifecycle.json` |
| Non-human identities and service accounts | LIVE-016, LIVE-017 | `live-016-iam-okta-mfa-lifecycle.json`, `live-017-secrets-management.json` |
| Secrets management | LIVE-017 | `live-017-secrets-management.json` |
| Encryption in transit and at rest | LIVE-018, LIVE-019, LIVE-020 | `live-018-api-gateway-waf-tls-scanning.json`, `live-019-lower-env-backup-dr.json`, `live-020-platform-network-db-hardening.json` |
| Internet-facing service gateway | LIVE-018 | `live-018-api-gateway-waf-tls-scanning.json` |
| Application security scanning | LIVE-018 | `live-018-api-gateway-waf-tls-scanning.json` |
| API security | LIVE-016, LIVE-018 | `live-016-iam-okta-mfa-lifecycle.json`, `live-018-api-gateway-waf-tls-scanning.json` |
| Lower-environment data | LIVE-019 | `live-019-lower-env-backup-dr.json` |
| Backup and restore | LIVE-019 | `live-019-lower-env-backup-dr.json` |
| Database security | LIVE-019, LIVE-020 | `live-019-lower-env-backup-dr.json`, `live-020-platform-network-db-hardening.json` |
| Host, endpoint, and patch posture | LIVE-020 | `live-020-platform-network-db-hardening.json` |
| Cloud controls | LIVE-016, LIVE-020, LIVE-021 | `live-016-iam-okta-mfa-lifecycle.json`, `live-020-platform-network-db-hardening.json`, `live-021-siem-monitoring-access-review.json` |
| Security monitoring and SIEM | LIVE-021 | `live-021-siem-monitoring-access-review.json` |
| Mobile application controls | Scope-dependent | Add a product-specific retained evidence file or explicit out-of-scope decision before a mobile release. |

The decision file should keep the `baseline_sections` array on each work item,
even though the gate keys promotion on the `LIVE-*` IDs. That makes the retained
bundle auditable both ways: by release blocker and by PDS baseline section.

## Expected Evidence

| Work item | Baseline sections | Evidence owner | Evidence to retain |
| --- | --- | --- | --- |
| LIVE-015 | Exceptions and risk acceptance | Release authority + security | Final `pds-baseline-decision.json`, baseline r4.5 approval record, evidence index, and any risk-acceptance artifacts for deferred controls. |
| LIVE-016 | Human IAM and SSO; user lifecycle management; authorization / least privilege; non-human identities; API security; cloud controls | IAM + security + platform | Okta OIDC/SAML configuration proof, MFA policy proof, SCIM or lifecycle API evidence, role/group access matrix, admin and vendor access-review evidence. |
| LIVE-017 | Secrets management; non-human identities | Security + platform | CyberArk or KMS configuration proof, dynamic secret evidence where applicable, secret access/modification audit logs, Wiz or approved CD scan evidence. |
| LIVE-018 | Encryption in transit; internet-facing gateway; application security scanning; API security | Network + platform + security | WAF/firewall policy proof, TLS/certificate evidence, API inventory/security-tooling evidence, SAST/DAST artifacts with Critical/High findings remediated or risk-accepted. |
| LIVE-019 | Lower-environment data; backup and restore; database security; encryption at rest | SRE/platform + data owners + security | Synthetic or de-identified lower-environment data proof, encrypted backup evidence, retention policy, RPO/RTO statement, restore test or DR drill artifact. |
| LIVE-020 | Host, endpoint, and patch posture; cloud controls; database security; network hardening | Platform + network + database operations | CIS hardening evidence, XDR/EDR/MDM evidence, patch/EOL report, database service-account proof, firewall/traffic-flow evidence, vulnerability SLA evidence. |
| LIVE-021 | Security monitoring and SIEM; cloud controls; access reviews | Security operations + SRE + IAM | SIEM export evidence, retention proof, alert routing, service-account change monitoring, privileged/admin access reviews. |

Evidence files may be JSON summaries, redacted exports, PDFs, signed attestations,
or links retained by the approved artifact system. If a file contains only a
link to an external system, it must also include enough metadata for later
review: system name, record URL or identifier, owner, approver, collection time,
scope, and hash or immutable version when available.

## Validation Command

Before a branch is pushed for release review, run the local ready-shape preflight:

```bash
scripts/ci/pds-baseline-ready-fixture.sh
```

This writes a synthetic bundle under
`target/appfw/pds-ready-fixture/` and runs the real PDS baseline gate with
`APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true`. The resulting
`target/appfw/pds-ready-fixture/pds-security-baseline.json` must show
`ok:true`, `release_ready:true`, `live_evidence_required:true`, zero failures,
all LIVE-015 through LIVE-021 items covered, and retained decision/evidence
artifacts with SHA-256 digests and byte sizes. This is a structural CI
readiness check only; it is synthetic and is not release-authority approval.

Run the PDS baseline gate in live-required mode:

```bash
APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true \
APPFW_PDS_BASELINE_DECISION_FILE=target/appfw/pds-baseline-decision.json \
scripts/appfw framework pds-baseline --json
```

For production release promotion, the resulting
`target/appfw/pds-security-baseline.json` must show:

```json
{
  "ok": true,
  "release_ready": true,
  "live_evidence_required": true
}
```

It must also include non-empty `decision_summary.covered_work_items` for every
LIVE-015 through LIVE-021 item and retained `decision_artifact` /
`evidence_artifacts` entries with SHA-256 digests and byte sizes.

## What This Does Not Prove

The repository can validate schema, retained paths, hashes, byte sizes, expiry
windows, and work-item coverage. It cannot independently prove that a screenshot,
attestation, access export, or policy document is truthful and approved. That
semantic decision remains with release authority, security, IAM, platform,
network, SRE, and database operations.
