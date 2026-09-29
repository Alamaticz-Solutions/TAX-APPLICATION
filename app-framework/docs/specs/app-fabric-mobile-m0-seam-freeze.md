# App Fabric Mobile M0 Seam-Freeze Record

| Field | Value |
| --- | --- |
| Record | `AF-MOBILE-M0-SEAM-2026-08-03` |
| Planning freeze base | `origin/main@2de105e001ab8029eb21bce4691eded49e48ff81` |
| Accepted Integration base/current main at carrier validation | `origin/main@750a58b1ff78f9cd76f7992ca5dd04a0bde6dc3e`; exact destination pipeline `#385` successful |
| Stage | Prototype planning, pre-candidate |
| Planning seam status | one bounded AF-M01.4 M0-05 Integration lane admitted and history-preservingly assembled on a local unpushed carrier; proposed freeze not ratified; final exact-SHA evidence and destination acceptance remain open |
| Runtime seam status | not implemented or accepted |
| Candidate ready | false |
| Release ready | false |
| Release authority | none |
| Source work admitted | bounded human-directed M0-05 containment only |

## Purpose

This record is the integration handrail for the first App Fabric mobile
implementation family. It makes the decisions already reached in research and
architecture review easy to consume without turning them into a roadmap,
Product Increment, Assignment, enterprise standard, or readiness claim.

The [Mobile Operating Context](../start/app-fabric-mobile-operating-context.md)
is the entry point. The
[Mobile UX Master Plan](./app-fabric-mobile-ux-master-plan.md) contains the
full implementation sequence and evidence model.

## Proposed Planning Seams

