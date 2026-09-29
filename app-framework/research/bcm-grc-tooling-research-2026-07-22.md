# BCM And GRC Tooling Research — 2026-07-22

> **Status: research evidence base, not guidance.** Companion guidance:
> [BCM Experience Layer Guidance](bcm-experience-layer-guidance-2026-07-22.md).
> Method: fan-out research harness (5 search angles, 23 sources fetched, 98
> claims extracted, top 25 adversarially verified by three independent
> refutation votes each, with live re-fetches on 2026-07-21/22). Evidence
> classes are kept explicit throughout: **confirmed** (survived 3-vote
> refutation against the primary source), **refuted**, **unverified/flagged**
> (extracted but verification incomplete — a network outage ended eight
> verification panels early — or paywalled/vendor-reported). Confirmed means
> the source says it, dated; it does not mean the capability performs well.
> All licensing findings must be re-verified in writing at negotiation.
> Disposition of anything here belongs to the named business, product, and
> platform owners.

## Research Question

Should PDS Health (1,200+ practices) layer a PDS-owned experience on top of
ServiceNow BCM/Risk — with ServiceNow remaining the workflow engine — and
what do BCM/GRC market facts say about where such a layer would add value?
Inputs: the internal Baker–Chris exchange (below), ServiceNow product/docs
pages, specialist-vendor primary sources, and analyst/review signals.

## Internal Context (primary evidence: bcm/baker-chrisc-chat.md)

- The executive sponsor (Baker) has directed a single-platform posture:
  "For Risk, Compliance & BCM > I want us on a single platform > ServiceNow…
  Not 3 shiny new tools." Origami Risk is also expected to move to
  ServiceNow (owner: Stephanie Casey), and Baker wants BCM + risk + that
  migration run as one program.
- Chris Chock (Director, Information Assurance and Business Resilience) ran
  a five-product BCM evaluation in which **ServiceNow scored lowest of the
  five**. Working licensing estimate (via Sabrina): **~$45K/year**, with
  implementation roughly **2.5× license cost**; three ServiceNow modules
  assumed necessary (BCM, ERM/risk, Crisis Management).
- Chris's evaluation findings against ServiceNow: the vendor "struggled to
  show" risk modeling and dependency visualization in practice, and "the
  tool was not designed for our non-technical business managers to help
  update their own BIAs independently."
