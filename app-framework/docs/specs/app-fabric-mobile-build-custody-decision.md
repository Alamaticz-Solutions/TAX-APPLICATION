# App Fabric Mobile Build Custody Decision Record

| Field | Value |
| --- | --- |
| Record | `AF-MOBILE-M0-04` |
| Lifecycle position | Explore / Prototype, pre-candidate |
| Status | Candidate decision parked; Prototype-safe provisional posture recorded |
| Provisional posture | Enterprise/developer-controlled local builds with synthetic data and development credentials only |
| Assessment date | 2026-08-03 |
| App Framework baseline | accepted `origin/main@2de105e001ab8029eb21bce4691eded49e48ff81`; this record and its master plan are branch-local planning additions |
| Strategy baseline | `pds-technology-strategy` commit `046e532c0e41b6911171f3d9aa961c0898d66140`; deployment/upstream currency not proved |
| Decision authority | None. This record neither authorizes source/secret egress nor approves a vendor, contract, signing model, store submission, candidate build, OTA service, or release. |

## Decision to be made

Before candidate distribution, the accountable authorities must decide where
mobile source is assembled, where native builds run, who controls signing and
submission credentials, how JavaScript/asset updates are signed and delivered,
and how the organization proves provenance, recovery, revocation, continuity,
and exit.

The decision must be explicit per platform and lane. “Use Expo” is not a custody
decision: React Native and Expo tooling can participate in cloud, local, and
split operating paths, each with different source, credential, service, and
recovery boundaries.

This record implements M0-04 of
[the App Fabric mobile UX master plan](./app-fabric-mobile-ux-master-plan.md)
and is subordinate to that plan's Prototype boundaries. It is paired with
[the mobile technology evaluation brief](./app-fabric-mobile-technology-evaluation-brief.md).

## Authority and strategy binding

| Source | Version / baseline | Authority used here | Relevance |
| --- | --- | --- | --- |
| Enterprise Technology Strategy 2026–2030 | `2026-07-31-draft.3` at the bound strategy commit | Governing within the authored BOK; draft and not recorded here as ratified | DR-02 requires authority/exit evaluation; DR-03 requires trust/recovery; DR-05 requires representative evidence |
| Machine-readable application contract | `2026-07-31-draft.3` | Machine contract; draft | Requires source-bound application and evidence identity; conditional trust/economics groups remain trigger-driven during Explore |
| Application Fabric Strategic Direction | `2026-08-01-draft.2` | Companion; pending major decision; no governing effect | Calls for pinned compatibility, rollout rings, rollback, failure containment, support, and a clear management/runtime dependency boundary |
| Mobile evidence review | `2026-08-01-evidence.2` | Evidence only; no approval authority | Identifies build, signing, update, store, support, and recovery as unproved parts of the reference architecture |
| Mobile and Application Fabric decision packets | Unversioned at the bound commit; pending major decisions | Operating/release-excluded; no governing effect | Preserve the unresolved channel and common-Fabric choices |
| Application Fabric strategic-direction and strategy/EA-interface gates | Open at the bound commit | Open gates | This record cannot ratify the Fabric or enterprise architecture posture |

The repository currently contains development, preview, and production-candidate
profiles in `examples/products/crm/mobile/eas.json`. Their presence is
configuration evidence only. It does not prove an Expo organization, service
entitlement, build runner, credential source, update service, store account,
contract, security review, or successful signed build. The master plan records
all of those operational concerns as unresolved.

## Custody surface

The final decision must locate an accountable owner, system of custody, access
policy, retention policy, recovery path, and evidence record for every asset
below:

| Asset or activity | Custody questions that must be answered |
| --- | --- |
| Source and dependency metadata | What files leave the enterprise boundary; to which tenant, region, processor, and subprocessors; under what encryption, access, retention, deletion, legal, and incident terms? |
| Build environment | Who controls runner images, native SDKs, package registries, caches, environment variables, network egress, logs, and reproducibility? |
| Build-time secrets | Which secrets are required, how are they materialized, masked, rotated, revoked, audited, and prevented from entering source, bundles, logs, or caches? |
| Apple signing assets | Who owns certificates, provisioning profiles, App Store Connect roles/API keys, bundle identifiers, revocation, renewal, and account recovery? |
| Android signing assets | Who owns upload/app-signing keys, Play Console roles/API credentials, key upgrade/recovery, package names, and emergency revocation? |
| Development and preview credentials | How are lower-assurance credentials isolated from candidate/production credentials and prevented from widening distribution? |
| OTA update signing | Who controls the private key, certificate rotation, runtime/channel policy, emergency halt, source-bound forward fix or proved embedded fallback, and proof that an update cannot cross native/API/data compatibility bounds? |
| Artifacts and evidence | Where binaries, symbols/source maps, manifests, SBOMs, attestations, logs, test evidence, approvals, and hashes are retained and linked to exact source/config/dependency identities? |
| Submission and promotion | Which human/service roles can build, sign, submit, promote, halt, replace/recover, or retire; what separation of duties and break-glass recovery applies? |
| Service continuity and exit | How can builds, signing, updates, and release recovery continue during vendor outage, account lockout, contract exit, compromised key, staff turnover, or lost runner? |

## Credible custody paths

No path is approved by this record. The matrix identifies what must be proved,
not a weighted vendor score.

| Path | Operating shape | Strengths | Material risks and unresolved proof | Current disposition |
| --- | --- | --- | --- | --- |
| EAS Cloud build with EAS-hosted/managed credentials | Source archive and dependency metadata are uploaded to Expo-hosted build infrastructure; eligible credentials are stored and used through the Expo service; EAS-hosted update/distribution capabilities may also be selected | Lowest runner/toolchain setup burden; integrated Expo build and credential workflows; can accelerate bounded development and preview evidence | Third-party source and credential custody; tenant/role configuration; region, encryption, retention/deletion, logs, subprocessors, audit, incident, availability, recovery, contract, entitlement, exit, provenance, and organization-account ownership require review. Service convenience must not be mistaken for enterprise approval | Credible option; **not authorized** for enterprise source, secrets, signing, or candidate use by this record |
| Enterprise/self-hosted build and signing | Native/Expo-compatible builds run on enterprise-controlled developer or CI infrastructure; secrets/signing assets are materialized from enterprise custody; enterprise roles submit/promote | Strongest direct control over source egress, runners, keys, logs, network, attestations, recovery, and policy integration | Organization owns macOS and Android runner capacity, SDK/JDK/Node/fastlane/CocoaPods maintenance, patching, caching, reproducibility, availability, support, and specialist skills. EAS local mode still has documented service communication and feature differences that must be understood if used | **Prototype-safe provisional posture** for local proof; credible candidate option pending operational and economic evidence |
| Split build/signing path | Examples include cloud-built non-distributable/dev artifacts with candidate builds/signing/submission on enterprise infrastructure, or enterprise-controlled builds with an independently selected hosted update service and customer-held update-signing key | Can place higher-risk candidate credentials and promotion authority under enterprise control while retaining selected managed development/update convenience | Platform feasibility differs: iOS and Android packaging/signing steps cannot be abstractly separated. Cloud source egress may remain even when credentials are local; official EAS documentation says local credentials supplied to cloud builds are uploaded for the job and then disposed. Two paths increase parity, provenance, support, and disaster-recovery burden | Leading architecture to **evaluate**, not a selected candidate posture; prove each platform/lane boundary end to end |

“Split” must be diagrammed as actual custody transitions. A label such as
“customer-managed keys” is insufficient if a key, password, source archive, or
signing request still enters a hosted job. Conversely, a self-hosted runner is
not sufficient if its identity, network, log, cache, patch, or recovery custody
is undefined.

## Prototype-safe provisional posture

To maximize pre-candidate progress without pre-deciding enterprise custody:

1. Run local simulator/emulator and, where needed, local native development
   builds on developer- or enterprise-controlled infrastructure.
2. Use synthetic/non-sensitive identities and data, development-only package
   identifiers where practical, and development credentials that cannot sign or
   submit a candidate/production application.
3. Do not place organization production certificates, profiles, keys, store
   credentials, enterprise secrets, PHI/PII, real tenant data, or regulated
   source material in the reference lane.
4. Keep signing files and credential descriptors out of version control.
   Materialize any bounded development credential through an approved local
   secret mechanism, with least privilege and an owner, expiry, and revocation
   path.
5. Treat the checked-in EAS profiles as design/configuration scaffolding only.
   Do not infer that cloud build, cloud credential storage, store submission, or
   production OTA is enabled.
6. Do not upload enterprise source or dependency metadata to EAS Cloud under
   authority of this record. A bounded cloud trial, if desired, needs an
   explicit source-classification/egress owner and sanitized scope; its result
   is vendor evaluation evidence, not candidate approval.
7. Do not operate a production OTA lane. Prototype update experiments must be
   non-production, runtime-bounded, source/config/dependency identified, and
   recoverable by a known-good local binary, source-bound OTA forward fix, or
   an embedded-bundle fallback only where that mechanism is implemented and
   proved.
8. Retain exact source SHA, lock/config hashes, toolchain versions, platform,
   build command, credential class (never the secret), artifact hash, test
   result, and operator for each proof build.

This posture makes local development technically eligible after an exact
source Assignment and Program Flow Controller admission. It does not itself
admit work. It deliberately parks the candidate decision until accountable
owners can evaluate the three paths, and neither prohibits a future EAS Cloud
selection nor requires permanent self-hosting.

## Candidate decision record fields

The parked decision becomes actionable only when all applicable fields below
have evidence or an explicit authorized disposition:

| Decision field | Required value / evidence | Current state |
| --- | --- | --- |
| Named application proposal, journey, and distribution audience | Source-bound proposal and lane | Missing |
| Accountable decision owner and approval record | Human/business and technical authorities with decision date | Missing |
| Selected path per iOS/Android and development/preview/candidate/production lane | Concrete custody/data-flow diagram, not a vendor label | Missing |
| Source classification and egress | Included/excluded paths, data classification, region, processors, retention/deletion, legal and security disposition | Missing |
| Expo tenant/plan/contract, if used | Organization ownership, entitlement, SSO/MFA, RBAC, audit, subprocessors, DPA/terms, support/SLA, billing and exit | Missing |
| Build runners and toolchains | Image/config ownership, pinned versions, patching, registries, cache/log policy, network controls, capacity, backup and recovery | Missing |
| Apple and Google organizational accounts | Legal owner, admins, least-privilege roles, API/service accounts, recovery contacts and separation of duties | Missing |
| Credential source per lane/platform | Certificate/key/profile custody, generation, escrow/backup, access, rotation, expiry, revocation and compromise playbook | Missing |
| OTA service and signing | Service/tenant, customer/vendor custody, runtime/channel policy, private-key controls, certificate rotation, emergency halt, source-bound forward recovery/embedded fallback and store-policy review | Missing |
| Provenance and reproducibility | Exact source/config/lock/toolchain identity, SBOM, attestation, artifact hash/signature, independent verification and retained logs | Missing |
| Promotion and submission control | Build/sign/submit/promote/halt/recover/retire roles, approvals, evidence binding and break-glass path | Missing |
| Security and privacy threat assessment | Source/secret supply-chain, runner compromise, malicious update, account takeover, dependency attack, log/data leakage and incident response | Missing |
| Availability, recovery, revocation and exit | Tested outage/account-loss/key-compromise/vendor-exit procedures with RTO/RPO or other named recovery thresholds | Missing |
| Lifecycle economics | Service plans, macOS/Android capacity, labor, support, audit/compliance, store accounts, incident/recovery and exit costs | Missing |
| Persistent build/release/update support owner | Named team/service with operating capacity and escalation route | Missing |
| Candidate acceptance and expiry | Approvers, evidence timestamp, expiry/revalidation conditions and open accepted-risk references | Missing |

