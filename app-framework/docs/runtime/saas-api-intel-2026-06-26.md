# SaaS API Intelligence Snapshot (2026-06-26)

> **Status: research input.** This page distills the local
> `vendor_api_specs_2026-06-26/` corpus into implementation guidance for the
> App Framework SaaS connector plan. The raw vendor corpus is not framework
> source and should not be committed as connector code. Treat this page as
> evidence-backed planning material for provider skeletons, schema projections,
> sync descriptors, and live-certification requests.

Read this with [SaaS Connectors](saas-connectors.md) and
[SaaS Connector Certification](saas-certification.md). The conclusions below
preserve the v1 architecture decision: SaaS providers are `EXTERNAL_API`
providers, named-operation first, read-first, and outside database CRUD parity.

## Corpus

The snapshot was retrieved on 2026-06-26 and contains:

| Vendor | Captured scope | Machine-readable coverage | Planning status |
| --- | --- | --- | --- |
| Salesforce | Health Cloud plus REST API, Summer '26 / API 67.0 / docs 262.0 | Partial: metadata JSON, PDFs, REST article JSON, 350 Health Cloud article payloads | Strong enough for Salesforce v1 read skeleton and Health Cloud projection candidates; authenticated org describe/OpenAPI still required before live certification. |
| Workday | HCM Human_Resources WWS v46.1 / 2026R1 | Strong summary; raw WSDL/XSD is not present in this checkout | Strong enough for the existing Workday read skeleton around selected `Get_*` operations; operation-specific request-shape evidence, tenant auth, and live response samples still required before promoting additional operations. |
| ServiceNow | Australia public docs for Table API and REST API Explorer | Weak: public pages are Fluid Topics shells | Metadata-only evidence-gated skeleton exists. Do not build executable Table API requests or hard-code table catalogs without authenticated instance exports. |
| iCIMS | Public Talent Cloud / ATS developer community pages | Weak: detailed API docs and models are login-gated | Metadata-only evidence-gated skeleton exists. Do not build executable recruiting API requests until gated docs or exported specs are available. |
| Anaplan | Integration API v2 and Authentication API Apiary docs | Partial: rendered Apiary payloads | Network-free provider skeleton exists for server-bound model status/files reads and registry-gated read-request/chunk templates. Keep auth, watermark, rate-limit, metrics, freshness, and writes below certified until tenant evidence exists. |
| Oracle Fusion Financials | 26B OpenAPI 3.0 JSON | Strong: official OpenAPI, 1,391 paths | Network-free request-planning skeleton exists for a tiny allowlisted LOV surface. Keep auth, rate limits, watermarks, redaction, metrics, freshness, generic OpenAPI operations, and writes below certified until tenant evidence exists. |

Local evidence paths used:

- `vendor_api_specs_2026-06-26/README.md`
- `vendor_api_specs_2026-06-26/manifest.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/salesforce-health-cloud-catalog.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/workday-human-resources-catalog.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/servicenow-public-docs-summary.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/icims-public-docs-summary.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/anaplan-apiary-summary.json`
- `vendor_api_specs_2026-06-26/enrichment/extracted/oracle-financials-openapi-summary.json`

## Cross-Provider Findings

### Keep Named Operations As The Boundary

The corpus reinforces the architecture choice to avoid a generic `QueryIR` to
vendor-query compiler in v1.

- Salesforce supports SOQL and REST query endpoints, but safe use depends on
  fixed object names, fixed field sets, capped filters, and bound values.
- Workday WWS exposes hundreds of SOAP operations with typed request/response
  shapes. The right v1 surface is selected `Get_*` operations, not an arbitrary
  XML request builder.
- ServiceNow Table API likely uses `sysparm_query`, `sysparm_fields`, `limit`,
  and `offset`, but real table dictionaries and ACLs are instance-specific.
- iCIMS public docs confirm REST/API availability but not enough endpoint,
  schema, or auth detail for a safe operation registry.
