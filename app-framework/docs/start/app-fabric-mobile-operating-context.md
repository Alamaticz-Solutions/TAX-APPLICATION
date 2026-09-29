# App Fabric Mobile Operating Context

Read this file first when continuing App Fabric mobile work. It preserves the
research, architecture decisions, unresolved questions, and safe technical
path without requiring a new agent to reconstruct the conversation. Live
delivery status is intentionally owned by the canonical Product Increment
portfolio and plan linked below, not by a second mobile ledger.

## Current Snapshot

| Field | Value |
| --- | --- |
| Snapshot | `AF-MOBILE-OPERATING-CONTEXT-2026-08-03` |
| Planning freeze base | `origin/main@2de105e001ab8029eb21bce4691eded49e48ff81` |
| Latest observed destination | accepted `origin/main@750a58b1ff78f9cd76f7992ca5dd04a0bde6dc3e`; exact post-merge pipeline `#385` succeeded |
| Integration refresh | active under PFC Assignment comment `835555085`, Amendment r2 comment `835556572`, and isolation-only Amendment r3 comment `835574654`; `integrate/app-fabric-mobile-m0` preserves the accepted base and both immutable inputs; nine-root authority reconciliation and full Class D proof remain required |
| Planning branch | `docs/app-fabric-mobile-m0` |
| Immutable M0-05 producer | `fix/app-fabric-mobile-readiness-containment@a28bee04cb660ab1fff0b2ab25b26b17607b7044`, based on planning commit `a9b3646289689de824c0fbde3fdb805312fbb196`; pushed, immutable, and consumed history-preservingly by the active Integration carrier |
| Progress carrier | `docs/app-fabric-mobile-delivery-loop@a6f87c38c97fd564b09a4a4c4617dca3245b8132`, a reviewed dependent context packet consumed history-preservingly by Integration; no separate PR |
| Live Product Increment | [`AF-M01.4`](../specs/af-m01.4-mobile-read.product-increment.json) in the [canonical portfolio](../specs/product-increment-portfolio.json) |
| Lifecycle | Prototype / Explore, pre-candidate |
| Delivery profile | `accelerated` |
| Maturity | experimental |
| Candidate ready | false |
| Release ready | false |
| Release authority | none |
| Source-producing mobile work admitted | exactly one bounded M0-05 Integration lane; M0-02 is ready next but is not WIP-admitted |
| M0-05 status | assembled history-preservingly at pre-reconciliation carrier `d3e0be621c433237c34722ae3acb83d2c8ab8f1e`; authority reconciliation and destination-current Class D proof/review active; branch unpushed with no PR |
| Dated machine snapshot | [`app-fabric-mobile-operating-context.json`](./app-fabric-mobile-operating-context.json), validated by its [planning schema](./app-fabric-mobile-operating-context.schema.json); immutable research/review snapshot, not live delivery state |

The clean branches were created because the original shared checkout was dirty,
stale relative to `origin/main`, and contained unrelated human/agent work. Do
not commit this package from that checkout, copy its unrelated changes, or use
its generated files as source. Local worktree paths are intentionally not part
of the durable contract. The M0-05 producer stays immutable; continue delivery
context only through the assigned Integration carrier, and use the planning
history for the unchanged research package. Do not mutate either immutable
producer input while reconciliation or proof is active.

## Human-Agent Collaboration Loop

This is the deliberately small operating loop for the implementation journey:

1. **One live record.** Update the `AF-M01.4` portfolio entry and validated plan
   only when an evidence-bearing event changes truth: Assignment, push,
   independent review, Integration, dependency, acceptance, or stop/park
   decision. This operating guide carries durable technical context; its dated
   JSON companion stays immutable.
2. **One human checkpoint.** While work is active, the Branch Manager presents
   a checkpoint at material transitions and at least every 48-72 hours:
   `Current`, `Evidence`, `Next`, `Waiting for`, and `Human judgment needed`.
   Human-facing labels may say `Working`, `With reviewer`, `With Branch
   Manager`, `Ready next`, `Waiting for ...`, `Done at SHA`, or `Parked`; the
   validated schema values remain canonical underneath.
3. **One pull decision.** The human asks: *What is accepted? What moves next?
   Where is my judgment needed?* The Product Owner owns value and acceptance;
   the Fabric/Mobile Architect owns technical slicing and proof; the Program
   Flow Controller owns admission/WIP; the Branch Manager owns remote and
   Integration truth. Candidate, distribution, release, and accepted risk stay
   separate human decisions.

