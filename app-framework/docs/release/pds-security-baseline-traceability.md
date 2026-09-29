# PDS Security Baseline Traceability

This guide maps the PDS Health IT Security Baseline Standard r4.5 into the App
Framework release-readiness model.

The source baseline is an internal PDS document provided to the assessment as a
local PDF: `PDS IT Security Baseline Standard r4.5.pdf`. The source PDF should
not be committed to this repository unless the document owner approves that
distribution. Treat this page as release traceability, not as a replacement for
the authoritative baseline.

## Source Metadata

| Field | Value |
| --- | --- |
| Standard | PDS Health IT Security Baseline Standard |
| Version | r4.5 |
| Approval date | 2025-11-08 |
| Publish date | 2025-11-20 |
| Next review date | 2026-11-08 |
| Release-readiness impact | Required for PDS production-readiness claims |

## Release Posture

For a PDS production release, `scripts/appfw framework release-check --json`
is necessary but not sufficient. The release bundle must also show that PDS
baseline controls are either:

- enforced by framework code or generated product configuration;
- proven by retained CI/CD, IaC, deployment, identity, observability, or
  security-tooling evidence; or
- covered by an approved, time-boxed risk exception from the required authority.

The live-environment work items are tracked in
[`live-environment-work-items.md`](live-environment-work-items.md). Repository
changes should keep the commands and artifact contracts below stable so CI/CD
and platform teams can produce the missing evidence without inventing a parallel
release process.

In this repo, `LIVE-*` IDs are release-gate checkpoint IDs. They are not PDS
baseline section numbers. The prefix means the item needs live-environment or
release-authority proof; the number gives automation a stable identifier for
failures, remediation lists, and release evidence checks. PDS reviewers should
read the bundle by the baseline section names in `baseline_sections`; the
release gate uses the `LIVE-*` IDs to make sure every required proof bucket is
covered.

Machine-readable baseline posture is emitted by:

```bash
scripts/appfw framework pds-baseline --json
```

The command writes `target/appfw/pds-security-baseline.json`. In production
release lanes, set `APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true` and
provide `APPFW_PDS_BASELINE_DECISION_FILE` with a JSON decision artifact that
declares `ok:true` and `release_ready:true`. The decision artifact must also
map every PDS work item, LIVE-015 through LIVE-021, to retained evidence or to
an approved risk exception. Additional retained evidence files can be listed in
`APPFW_PDS_BASELINE_EVIDENCE_FILES`. The generated
`pds-security-baseline.json` records retained decision and evidence artifacts
with SHA-256 digests and byte sizes; the final release evidence gate recomputes
those values before promotion. Risk exceptions must expire within the retained
`risk_acceptance.max_days` policy, which defaults to 365 days and can be
tightened, but not raised above 365, with
`APPFW_PDS_BASELINE_RISK_ACCEPTANCE_MAX_DAYS`.

Use [`pds-baseline-evidence-kit.md`](pds-baseline-evidence-kit.md) and
[`templates/pds-baseline-decision.template.json`](templates/pds-baseline-decision.template.json)
to prepare the release-authority bundle. The template is intentionally
non-release-ready; it must be copied into the release artifact directory and
filled with real evidence or risk acceptances before the live-required gate can
pass.

## Decision Artifact Schema

Production release lanes must provide a decision artifact with this shape:

```json
{
  "ok": true,
  "release_ready": true,
  "baseline": {
    "version": "r4.5"
  },
  "release_authority": {
    "owner": "release owner or team",
    "approver": "security or release approver",
    "approved_at_utc": "2026-06-11T00:00:00Z"
  },
  "work_items": [
    {
      "id": "LIVE-015",
      "baseline_sections": ["Exceptions and risk acceptance"],
      "status": "evidence",
      "evidence_files": ["target/appfw/pds-baseline/live-015.json"]
    },
    {
      "id": "LIVE-016",
      "baseline_sections": [
        "Human IAM and SSO",
        "User lifecycle management",
        "Authorization / least privilege"
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
  ]
}
```

The command accepts either `work_items` or `categories`. Entries may identify a
single work item with `id` or `work_item_id`, or multiple work items with
`work_item_ids`. `approved_at_utc` must be an ISO-8601 timestamp with timezone
and must not be in the future. A status of `evidence` requires at least one
existing evidence file. A status of `risk-accepted` requires owner, approver,
scope, unexpired `expires_on`, rationale, compensating controls, and follow-up.
For a production-ready bundle, the decision file and every evidence file must
resolve inside the release artifact directory. Relative evidence paths are
resolved against the release artifact directory first, then the decision-file
directory and repository root; paths outside the artifact directory are rejected
as non-retained evidence. Retained evidence files must also be non-empty.

## Traceability Matrix