- Oracle and Anaplan show that later providers may have huge OpenAPI catalogs or
  asynchronous read-request flows; both still fit named operations better than a
  framework-wide arbitrary call surface.

### Watermark Support Exists, But Differs By Vendor

Use provider-specific watermark adapters behind the common sync descriptor
model. The base `.appfw/model/sync/*.yaml` descriptor contract is now validated
before generation; provider-specific watermark adapter modes remain a follow-on
contract layer.

| Vendor | Watermark candidate | Confidence |
| --- | --- | --- |
| Salesforce | REST/SOAP object support for `getUpdated()` / `getDeleted()` on many objects, plus SOQL predicates over update timestamps where object describe confirms fields. | High for standard/queryable objects; needs org describe before live use. |
| Workday | WWS request filters include `Updated_From`, `Updated_Through`, `As_Of_Effective_Date`, and `As_Of_Entry_DateTime`; response filters include page/count semantics. | High for selected operations; needs operation-specific request shape tests. |
| ServiceNow | Likely `sys_updated_on` plus `sys_id` tie-breaker in `sysparm_query`. | Medium; must be confirmed from target instance/export. |
| iCIMS | Unknown from public corpus. | Low; requires gated docs or exported examples. |
| Anaplan | Async read requests and task/request IDs are visible; conventional entity watermark semantics are not established. | Candidate only. |
| Oracle | `q`, `limit`, `offset`, `orderBy`, and `finder` are common; no universal watermark can be inferred from the summary. | Candidate only; resource-specific. |

### Governance Is A First-Class Connector Input

All supported-provider candidates can expose regulated or confidential data.
The sync descriptor and schema projection must declare classification before
materialization:

- Salesforce Health Cloud contains PHI-oriented health, coverage, clinical,
  care program, appointment, and benefit verification data.
- Workday Human_Resources contains employee PII, government IDs, demographic
  data, employment history, photos, and organizational assignments.
- ServiceNow tables can include employee/service data, incidents, configuration
  items, business applications, and customer-specific custom fields.
- iCIMS ATS data can include candidate PII, application history, assessment
  data, and customer-specific profile fields.
- Oracle Fusion Financials and Anaplan carry financial/confidential operational
  data. Their code-bearing skeletons must stay named-operation first, with
  tenant allow-lists and live certification before broader resources are
  exposed.

## Salesforce

### What Is Confirmed

The Salesforce corpus captures Health Cloud plus REST API docs for Summer '26,
API 67.0, docs 262.0. The local crawl captured 350 Health Cloud articles, with
267 discovered links left in the queue, so treat the corpus as strong but not
exhaustive.

The REST article summary confirms:

- REST access is via OAuth 2.0 through External Client Apps or legacy Connected
  Apps. The local docs recommend External Client Apps because new Connected App
  creation is restricted as of Spring '26.
- API calls use `Authorization: Bearer <token>`; `401` indicates an expired or
  invalid session ID or OAuth token.
- Base REST paths follow
  `https://<MyDomainName>.my.salesforce.com/services/data/v67.0/...`.
- OAuth/Bearer-token REST usage for query-related endpoints.
- SOQL reads use `GET /services/data/v67.0/query?q=<SOQL>`.
- Deleted-inclusive reads use `GET /services/data/v67.0/queryAll?q=<SOQL>`.
- Record reads use `/services/data/v67.0/sobjects/<Object>/<Id>`.
- Query responses contain `totalSize`, `done`, `records`, and optionally
  `nextRecordsUrl`; follow `nextRecordsUrl` without adding caller parameters.
- Synchronous SOQL returns up to 2,000 records per batch, sometimes fewer.
- Limit information via `Sforce-Limit-Info` headers.
- Limits can also be read at `GET /services/data/vXX.X/limits/`, with
  max/remaining values that are accurate within about five minutes.
- `REQUEST_LIMIT_EXCEEDED` error semantics in REST error docs.
- Composite API calls count as one call, but should stay out of v1 unless a
  named operation explicitly needs and tests them.
