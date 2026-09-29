# React Native Mobile App Contract

This guide is the App Framework contract for adding native mobile products with
React Native and Expo. It preserves the mobile research and architecture
decision made on 2026-07-01: **React Native + Expo is the primary mobile target
to implement; PWA is a responsive web fallback and optional installable web
surface, not the mobile strategy.**

That is an App Framework reference-implementation decision, not an
enterprise-wide mobile standard or a decision that every business journey
needs an installed app. The program-wide architecture, comparisons, evidence
sequence, and current research/planning continuation context live in the
[App Fabric Mobile UX Master Plan](../specs/app-fabric-mobile-ux-master-plan.md),
[Technology Evaluation Brief](../specs/app-fabric-mobile-technology-evaluation-brief.md),
and [Mobile Operating Context](../start/app-fabric-mobile-operating-context.md).
Live work and execution state remain in the canonical Product Increment plan
and Program Flow Controller topology.

Use this guide when a product team has a mobile HTML mockup, mobile workflow
brief, screenshots, or a citizen-developed mobile prototype and needs an
enterprise React Native app that still follows App Framework's contract-first,
evidence-gated, generated-boundary model.

Agent procedure: use the `product-mobile-react-native` skill and retain the
`mobile-plan` artifact before creating or changing product-owned `mobile/`
source.

## Current Decision

The mobile target is React Native with Expo:

- React Native New Architecture is the baseline assumption for generated mobile
  apps. It is default in modern React Native releases, provides type-safe native
  module/codegen paths, removes the old bridge from the primary path, and is
  documented as production-ready by the React Native team.
- Expo is the default application framework and development-tooling target. Use
  Expo Router for file-based native navigation unless a product has a
  documented exception. Build, signing, submission, and update-service custody
  remain separate decisions.
- EAS Build, Submit, and Update are the documented reference publish path and
  checked-in configuration scaffold, not an authorization to upload enterprise
  source or credentials, use hosted signing, distribute a candidate, or enable
  production OTA. The
  [build-custody decision](../specs/app-fabric-mobile-build-custody-decision.md)
  must be resolved before those tripwires.
- PWA support remains a web quality expectation for product frontends, but PWA
  evidence does not prove native mobile UX.

External sources checked 2026-07-01:

- React Native New Architecture:
  <https://reactnative.dev/blog/2024/10/23/the-new-architecture-is-here>
- Expo Router:
  <https://docs.expo.dev/router/introduction/>
- EAS Build:
  <https://docs.expo.dev/build/introduction/>
- EAS Submit:
  <https://docs.expo.dev/submit/introduction/>
- EAS Update:
  <https://docs.expo.dev/eas-update/introduction/>
- Expo AuthSession:
  <https://docs.expo.dev/versions/latest/sdk/auth-session/>
- Expo SecureStore:
  <https://docs.expo.dev/versions/latest/sdk/securestore/>
- Expo push notifications:
  <https://docs.expo.dev/push-notifications/overview/>
- Expo development builds:
  <https://docs.expo.dev/develop/development-builds/introduction/>
- Expo Android Emulator:
  <https://docs.expo.dev/workflow/android-studio-emulator/>
- Expo iOS Simulator:
  <https://docs.expo.dev/workflow/ios-simulator/>
- Expo unit testing with Jest:
  <https://docs.expo.dev/develop/unit-testing/>
- Expo E2E tests with Maestro:
  <https://docs.expo.dev/eas/workflows/examples/e2e-tests/>

## Relationship To Web

Mobile and web share product intent. They do not share layout trees.

The durable rule:

```text
Generate web and mobile from the same product experience contract.
Do not generate mobile from the web component hierarchy.
```

Shared across web and mobile:

- `.appfw/model` entities, relationships, standard methods, custom methods,
  validation, policies, data classifications, and API scenarios.
- generated GraphQL/API client shape and UI/experience metadata.
- auth, tenant, policy-denied, validation, request ID, correlation ID, and
  audit semantics.
- PDS Health design tokens through platform-specific adapters.
- workflow identity: "approve request", "review task", "edit account",
  "view audit", "search record".

Platform-specific to mobile:

- native navigation: tabs, stacks, sheets, modals, deep links, back behavior,
  and notification launches;