| Baseline area | Repo-supported controls | Required release evidence | Current gap / work item |
| --- | --- | --- | --- |
| Encryption in transit and at rest | Data-source TLS validation, fail-closed managed TLS guidance, security headers, secret redaction | TLS 1.2+ endpoint proof, approved cipher posture, certificate inventory, database/storage encryption evidence, encrypted backup evidence | Live environment; see LIVE-018, LIVE-019, LIVE-020 |
| Lower-environment data | Product contract forbids committed PHI/ePHI/secrets; PHI log lint | Evidence that lower environments use synthetic or de-identified data, or an approved security risk exception with production-equivalent controls | Live environment; see LIVE-019 |
| Human IAM and SSO | Okta/JWT flow documented for product frontends and runtime auth; role/policy enforcement is generated and tested | Okta OIDC/SAML configuration proof, MFA policy proof for internet-facing and privileged access, no shared accounts, business owner assignment | IAM/deployment evidence; see LIVE-016 |
| User lifecycle management | Product identity is centralized at runtime boundaries | SCIM support proof or documented user lifecycle APIs/automation for provisioning, updates, and deprovisioning | Architecture plus IAM evidence; see LIVE-016 |
| Authorization / least privilege | Deny-by-default policy, tenant isolation, access filters, generated role contracts, provider security tests | Live tenant/IDOR/denied-mutation evidence across certified providers; role/group access matrix and owner approval | Partly repo, live proof required; see LIVE-002, LIVE-016 |
| Non-human identities and service accounts | Secret values are kept out of manifests; provider credentials are runtime secrets | Dedicated service accounts per integration, minimum necessary privileges, naming/ownership, rotation, no interactive use, access review evidence | Live IAM/secrets evidence; see LIVE-016, LIVE-017 |
| Secrets management | Secret scan, redaction checks, runtime secret-resolution guidance, no secret values in generated contracts | CyberArk or KMS distribution proof, dynamic secret posture where applicable, secret access/modification audit logs retained at least 90 days, Wiz or approved CD scan evidence | Live security/platform evidence; see LIVE-017 |
| Host, endpoint, and patch posture | Dependency and SCA gates; deployment guidance keeps host ownership outside generated product code | XDR/EDR/MDM proof, authorized software evidence, OS/app patch compliance, EOL exception evidence, 30-day High/Critical vulnerability remediation tracking | Platform/IaC evidence; see LIVE-020 |
| Database security | Provider TLS modes, least-privilege connection guidance, provider parity tests | CIS Level 1 hardening evidence, database service-account proof, encrypted backups, patch/EOL compliance, least-privilege database access evidence | Platform/provider environment; see LIVE-019, LIVE-020 |
| Backup and restore | Deployment reference names release/deployment evidence paths | Encrypted backups, retention policy, RPO/RTO, restore test or DR drill evidence before go-live | Platform/SRE evidence; see LIVE-019 |
| Security monitoring and SIEM | Structured logs, audit events, audit hash chains, redaction, ops-certification | SIEM export proof, retention proof, alert routing, service-account change monitoring, forensic log availability | Live ops/security evidence; see LIVE-005, LIVE-012, LIVE-021 |
| Internet-facing service gateway | HSTS/security header support, CORS/CSP posture, runtime route hardening | Firewall/WAF policy proof, deny-all logging, threat profiles, source/destination least privilege, true source attribution through proxy/NAT | Network/IaC evidence; see LIVE-018 |
| Application security scanning | Security-assurance decision gate, SCA, secret scan, PHI log lint, local DAST/SAST placeholders | Enterprise DAST and SAST artifacts with Critical/High findings remediated before production, OWASP API Top 10 coverage | Security tooling; see LIVE-007, LIVE-018 |
| API security | JWT/OIDC runtime auth, bounded requests, rate limits, QueryIR validation, policy enforcement, audit | Okta-generated token posture, 24-hour token rotation where applicable, API inventory, API security tooling onboarding, gateway/WAF routing | IAM/security/platform evidence; see LIVE-016, LIVE-018 |
| Cloud controls | Deployment reference separates repo config from platform-owned cloud controls | IdP integration, SIEM integration, restricted admin access, network security controls, SaaS metadata-only posture for sensitive data | IaC/platform evidence; see LIVE-016, LIVE-020, LIVE-021 |
| Mobile application controls | Not a current framework release surface unless a product ships mobile artifacts | If mobile is in release scope: TLS, Okta/SSO/MFA, platform keystore/keychain, session timeout, MDM wipe, certificate pinning, security approval | Scope-dependent; add product work item before mobile release |
| Exceptions and risk acceptance | Security-assurance decision supports formal risk acceptance artifacts | Owner, approver, scope, expiry, rationale, compensating controls, and follow-up plan for every deferred baseline category | Release authority evidence; see LIVE-015 |

## Repo Actions

The repo can help by keeping these contracts explicit:

- `release-check.json` must expose `release_ready`, `failure_summary`, and
  remediation work items when live evidence is missing.
- `pds-security-baseline.json` must expose `release_ready`,
  `failure_summary.root_causes`, and baseline remediation work items.
- Security-assurance and release-evidence checks must continue to reject local
  placeholders when production attestations are required.
- Product docs must continue to state that Okta/IdP, SIEM, CyberArk/KMS, WAF,
  backup/restore, host hardening, and managed cloud controls are deployment
  responsibilities, not generated source-code defaults.
- Any product starter or generated template that stores, processes, or displays
  PHI/PII/PCI must preserve deny-by-default policy, tenant isolation, redacted
  logging, and synthetic lower-environment data guidance.

## Release Authority Actions

Before claiming PDS production readiness, release authority must retain one
evidence bundle that includes:

- a baseline traceability decision for every row above;
- live provider and live security certification artifacts;
- IdP/IAM, SCIM or lifecycle, and MFA evidence;
- CyberArk/KMS and secret audit evidence;
- gateway/WAF/TLS/certificate/API inventory evidence;
- SIEM/export/retention and runbook evidence;
- vulnerability SLA and High/Critical remediation evidence;
- backup/restore and DR evidence; and
- any approved risk exceptions with expiry and follow-up.