| Seam | Proposed planning contract | Source artifact | Implementation state |
| --- | --- | --- | --- |
| Channel choice | Responsive web is a complete channel and mandatory comparator; it does not become native through breakpoints. Installed native is selected only when journey evidence warrants it. | [Technology Evaluation Brief](./app-fabric-mobile-technology-evaluation-brief.md) | comparator and real journey not yet implemented |
| Reference technology | React Native plus Expo remains App Framework's selected reference target and a reversible proof hypothesis; it is not an enterprise-wide standard or business-channel mandate. | Technology Evaluation Brief; [React Native guide](../frontend/mobile-react-native.md) | scaffold exists; integrated capability absent |
| Renderer model | Closed projections are `web-dom`, `native-ios`, `native-android`; closed canonical size classes are `compact`, `medium`, `expanded`. Renderer is not a theme. | [ADR 0017](../architecture/adr/0017-web-native-experience-projection-boundary.md) | catalog and native renderer work open |
| Parity | Channels share stable intent, state, action, policy, accessibility, telemetry, and resumption semantics; layout trees, navigation, interaction mechanics, lifecycle, and implementations remain renderer-owned. | ADR 0017; [experience schema](./app-fabric-experience-contract.schema.json) | schema/example are planning contracts, not generator inputs |
| Design System | One semantic catalog manifest feeds an actual React DOM catalog and an actual React Native catalog executed separately on iOS and Android. A phone-shaped DOM frame is web evidence only. | ADR 0017 | native catalog/package not implemented |
| Mobile topology | Native is opt-in. Pending-registration fixtures carry `app_id:null` plus a local-only `prototype_app_ref`; only the Fabric registry may return the durable `app_id`. The profile maps `native-ios`/`native-android` to FAB-A1 registration and component-snapshot surfaces and declares public-client auth, capabilities, build/update/distribution posture, test matrix, ownership, and forbidden inputs. | [manifest profile schema](./app-fabric-mobile-manifest-profile.schema.json) | not integrated into canonical app manifest/config or an authoritative registration record |
| API wire | Current App Framework v1 list envelope is `items` plus snake-case count/cursor/page fields; current record ID variable is GraphQL `String!`. Generated projections may not invent Relay fields or consume generated web/mobile output. | [API seam](./app-fabric-mobile-api-seam.md); [golden operations](./app-fabric-mobile-api-compatibility.graphql) | blocker: accepted-main mobile output is incompatible |
| Identity | Native is a public OIDC client using Authorization Code plus PKCE; no embedded client secret. Access tokens stay in memory, and the server's real verifier owns issuer/audience/client, tenant, scope/policy, token-time, and key validation with local bypass disabled. | Master plan; [auth profile schema](./app-fabric-mobile-auth-profile.schema.json) and [synthetic example](./app-fabric-mobile-auth-profile.example.json) | planning schema exists; local verifier integration/evidence checker and multi-client runtime support open; enterprise IdP decision deferred |
| Build custody | Prototype uses enterprise/developer-controlled local builds, synthetic data, and development credentials. Cloud, self-hosted, or split candidate custody requires a named decision and proof. | [Build Custody Decision](./app-fabric-mobile-build-custody-decision.md) | candidate decision parked |
| Updates | Native-binary and OTA-compatible changes are distinct. Production OTA cannot bypass exact-source candidate/release proof or named human promotion authority. | [Hosted Product Factory mobile platform surfaces](./hosted-product-factory.md#mobile-platform-surfaces); exact accepted [registration](../../scripts/fabric-contracts/fabric-app-registration.v1.schema.json), [component snapshot](../../scripts/fabric-contracts/app-component-snapshot.v1.schema.json), and [observation event](../../scripts/fabric-contracts/fabric-observation-event.v1.schema.json) wire schemas | runtime compatibility, signing, rollout, halt, recovery, and service authority open |
| Maturity | Every M0/M1 artifact stays `experimental`, `candidate_ready:false`, `release_ready:false`, `release_authority:none`. | Master plan and machine context | M0-05 producer/staging/consumer containment assembled locally; corrected exact-SHA proof, review, guard, and destination acceptance remain open; candidate checker not implemented |

## Explicitly Not Frozen

These fields require later named owners or representative evidence and must not
be guessed by implementation agents:

- the business journey, outcome baseline, benefit threshold, adoption target,
  and channel decision for Patient Mobile, Team Member Mobile, or any other
  product proposition;
- numeric web/native size-class breakpoint tokens and the supported OS/device
  matrix;
- enterprise IdP client registrations, issuer/audience topology, token/session
  lifetimes, risk-based step-up, and production identity;
- offline eligibility, local retention, notification content, biometrics,
  camera/files/share, screenshot/clipboard behavior, or other native
  capabilities by data classification;
- approved candidate build/distribution service, source and secret egress,
  signing custody, store/MDM ownership, OTA service, and recovery thresholds;
- crash/device-health, store-console, build-service, analytics, privacy,
  support, finance, and release authorities;
- candidate performance budgets, accessibility sampling, external-device
  evidence, service levels, support model, and lifecycle economics; and
- standalone product binary versus a shared enterprise shell beyond the
  bounded standalone reference proof.

## Freeze Disposition

Integration and the Fabric Architect may review the exact seam and recommend or
draft future Assignment scopes for issuance/admission by the authorized Program
Flow Controller or human authority. Those scopes may cover normalized
generator/API work, PDS Native/catalog work, and native shell work without
reopening the basic architecture. The proposed planning seam is **not yet a
ratified freeze** and is not sufficiently implemented to fan out broader
producers:

1. PFC Assignment comment `835555085`, narrowly amended by root-only comment
   `835556572`, admits exactly one AF-M01.4 M0-05 Integration lane; M0-02
   remains unadmitted;
2. accepted Integration base/current main at carrier validation is
   `750a58b1ff78f9cd76f7992ca5dd04a0bde6dc3e`, with exact destination pipeline
   `#385` successful; Integration assembled immutable inputs
   `a28bee04cb660ab1fff0b2ab25b26b17607b7044` and
   `a6f87c38c97fd564b09a4a4c4617dca3245b8132` history-preservingly on the
   local unpushed carrier, which is Integration evidence rather than an
   accepted destination;
3. the mobile API generator emits a schema-invalid wire shape and the required
   source-owned red real-schema golden test is not yet admitted or implemented;
4. the experience and manifest schemas are not canonical generator/config
   inputs;
5. the Prototype auth planning schema exists, but runtime integration, its
   semantic/evidence checker, and the general evidence-result schema remain to
   be implemented;
6. schema validation is shape-oriented and does not yet enforce source/evidence
   binding, conservative classification, exact auth-client allowlists,
   safe/digest-bound references, ownership disjointness, channel/test
   feasibility, or operation/evidence referential integrity;
7. PDS Native and its executable native catalog do not exist as qualified
   packages; and
8. identity, build, update, device, candidate, store, operations, and support
   evidence remains open by design.

M0-05 now contains the legacy false `release_ready` path at the producer,
staging, Wave 2 consumer, and strict retained-evidence consumer, with an
adversarial regression. It rejects top-level and nested readiness claims in
all raw staged inputs, rejects malformed JSON, duplicate object keys, and any
present readiness field that is not the exact JSON boolean `false`, stages only
the canonical serialization of the validated value, deletes unsafe/stale
targets, and independently checks nested and duplicate-key Wave 2 claims.
Local Integration assembly is complete. Final corrected exact-SHA Class D
proof, independent comprehensive review, guard, push/PR, human main-merge
judgment, and post-merge destination verification are required before
acceptance. M0-02 remains unadmitted and may not start under this Assignment. It
may begin only through a separate exact Assignment issued/admitted by the
authorized PFC or human authority, subject to available WIP and no scope
collision; its red result remains non-authoritative until M0-05 receives
destination acceptance. Once Integration retains destination-accepted
containment, that red proof, and the seam SHA, the first M1-01
API/operation-transport slice normalizes the contract in `app_gen`, makes the
test green, emits web/native clients from the same in-memory IR, and regenerates
the CRM fixture. It must not patch generated outputs or read generated
web/mobile files as source.

## Reopen Triggers

Reopen this planning freeze when:

- the canonical backend API changes its identifier, pagination, error, tenant,
  or authorization semantics;
- representative web/native proof contradicts the shared semantic contract;
- React Native/Expo cannot meet a named accessibility, security, performance,
  lifecycle, device, or support need without disproportionate platform forks;
- the strategy BOK closes or materially changes the Application Fabric/mobile
  gates;
- a real business proposition introduces regulated data, safety-significant
  behavior, external writes, or a shared-shell request; or
- candidate distribution, enterprise identity, hosted build, signing, OTA, or
  store/MDM work is proposed.

## Role Card Check

- **Card used:** Architect Agent, with Fabric Architect consultation and App
  Framework Research Steward advisory input.
- **Within role:** technical seams, source hierarchy, architecture coherence,
  proof sequence, and explicit implementation blockers.
- **Authority not assumed:** Product acceptance, strategy ratification,
  Assignment/WIP admission, funding, risk acceptance, candidate, release,
  merge, or push.
- **Routed:** value and named journey to Product; exact source work to the
  Product Increment/PFC system; enterprise posture to EAC; identity/security,
  build/store, finance, operations, and release decisions to their named human
  authorities.
- **Drift signal:** `watch`; local M0-05 Integration assembly is complete, but
  it is not destination acceptance. Final corrected exact-SHA evidence, human
  main-merge judgment, post-merge destination verification, and all
  implementation beyond the bounded M0-05 containment remain open.