- Error bodies usually include `message`, `errorCode`, and sometimes `fields`.
  Important statuses include `400`, `401`, `403`, `404`, `409`, `414`, `431`,
  and `500` / `502` / `503`. Conditional-request statuses `304`, `412`, and
  `428` are useful later for metadata caching.

The Health Cloud article catalog contains 154 object pages, 43 data model
pages, and 27 action pages. Many object pages expose supported calls including
`describeSObjects()`, `getDeleted()`, `getUpdated()`, `query()`, `retrieve()`,
and `search()`. Example queryable Health Cloud object candidates include:

- `CareProgram`
- `CareProgramEnrollee`
- `CareProgramTeamMember`
- `CareBenefitVerifyRequest`
- `CoverageBenefitItem`
- `CoverageBenefitItemLimit`
- `ClinicalEncounter`
- `ClinicalMeasure`
- `AccountServicePreference`
- `ServiceAppointmentGroup`

Some important pages have no supported-call list in the extracted catalog, such
as `CoverageBenefit`, `MemberPlan`, and `PlanBenefit`. That does not prove they
are unusable; it means the provider must verify with authenticated org describe
before registering named operations for them.

High-value read domains from the Health Cloud corpus include:

- Clinical and FHIR R4: `ClinicalEncounter`, `HealthCondition`,
  `CareObservation`, `ClinicalServiceRequest`, `MedicationRequest`,
  `MedicationStatement`, `PatientImmunization`, `PatientMedicalProcedure`, and
  `DiagnosticSummary`.
- Care programs and care management: `CareProgram`, `CareProgramEnrollee`,
  `CarePlan`, `CarePlanActivity`, `CareGap`, and `CareTask`.
- Coverage, benefits, and prior authorization: `MemberPlan`,
  `CoverageBenefit`, `CoverageBenefitItem`, `CareBenefitVerifyRequest`,
  `CarePreauth`, and `CarePreauthItem`.
- Home health and appointments: `CareServiceVisit`, `Visit`, and Service
  Appointment custom fields.
- Documents and OCR: `ReceivedDocument`, `DocumentChecklistItem`, and
  `OcrDocumentScanResult`.

Many object pages expose associated `ChangeEvent` and `History` artifacts. Those
are useful later for CDC/event sync, but v1 should start with bounded SOQL reads
and `getUpdated()` / `getDeleted()` where describe confirms object support.
`SourceSystemModified` or `SourceSystemModifiedDateTime` appears on many Health
Cloud objects and can be useful as a source-system watermark. The local captures
do not broadly document `LastModifiedDate` or `SystemModstamp`, so do not require
those fields from this evidence alone.

### V1 Named Operation Candidates

Keep v1 read-only and projection-safe:

| Operation | Vendor shape | Notes |
| --- | --- | --- |
| `salesforce.account_by_id` | SOQL or REST record retrieve for `Account` | Existing skeleton-compatible CRM baseline; org describe confirms fields. |
| `salesforce.updated_accounts` | SOQL query with bounded timestamp predicate or `getUpdated()` based flow | Use fixed selected fields, max rows, and `nextRecordsUrl`. |
| `salesforce.account.get_updated_ids` | `GET /sobjects/Account/updated/` with fixed `start` / `end` UTC parameters | ID-only Account sync primitive; Health Cloud variants remain describe-gated. |
| `salesforce.account.get_deleted_ids` | `GET /sobjects/Account/deleted/` with fixed `start` / `end` UTC parameters | ID-only delete-log primitive; response caps and retention-window handling remain provider-owned. |
| `salesforce.health.fetch_limits` | `GET /services/data/vXX.X/limits/` | Useful for smoke tests and rate visibility without reading business data. |
| `salesforce.health.get_updated_ids` | Object-specific `getUpdated()` timeframe read | Internal sync primitive; registered but unsupported until org describe confirms the object allow-list. |
| `salesforce.health.get_deleted_ids` | Object-specific `getDeleted()` timeframe read | Internal sync primitive; registered but unsupported until org describe confirms the object allow-list and delete-log retention. |
| `salesforce.health.care_programs_updated_since` | Queryable `CareProgram` projection | Catalog-backed metadata candidate; PHI/PII gated until org describe and governance evidence. |
| `salesforce.health.care_program_enrollees_by_program` | Queryable `CareProgramEnrollee` projection by `CareProgramId` | Catalog-backed metadata candidate; PHI/PII likely, redaction and retention required. |
| `salesforce.health.coverage_benefit_items_by_member` | Queryable `CoverageBenefitItem` projection by `MemberId` | Catalog-backed metadata candidate; PHI/financial-benefit sensitivity, avoid broad list reads. |
| `salesforce.health.clinical_encounters_updated_since` | Queryable `ClinicalEncounter` projection | Catalog-backed metadata candidate; PHI-heavy and only after governance gates. |
| `salesforce.health.appointment_resources` | `POST /services/data/v65.0/connect/health/appointment-management/resources` | Future read-style action candidate; not registered in the v1 code skeleton yet. |
| `salesforce.health.appointment_slots` | `POST /services/data/v66.0/connect/health/appointment-management/slots` | Future read-style action candidate; requires Home Health/appointment permission evidence. |
| `salesforce.health.context_data` | `GET /services/data/v66.0/actions/custom/contextDataProvider/DefaultContextDataProvider` | Future context action candidate; only after context definition allow-listing. |