Human-readable current/next status:

```bash
node scripts/check-product-increment-portfolio.mjs \
  --portfolio docs/specs/product-increment-portfolio.json \
  --id AF-M01.4
```

Agents use the same command with `--json`; any temporary roadmap or
visualization projects that output rather than becoming another source of
truth. This loop needs no new dashboard, CLI command, heartbeat, or mobile-only
status schema.

## Review And Integration Truth

The first comprehensive Framework PR Review Agent pass inspected the exact
pre-fix implementation and returned `NO-GO`. Its critical finding was real:
otherwise valid product-authored audit, disposition, device, and store inputs
could carry top-level or nested `candidate_ready:true` or
`release_ready:true`, and raw staging copied those claims unchanged. The
current source strictly decodes each input, rejects malformed JSON and duplicate
object keys, requires every present readiness field to be the exact JSON
boolean `false`, reports the offending path/type/value, and stages only a
canonical serialization of the validated value. It deletes unsafe or stale
targets, recursively checks every retained U5 artifact, and makes both Wave 2
and the strict release consumer reject nested, type-confused, or duplicate-key
evidence independently. Producer guidance and checked-in fixtures are aligned
with the same non-authoritative contract. The exact-state comprehensive
rereview returned `GO WITH CONDITIONS`; its only finding is the already-routed
destination-current Integration proof condition.

The historical planning freeze remains based on `2de105e...`. Accepted
destination `750a58b1...` includes PR #464 and its successful exact-main
pipeline #385. Under Assignment `835555085`, Amendment r2 `835556572`, and
isolation-only Amendment r3 `835574654`, whose ninth authored root is limited
to device-tooling fixture backup/restore isolation, Integration merged the
exact immutable producer and reviewed descendant with history preserved.
Automatic overlap resolution retained the isolated PDS
expected-failure evidence behavior. Authority reconciliation, full docs,
generation, tests, handoff, manifest proof, and comprehensive review still
must pass on the final assembled SHA; the unpushed pre-reconciliation carrier
`d3e0be62...` is not itself acceptance evidence.

Branch-local accelerated proof is green: 53 focused mobile examples, 525 full
docs examples, CLI contracts (including the adversarial containment test),
framework validation, generated drift, fast tests, and offline dependency
posture all passed. The full docs run took 170393ms against an advisory,
unenforced 60000ms budget. The edited CRM Jest assertions were not executed
because the isolated mobile workspace has no `node_modules`; dependencies were
not installed merely to remove that declared pre-candidate evidence gap.

## Authority Order

When documents appear to conflict, use this order and preserve each document's
declared authority:

1. [Roadmap](../release/roadmap.md) governs priority and posture.
2. The [canonical Product Increment portfolio](../specs/product-increment-portfolio.json),
   [validated AF-M01.4 plan](../specs/af-m01.4-mobile-read.product-increment.json),
   and current authorized PFC Assignments govern live work, admission, and WIP.
3. [App Fabric Master Implementation Plan](../release/app-fabric-master-implementation-plan.md)
   governs integrated sequence, effort, dependencies, gates, and proof
   outcomes. It is explicitly not a live execution or admission ledger.