The decision owner is not invented here. Product/business, enterprise
architecture, security/privacy/data, legal/procurement, Finance/contract,
platform/store administration, release, and persistent operations must be
routed according to the proposal and triggered concerns.

## Security, egress, provenance, and recovery criteria

### Security and secret custody

A candidate path must prove least privilege, strong human/service
authentication, role auditability, separation of development from
candidate/production credentials, secret masking, non-exportability where
required, rotation/revocation, dependency and runner hardening, and an incident
playbook. No mobile client secret may be embedded in source or binary. Signing
authority and update authority require explicit owners and emergency removal.

### Source and data egress

The decision must enumerate the actual archive, generated files, dependency
metadata, environment/configuration, logs, symbols, and credentials crossing
each boundary. `.gitignore` is not a data-classification or DLP control. The
assessment must cover tenant isolation, encryption, region, processors,
retention/deletion, operator access, legal/contract terms, breach notification,
and proof that PHI/PII or real tenant data is absent unless separately approved.

### Provenance and update integrity

Every candidate binary and update must bind source SHA, dependency lock,
configuration, native runtime, API compatibility range, toolchain/runner
identity, artifact hash, tests, approvals, and promotion history. The build must
emit or retain an SBOM and an attestation appropriate to the selected path.
Updates must be cryptographically verified, runtime/channel constrained,
promote the exact tested bundle, expose health/stop signals, and prevent an OTA
from crossing native module, permission, entitlement, privacy-manifest, schema,
policy, security, or backend-compatibility boundaries.

### Recovery, continuity, and exit

Use two explicit recovery state machines. OTA recovery halts rollout and ships
a source-bound forward fix, or selects a previously embedded bundle only when
that exact fallback is implemented and proved. Native-binary recovery halts
the store/MDM rollout and stages a replacement; it may use a platform-supported
rollback only when the named authority and retained evidence prove it. Neither
path can atomically recall bytes from offline devices, so the backend retains a
skew-aware compatibility window and safe unsupported-client response.

Before candidate use, prove—not only document—recovery for a failed build,
bad update, expired/revoked certificate, compromised key, lost administrator,
vendor outage, unavailable runner, and vendor exit. Preserve organization-owned
package/bundle identifiers and store accounts, reconstructible toolchains,
credential recovery or controlled reissuance, a known-good binary/update,
halt/forward-fix/replacement controls, and a tested path that does not depend
on one employee or inaccessible vendor account.

Recovery thresholds, service levels, and acceptable outage/cost are missing and
must be supplied by the application and operating owners before candidate.

## Candidate tripwires and stop conditions

Stop the Prototype lane and obtain the applicable decision before any of these:

- enterprise source, non-public dependency metadata, credentials, signing
  requests, or signing material would enter hosted build/update infrastructure;
- enterprise non-production identity, PHI/PII, real tenant data, live provider
  writes, or executable external actions enter the application;
- an organization bundle/package identifier, production-capable certificate or
  key, App Store Connect/Play credential, or production update-signing key is
  used;
- TestFlight, Play internal/closed, store, MDM, or other external signed
  distribution is attempted;
- production OTA, candidate/qualified/released language, contractual service
  dependence, or a package/support promise is proposed;
- build/update provenance cannot be tied to exact source and configuration;
- the applicable OTA/binary recovery path, account recovery, key revocation, or
  vendor/runner exit cannot be
  demonstrated; or
- a cloud/local split behaves differently enough that the tested artifact is
  not representative of the promoted artifact.

At a tripwire, the permitted disposition is evaluate, revise, defer, or stop.
This record does not supply a risk acceptance.

## Smallest useful custody proof

Without crossing the Prototype boundary, the team can:

1. inventory the complete local iOS/Android build inputs and emitted artifacts;
2. produce deterministic-enough clean local builds twice from the same source
   and compare source/config/lock/toolchain/artifact identities;
3. demonstrate that development credentials are absent from source and can be
   revoked/replaced without code changes;
4. exercise a bounded non-production update, incompatible-runtime rejection,
   rollout halt, source-bound forward fix or proved embedded fallback, offline
   startup, and known-good binary replacement;
5. draft concrete data-flow diagrams for EAS Cloud, self-hosted, and split paths;
6. if separately authorized, run a sanitized EAS Cloud evaluation and record
   the actual uploaded paths, roles, logs, credentials behavior, artifact
   provenance, deletion, outage, and exit observations; and
7. compare effort, lead time, defects, recovery steps, service consumption, and
   ongoing operational burden without claiming lifecycle economics until a
   Finance/contract owner supplies the model and ceiling.

## Current primary vendor evidence

The following official Expo documentation was reviewed on 2026-08-03 to define
the paths, not to approve them. Vendor behavior, entitlements, security terms,
and platform policies are time-sensitive and must be revalidated at candidate
decision time.

| Official source | Relevant current behavior |
| --- | --- |
| [Android build process](https://docs.expo.dev/build-reference/android-builds/) | Describes creation and upload of a project tarball to hosted build storage, a fresh build container, and credential sourcing from local configuration or EAS servers |
| [Run EAS Build locally](https://docs.expo.dev/build-reference/local-builds/) | Describes `--local` on customer infrastructure, its remaining EAS service communication, local toolchain responsibility, and differences from hosted builds |
| [EAS managed credentials](https://docs.expo.dev/app-signing/managed-credentials/) | Describes EAS-hosted signing credentials and teammate build initiation |
| [Use existing credentials](https://docs.expo.dev/app-signing/existing-credentials/) | Describes hosted versus local credential choices and upload/disposal behavior for local credentials used by cloud jobs |
| [Local credentials](https://docs.expo.dev/app-signing/local-credentials/) | Describes `credentialsSource: local`; credentials and passwords still require secure materialization outside source control |
| [EAS Update code signing](https://docs.expo.dev/eas-update/code-signing/) | Describes client verification using an embedded certificate and a customer-controlled private key; current plan/entitlement availability must be checked before selection |
| [Expo Updates API](https://docs.expo.dev/versions/latest/sdk/updates/) | Describes EAS-hosted or custom update services and the `updates.url`/`runtimeVersion` compatibility controls |
| [EAS Update deployment](https://docs.expo.dev/eas-update/deployment/) | Describes environments/channels, runtime versions, and promotion of an exact tested update |
| [EAS Build troubleshooting](https://docs.expo.dev/build-reference/troubleshooting/) | Reinforces that source files are uploaded for cloud builds and that sensitive files must not be placed in project source |

These pages do not answer PDS-specific classification, legal, privacy,
procurement, architecture, control, support, or risk questions. Marketing or
documentation claims are not substitutes for contract terms, security evidence,
configuration inspection, or a representative recovery test.

## Role Card Check

- **Card used:** Architect Agent consuming the App Framework Research Steward
  procedure as advisory evidence.
- **Work within role:** Compared build-custody paths, identified actual custody
  assets and triggers, recorded a reversible local Prototype posture, and
  defined candidate evidence fields.
- **Authority not assumed:** No source/secret egress, vendor, contract,
  architecture, risk, signing, distribution, candidate, release, WIP, or
  investment decision was approved.
- **Routed decisions:** Source classification/egress, enterprise architecture,
  security/privacy/data, legal/procurement, Finance/contract, platform/store
  administration, release, and persistent operations remain with their named or
  still-missing authorities.
- **Drift signal:** `watch`; escalate if checked-in EAS profiles are treated as
  approval, if “local credentials” is represented as never entering a cloud
  job, if this provisional local posture becomes a permanent standard without
  evidence, or if any Prototype build is represented as candidate/release
  proof.
