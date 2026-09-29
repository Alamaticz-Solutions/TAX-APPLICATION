# App Fabric Mobile Technology Evaluation Brief

| Field | Value |
| --- | --- |
| Record | `AF-MOBILE-M0-00` |
| Lifecycle position | Explore / Prototype, pre-candidate |
| Disposition | App Framework reference target retained for bounded proof; no enterprise mobile standard or business-channel mandate selected |
| Assessment date | 2026-08-03 |
| App Framework baseline | accepted `origin/main@2de105e001ab8029eb21bce4691eded49e48ff81`; this record and its master plan are branch-local planning additions |
| Strategy baseline | `pds-technology-strategy` commit `046e532c0e41b6911171f3d9aa961c0898d66140`; deployment/upstream currency not proved |
| Decision authority | None. This is an evaluation brief, not an architecture approval, investment approval, security acceptance, release approval, or enterprise standard. |

## Purpose and decision frame

This brief formalizes the Explore-stage options record required by
[the App Fabric mobile UX master plan](./app-fabric-mobile-ux-master-plan.md).
It asks a deliberately bounded question:

> Which delivery-channel and implementation hypothesis should App Framework
> prove first so that a downstream product team can deliver an excellent
> small-screen experience without mistaking responsive web behavior for native
> evidence or creating a universal mobile mandate?

The present answer is to retain the repository's React Native plus Expo
reference target and run it as a reversible proof against a responsive-web/PWA
comparator while retaining platform-native, other cross-platform,
vendor-native, and no-installed-app paths as real alternatives. Standalone and
shared-shell packaging remain separate choices; neither is implied by the
implementation technology.

This reconciles two different decision scopes. The repository's current
[React Native Mobile App Contract](../frontend/mobile-react-native.md) selects
React Native plus Expo for App Framework implementation. The newer strategy
BOK evidence does not ratify that implementation choice as an enterprise-wide
standard, prove a business case, or decide a channel for a named journey. This
brief narrows and tests the former; it does not silently claim the latter.

This recommendation does **not** answer whether any named PDS business service
needs an installed application. No authoritative proposal ID, named business
service owner, approved journey baseline, quantified target, lifecycle budget,
or support owner exists in the consulted evidence. Those absences are decision
inputs, not administrative blanks to be inferred by the framework team.

## Authority and source binding

The sources below are bound exactly as consulted. Their authority classes must
not be flattened into a single notion of “strategy.”

| Source | Version / baseline | Authority used here | Effect on this brief |
| --- | --- | --- | --- |
| Enterprise Technology Strategy 2026–2030 | `2026-07-31-draft.3` at strategy commit above | Governing within the authored BOK; draft and not recorded here as ratified | Supplies decision rules, Explore-stage method, proof requirements, experience constraints, and retained authority boundaries |
| Machine-readable application contract | `2026-07-31-draft.3` | Machine contract; draft | Supplies application proposal/evidence fields and permits uninvoked conditional groups to remain not applicable during Explore |
| Application Fabric Strategic Direction | `2026-08-01-draft.2` | Companion; pending major decision; no governing effect | Challenges the proof to use one lifecycle/evidence spine without assuming one runtime or universal intake |
| Mobile slice evidence review `review.slice-mobile-2026-08` | `2026-08-01-evidence.2` | Evidence only; no approval authority | Identifies the native-evidence gap and records React Native plus Expo only as an implementation hypothesis |
| Mobile Channel and Installed Application Posture decision packet | Unversioned at the bound commit; pending major decision | Operating/release-excluded; no governing effect | Preserves unresolved channel, packaging, and evidence questions |
| Application Fabric strategic-direction decision packet | Unversioned at the bound commit; pending major decision | Operating/release-excluded; no governing effect | Preserves the unresolved common-Fabric decision |
| `gate.application-fabric-strategic-direction` | Open at the bound commit | Open gate | Prohibits treating the companion posture as ratified enterprise direction |
| `gate.strategy-enterprise-architecture-interface` | Open at the bound commit | Open gate | Preserves enterprise architecture and council decision rights |

The brief applies strategy rules DR-01 through DR-05, the Experience Platforms
requirements T6-R1 through T6-R5 and prohibitions T6-P1/T6-P2, the eight-stage
evaluation framework, and the application proposal/evidence protocol. It does
not close either open strategy gate.

## Scope and no-transfer boundary

The reference proof is an **App Framework capability proof** using synthetic or
non-sensitive representative data. It can establish that the framework can
declare, generate, build, test, observe, update, and recover a mobile surface.
It cannot establish that a particular business service should adopt that
surface.