Do not expose Health Cloud action APIs as v1 writes. Many action pages are POST,
PATCH, or PUT business operations, such as booking/canceling appointments,
creating referrals, processing documents, or parsing eligibility responses.
They are future governed mutations, not read scaffolding.

Provider internals may implement primitive request builders for query, queryAll,
record get, org limits, updated IDs, and deleted IDs. Those primitives are not
product-visible operations and must not allow callers to submit arbitrary SOQL,
object names, field names, or URLs.

### Implementation Implications

- Pin Salesforce schema metadata to API 67.0 for this snapshot or document why a
  lower API pin is selected for compatibility.
- The provider registry should own object name, selected fields, max rows,
  allowed predicate slots, timeout, and pagination budget.
- SOQL builders must bind or encode values structurally; caller input must not
  choose object names, fields, raw SOQL, URL paths, or arbitrary filters.
- `nextRecordsUrl` continuation uses the shared URL-cursor continuation shape
  in `appfw_saas_core`.
- Account `getUpdated` / `getDeleted` ID-only request planning and response
  summaries are compiler-contracted for the fixed Account object. Generic
  Health Cloud change-ID operations remain unsupported until authenticated org
  describe confirms object support.
- Salesforce API-67 snapshot metadata is exported from the provider using the
  local information-package versions, implementation metadata, and extracted
  Health Cloud catalog. The four Health Cloud projection candidates are
  catalog-backed and describe-gated; they do not add executable SOQL fields or
  caller-selected object names.
- Rate-limit handling should parse both REST error codes and limit headers.
- Treat this connector as PHI/PII-heavy. Patient/member identity, clinical
  conditions, medications, immunizations, diagnostic summaries, benefit details,
  preauthorization data, forms, and OCR text require strict field allow-lists,
  redacted logs, token encryption, and avoidance of raw result payloads in
  traces.
- Preserve Salesforce org and user permission filtering. Some Health Cloud
  fields/features require org preferences or permission sets, such as FHIR R4
  clinical model settings, Home Health settings, document/intelligent workspace
  licenses, and Data Protection and Privacy for some enrollment fields.
- `Account.SourceSystemIdentifier` is documented as not encryptable in the
  captured Health Cloud material; classify it sensitive even when Salesforce
  cannot encrypt it.
- Live certification requires an authenticated org describe/export, redacted
  sample responses, and a live read proving token acquisition, pagination,
  redaction, and audit/metrics.

## Workday

### What Is Confirmed

The Workday corpus captures HCM Human_Resources WWS v46.1 / 2026R1 as SOAP
document/literal WSDL plus XSD.

The extracted catalog reports:

- 322 operations total.
- 146 `Get_*` operations and 117 `Put_*` operations.
- Fault types include `Validation_Fault`, `Processing_Fault`, and
  `Authentication_Fault`.
- Response/filter hints include `Page`, `Count`, `As_Of_Effective_Date`,
  `As_Of_Entry_DateTime`, `Updated_From`, and `Updated_Through`.
- Worker-related read candidates include `Get_Workers`, `Get_Worker_Profile`,
  `Get_Employee`, `Get_Contingent_Worker`, `Get_Worker_Event_History`,
  `Get_Organizations`, `Get_Locations`, and `Get_Job_Profiles`.

Extraction-time WSDL/XSD review captured in the catalog confirms the WSDL
service is `Human_ResourcesService`, the port is `Human_Resources`, and the
SOAP address is relative (`Human_Resources`). The captured WSDL/XSD details do
not declare a complete auth scheme such as WS-Security policy, OAuth, HTTP
Basic, bearer, certificate auth, or SAML.
`Authentication_Fault` is present, but tenant auth is a Workday application and
tenant configuration concern. The provider must treat the configured WWS auth
mechanism, tenant hostname, service endpoint, integration-system user or client
model, and secret-handling rules as live evidence.

Common request shape should be modeled explicitly:

- `Request_References` for exact IDs/references.
- `Request_Criteria` for bounded filters.
- `Response_Filter` for `As_Of_Effective_Date`, `As_Of_Entry_DateTime`, `Page`,
  and `Count`.
- `Response_Group` for least-data include flags.

The XSD reports `Count` as `1..999`, with default `100`. Paging should set
`As_Of_Entry_DateTime` when `Page` is used so page traversal is stable. Response
results include `Total_Results`, `Total_Pages`, `Page_Results`, and `Page`.

`workday.get_server_timestamp` now has a network-free typed request template
and UTC timestamp parser. It is useful for anchoring an upper-bound watermark to
Workday server time before paged reads, but live tenant samples are still needed
before this freshness evidence becomes live certification.

### V1 Named Operation Candidates

Use selected `Get_*` operations only:

| Operation | Vendor shape | Notes |
| --- | --- | --- |
| `workday.list_workers` | `Get_Workers` with response filter, response group, and optional transaction-log criteria | Primary HCM sync candidate; PII-heavy. |
| `workday.get_worker` | `Get_Workers` with `Worker_Request_References`, or `Get_Worker_Profile` for narrower payloads | Requires Workday ID/reference mapping. |
| `workday.list_worker_events` | `Get_Worker_Event_History` with worker reference and event date range | Audit/history use case; sensitive. |
| `workday.list_organizations` | `Get_Organizations` with filters | Useful reference projection; org transaction log does not cover all name/code changes. |
| `workday.list_locations` | `Get_Locations` | Useful reference-data projection. |
| `workday.list_job_profiles` | `Get_Job_Profiles` | Useful reference-data projection. |
| `workday.get_server_timestamp` | `Get_Server_Timestamp` | Executable offline metadata read; anchors freshness/watermark windows, pending live sample evidence. |
| `workday.list_former_workers` | `Get_Former_Workers` | Planned/evidence-gated only. The extracted catalog proves operation presence, but not enough request-shape detail to emit SOAP safely. |
| `workday.get_workday_accounts` | `Get_Workday_Account` | Planned/evidence-gated only. Identity/security adjacency requires request-shape evidence and governance review before execution. |

### Implementation Implications

- Continue the existing Workday provider skeleton with bounded SOAP envelope
  builders and XML response decoder contracts; do not create a generic SOAP
  proxy.
- Use the WSDL/XSD catalog to generate or validate operation request/response
  shapes, but keep runtime exposure to named operations.
- The extracted catalog proves some operation names only. Runnable Workday
  operations require operation-specific SOAP shape evidence plus governance and
  redaction review before they can move from planned to executable.
- Model Workday object references explicitly. Do not rely on display names as
  stable IDs.
- Pagination should bind `Page` and `Count` through provider-owned request
  builders.