- Chris's definition of the real product: a live connected model of
  operations (process → location → system → vendor → plan) that a small BCM
  team can run as an ISO 22301 audit-ready program; explicitly **not**
  dashboards, and explicitly not something to hand-build ("comparable to
  building our own EHR").
- Baker's process asks: documented business requirements to evaluate
  against; a **configured ServiceNow proof of concept in the PDS
  environment against a real scenario**; an explicit sequencing decision on
  the **CMDB build (still pending) versus BCM implementation**; and honest
  total-cost comparison (noting Fusion stacks on Salesforce licensing).

## Confirmed Findings — ServiceNow BCM/IRM (primary docs)

| # | Finding (dated, sourced) | Vote |
| --- | --- | --- |
| 1 | ServiceNow BCM (Yokohama docs) is structured into four functional areas: Business Impact Analysis, Business Continuity Planning, Exercises, and Crisis Management (crisis events, including a crisis map). Documented product structure. | 3-0 |
| 2 | BCM is **not part of the base platform**: it is a ServiceNow Store application requiring entitlement and separate installation — a separate subscription on top of the platform. Exact licensing units are not stated in docs; re-verify at negotiation. | 3-0 |
| 3 | **BCM dependency mapping is CMDB-driven**: dependencies are added from CI relationships and kept current by scheduled auto-update jobs propagating CMDB changes into impact analyses and plans. Dependency-mapping quality is therefore bounded by CMDB/CSDM data quality. | 3-0 |
| 4 | ServiceNow's documented crisis **notification** story is an integration with **Everbridge** (a specialist competitor) for emergency mass notification, not a fully native mass-notification engine. | 3-0 |
| 5 | As of BCM 5.x.x (Xanadu docs), the **BCM Configurable Workspace** is the invested modern BCM UI and the **BCM Classic Workspace is on an explicit deprecation path** (hidden from navigation, URL-only, unsupported with new features). ServiceNow churns its own UI layers; an API-level custom UX bypasses that churn but duplicates workspace function. | 3-0 |
| 6 | ServiceNow documents the BCM application as a central workbench for **seven user types including "business users"**, with a role-driven workspace per persona. Business users are a named out-of-box persona; the page contains **no licensing/subscription-unit information**. | 3-0 |
| 7 | A dedicated **`sn_grc.business_user` role** exists (KB0864247), intended for users who only complete GRC tasks assigned to them — e.g., responding to attestations or risk assessments. | 3-0 |
| 8 | The 14.x permission snapshot for `sn_grc.business_user` includes: take attestations, acknowledge policies, request policy exceptions, respond to evidence requests, take risk assessments, respond to risk identification questionnaires — the write actions a thin business-contributor front end would invoke. | 3-0 |
| 9 | That same 14.x snapshot covers **only** Policy & Compliance, Risk Management, and PPM integration — **no BCM (`sn_bcm`) or BIA permissions are listed**. Whether the business-user lane covers BIA contribution in BCM is **not established** and must be verified in BCM product docs / with ServiceNow. | 3-0 |

## Confirmed Findings — Specialists And Market

| # | Finding (dated, sourced) | Vote |
| --- | --- | --- |
| 10 | **Fusion Framework System is a native Salesforce managed package** requiring Platform Cloud, compatible only with Salesforce Enterprise/Unlimited editions (AppExchange listing metadata). Fusion deployment stacks on Salesforce platform licensing — confirming Baker's cost point. | 3-0 |
| 11 | Fusion's listed pricing floor is **$30,000 USD/company/year** (company-based), full pricing vendor-gated; whether Salesforce licenses are embedded or separate is **not stated** — re-verify at negotiation. | 3-0 |
| 12 | Forrester published *The Forrester Wave: GRC Platforms, Q2 2026* (blog 2026-05-27; 12 vendors evaluated). The public blog names **no vendors**; any Leader placement claims come from the paywalled report. | 3-0 |
| 13 | The Gartner MQ document (October 2025 era) is client-paywalled; the public abstract names no vendors. ServiceNow-vs-specialist MQ positioning is **unverifiable from public sources**. | 3-0 |
| 14 | LogicGate self-reports being one of four Leaders in the Q2 2026 GRC Wave (vendor press release; other Leaders unnamed; ServiceNow's position not establishable from this source). | 3-0 |
| 15 | Gartner Peer Insights (retrieved 2026-07-22): ServiceNow IRM and MetricStream have **identical 3.9/5 overall ratings** (71 vs 47 reviews) — summary-level rating parity between ServiceNow and a GRC specialist; per-criterion breakdowns are login-gated. | 2-0 |
| 16 | Same source: ServiceNow IRM shows **76% willingness-to-recommend vs MetricStream's 62%**, with a more favorable star distribution — customer-sentiment evidence that ServiceNow IRM is not viewed as inferior to at least this specialist. | 3-0 |

## Refuted

- "Origami Risk's RMIS is an insurance/claims-operations system (TCOR,
  claims, incident/event, insurance program, allocations, exposure), so
  migrating Origami to ServiceNow is a claims-RMIS replacement, not a BCM
  replacement." — **REFUTED 0-3.** Origami's own positioning is broader
  than the claims-side module list. Treat the Origami→ServiceNow migration
  scope as **unknown until Stephanie Casey's inventory**; do not assume it
  is claims-only or BCM-free.

## Unverified / Flagged (verification incomplete or gated)

Network failures ended these panels early (`erroredVotes` > 0) or the
source is paywalled/vendor-reported. Use as leads, not facts:

- ServiceNow was named a Leader in the Forrester GRC Wave **Q4 2023**
  (vendor-repeated; 1 valid supporting vote; report gated).
- ServiceNow's "highest scores in 12 criteria" landing-page claim may be
  recycled copy across Wave cycles — verify against the actual report.
- Gartner featured-review critical themes for ServiceNow IRM: customization
  rigidity and dependence on implementation partners (supports the
  configuration-effort premise; panel errored).
- Info-Tech SoftwareReviews (viewed 2026-07-22): ServiceNow BCM composite
  **7.6 vs Fusion 6.8**, likeliness-to-recommend 90% vs 81%, plan-to-renew
  95% vs 82% — a **counter-signal favoring ServiceNow** against the
  specialist narrative (panels errored; small samples).
- Consultancy estimate (advisori.de, 2026): ServiceNow BCM typically
  included in enterprise licensing or **EUR 30,000–100,000/year** as an
  add-on; and an opinion that integrated platforms trade BCM depth for
  consolidation. Unsourced estimates; re-verify with ServiceNow.
- ServiceNow community threads (extracted, never verified — treat strictly
  as questions for the account team): IRM/GRC subscription "allocation"
  calculation; a **"GRC Business User Lite"** plugin; the **"BCM Lite"**
  license model; roles-within-IRM-user-license. These suggest a lite/
  business-user licensing lane relevant to 1,200-practice BIA contributors
  — **confirm scope and channel-agnosticism in writing**.

## What The Evidence Base Does And Does Not Establish

Established: ServiceNow BCM's documented shape (four areas, Store-app SKU,
CMDB-bounded dependencies, Everbridge-based notification, invested-but-
churning first-party UI, named business-user persona, a business-user role
whose documented permissions **do not currently include BCM/BIA**); Fusion's
Salesforce stacking and pricing floor; and that public analyst positioning
cannot arbitrate ServiceNow-vs-specialist quality. Review-site signals are
mixed and directionally **kinder to ServiceNow** than the internal
five-product evaluation was.

Not established: how well ServiceNow BCM's BIA self-service and dependency
visualization actually perform for non-technical practice managers (the
internal evaluation says poorly; Info-Tech reviewers disagree; both are
small-sample). That question is empirically decidable by **Baker's own ask**
— the configured PoC against a real scenario — which is therefore the single
most valuable next evidence step regardless of any experience-layer
decision.

## Sources

Primary: ServiceNow Yokohama BCM overview docs; Xanadu BCM workspace docs;
ServiceNow KB0864247; Salesforce AppExchange Fusion listing; Origami Risk
solutions pages; Forrester Wave Q2 2026 announcement blog; Gartner document
abstract. Secondary/vendor-reported: LogicGate press release; ServiceNow
Forrester landing page; Gartner Peer Insights and Info-Tech comparison
pages. Forum/lead-quality: ServiceNow community licensing threads. Blog:
advisori.de comparison. Retrieved live 2026-07-21/22.
