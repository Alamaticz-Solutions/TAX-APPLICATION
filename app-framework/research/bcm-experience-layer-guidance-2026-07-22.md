# BCM Experience Layer On ServiceNow — Guidance (2026-07-22)

> **Status: recommendation for named owners; nothing here is funded or
> sequenced.** Evidence basis:
> [BCM And GRC Tooling Research](bcm-grc-tooling-research-2026-07-22.md) and
> the internal Baker–Chris exchange. Framework fit derives from the
> [Enterprise App Fabric Strategy](../docs/strategy/enterprise-app-fabric.md)
> (selection rubric, tiered actionability, licensing posture, OMA) and the
> [Permissioned Archetype Contract](../docs/specs/permissioned-archetype-contract.md).
> Sequencing authority remains the
> [Roadmap Outcome Goal Register](../docs/release/roadmap.md#outcome-goal-register);
> activation requires the named owners and an explicit human decision.

## Verdict

**Yes — selectively, and it strengthens rather than fights the platform
decision.** Baker's directive (ServiceNow as the single Risk/Compliance/BCM
platform) and Chris Chock's evaluation finding (ServiceNow scored last of
five, specifically weak at non-technical BIA self-service and dependency
visualization) are both satisfiable with the same architecture the fabric
strategy already prescribes: **ServiceNow runs the BCM/ERM/Crisis engine —
records, recertification cycles, workflows, audit trail — and App Framework
selectively layers the PDS-owned experiences at the practice edge where the
evaluation found ServiceNow weakest.** This is not a third tool; it is the
adoption surface for the tool Baker chose, for an audience (1,200+ practice
leaders) that ServiceNow's own docs name as a persona but whose documented
business-user role does not yet cover BCM.

Honesty first: the confirmed evidence does not prove ServiceNow's native
BIA experience is inadequate — the internal evaluation says so, some public
reviewers disagree. **Baker's configured PoC is the deciding experiment**,
and this guidance is structured so the PoC answers the layer question at the
same time it answers the module question.

## What ServiceNow Keeps (never rebuild — Chris's "EHR" warning is correct)