- Watermark sync should use operation-specific transaction-log support where
  present. `Get_Workers` supports `Transaction_Log_Criteria_Data` with
  `Updated_From`, `Updated_Through`, `Effective_From`, and `Effective_Through`;
  paired from/through values are required when one side is supplied.
- Use `Get_Server_Timestamp` to capture a stable upper bound before paging; the
  current provider keeps this as a one-row metadata read and reports freshness
  evidence as unsupported-with-offline-evidence until live tenant samples exist.
- Treat `Get_Organizations` deltas carefully: the local docs warn that the
  transaction log does not capture organization name/code changes, so the
  provider may need periodic full reconciliation or special handling.
- Reads of worker personal information, demographics, photos, government IDs,
  and related persons need classification, field redaction, and retention policy
  before materialization.
- Worker response groups default broadly if omitted, including reference,
  personal, employment, compensation, organization, and role data. Named
  operations must set explicit response-group include flags for least-data
  retrieval.
- Photo and document operations (`Get_Worker_Photos`, `Get_Person_Photos`,
  `Get_Former_Worker_Documents`) should be separate gated operations; local XSD
  review indicates photo data is not secured beyond service execution access.
- All `Put_*`, `Update_*`, `Add_*`, `Assign_*`, `Import_*`, and `Submit_*`
  operations remain unsupported in v1.

## ServiceNow

### What Is Confirmed

The local public ServiceNow pages were captured as Fluid Topics shells. The
extracted summary provides likely platform patterns but explicitly warns that
accurate schemas and OpenAPI exports require authenticated instance access.

Likely patterns to verify in a target instance:

- Table API path: `/api/now/table/{tableName}`
- Query parameters: `sysparm_query`, `sysparm_fields`, `sysparm_limit`,
  `sysparm_offset`, `sysparm_display_value`, and
  `sysparm_exclude_reference_link`
- Watermark fields: `sys_updated_on` plus `sys_id`
- Candidate ITSM tables: `incident`, `problem`, `change_request`, `cmdb_ci`

### V1 Posture

ServiceNow now has a metadata-only `appfw_provider_servicenow` skeleton. It
stays evidence-requested until a non-production instance export is available.
Do not hard-code table catalogs or field lists from public docs.

Cataloged but unsupported operation names:

- `servicenow.table.export_schema_from_instance`
- `servicenow.table.query_incremental`
- `servicenow.table.fetch_by_sys_ids`
- `servicenow.table.fetch_reference_values`

Potential v1 reads after evidence:

- `servicenow.incidents_updated_since`
- `servicenow.incident_by_sys_id`
- `servicenow.problems_updated_since`
- `servicenow.change_requests_updated_since`
- `servicenow.cmdb_ci_updated_since`

Required evidence:

- Target non-prod instance URL and selected auth method, with no secrets in
  docs or fixtures.
- REST API Explorer OpenAPI export for Table API and selected tables.
- Table dictionary/schema exports for selected tables, including field types,
  references, readable fields, and required roles.
- Integration user role/ACL matrix and enabled plugin/module list.
- Redacted list/get responses with headers, pagination, errors, and an
  incremental `sys_updated_on` query.

## iCIMS

### What Is Confirmed

The public iCIMS corpus confirms developer community and integration guidance,
but detailed API docs and data models are login-gated. Public guidance confirms:

- REST API usage.
- Standard-field emphasis.
- Scalable, repeatable integrations.
- Customer-specific integrations do not get a public sandbox.
- Validation expects field matrices, auth method details, customer/partner
  credential handling, and avoidance of hardcoded customer-varying values.

### V1 Posture

iCIMS stays evidence-requested until Developer Community docs or exported specs
are available. It now has a metadata-only `appfw_provider_icims` skeleton whose
auth mode remains access-gated and unknown. The architecture page's prior
examples (`candidate`, `job`, `application`, `workflow`) are reasonable domain
hypotheses, not confirmed v1 operation contracts from this corpus.

Cataloged but unsupported operation names:

- `icims.discovery.fetch_api_contract`
- `icims.recruiting.query_candidates_incremental`
- `icims.recruiting.query_jobs_incremental`
- `icims.recruiting.query_applications_incremental`

Potential reads after gated evidence:

- Candidate/profile reads using approved Applicant Tracking profile types.
- Job/requisition reads using approved standard fields.
- Application/workflow-status reads only after schema and pagination behavior
  are available.

Required evidence:

- Exported docs for "Connecting and Authenticating", "ICIMS Applicant Tracking
  APIs", and the relevant data model/schema pages.
- Confirmation of partner-marketplace versus customer-specific integration
  path.
- Field matrix by profile type, including read-only versus read/write and
  standard versus custom fields.
- Auth method, IP allowlist requirements, package IDs, customer ID handling,
  and partner credential model.
- Redacted sample GET responses, headers, pagination examples, rate-limit/error
  examples, and current release notes behind login.

## Candidate Providers

### Anaplan

Anaplan now has a network-free Integration API v2 provider skeleton. The corpus
is useful because it exercises a different external API connector design:
server-bound workspace/model paths, offset collection reads, read-request pages,
and chunk downloads. It still must not claim live auth, watermarks, rate limits,
metrics, freshness, or writes until product scope and tenant evidence exist.

Confirmed from the extracted Apiary summary:

- Base URLs include `https://api.anaplan.com/2/0/` and
  `https://auth.anaplan.com/`.
- Auth endpoints include `/token/authenticate`, `/token/validate`,
  `/token/refresh`, and `/token/logout`.
- Auth uses `Authorization: AnaplanAuthToken {token}` after authentication.
  Token creation supports basic username/password and CA certificate auth.
- Resource paths are workspace/model scoped and include models, lists, views,
  dimensions, tasks, and read requests.
- Pagination includes `limit`, `offset`, `sort`, and `meta.paging` fields such
  as `currentPageSize`, `next`, `previous`, `offset`, and `totalSize`.
- No numeric rate limits were captured from the local Apiary payload. Providers
  should throttle per workspace/model and back off on `429`, `503`,
  task-pending, and transient transport responses.
- File/dump workflows support numbered chunk downloads that the client
  concatenates; the local metadata does not establish a universal chunk-size cap.
- Docs warn clients not to assume fixed response fields; parsers should ignore
  unknown fields.

Design implication: `appfw_saas_core` shared continuation shapes now have a
concrete Anaplan user. The provider skeleton compiler-contracts only
offline-safe areas: named operation registration, request binding,
offset/read-request/chunk continuations, tenant workspace/model binding, API
base path pinning, result caps, paging summaries, and mutation rejection. View
and file operations are registry-gated; a product cannot pass arbitrary
workspace IDs, model IDs, view IDs, file IDs, paths, or raw URLs.

Current and future read/export named operations include
`anaplan.model.get_status`, `anaplan.files.list`,
`anaplan.view.create_read_request`, `anaplan.view.get_read_request`,
`anaplan.view.get_read_page`, and `anaplan.file.download_chunk`. Side-effecting
operations such as `upload_file_chunk`, `complete_file_upload`, `run_import`,
`run_action`, and `run_process` require separate governed-write gates.

### Oracle Fusion Financials

Oracle Fusion Financials now has a network-free request-planning skeleton in
`appfw_provider_oracle_financials`. It is a strong example of large
OpenAPI-backed SaaS, but the implementation intentionally exposes only a tiny
offline-safe starter surface. Live auth, tenant data-security semantics,
resource-level scoping, rate limits, metrics, freshness, redaction, and writes
remain below certification.

Confirmed from the extracted OpenAPI summary:

- OpenAPI 3.0.0, version `2026.03.27`, 1,391 paths and 2,241 operations.
- 1,325 GET operations, plus POST/PATCH/DELETE/PUT operations.
- Common parameters include `q`, `limit`, `offset`, `fields`, `expand`,
  `onlyData`, `links`, `totalResults`, `orderBy`, `finder`,
  `REST-Framework-Version`, and `Metadata-Context`.