In particular:

- no Patient Mobile, Team Member Mobile, clinician, member, or workforce
  proposition is approved by this record;
- no evidence produced here transfers to such a proposition as outcome,
  adoption, workflow, accessibility, safety, privacy, support, or lifecycle-
  economics evidence;
- every named business journey requires its own proposal/evaluation brief,
  accountable outcome owner, credible channel comparators, representative
  population and conditions, success/failure thresholds, and disposition;
- responsive web, vendor-native workflow, messaging, or no new surface must
  remain credible options in those evaluations; and
- clinical and SaaS systems retain semantic and transactional authority.
  Mobile may compose context and actions through governed contracts; it does
  not become an alternate system of record.

## Needs and hard constraints

The reference proof should test the following needs without converting them
into an unqualified product promise:

1. A shared semantic contract can project into genuinely channel-appropriate
   web, iOS, and Android layouts and behaviors rather than a DOM-shaped mobile
   screen.
2. The native channel can prove touch, navigation, focus, keyboard, safe-area,
   lifecycle, deep-link, secure-storage, network-transition, accessibility,
   and recovery behaviors on representative devices.
3. The server can preserve authorization, audit, idempotency, and named action
   semantics for a public client without trusting UI state.
4. Generation remains deterministic, ownership-aware, and reversible; product
   teams can opt out without forking the framework.
5. Builds and updates have explicit source, credential, signing, provenance,
   halt/forward-fix/binary-replacement, and support custody before candidate
   distribution.
6. Any custom surface must demonstrate material benefit over credible vendor or
   web paths and identify persistent ownership plus plausible second and third
   reuse before an enterprise platform claim.

Hard constraints for this Explore proof are: synthetic/non-sensitive data,
bounded users and environments, no clinical or SaaS authority transfer, no
production write path, no external distribution promise, no production OTA,
and no candidate/qualified/released claim.

## Credible delivery-channel options

No weighted composite score is used. The disposition follows constraints,
representative evidence, reversibility, and the current decision stage.

| Option | Where it can win | Principal costs and risks | Evidence status | Explore disposition |
| --- | --- | --- | --- | --- |
| Responsive web, with optional PWA capabilities | Broad reach, immediate deployment, shared web implementation, low install friction; can be the final answer when device/lifecycle or native interaction value is weak | Breakpoints alone do not create native navigation, platform conventions, background/lifecycle integration, store delivery, dependable secure storage, or native-quality offline/notification behavior | Existing web and design-system evidence is useful as a comparator; it is not native proof | Retain as mandatory comparator and valid end state |
| React Native plus Expo | One TypeScript-oriented native reference, access to native controls and platform APIs, Expo development/build/update ecosystem, and alignment with the repository's current scaffold direction | Native escape hatches, dependency/runtime compatibility, Expo service custody, app-size/startup/rendering risks, dual-platform QA, and ongoing native expertise remain; no enterprise qualification exists | Repository direction and scaffold exist; integrated auth, representative device behavior, operational update recovery, candidate custody, and lifecycle economics are unproved | **Leading bounded reference hypothesis**, not an enterprise standard |
| Platform-native iOS and Android | Maximum platform fidelity, earliest access to platform capabilities, direct control over performance and OS integration | Two implementations and specialist teams; semantic/UI drift, duplicated release work, and higher persistent support cost unless a journey materially requires it | Not compared under representative conditions in this repo | Retain as escalation path when measurable native capability or performance thresholds defeat the cross-platform reference |
| Other cross-platform implementation | May offer a better rendering, performance, language, ecosystem, or existing-workforce fit for a particular product | A second framework stack increases skills, generator, design-system, dependency, build, and support surface; benefits are currently hypothetical | No representative comparator evidence | Do not standardize; admit a bounded comparator only when a named hypothesis and owner identify a material advantage |
| Vendor-native capability | Preserves vendor workflow and authority, can reduce custom build/support burden, and may provide stronger domain integration | Fragmented journeys, limited experience control, licensing and roadmap dependency, variable cross-vendor context continuity | Must be assessed for each named journey and vendor; no general conclusion is supportable | Mandatory comparator when a vendor already owns the workflow |
| Defer or no installed application | Avoids install, security, release, support, and lifecycle cost when the outcome can be met through existing channels | May leave genuine mobility, offline, device, or attention needs unmet | No named journey has yet proved the need for an installed app | Valid current disposition for every unowned or uneconomic proposition |