- screen composition, touch density, gesture affordances, safe areas, keyboard
  avoidance, haptics, and app lifecycle handling;
- secure storage, biometrics, local cache/offline policy, push notifications,
  camera/file/share integrations, and OS permissions;
- mobile-specific release evidence and store metadata.

Example mapping:

| Product Intent | Web Projection | Mobile Projection |
| --- | --- | --- |
| Work queue | sidebar plus dense table, filters, bulk actions | `Today` tab with prioritized cards, search, pull-to-refresh, action sheets |
| Record detail | split panel or wide detail route | stack detail screen with sections, sticky primary action, native back |
| Edit form | multi-column form and inline validation | single-column form, keyboard-aware layout, field groups, save sheet |
| Governed write | command surface plus confirm dialog | intent preview screen/sheet, biometric or explicit confirmation when required, audit result |
| Audit/activity | side panel or page | activity tab, push deep links, offline-safe status states |

## Source Inputs

Before writing React Native code, inspect these sources:

| Source | Purpose |
| --- | --- |
| `.appfw/model` | Product entities, relationships, policies, classifications, custom methods, and API scenarios. |
| generated GraphQL schema and client | Operation names, variables, response shapes, pagination, filters, validation, and policy errors. |
| `mobile/src/generated/appfw-mobile-contract.ts` | Canonical generated mobile API/experience input. Web-generated TypeScript may be inspected only as migration or diagnostic evidence; it is never canonical mobile input. |
| `docs/frontend/product-frontend.md` | Web frontend ownership boundary and shared generated-contract assumptions. |
| `docs/frontend/pds-health-design-system.md` | PDS token and component governance. |
| mobile HTML mockup or screenshots | Visual and workflow evidence only. Do not copy DOM, CSS, mock data, or local-only state. |
| `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json` | Retained source hash, checklist, and conversion evidence for a mobile mockup. |

## Workspace Shape

A downstream product mobile app should converge on this logical layout:

```text
mobile/
|-- app/
|   |-- _layout.tsx
|   |-- index.tsx
|   |-- tasks/
|   |-- entities/
|   |-- activity/
|   `-- settings/
|-- src/
|   |-- generated/
|   |   |-- appfw-mobile-contract.ts
|   |   `-- graphql/
|   |-- features/
|   |-- components/
|   |-- design/
|   |   `-- pdsNativeTokens.ts
|   |-- lib/
|   |   |-- appfwClient.ts
|   |   |-- authSession.ts
|   |   |-- tenantContext.ts
|   |   |-- secureStorage.ts
|   |   `-- notifications.ts
|   `-- test/
|-- .appfw-mobile/
|   |-- ownership.json
|   |-- scaffold-manifest.json
|   |-- typecheck-evidence.json
|   |-- test-evidence.json
|   |-- expo-doctor-evidence.json
|   |-- npm-audit-evidence.json
|   |-- npm-audit-disposition.template.json
|   |-- device-tooling-evidence.json
|   |-- device-evidence.json
|   `-- store-track-evidence.json
|-- app.json
|-- eas.json
|-- package.json
|-- package-lock.json
`-- tsconfig.json
```

`app_gen` emits the generated mobile contract, PDS native tokens, entity screen
and route shells, client, ownership metadata, and scaffold manifest. Product
agents own the remaining workflow source while preserving these boundary names;
generator changes must recreate generated files deterministically and preserve
human-owned files.

## Ownership Boundary

Generated and overwrite-safe:

- `mobile/src/generated/appfw-mobile-contract.ts`;
- `mobile/src/generated/graphql/`;
- generated route registry and entity/action metadata;
- `mobile/app/entities/**` generated entity route shells;
- PDS native token adapter generated from canonical PDS token source;
- `mobile/.appfw-mobile/ownership.json`;
- `mobile/.appfw-mobile/scaffold-manifest.json`;
- default screen skeletons for generated entity/workflow routes when the target
  is implemented.

Human-owned and preserved:

- `mobile/app/**`, except the generated `mobile/app/entities/**` entity route
  shell subtree and generated route registry imports marked in ownership
  metadata;
- `mobile/src/features/` product workflow screens;
- `mobile/src/components/` product-specific composites;
- `mobile/src/lib/` app-specific native adapters, auth wiring, secure-storage
  policy, notification handlers, and offline sync glue;
- mobile tests, copy, screenshots, and store metadata.

Do not place product mobile behavior in `admin_ui`, the web product
`frontend/`, or framework-owned `app_gen` templates from a product branch.

## HTML Mockup Conversion

Run the retained planning command first:

```bash
scripts/appfw product mobile-plan --ui-artifact prototype.html --json
```

The command writes:

```text
.appfw/target/appfw/mobile-rn-conversion-plan.json
```

Then convert the mockup by procedure:

1. Extract screens, navigation intent, modal/sheet behavior, gestures,
   visible data, actions, state variants, and device assumptions.
2. Map every screen and action to `.appfw/model`, generated operations, product
   services, or documented integration boundaries.
3. Decide native navigation by workflow archetype. Do not mirror desktop
   sidebars, dense tables, or split panels by default.
4. Translate visual style to PDS native tokens. Do not copy CSS declarations
   blindly; React Native layout, typography, safe areas, and touch targets are
   platform-native surfaces.
5. Replace mock data with typed client calls and explicit loading, empty,
   validation, policy-denied, auth, provider, offline, and unexpected-error
   states.
6. Add auth, tenant context, request/correlation IDs, denied-state UX, and
   audit surfaces before claiming the screen is product-ready.
7. Gate secure storage, biometrics, local cache, file/device APIs, and push
   notifications with data-classification, retention, and platform-permission
   decisions.

The mockup is evidence of intent. It is not implementation source.

## Navigation Defaults

Use mobile-native defaults by app archetype:

| Archetype | Default Mobile Navigation |
| --- | --- |
| Task/approval app | bottom tabs: `Today`, `Work`, `Search`, `Activity`, `Settings`; stack detail screens; action preview sheet |
| Entity CRUD app | tabs: `Home`, primary entity/work queue, `Search`, `Activity`, `Settings`; generated entity stacks |
| Dashboard-heavy app | `Home` summary, alert list, drill-in stacks, saved views, refresh controls |
| Governed-write app | intent preview, explicit confirmation, action result, audit/undo status, notification deep links |
| Field/file workflow | camera/file/share flows behind explicit permission and data-classification gates |

Generated mobile navigation should start from product model metadata, workflow
importance, standard/custom methods, permissions, and manifest topology. Product
teams may override route grouping in a product-owned mobile navigation config,
but route identifiers must remain traceable to generated contract entries.

## Security And Data Governance

Mobile clients are data surfaces. Treat local persistence as a gated capability:

- Access tokens stay in memory. Renewal material, when an IdP-approved flow
  requires it, may use OS-backed secure storage under an explicit lifecycle and
  data-classification decision. No client secret or signing key is embedded in
  mobile source or binary.
- SecureStore is allowed for small, appropriate values only; large or regulated
  payloads need an explicit storage decision.
- PHI/ePHI or sensitive tenant data must not be cached offline until retention,
  deletion, tombstone, re-authentication, and device-loss behavior are
  documented and evidenced.
- Push notification payloads must avoid sensitive content unless a release
  authority explicitly approves the data class.
- The mobile UI reflects backend authorization and fails closed on policy
  errors; it never becomes the authorization authority.
- Native permissions must be declared, justified, and tested. A new permission
  means a new binary release, not an EAS Update-only release.

## Development And Test Environment

Use Expo development builds for product mobile work. Expo Go is useful for
learning and quick experiments, but a production-grade App Framework mobile app
must test the native app it will ship: app identity, splash assets,
permissions, deep links, push configuration, native dependencies, and runtime
version behavior.

The normal local loop is:

```bash
cd mobile
npx expo start --dev-client
```

Install a development build on the target simulator, emulator, or device before
starting that loop. The Prototype-safe default is a local,
enterprise/developer-controlled build. EAS Cloud or another hosted builder may
be used only after the applicable source-egress and credential-custody owner
authorizes its bounded scope.

Simulator and emulator guidance:

- iOS Simulator requires macOS with Xcode and an installed iOS simulator. It is
  good for rapid screen, navigation, layout, and basic API checks, but it does
  not cover all hardware or background behavior.
- Android Emulator comes from Android Studio, Android SDK, and Android Virtual
  Device tooling. Start with a current Pixel-class device profile unless the
  product has a more specific device requirement.
- Physical devices are required before release for push notifications,
  biometrics, camera/file workflows, background lifecycle, MDM posture, real
  networking, and any workflow tied to hardware or OS permission behavior.

Do not put a developer's local simulator name, emulator ID, Xcode device UUID,
or Android Virtual Device name into `.appfw/model`. Those are machine-local
tooling facts. App Framework configuration should describe the required mobile
capability and evidence matrix; the CLI should detect the local tools that can
satisfy it.

The product mobile contract should eventually support configuration shaped like
this:

```yaml
mobile:
  target: react-native-expo
  platforms:
    - ios
    - android
  test_matrix:
    local_smoke:
      - ios_simulator
      - android_emulator
    release_required:
      - ios_physical_or_testflight
      - android_physical_or_play_internal
    device_required_capabilities:
      - push_notifications
      - biometrics
      - deep_links
      - secure_storage
      - offline_resume
```

This is a target contract, not a current generator schema. Until it exists,
record the same intent in the retained mobile plan, product mobile docs, and
handoff evidence.

Local API reachability is platform-specific:

- iOS Simulator can usually reach a backend on the developer machine through
  `localhost`.
- Android Emulator usually reaches the developer machine through `10.0.2.2`.
- Physical devices need a LAN-reachable, tunneled, or deployed HTTPS endpoint.

Generate environment profiles for those API base URLs, but keep secrets,
refresh tokens, signing material, and tenant data out of committed mobile
configuration.

Testing layers:

- static scaffold diagnostics: `scripts/appfw product mobile-test --json`,
  which is non-authoritative and keeps candidate/release readiness false;
- dependency reproducibility: commit `mobile/package-lock.json` and run
  `npm ci`, not an unpinned install, in CI and release lanes;
- local retained evidence: `scripts/appfw product mobile-test --run-local --json`
  after `mobile/` dependencies are installed, which runs `npm run typecheck`,
  `npm run test`, `npm run doctor`, and `npm audit --omit=dev --json`;
- dependency assurance: act on the retained runtime audit evidence by updating
  the SDK/dependencies or retaining a governed decision for remaining Expo/RN
  chain findings before the future source-bound candidate checker evaluates the
  app;
- current CRM reference posture: the Wave 2 scaffold uses Expo SDK 57 with a
  deterministic lockfile, React `19.2.3`, and React Native `0.85.3`. Expo
  Doctor currently prefers React Native `0.86.0`, but `jest-expo@57.0.0` still
  resolves cleanly with the `0.85` Jest preset chain. The scaffold retains
  local `mobile-test --run-local` evidence with the React Native install check
  excluded and the directory check disabled; rerun Doctor with required network
  and directory checks before a future source-bound candidate checker consumes
  the observation.
- runtime-audit disposition: if runtime audit findings remain, retain
  `mobile/.appfw-mobile/npm-audit-disposition.json` with an explicit,
  release-approved, unexpired decision tied to
  `mobile/.appfw-mobile/npm-audit-evidence.json`. A checked-in
  `npm-audit-disposition.template.json` may document retained findings and
  promotion steps, but it is not release evidence;
- local tooling posture: `scripts/appfw product mobile-test --device-preflight
  --json`, which records iOS simulator, Android emulator, EAS, and Maestro tool
  availability without claiming device certification;
- TypeScript and lint checks from `mobile/package.json`;
- Jest with `jest-expo` and React Native Testing Library for units,
  components, and route-level integration tests;
- `npx expo-doctor` for Expo dependency/config health;
- simulator/emulator smoke tests for navigation, layout, auth, API reachability,
  denied states, validation, and degraded/offline behavior;
- Maestro E2E flows locally and/or in EAS Workflows for release-significant
  happy paths and governed-write confirmation flows;
- physical-device evidence before release for push, biometrics, deep links,
  secure storage, background behavior, and permission prompts.

The future source-bound mobile checker owns candidate evidence derived from
these layers. Named human authorities—not `mobile-test`, the checker, or staged
U5 inputs—own distribution, OTA, store/MDM, and release decisions.

Future source-bound candidate tooling should extend the existing diagnostic
preflight into retained simulator/device execution while continuing to detect
Xcode, `xcrun simctl`, Android SDK, `adb`, emulator availability, Expo CLI/EAS
CLI, Maestro, and device connectivity. It should report missing local tooling
as evidence, not mutate `.appfw/model` or infer human distribution authority.

## Publish Path

This section describes future lane semantics, not current distribution or
vendor authorization. During the present Prototype posture only local
development builds with synthetic data and development credentials are in
scope. TestFlight, Play internal/closed, MDM, hosted source/signing custody,
production-capable credentials, and production OTA are candidate tripwires.

Future test and candidate lanes:

- `development`: Expo development build for local device/simulator debugging.
- `preview`: approved QA/UAT build distributed through an internal mechanism
  that is not a TestFlight, Play, MDM, or production OTA channel.
- `production-candidate`: signed build submitted to TestFlight and Play testing
  tracks with release evidence retained.

Potential production lanes after the build-custody, security, store, support,
candidate, and release decisions are approved:

- iOS: an approved hosted, enterprise, or split build/signing path produces the
  signed `.ipa`; an approved submission path uploads it to App Store Connect;
  production release still goes through App Review.
- Android: an approved hosted, enterprise, or split build/signing path produces
  the signed Android App Bundle; an approved submission path uploads it to the
  selected Play track; production release uses Play rollout controls.
- An approved OTA service may patch JavaScript, styling, and assets only when
  runtime and API compatibility are intact. Native code, permissions, SDK
  changes, entitlements, and native dependencies require a new binary.

Release evidence should retain build IDs, runtime version, update channel,
binary digest, signing identity, store track, tested device matrix, store
metadata status, and mobile-specific security decisions.

## Replacing Template Evidence

The checked-in CRM mobile evidence may include template or local-preflight
files so the verifier has stable paths to inspect. Those files are intentionally
not release evidence. Replace them only with evidence produced from the target
app build:

| Placeholder | Replace With |
| --- | --- |
| `mobile/.appfw-mobile/device-evidence.json` | simulator or physical-device smoke evidence from the signed or development build, including platform, OS version, device/simulator name, app build identifier, runtime version, workflow matrix, auth/tenant/policy-denied checks, and request/correlation ID visibility |
| `mobile/.appfw-mobile/store-track-evidence.json` | TestFlight, Google Play internal/closed track, or enterprise distribution evidence, including submitted build ID, signed binary digest, signing identity, runtime version, update channel, track name, and store metadata status |
| `mobile/.appfw-mobile/npm-audit-disposition.template.json` | `mobile/.appfw-mobile/npm-audit-disposition.json` only when release/security authority approves retained runtime audit findings and the disposition matches the current `npm-audit-evidence.json` summary |

All compatibility-named inputs must retain `candidate_ready:false` and
`release_ready:false`, including accurate non-placeholder observations.
Template-only, static-scaffold, not-run, or placeholder records additionally
fail their legacy diagnostic conditions. Local `device-preflight` evidence is
useful for planning, but it proves tooling availability only; it is not a
substitute for running the app on the target simulator, device, or
store/enterprise track, and none of these legacy inputs grants promotion
authority.

## Verification

Before handoff, run the product checks that apply to the current implementation
state:

```bash
scripts/appfw product mobile-plan --ui-artifact prototype.html --json
scripts/appfw product validate --json
scripts/appfw product generate --check --json
cd mobile
npm ci
npm run typecheck
npm run test
npm run doctor
npm audit --omit=dev --json
cd ..
scripts/appfw product mobile-test --run-local --json
scripts/appfw product mobile-test --device-preflight --json
scripts/appfw product handoff --json
```

The static verifier exists now and retains missing-workspace evidence, but it
is intentionally **non-authoritative**. M0-05 containment forces every
`mobile-test` invocation to emit `candidate_ready:false`,
`release_ready:false`, `release_authority:"none"`, and
`readiness_authority.status:"contained-non-authoritative"`. The old conjunction
is retained only as `legacy_evidence_satisfied`; stale or forged legacy true
reports are rejected by Wave 2 and strict release-evidence consumers. No
`mobile-test` combination may advance candidate, release, distribution, OTA,
or store/MDM state. Its checks are diagnostics for scaffold and evidence
planning only; they do not cover real API/auth, both platforms, exact-source
provenance, Fabric D2 authorities, update recovery, or comprehensive review.
After mobile dependencies are installed, `scripts/appfw product mobile-test
--run-local --json` records the local npm/Expo checks in
`mobile/.appfw-mobile/typecheck-evidence.json`,
`mobile/.appfw-mobile/test-evidence.json`, and
`mobile/.appfw-mobile/expo-doctor-evidence.json`; it also records runtime audit
posture in `mobile/.appfw-mobile/npm-audit-evidence.json`. `mobile-test`
surfaces the retained audit evidence status, exit code, vulnerability summaries
from fresh audit runs or preserved prior summaries when the audit endpoint is
unavailable, remediation counts for available and semver-major fixes, direct
affected packages, and `disposition_required:true` when audit findings remain.
It validates an optional disposition at
`mobile/.appfw-mobile/npm-audit-disposition.json` only as a legacy diagnostic;
no disposition can unlock candidate or release readiness in `mobile-test`.
Retained audit evidence must still exist, validate, and match
`current_evidence_summary`. Placeholder authority fields, template-only markers,
or template rationale text keep the diagnostic disposition invalid. Local test
evidence must execute real tests, not only pass with `No tests found`, and Expo
Doctor evidence that ignored Expo API network failures does not satisfy the
legacy diagnostic condition,
and satisfying this legacy verifier only changes
`legacy_evidence_satisfied`; candidate and release fields stay false.
The device preflight mode
records local simulator/emulator/store/E2E tooling posture in
`mobile/.appfw-mobile/device-tooling-evidence.json`. Passing typecheck, tests,
Expo doctor, and device tooling preflight still is not device certification;
simulator/physical-device evidence in `mobile/.appfw-mobile/device-evidence.json`,
store-track evidence in `mobile/.appfw-mobile/store-track-evidence.json`, and
native-capability evidence remain distinct candidate inputs and human-release
obligations. Placeholder
`device-evidence.json` and `store-track-evidence.json` files may document the
required evidence location and verifier contract, but `status:not-run`,
`static_scaffold_only:true`, `template_only:true`, or `release_ready:false`
means they do not satisfy even the legacy condition. Valid legacy inputs still
cannot change top-level candidate/release readiness.
The current CRM
scaffold has passing retained typecheck and real unit-test evidence; Expo Doctor
still records an Expo API network-bypass, so it remains non-release-grade until
rerun with network access. A passing local `mobile-test --run-local` also does
not override dependency-audit disposition; the retained audit details preserve
10 moderate runtime Expo-chain findings from the last successful audit, and the
latest audit attempt records `audit_unavailable`. Rerun with registry access,
then upgrade away or formally disposition any remaining findings before a
future source-bound candidate checker can consider the evidence.
`scripts/appfw framework wave2-status --json` now strictly rejects malformed or
duplicate-key legacy mobile reports and any present `candidate_ready` or
`release_ready` field that is not the exact JSON boolean `false`; strict
retained-evidence validation independently enforces the same boundary. Neither
is the future M6b source-bound mobile candidate checker.
The current mobile generator target emits the first framework-owned React Native
surfaces. It writes/checks the generated mobile contract, PDS native token
bridge, reusable generated entity screen, generated policy-aware GraphQL data
client, all primary entity Expo Router route shells, ownership metadata, and
scaffold manifest; it does not yet emit product-owned native workflow screens,
endpoint/auth runtime wiring, or simulator/device evidence. It then delegates to
`mobile-test` and records whether the product-owned scaffold and retained
evidence are present:

```bash
scripts/appfw product generate --target mobile-rn --json
scripts/appfw product generate --target mobile-rn --check --json
scripts/appfw product mobile-test --json
```

The target writes `.appfw/target/appfw/mobile-rn-generate.json` with
`generator_status:"all-entity-route-shells-emitted"` and `write_actions` for
the generated files. In `--check` mode, matching generated files report
`action:"verified"` and drift returns a non-zero exit. The full generator is not
complete until product-owned native workflows wire endpoint/auth context into
the generated mobile data client, those workflows preserve the same
loading/empty/policy-denied and validation posture as the web frontend, and npm
execution plus simulator/device evidence are retained.