- Security is not declared in the captured OpenAPI summary, so auth must be
  supplied from Oracle docs or tenant evidence before live execution.
- Tags include bank accounts, expenses, invoices, collections, credit data, ERP
  integrations, and other confidential financial domains.
- Collection responses commonly use `items`, `count`, `hasMore`, `limit`,
  `offset`, `links`, and optional `totalResults`.
- Error modeling is weak in the captured OpenAPI: most operations use `default`
  responses, with explicit `400`, `401`, and `500` concentrated in the ERP
  integration area.

Current executable starter operations are intentionally limited to the sample
LOV paths present in the extracted summary:

- `oracle.financials.accounting_period_status_lov.list`
- `oracle.financials.accounting_period_status_lov.get`

The skeleton also catalogs broader corpus names without executing them:
`oracle.financials.openapi.describe_resource`,
`oracle.financials.collection_query`, `oracle.financials.fetch_by_id`,
`oracle.financials.list_lov`, and
`oracle.financials.submit_erpintegration`. The final name is write-gated and
remains a dry-run catalog entry only.

Design implication: future OpenAPI ingestion must filter the huge catalog down
to approved named operations, apply classification by resource family, and avoid
making every GET path a product-visible read. The planner compiler-contracts
fixed paths, `onlyData=true`, `links=false`, offset/limit caps, tenant
REST-framework/version headers, tenant host validation, and mutation rejection.
It deliberately does not expose caller-supplied `q`, `finder`, `orderBy`,
`expand`, arbitrary resource paths, or generic POST/PATCH bodies.

Potential future operations, after tenant allow-list and certification evidence,
include `getall_invoices`, invoice actions such as `validateInvoice`,
`cancelInvoice`, `calculateTax`, and `applyPrepayments`, `getall_expenses`,
`getall_payablesPayments`, ERP Data Integrations file upload/status/exception
reads, and BOSS `$query` only as a separate advanced family.

## Next Implementation Backlog

1. Keep Salesforce as the reference code-bearing provider. Continue moving the
   skeleton from CRM examples toward API-67-aware named read operation metadata
   and build on the landed Account `getUpdated` / `getDeleted` ID primitives,
   while still requiring org describe before Health Cloud objects become
   live-certified.
2. Continue the Workday code-bearing skeleton around Human_Resources WWS
   `Get_*` operations, building on the landed server timestamp/freshness
   metadata with effective-date semantics, HR PII redaction, and live tenant
   samples.
3. Build on the Anaplan provider skeleton only through tenant-approved operation
   registration, read-request response summaries, chunk metadata summaries, and
   certificate/token evidence. Keep raw workspace/model/view/file paths out of
   product code.
4. Build on the Oracle Financials request-planning skeleton only through
   tenant-approved resource allow-lists, fixed projections, escaped predicate
   builders, and live auth/business-unit/ledger scoping evidence.
5. Use
   [SaaS Authenticated Export Evidence](saas-authenticated-export-evidence.md)
   for ServiceNow and iCIMS requests so product teams know exactly which
   authenticated exports, schemas, field matrices, sample responses, and
   role/ACL data are required. Their metadata-only skeletons remain
   all-unsupported until that evidence exists.
6. Extend the landed sync descriptor validation with provider-specific
   watermark modes:
   `salesforce_get_updated`, `soql_timestamp`, `workday_updated_range`,
   `servicenow_sys_updated_on`, and `provider_custom`.
7. Adopt the landed `appfw_saas_core` continuation and transfer primitives in
   provider registries and certification fixtures: URL cursor
   (`nextRecordsUrl`), page/count, offset/limit, async task, read-request page,
   and chunk-download metadata/caps.
7. Keep all writes unsupported unless a named mutation has separate live
   write-safety evidence, audit/redaction, idempotency behavior, tenant binding,
   and explicit exposure gates.

## Merge Guidance

This page is safe to merge without committing the raw
`vendor_api_specs_2026-06-26/` corpus. It captures the implementation decisions
and evidence gaps the next provider branches should consume.