- BIA data model, scoring, and recertification scheduling engine
- Continuity plan objects, exercise management, corrective-action workflow
- Risk register, ERM scoring, policy/compliance attestation engine
- Crisis event records and task workflow; **Everbridge remains the
  notification engine** (that is ServiceNow's own documented architecture)
- CMDB/CSDM as the dependency substrate (confirmed: BCM dependencies are
  CI-relationship-driven with scheduled auto-update)
- The BCM Configurable Workspace for the BCM team — specialists stay in the
  native, vendor-invested workbench

## What The Layer Adds (three candidate experiences, rubric-scored)

1. **Practice BIA & recertification journey** — the flagship. A guided,
   PDS-designed BIA contribution/update/attest experience for non-technical
   practice and department leaders: plain-language wizard, prefilled from
   existing records, mobile-friendly later, recertification nudges tied to
   ServiceNow's cycle engine, one obvious "what changed since last year"
   path. Rubric fit: named high-frequency-at-population-scale task (1,200
   practices × annual+eventful updates), persona is exactly field/office
   staff, vendor UX risk is real (Classic→Configurable churn is confirmed),
   and the evaluation flagged this precise gap.
2. **Impact and dependency console** — read-only first. Process → location
   → system → vendor → plan graph for a named scenario ("hurricane over
   three Florida practices"), rendered with the PDS design system's graph
   components. Confirmed constraint: this is only as good as the CMDB, so
   it is **gated on the CMDB milestone** — which independently validates
   Baker's instinct to sequence CMDB before (or as gate for) BCM value.
3. **Crisis activation & status experience** — site-leader-facing crisis
   status, task acknowledgment, and check-in surfaces during an event;
   deep-link into the ServiceNow workspace for the BCM team; Everbridge
   continues to page people. Read + acknowledge first; mobile continuation
   only when the framework's mobile lane graduates.

Do not layer: BCM-manager workbench, plan authoring, exercise design,
risk quantification, program administration — low-population, specialist,
native-adequate surfaces.

## Decision-Rule Answers (fabric doc)

1. **Journey theme and named owner:** Business Resilience journey;
   candidate owner **Chris Chock (Director, Information Assurance and
   Business Resilience)** with Baker as executive sponsor — funding must be
   named before build (binding rubric rule).
2. **Actionability tier per step, in writing:** candidate tiers —
   BIA read `api_readable` (Table API on BCM Store-app tables; confirm
   scoped-app ACLs); BIA submit/attest `api_actionable` candidate (must be
   proven on the PDS instance via one low-risk mutation with per-user
   delegated auth, per the fabric's ServiceNow certification path);
   dependency graph `api_readable` from CMDB/BCM relationships; crisis
   tasks read + acknowledge `api_actionable` candidate; mass notification
   `out_of_scope` (Everbridge). **No tier is certified yet** — that is
   BCM-P1/P2 work below.
3. **Object models to adopt:** `sn_bcm_*` (BIA, plans, dependencies,
   crisis), relevant `sn_risk_*`/`sn_grc_*` (attestations, assessments),
   CMDB CI/relationship slices — verbatim adoption behind version-pinned
   generated contracts with tenant-config drift subchecks (OMA); never a
   PDS canonical resilience model.
4. **Metric:** BIA coverage % across practices, median time-to-BIA-
   completion, recertification on-time %, exercise corrective-action
   closure, crisis time-to-acknowledge per site. Baseline = native
   ServiceNow PoC. If the layer cannot beat native on these, retire it.
5. **Ownership split:** ServiceNow owns engine + records + workflow;
   App Framework owns the practice-edge experience, identity-preserving
   read/action contracts, and telemetry; Everbridge owns notification;
   CMDB program owns the dependency substrate.

## Licensing Posture (extend the existing order-form asks)

Confirmed: BCM is a separate Store-app entitlement; the documented
`sn_grc.business_user` role covers Risk/Policy attestations and assessments
but **lists no BCM/BIA permissions** (14.x snapshot); community threads
reference "BCM Lite" and "GRC Business User Lite" lanes (unverified).
ServiceNow licensing is interface-agnostic (fabric doc), so the layer
neither saves nor costs licenses by itself — but the BIA-contributor class
must be settled in writing. Add to the existing five ServiceNow order-form
asks:

6. **BIA-contributor licensing defined:** the license class (and price) for
   practice/department users contributing and attesting BIAs, risk
   assessments, and crisis acknowledgments — confirmed **channel-agnostic**
   (native UI, portal, or PDS-built UX over APIs identical).
7. **BCM/IRM table API access confirmed:** REST/Table API read and the
   specific low-risk mutations on `sn_bcm_*`/`sn_grc_*` scoped tables under
   per-user delegated OAuth, without an Extend-equivalent or integration
   surcharge.

Scale note: at ~$45K/yr licensing (+~2.5× implementation), the underlying
tool is inexpensive. The layer's case is **program credibility and
adoption** (ISO 22301 evidence quality, BIA coverage across 1,200
practices, crisis readiness), never license arbitrage. Per-user delegated
auth everywhere; no service-account laundering.

## What It Would Take (Nexus-similar implementation shape)

Phases mirror the register's governed-read → governed-action → product-
proof pattern (AF-OG03/AF-OG04/AF-OG07) and the existing framework
machinery. Notably, **AF-OG03 is currently held awaiting exactly this: an
explicitly selected product/source object.** BCM/BIA is now a concrete
candidate for that selection, competing with Team Member Journey objects —
a Program PM decision, not presumed here.

- **BCM-P0 — Charter and shared PoC (now; documentation-only).** Attach to
  Chris's business-requirements effort and Baker's configured-ServiceNow-PoC
  ask. One artifact: named owner + funding, the BIA-self-service acceptance
  bar the PoC must meet, the tier table above, the two licensing asks, and
  the CMDB gate definition. The PoC instance doubles as the fabric's
  evidence source. Exit: platform decision confirmed; layer go/no-go
  criteria agreed **before** anyone builds.
- **BCM-P1 — Object model intake (framework capability, small build).**
  Authenticated dictionary/schema export of `sn_bcm_*`, relevant
  `sn_risk_*`/`sn_grc_*`, and CMDB slices from the PoC instance; ingest via
  the intake pipeline's planned `saas-export` source kind (the named gap in
  the fabric doc) into a reviewed model proposal with drift subchecks.
  Exit: version-pinned adopted model + tenant-config drift defense.
- **BCM-P2 — Governed read vertical (AF-OG03 pattern).** One permissioned,
  version-pinned read of BIA records, plans, and dependencies through the
  permissioned-archetype contract (fail-closed row/field masks), fixtures
  first, then live against the PoC instance. Exit: read tier certified;
  freshness semantics ("as of") visible.
- **BCM-P3 — Experience proof (AF-OG07-style walking skeleton).** The
  practice BIA journey (queue/detail + guided wizard) and the read-only
  dependency graph on the PDS design system; measured against the native
  PoC on the P0 metrics with real practice users. Exit: user-testable
  go/narrow/stop decision.
- **BCM-P4 — Governed action (AF-OG04/G1 pattern).** BIA submit/update and
  attestation as preview → confirm → execute operations under per-user
  delegated OAuth with idempotency, audit, and reconciliation; then crisis
  task acknowledgment. Exit: action tier certified live.
- **BCM-P5 — Crisis console and scale-out.** Crisis status/check-in
  experience (read + acknowledge + deep-link), rollout beyond pilot
  practices, mobile continuation when the mobile lane graduates.

Hard gates across all phases: named owner + funding stays live; licensing
asks answered in writing; **CMDB maturity gate before any dependency-value
claim** (confirmed CI-relationship dependence); register disposition by the
Program PM; the protected-window rules (this is shaped now, built on
explicit activation).

## Risks And Stop Rules

- **Native-is-good-enough:** if the configured PoC meets the BIA
  self-service bar with workspace configuration, stop at P0 — the layer was
  insurance, not destiny. This outcome is a win (cheapest path).
- **CMDB immaturity:** without CI relationships, the dependency console is
  an empty map — hold P3's graph slice until the CMDB gate passes.
- **Licensing ambiguity:** if ServiceNow will not put BIA-contributor
  licensing and API access in writing, the practice-edge journey's
  economics are unknowable — hold P4.
- **Owner/funding loss:** rubric rule six is binding; no named funded owner,
  no build.
- **Scope creep toward the engine:** any pull to reimplement scoring,
  scheduling, or plan logic in the layer is a stop signal — that is the
  "building our own EHR" failure Chris named.
- **Origami scope surprise:** the claims-only framing of Origami was
  refuted; align with Stephanie Casey's inventory before assuming what
  lands in ServiceNow risk tables.

## Why This Also Serves The Bigger Picture

The same contracts this journey needs — ServiceNow schema intake
(`saas-export`), permissioned read, per-user delegated action, drift
defense, PDS-designed field-staff journeys — are the fabric's core
capabilities for every later journey (Team Member Journey, CTO, PDS Connect
successor). BCM is a small, well-bounded, executive-sponsored candidate to
prove them: modest data volumes, a real crisis-readiness outcome, a named
Director-level owner, and a population (1,200 practices) that demonstrates
the requester-lane economics. If the Program PM selects it as the AF-OG03
source object, it becomes the fabric's first proof; if not, this guidance
holds until sequencing allows it.