4. The [Hosted Product Factory specification](../specs/hosted-product-factory.md),
   especially its [mobile platform surfaces](../specs/hosted-product-factory.md#mobile-platform-surfaces),
   governs normative Fabric mobile decisions, contracts, and gates. The
   [Hosted Product Factory architecture](../architecture/hosted-product-factory-architecture.md)
   governs target management-plane design and authority boundaries.
5. [App Fabric Mobile UX Master Plan](../specs/app-fabric-mobile-ux-master-plan.md)
   is the task-sponsor-accepted technical plan and candidate backlog for
   app-side mobile capability. It is subordinate to items 1 through 4.
6. The proposed [M0 Seam-Freeze Record](../specs/app-fabric-mobile-m0-seam-freeze.md),
   [ADR 0017](../architecture/adr/0017-web-native-experience-projection-boundary.md),
   and the versioned planning schemas define the current technical seams.
7. The consulted `pds-technology-strategy` BOK at
   `046e532c0e41b6911171f3d9aa961c0898d66140` supplies draft strategy rules,
   companion direction, evidence, and open gates with their exact authority
   labels. Draft/evidence/decision-packet material is not silently promoted to
   ratified enterprise policy.

Wayne's narrow authorization and the resulting PFC Assignment admit only the
bounded M0-05 Integration lane. M0-02 is the next ready technical card but has
no WIP admission or source Assignment. This direction does not
grant Product acceptance, enterprise architecture ratification, broader
WIP/source ownership, funding, risk acceptance, candidate promotion, store
submission, release, main-merge, or direct-main push authority. Reviewed
Integration-branch push and PR creation remain gated by the assigned proof,
independent review, and repository guard.

## Decisions To Carry Forward

1. **Responsive web can be excellent mobile web, but not native by
   breakpoints alone.** Share semantic intent, data, auth/policy, state,
   accessibility intent, telemetry, and resumption. Use separate web/iOS/
   Android layout, navigation, interaction, lifecycle, device, and evidence
   projections.
2. **The Design System must expose real renderer modes.** Keep one semantic
   manifest with `web-dom`, `native-ios`, and `native-android` projections and
   `compact`, `medium`, `expanded` size classes. Run an actual React DOM catalog
   and an actual React Native catalog on both native platforms. Phone-shaped
   DOM previews are web evidence only.
3. **React Native plus Expo is the App Framework reference target.** Treat it
   as a reversible bounded proof against responsive web and other credible
   options, not an enterprise-wide mandate or evidence that a named product
   needs an installed app.
4. **The app is an untrusted public client.** Use OIDC Authorization Code plus
   PKCE, no embedded secret, short-lived access tokens held in memory, and only
   IdP-approved renewal material in OS-backed secure storage. Safe logout/
   renewal and server-side issuer/audience/client, tenant, policy, scope, time,
   and key validation remain authoritative. Local synthetic proof comes before
   enterprise non-production identity.
5. **Updates have two physics.** Native changes require a new signed binary;
   eligible JavaScript/assets may use a runtime-bound OTA lane. Production OTA
   never bypasses exact-source candidate/release evidence or named human
   promotion.
6. **Use simulators/emulators in local development.** They are the fast
   deterministic inner loop and CI-compatible development proof. They do not
   replace representative physical devices, real accessibility technology,
   signed builds, or store/MDM evidence.
7. **Build custody is parked, not assumed.** Prototype work is local,
   enterprise/developer-controlled, synthetic-only. Hosted, self-hosted, or
   split build/sign/update custody needs an explicit candidate decision.
8. **One complete cross-channel journey precedes broad screen generation.** A
   second independent consumer is required before candidate distribution and a
   third/economic comparison before scale claims.
9. **Acceleration changes proof timing, not truth.** Source hierarchy,
   generated drift, secrets, identity/tenant/policy, dependency integrity, and
   false-readiness controls remain blocking. Candidate/store/SRA/CAB/production
   evidence stays visibly deferred, never implied.
10. **Fabric identity is registry-minted.** Pending-registration planning
    fixtures use `app_id:null` and a namespaced, local-only
    `prototype_app_ref`. Product, generator, and framework code may not mint a
    durable `app_id` or emit authoritative fleet observations from that
    provisional reference.

## Retained Research And Planning Package

| M0 item | Artifact | Current disposition |
| --- | --- | --- |
| Master implementation plan | [Mobile UX Master Plan](../specs/app-fabric-mobile-ux-master-plan.md) | complete planning baseline; bounded M0-05 execution admitted |
| M0-00 options | [Technology Evaluation Brief](../specs/app-fabric-mobile-technology-evaluation-brief.md) | RN/Expo reference target retained for bounded comparison; business/channel decision open |
| M0-01 experience contract | [schema](../specs/app-fabric-experience-contract.schema.json) and [synthetic example](../specs/app-fabric-experience-contract.example.json) | planning contract v1; not canonical generator input |
| M0-01A renderer boundary | [ADR 0017](../architecture/adr/0017-web-native-experience-projection-boundary.md) | accepted for Prototype architecture; renderer implementation and qualification open |
| M0-02 API seam | [compatibility record](../specs/app-fabric-mobile-api-seam.md) and [GraphQL operations](../specs/app-fabric-mobile-api-compatibility.graphql) | wire decision recorded; accepted-main generated mobile client is incompatible; source-owned executable red test not yet admitted |
| M0-03 mobile topology and local auth | [profile schema](../specs/app-fabric-mobile-manifest-profile.schema.json), [profile example](../specs/app-fabric-mobile-manifest-profile.example.json), [auth schema](../specs/app-fabric-mobile-auth-profile.schema.json), and [auth example](../specs/app-fabric-mobile-auth-profile.example.json) | fail-closed opt-in/local-synthetic planning profiles; app-manifest/runtime integration and semantic/evidence checkers open |
| M0-04 build custody | [decision record](../specs/app-fabric-mobile-build-custody-decision.md) | Prototype local posture recorded; candidate decision parked |
| M0-05 false-readiness containment | `scripts/appfw`, strict evidence validation, focused regression, product fixtures/skill, introspection, and CLI/docs checks | immutable producer and reviewed descendant assembled on the assigned Integration carrier; reconciliation and destination-current Class D proof/review active; no PR or acceptance claim |
| M0 convergence | [Seam-Freeze Record](../specs/app-fabric-mobile-m0-seam-freeze.md) | proposed freeze package assembled; exact commit/Integration disposition, runtime implementation, and admission open |

The machine examples are synthetic and intentionally carry no real tenant,
identity, endpoint, business journey, distribution, or release authority.

## Critical Blocker Found

The accepted-main mobile generator is not compatible with its generated
backend:

- backend list results use `items`, `query_count`, cursor and page fields;
- generated mobile queries request `nodes`, `totalCount`, and `pageInfo`;
- generated metadata/client code carries semantic `JsonValue` onto the wire,
  while the backend GraphQL scalar is `JSON`;
- backend record lookup expects `String!`; generated mobile metadata emits
  `ID!`; and
- the current mobile test mocks and asserts the incorrect Relay-like response.

This blocks a real reference journey and every candidate/native-readiness
claim. Do not repair generated CRM files. The correction must establish a
canonical in-memory operation transport contract in `app_gen`, emit web/native
projections from it, validate source-owned GraphQL documents against the real
generated schema, and regenerate fixtures.

## Planning-Contract Promotion Conditions

The M0 JSON Schemas are deliberately non-executable planning contracts. Draft
2020-12 meta-validation and example validation prove shape and selected scalar
fail-closed posture only; they do not prove the cross-record semantics below.
These are important promotion conditions, not defects that block retaining the
current pre-candidate planning package:

- operating-context authority membership, exact named decisions/blockers, and
  Markdown/JSON/source synchronization need a repo-owned semantic checker;
- experience `prototype` readiness must bind to real source/build/renderer
  evidence, and Fabric classification must be conservatively derived rather
  than allowing a downgrade;
- native client profile IDs must be unique and exactly equal the auth server's
  accepted allowlist before server projection;
- manifest references must be safe-root, schema-identity, version, digest, and
  same-app bound, while ownership paths must reject absolute/traversal/symlink
  escape and generated/product overlap;
- channel disposition, capability/test feasibility, physical-device/auth
  profile alignment, unique journey/task/action IDs, and operation/evidence
  references need referential-integrity checks.

Until those checkers exist, none of these schemas may drive generator,
registry, auth-server, candidate, or release behavior.

## Next Highest-Value Move

The Integration Branch Manager now owns canonical card `M0-05` under exact
Assignment `835555085`, Amendment r2 `835556572`, and isolation-only Amendment
r3 `835574654`: complete the nine-root reconciliation against accepted PR #464
authority, with the ninth root limited to device-tooling fixture backup/restore
isolation, and run full Class D proof plus comprehensive review on the final
assembled SHA. The implementation
already:

- keeps aggregate candidate/release fields false while preserving the old
  conjunction as `legacy_evidence_satisfied`;
- strictly rejects raw audit, disposition, device, or store sources that are
  malformed, contain duplicate object keys, or contain any top-level or nested
  readiness field other than exact JSON boolean `false`; it reports the
  offending path/type/value, removes the staged target, and canonicalizes safe
  JSON before staging;
- emits explicit `contained-non-authoritative` M0-05 authority metadata;
- recursively proves every staged U5 artifact is contained;
- rejects forged, nested, duplicate-key, type-confused, or pre-containment
  legacy readiness in `wave2-status`; and
- makes strict release-evidence validation independently reject top-level and
  nested U5 readiness fields unless they are exact false, while also rejecting
  duplicate-key Wave 2 JSON.

M0-02 is the next ready technical card but is not WIP-admitted. Once an exact
Assignment names its accepted base, owner, worktree, write roots, reviewer, and
stop rule, it turns the proposed wire document into a source-owned executable
real-schema test retained red against the current generator. M0-05 must be
accepted before its result is treated as authoritative mobile evidence. Once
Integration records containment, that red proof, and the seam SHA, execute the
API/operation-transport slice of canonical card `M1-01` to make it green:

```text
normalized model + backend wire conventions
                 |
                 v
canonical operation transport IR in app_gen
          |                         |
          v                         v
   web client projection     native client projection
          \_________________________/
                     |
                     v
 real generated-schema validation + backend execution
```

Each Assignment must name the accepted base, producer, reviewer, worktree,
write roots, generator and fixture ownership, red/green tests, stop rule, and
integration destination. M1-01 must remove mobile generation's dependence on
generated web TypeScript and prior generated mobile output. Broader shell, PDS
Native, identity, and journey lanes consume this seam; they must not invent an
API workaround in parallel.

Recommended first producer family after the API/experience seams receive exact
freeze SHAs:

- generator/API normalization (`M1`);
- minimum PDS Native plus actual native catalog (`M2`); and
- native shell/runtime (`M3`).

At most three disjoint source producers are a planning ceiling, not a WIP
decision. M4a identity follows accepted M3; M5 journey waits for accepted M1,
M2, M3, and M4a seams.

## Hard Stops

Stop and route a decision before:

- real PHI/PII or tenant data, production identity, live provider writes, or a
  regulated/safety-significant journey enters the lane;
- TestFlight, Play internal/closed, MDM, external preview, store submission,
  production OTA, or production-capable signing material is used;
- a shared enterprise shell, support/package promise, enterprise mobile
  standard, candidate/qualified/released claim, or accepted risk is proposed;
- a generated file or retained target report becomes canonical source;
- the API mismatch remains hidden by mocks or static-only evidence; or
- work would mutate an unassigned root or consume another worktree's
  uncommitted state.

## Clean Continuation Checklist

1. Read the `AF-M01.4` portfolio entry and validated plan for current/next
   truth; use this guide for durable technical context and treat its JSON
   companion as the immutable 2026-08-03 snapshot.
2. Keep `fix/app-fabric-mobile-readiness-containment@a28bee04...` immutable.
   The active Branch Manager has consumed the reviewed dependent progress
   carrier history-preservingly; no separate PR is created for it.
3. Continue only on `integrate/app-fabric-mobile-m0`, frozen from accepted
   `origin/main@750a58b1ff78f9cd76f7992ca5dd04a0bde6dc3e` with both immutable
   inputs merged history-preservingly. Complete the assigned reconciliation
   and destination-current Class D proof there; do not silently rebase or
   mutate a producer lane.
4. Treat M0-05 as the only admitted mobile source scope. M0-02 is `Ready next`,
   not active; require a new exact Assignment and PFC admission before source
   work starts.
5. Use isolated worktrees, preserve unrelated human changes, run focused
   accelerated proof on leaves, and run aggregate proof on the assembled SHA.
6. Keep `candidate_ready:false`, `release_ready:false`, and
   `release_authority:none` until the explicit graduation decision and full
   evidence say otherwise.

## Role Card Check

- **Card used:** Architect Agent plus Coding Agent for the immutable producer
  and bounded context descendant; Integration Branch Manager for the current
  nine-root reconciliation and Class D carrier; comprehensive Framework PR
  Review Agent route retained.
- **Within role:** architecture continuity, Product Increment technical slicing,
  truthful progress routing, immutable-input preservation, assigned
  reconciliation, aggregate proof, and exact-SHA promotion evidence.
- **Authority not assumed:** additional roadmap priority or WIP, Product
  acceptance, enterprise ratification, funding, risk, candidate, release,
  main merge, or direct release.
- **Routed:** value and acceptance to Product; technical seam fitness to Fabric
  Architecture; WIP admission to the Program Flow Controller; exact branch,
  PR, and pipeline truth to Integration; main-merge judgment to Wayne;
  security, privacy, build/store, finance, support, release, and accepted risk
  to named human authorities.
- **Drift signal:** `watch`; the single Product Increment system is the live
  progress authority, while full assembled-SHA proof/review and broader
  implementation/enterprise decisions remain open.