“Other cross-platform” is intentionally technology-neutral at this stage. A
specific alternative should enter only with an accountable comparator
hypothesis, not to manufacture a tool contest detached from a journey.

## Packaging is a separate decision

| Packaging path | Potential value | Risks and proof burden | Current posture |
| --- | --- | --- | --- |
| Standalone application | Independent brand, cadence, entitlement boundary, blast radius, telemetry, store listing, and product ownership | Repeated shell/release work across products; users may accumulate applications; shared context transitions need explicit contracts | Default **reference-proof packaging hypothesis** because it minimizes coupling and makes evidence attribution clearer; not a product mandate |
| Shared enterprise shell | Common identity/session, navigation, notifications, deep links, context handoff, and distribution could reduce repeated work across proven propositions | Creates a high-blast-radius platform, coupled release train, complex tenancy/entitlement and failure isolation, marketplace pressure, and persistent operating obligation | Separate future decision only after multiple owned propositions and reuse economics justify it; this proof must not accidentally create one |

The implementation choice does not decide packaging. React Native plus Expo can
support either arrangement, and a shared shell cannot be justified merely by
technical feasibility. T6-P1 and T6-P2 rule out a universal portal or speculative
marketplace claim without the required outcome and reuse evidence.

## Bounded reference hypothesis

**Hypothesis.** For a deliberately small, read-oriented reference journey,
React Native plus Expo can project the same governed App Fabric semantics as
responsive web while delivering measurably native navigation, interaction,
device, lifecycle, accessibility, and recovery behavior on representative iOS
and Android devices, without contaminating the web implementation or making the
framework/mobile service irreversible.

**Comparator.** The same reference journey in responsive web at equivalent
small-screen sizes. The comparison must include outcome completion, task steps,
resumption, accessibility, startup/interaction responsiveness, degraded-
network behavior, implementation/change cost, and support/release work. It must
not award “native” points merely because a binary launches.

**Representative conditions.** At minimum: current supported iOS and Android
simulator/emulator versions plus representative physical-device evidence before
candidate; compact and large text; screen reader and keyboard/switch-relevant
paths; light/dark and reduced-motion settings; interrupted foreground/background
lifecycle; deep-link return; offline/degraded/reconnected network; expired
session; API compatibility mismatch; and update recovery. Prototype work uses
synthetic identities and data only.

**Success threshold.** The proof succeeds only if it demonstrates all of these:

- channel-specific layouts and behaviors from a shared semantic contract, with
  no dependency on parsing generated web or prior generated mobile output;
- repeatable clean generation and local iOS/Android build/test evidence with
  declared ownership and drift detection;
- a named, source-bound comparison showing the native reference materially
  improves at least one relevant small-screen/device behavior without material
  regression in accessibility, security, recovery, or supportability;
- public-client auth and server enforcement can be tested without embedding a
  secret or trusting the client, while production identity remains out of
  scope until the candidate tripwire;
- binary/runtime/API compatibility, update promotion, source-bound recovery,
  and failure
  containment are demonstrable in a non-production lane; and
- adoption of the reference remains opt-in and removal returns the product to a
  supported web-only topology without data migration or server-authority
  change.

No numeric outcome, latency, cost, or adoption threshold is authorized yet.
Those thresholds must be named before a candidate proposition; the absence of
numbers means the proof can compare and discover, not claim business success.

**Failure threshold.** Treat the hypothesis as failed or materially weakened if
the proof cannot meet a hard constraint, if native behavior remains only a
responsive-web facsimile, if representative accessibility/security/recovery
regresses, if routine product changes require divergent hand-maintained channel
semantics, or if the cross-platform path misses a journey-specific native
capability/performance threshold that a credible platform-native or vendor path
meets.

**Cost ceiling.** Missing. A Finance/contract owner must set an Explore proof
ceiling and later provide lifecycle economics covering engineering, two-platform
QA, devices, accounts, build/update services, observability, accessibility,
security, stores/MDM, incident response, support, and retirement. Until then,
the proof may report effort and vendor consumption but cannot claim economic
preference.

**Blast radius.** One opt-in reference product profile, synthetic data,
non-production backend, development/preview distribution only, and no shared
enterprise shell or production identity dependency.

## Reversibility and stop triggers

Reversibility is an acceptance property, not a future cleanup promise. The
proof must preserve a technology-neutral semantic/experience contract, keep
native implementation behind an opt-in topology capability, isolate generated
from human-owned files, keep server authority independent of mobile, avoid
mobile-only persisted data formats, and document how to remove the mobile
surface and its service accounts/build configuration.

