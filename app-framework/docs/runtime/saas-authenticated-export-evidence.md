# SaaS Authenticated Export Evidence

> **Status: evidence request checklist.** Use this page when a product team asks
> for ServiceNow or iCIMS provider work. These vendors have metadata-only,
> evidence-gated skeletons, but stay request-execution blocked until
> authenticated exports, redacted samples, and security matrices are attached to
> the handoff package.

Read this with [SaaS Connectors](saas-connectors.md),
[SaaS Connector Certification](saas-certification.md), and the
[SaaS API Intelligence Snapshot](saas-api-intel-2026-06-26.md). The local
2026-06-26 vendor evidence says ServiceNow public pages are insufficient for
instance schemas, and iCIMS detailed API docs are access-gated. Do not fill
those gaps with inferred provider contracts.

## Shared Rules

- Do not commit raw vendor exports, screenshots with tenant data, access tokens,
  passwords, client secrets, hostnames that identify a real customer, or
  unredacted response bodies.
- Store credentials only in the runtime secret mechanism. Evidence documents may
  name required secret variables, but must never contain values.
- Redact or synthesize tenant identifiers, usernames, emails, phone numbers,
  candidate/person IDs, incident numbers, `sys_id` values, request IDs, and
  custom field values before retaining samples.
- Keep every proposed read behind a named-operation registry. Callers must not
  choose table names, field lists, raw encoded queries, REST paths, or custom
  search filters.
- Keep writes `Unsupported` unless a later named mutation has separate live
  write-safety evidence, audit/redaction, idempotency behavior, tenant binding,
  and exposure gates.
- Leave `RateLimitBackoff`, `PaginationCursoring`, `TenantScoping`, and other
  `SaasReadArea` entries below `LiveCertified` until live evidence actually
  runs and passes.

## ServiceNow Request

ServiceNow provider work requires an authenticated non-production instance
export. Public docs confirm the Table API shape to inspect, but table fields,
ACLs, plugins, domain separation, and OpenAPI output are tenant-specific.

Request this package from the product or platform owner:

- [ ] Instance family/version, target module scope, and selected non-production
      instance URL, redacted to a stable placeholder.
- [ ] Auth method summary for the integration path, preferably OAuth client
      credentials with an integration user/application registry. If a temporary
      live-smoke setup uses basic auth, record the exception and expiry.
- [ ] Integration user roles, group memberships, ACL expectations, data
      policies, and domain-separation posture.
- [ ] Enabled plugin/module inventory for the requested scope, such as ITSM,
      ESM, SPM, EA, CMDB, HRSD, or SecOps.
- [ ] REST API Explorer OpenAPI export for Table API and the selected tables.
- [ ] Table dictionary/schema exports for selected tables, including field
      names, types, references, display labels, readable fields, required roles,
      and custom fields.
- [ ] Redacted sample list and get responses for each selected table, including
      response headers and representative empty, denied, missing-table, and
      missing-plugin cases.
- [ ] Pagination evidence for the intended strategy: offset/limit behavior or a
      keyset-like `sys_updated_on` plus `sys_id` continuation.
- [ ] Incremental sync evidence for `sys_updated_on`, including ordering,
      timezone representation, and same-timestamp tie-break behavior.
- [ ] Rate-limit or throttle evidence, including any `429`, `Retry-After`, or
      instance policy documentation that the tenant can safely provide.
- [ ] Error examples that distinguish auth failure, role/ACL denial, missing
      table, missing plugin, malformed query, and transient platform failure.

Candidate reads after this evidence exists:

- `servicenow.incidents_updated_since`
- `servicenow.incident_by_sys_id`
- `servicenow.problems_updated_since`
- `servicenow.change_requests_updated_since`
- `servicenow.cmdb_ci_updated_since`

Implementation gates:

- Operation registry owns the table name, fixed `sysparm_fields`, display-value
  mode, reference-link mode, caps, timeout, and allowed predicates.
- Provider code builds encoded-query fragments from typed parameters; callers
  never pass raw `sysparm_query`.
- Schema drift or missing ACLs fail closed and appear as provider readiness or
  certification evidence, not as silent field drops.

## iCIMS Request

iCIMS provider work requires authenticated Developer Community API docs or a
customer-approved export. Public docs confirm REST API and repeatable
integration guidance, but they do not establish auth, pagination, rate limits,
entity schemas, or field-level behavior.

Request this package from the product or recruiting-platform owner:

- [ ] Authenticated docs or exported pages for connecting/authenticating,
      Applicant Tracking APIs, relevant profile/data model pages, error
      handling, pagination, and rate limits.
- [ ] Confirmation of integration lane: partner-marketplace product,
      customer-specific integration, or internal customer tenant integration.
- [ ] Sandbox or non-production tenant availability. If none exists, record the
      customer-approved evidence substitute and the live-certification limit.
- [ ] Auth method, credential ownership model, package IDs, customer ID/base URL
      binding, IP allowlist requirements, and token/secret rotation policy.
- [ ] Profile/entity field matrix for candidates/people, jobs/requisitions,
      applications/submissions, workflow/status, offers, and onboarding tasks as
      applicable.
- [ ] Field classification by standard versus custom, read-only versus
      read/write, required versus optional, and repeatable connector field
      versus tenant extension.
- [ ] Redacted sample GET/search responses, headers, pagination continuations,
      empty results, denied access, validation errors, and rate-limit or
      throttle examples.
- [ ] Release-note or API-version evidence behind login for the target tenant or
      Developer Community version.
- [ ] Marketplace validation or revalidation requirements if the integration is
      intended for repeatable customer distribution.

Candidate reads after this evidence exists:

- Candidate/profile incremental reads using approved Applicant Tracking profile
  types and standard fields.
- Job/requisition incremental reads using approved standard fields.
- Application/submission and workflow-status reads after schema, pagination, and
  lifecycle semantics are explicit.

Implementation gates:

- Do not implement provider auth or named operations until authenticated API
  docs or exported specs are attached.
- Keep custom fields as tenant-specific extension metadata unless a repeatable
  standard field contract exists.
- Treat candidate and application data as PII by default; logs, metrics, dead
  letters, and handoff evidence must use redacted identifiers or hashes.

## Backlog Routing

Until the evidence packages above exist, route implementation work as follows:

| Lane | Routing |
| --- | --- |
| Salesforce | Code-bearing Wave 1 reference connector remains first. Continue named reads, M2M auth, pagination, redaction, and materialized projection evidence. |
| Workday | Code-bearing skeleton may proceed from the local WWS/WSDL evidence, with tenant auth and live samples still required before live certification. |
| ServiceNow | Metadata-only evidence-gated skeleton. Accept authenticated instance exports, table dictionaries, ACL/plugin matrices, and redacted samples before request-planning or capability claims. |
| iCIMS | Metadata-only evidence-gated skeleton. Accept authenticated Developer Community/API docs, field matrices, sandbox or tenant export evidence, and marketplace requirements before request-planning or capability claims. |

Provider branches that discover a new ServiceNow or iCIMS contract requirement
should update this checklist and the SaaS certification handoff notes before
adding runtime code.