Stop the proof and route a new decision when any of the following occurs:

- a real Patient Mobile, Team Member Mobile, clinician, member, or workforce
  proposition is introduced without its own owner and evaluation;
- PHI/PII, real tenant data, enterprise non-production identity, live provider
  writes, executable external actions, or a regulated/safety-significant
  workflow would enter the lane;
- signed distribution expands beyond bounded development/preview use, including
  TestFlight, Play internal/closed, production stores, or enterprise MDM;
- production OTA, a package/support commitment, candidate/qualified/released
  language, or an enterprise standard is proposed;
- responsive web or vendor-native behavior meets the named outcome with lower
  lifecycle burden and no material native gap;
- platform-native evidence materially outperforms the cross-platform reference
  on a journey's required capability or threshold;
- Expo/React Native creates unacceptable source, credential, update, supply-
  chain, runtime compatibility, licensing, service-continuity, or recovery
  exposure;
- the reference requires a shared shell, new transactional authority, or
  irreversible server/data coupling to proceed; or
- no accountable owner, bounded budget, representative users, or support path
  can be named before the next stage.

At a stop, preserve the evidence and choose among revise, platform-native
escalation, vendor-native, responsive web/PWA, defer, or stop. Do not recast an
Explore failure as partial qualification.

## Evidence plan and next disposition

The smallest useful proof should retain source-bound evidence for:

1. a shared semantic contract and explicit web/iOS/Android projections;
2. responsive-web and React Native plus Expo implementations of the same
   synthetic reference journey;
3. simulator/emulator matrices during development and physical-device evidence
   before any candidate claim;
4. accessibility, lifecycle, deep-link, network, auth-expiry, compatibility,
   update halt/forward-fix/binary-replacement, telemetry, and support/recovery
   scenarios;
5. deterministic generation, ownership boundaries, clean-room build, dependency
   provenance, and removal/reversal;
6. recorded comparator effort, defects, operational steps, and vendor/service
   consumption; and
7. explicit pass/fail/unknown dispositions for every threshold and stop trigger.

The current Explore recommendation is **bounded proof**. Candidate entry is not
automatic. It requires a separate, source-bound decision that names the
business proposition, owners, thresholds, lifecycle economics, build/signing
custody, enterprise identity and threat posture, data/provider authority,
support model, representative evidence, and the chosen channel and packaging
disposition.

## Missing decisions and accountable parties

| Required input | Current state | Required before |
| --- | --- | --- |
| Authoritative application/proposal identity | Missing | Any product-specific evaluation or candidate claim |
| Named business outcome and baseline/target | Missing | Product reference-journey acceptance |
| Accountable product/business outcome owner | Missing | Product reference-journey acceptance |
| Finance/contract owner and proof cost ceiling | Missing | Material vendor consumption or candidate investment decision |
| Lifecycle economics and second/third reuse cases | Missing | Platform/shared-shell or enterprise-standard claim |
| Persistent operating and support owner | Missing | External preview or ongoing service commitment |
| Business service owner | Missing; required if the mobile surface becomes a material service | Candidate disposition |
| Enterprise architecture disposition | Open interface gate; no approval in this record | Candidate architecture decision |
| Security, privacy, data, clinical/safety, accessibility, legal/procurement reviews | Trigger-dependent and not invoked for this synthetic local proof | Before their respective data, identity, workflow, vendor, or distribution triggers |
| Platform/store account and release owner | Missing | Signed candidate distribution |

Missing parties remain `TBD`; this brief does not assign a person or governance
body beyond the authorities already named by the governing BOK.

## Role Card Check

- **Card used:** Architect Agent consuming the App Framework Research Steward
  procedure as advisory evidence.
- **Work within role:** Compared credible paths, bound evidence and authority,
  exposed missing proof/ownership/economics, and defined a reversible bounded
  evaluation.
- **Authority not assumed:** No product, architecture, funding, security,
  candidate, release, source-admission, WIP, or enterprise-standard decision was
  made.
- **Routed decisions:** Product/business outcomes, Finance/contract, enterprise
  architecture, security/privacy/data/clinical/legal/accessibility, release,
  build/signing custody, and persistent operations remain with their named or
  still-missing authorities.
- **Drift signal:** `watch`; escalate if this brief is cited as approval, if the
  React Native plus Expo hypothesis is presented as a mandated standard, if
  evidence is transferred to a named business proposition, or if an open BOK
  gate is represented as closed.
