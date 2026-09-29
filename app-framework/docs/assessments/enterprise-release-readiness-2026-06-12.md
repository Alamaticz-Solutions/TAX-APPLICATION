# Enterprise Release Readiness Scorecard - 2026-06-12

| | |
| --- | --- |
| Assessment type | Adversarial enterprise release-readiness reassessment |
| Assessment date | 2026-06-12 |
| Baseline | `origin/main` at `f801437c` after PR #319 and PR #320 merged |
| Working branch | `codex/release-scorecard-post-merge-refresh` |
| Scope | Framework release readiness, local live preflight, retained evidence quality, live-environment/PDS/release-identity blockers |
| Verdict | Locally CI-ready for provider/security preflight; not release-ready for enterprise production promotion |

## Executive Verdict

The current position has real traction. PR #315 merged the PDS baseline and
release identity evidence kits, PR #316 merged the release identity
decision-integrity hardening, PR #319 merged the PDS ready-shape preflight, and
PR #320 merged the ops ready-shape preflight. The two biggest governance
blockers now have concrete fill-and-validate bundle paths, retained decision
artifacts are hash/byte-bound before promotion, and current `main` can locally
exercise both live-required PDS and live-required ops evidence shapes before CI.
The offline fixture also retains synthetic PDS baseline, release identity,
security-certification, supply-chain, dependency-check, PHI lint, SBOM, secret
scan, and gitleaks child artifacts, fixture logs, synthetic retained decisions,
security provider-parity evidence, and PDS LIVE-015 through LIVE-021 evidence
files so local schema preflight exercises the same path, digest, byte-size,
`decision_file` matching, security cross-reference, supply-chain child-artifact
hashing, and PDS evidence coverage checks that strict CI will enforce.

The release is still blocked for enterprise production. The remaining blockers
are not primarily framework architecture blockers; they are release-authority
and live-environment evidence blockers:

- Local provider parity remains green. `target/appfw/provider-parity.json` is
  `ok:true`; postgres has 15 live-certified areas, mongo 15, mssql 14, and
  snowflake 12, with no failed live-certified areas.
- Local live security certification remains green.
  `target/appfw/security-certification.json` is `ok:true`,
  `release_ready:true`, and has zero retained provider security failures.
- `target/appfw/local-live-release-preflight.json` was regenerated on
  `f801437c` and is `ok:true`, `ci_ready:true`, and `release_ready:false` by
  design. It is branch-level CI readiness evidence, not production release
  authority.
- PDS baseline and release identity now have evidence-kit templates, runbooks,
  and retained-artifact integrity checks, but neither has a live
  release-authority decision/evidence bundle.
- The retained `release-check.json` and `release-evidence-check.json` are still
  stale relative to the green local preflight and current `main`.

## Headline Scores

| Composite | Score | Movement | Basis |
| --- | ---: | --- | --- |
| Engineering maturity | 9.4 / 10 | Up | Release evidence contracts, docs routing, local preflight, retained decision/distribution integrity, local fixture coverage, and PDS/ops ready-shape preflights are now merged on `main`. |
| Local CI-readiness | 9.5 / 10 | Up | Local live preflight is green on `f801437c` with 22/22 checks passing, the isolated release-evidence fixture exits green for schema validation, and PDS/ops ready-shape fixtures prove live-required evidence structure before CI. |
| Local release-gate integrity | 9.7 / 10 | Up | The gate distinguishes `ok` from `release_ready`, fails closed on live-authority blockers, hash/byte-binds retained decisions, supply-chain child artifacts, and release identity distribution artifacts, and locally exercises retained evidence paths before CI. |
| Enterprise release readiness | 6.7 / 10 | Up, still blocked | CI confidence is materially better after the merged local live-green run, but production release authority still needs fresh remote release evidence, real PDS/ops proof where in scope, and release identity approval. |
| Enterprise product readiness | 6.2 / 10 | Up, still blocked | Productization has concrete evidence kits, but still lacks approved tag/license/distribution/support proof and a second downstream adopter. |

## Updated Scorecard

| # | Dimension | Score | Status | Current evidence |
| ---: | --- | ---: | --- | --- |
| 1 | Config and generated-boundary integrity | 9.0 | Green | `scripts/appfw framework validate --json` returned `valid:true`, `0 errors`, `0 warnings`; `generate --check --json` passed. |
| 2 | Release-check contract correctness | 9.5 | Green locally | Release gates distinguish child artifact `ok`/`release_ready`, provider URL preflight, release-authority blockers, hash/byte-bound retained decision artifacts and supply-chain child artifacts, and security-certification cross-references. The local fixture now retains synthetic PDS, release-identity, security-certification, supply-chain, SBOM, dependency-check, PHI lint, secret-scan, and gitleaks child artifacts and logs so this contract is exercised offline. The retained `release-check.json` is stale and must be regenerated by the release lane. |
| 3 | Local live provider preflight | 9.5 | Green on current main | `target/appfw/local-live-release-preflight.json` was regenerated on `f801437c` with `ok:true`, `ci_ready:true`, 22/22 checks passing, all service, migration, backend startup, provider-test, provider artifact, and security-certification checks green. |
| 4 | Provider-backed certification gate implementation | 9.0 | Implemented | `scripts/appfw framework local-live-preflight --json` and the Bitbucket wrapper use distinct provider URLs; `provider-test --all` fails on any failing provider test binary. |
| 5 | Provider parity evidence | 8.0 | Green locally; P0 remote evidence pending | Current local `provider-parity.json` is `ok:true`; postgres has 15 live-certified areas, mongo 15, mssql 14, snowflake 12, and no failed live-certified areas. Remote release lane must regenerate this from the release SHA. |
| 6 | Security certification evidence | 8.2 | Green locally; P0 remote evidence pending | Current local `security-certification.json` is `ok:true`, `release_ready:true`, `failure_count:0`, with provider security contracts green. The local fixture now retains synthetic security provider-parity and provider-log artifacts to exercise strict cross-reference checks. Remote release lane must retain real security evidence as part of the release bundle. |
| 7 | Provider capability honesty | 8.5 | Improved | Stored routine live certification runs only where the capability matrix claims live support; MSSQL/Snowflake remain compiler-contracted and Mongo remains unsupported for SQL stored routines. |
| 8 | PDS Security Baseline r4.5 proof | 4.4 | P0 live-authority blocker; structurally preflighted | Traceability and a fail-safe evidence kit are merged; local fixture coverage exercises retained PDS decision/evidence artifacts for LIVE-015 through LIVE-021; and `scripts/ci/pds-baseline-ready-fixture.sh` proves the live-required PDS gate accepts a complete retained decision/evidence shape locally. The production `pds-security-baseline.json` is still `release_ready:false`, `live_evidence_required:false`, with no real live release-authority decision/evidence bundle. |
| 9 | Release identity, licensing, and distribution | 5.7 | P0 product blocker; integrity hardened | The release identity evidence kit is merged, retained decisions are hash/byte-bound, local fixture coverage exercises the strict retained-decision path, and final evidence now revalidates release-ready license files, package metadata, and retained distribution artifact SHA/byte metadata. The current repo still cannot pass production release identity: no `v*` tag, root license/notice, package license metadata, retained live decision, release notes, or distribution artifact. |
| 10 | Retained evidence freshness and provenance | 8.2 | Mixed | Local provider/security/PDS/ops/release-identity artifacts are refreshed locally on current `main`, and isolated fixture evidence proves retained-artifact validation for release authority, security, release identity, supply-chain, SBOM, dependency-check, PHI lint, secret-scan, and gitleaks evidence without stale workspace contamination. `release-check.json`, `release-evidence-check.json`, and the Bitbucket wrapper evidence are stale/non-authoritative relative to `f801437c`. |
| 11 | Supply-chain gate strength | 8.8 | Green locally; rerun required | Prior supply-chain evidence passed clippy, audit, deny, dependency-check, SBOM generation, and PHI lint with child artifacts. The local fixture now materializes retained dependency-check, PHI lint, SBOM manifest, rust/frontend SBOM, secret-scan, and gitleaks child artifacts and revalidates their path/hash/byte metadata. It must still be regenerated in the coherent remote release bundle. |
| 12 | Operations evidence | 6.6 | P1; structurally preflighted | Static/local ops evidence exists, and `scripts/ci/ops-certification-ready-fixture.sh` proves the live-required ops gate accepts a complete retained OTLP/readiness/metrics/Prometheus/Alertmanager/Grafana/runbook evidence shape locally. Production live ops proof is not complete until produced by the credentialed release lane. |
| 13 | Performance evidence | 5.5 | P1 | Performance evidence is not required in the current local mode; no release-authoritative live load/provider-performance proof is retained. |
| 14 | Downstream adoption proof | 5.5 | P1 | CRM remains the only downstream proof point; no second independently shaped product has completed the lifecycle. |
| 15 | Neo4j release scope | 7.0 | Watch | Release scope correctly keeps Neo4j outside CRUD certification; do not claim graph release support until a separate live graph lane exists. |

## Current Evidence Snapshot

Artifacts inspected on `origin/main` at `f801437c` after rerunning local live
preflight, PDS ready-shape preflight, ops ready-shape preflight, and retained
evidence fixture coverage:

```text
target/appfw/local-live-release-preflight.json
target/appfw/provider-parity.json
target/appfw/security-certification.json
target/appfw/pds-security-baseline.json
target/appfw/release-identity.json
target/appfw/release-check.json
target/appfw/release-evidence-check.json
target/appfw/local-fixture/release-evidence-check-local.json
target/appfw/pds-ready-fixture/pds-security-baseline.json
target/appfw/ops-ready-fixture/ops-certification.json
```

Current local green evidence:

- `local-live-release-preflight`: `ok:true`, `ci_ready:true`,
  `release_ready:false`, `release_authority:"local-preflight-only"`, 22/22
  retained checks passing.
- Provider services, TCP waits, Snowflake control-plane probe, four migrations,
  four backend startups, provider URL preflight, provider-test, provider
  artifact assertion, and security-certification all passed.
- `provider-parity`: `ok:true`, full mode for postgres, mongo, mssql, and
  snowflake; no failed live-certified areas.
- `security-certification`: `ok:true`, `release_ready:true`, `failure_count:0`.
- PDS and release identity evidence-kit templates parse and fail closed when
  used as production decisions.
- The PDS ready-shape preflight command `scripts/ci/pds-baseline-ready-fixture.sh`
  returns a synthetic `target/appfw/pds-ready-fixture/pds-security-baseline.json`
  with `ok:true`, `release_ready:true`, `live_evidence_required:true`,
  `failure_count:0`, no release blockers, all LIVE-015 through LIVE-021 items
  covered and evidence-backed, and seven retained evidence artifacts with
  SHA-256 digests and byte sizes. This proves gate structure only; it is not
  release approval.
- The ops ready-shape preflight command
  `scripts/ci/ops-certification-ready-fixture.sh` returns a synthetic
  `target/appfw/ops-ready-fixture/ops-certification.json` with `ok:true`,
  `release_ready:true`, `live_evidence_required:true`, `failure_count:0`, no
  release blockers, and nine retained live-ops evidence artifacts with SHA-256
  digests and byte sizes. This proves gate structure only; it is not live ops
  approval.
- Release identity decision artifacts now record SHA-256 and byte size when a
  retained decision file is present, and strict evidence revalidates both. A
  synthetic release-identity-ready evidence bundle was also run through
  `release-evidence-check --local-fixture`; it returned `ok:true`,
  `failure_count:0`, `release_blocker_count:6`, and no release-identity-related
  root causes while exercising license-file, package-metadata, and retained
  distribution-artifact validation.
- The isolated local fixture command
  `APPFW_RELEASE_ARTIFACT_DIR=target/appfw/local-fixture bash scripts/ci/release-evidence-check.sh --local-fixture`
  returns `ok:true`, `schema_ok:true`, `release_ready:false`,
  `failure_count:0`, `release_blocker_count:11`, and exercises retained PDS,
  release-identity, security-certification, supply-chain, dependency-check, PHI
  lint, SBOM manifest, rust/frontend SBOM, secret-scan, and gitleaks child
  artifacts, fixture logs, decision artifact path, SHA-256, byte-size,
  `decision_file` matching, security provider-parity/log cross-references, and
  PDS LIVE-015 through LIVE-021 evidence coverage checks.

Current blocker evidence:

- `pds-security-baseline`: `ok:true`, `release_ready:false`,
  `live_evidence_required:false`; no retained PDS decision or LIVE-015 through
  LIVE-021 evidence bundle exists.
- `release-identity`: `ok:false`, `release_ready:false`; no release-required
  mode, no `v*` tag, no root license/notice, no package license metadata, no
  retained decision artifact, no release notes, and no distribution artifact.
- `release-check.json` and `release-evidence-check.json` are stale relative to
  `f801437c` and still represent the last incomplete release bundle.

## Work Split: Codebase vs Live Evidence

| Lane | Current status | What belongs here | Exit evidence |
| --- | --- | --- | --- |
| Codebase / local CI-readiness | Strong | Keep `local-live-preflight`, `provider-test`, `security-certification`, `release-check`, docs, and evidence checks fail-closed and unambiguous. | Local preflight, validate, docs-check, generate-check, fast tests, and handoff pass on the branch being promoted. |
| Release evidence integrity | Stronger after latest merges | Retain SHA/byte-bound decision files and supply-chain child artifacts, reject stale or mutated release-authority artifacts, and keep the local fixture aligned with strict retained-artifact validation and security cross-references. | Strict release evidence recomputes retained decision artifact SHA-256 and byte size before promotion; local fixture evidence proves retained PDS, release-identity, security-certification, supply-chain, SBOM, dependency-check, PHI lint, secret-scan, and gitleaks paths before CI; PDS and ops ready-shape preflights prove complete live-required bundle shapes pass locally. |
| Remote live release execution | Not complete | Run the Bitbucket release lane from the final merged SHA with required CI credentials, provider services, and exported provider URLs. | Fresh `release-check.json`, `provider-parity.json`, `security-certification.json`, wrapper evidence, and handoff artifacts from the same SHA. |
| PDS release-authority evidence | Not complete | Use the PDS evidence kit to produce retained Security Baseline r4.5 decision/evidence or risk acceptances for LIVE-015 through LIVE-021. | `pds-security-baseline.json` has `live_evidence_required:true`, `ok:true`, `release_ready:true`, and an approved retained decision artifact. |
| Productization / governance | Not complete | Use the release identity evidence kit to approve release identity, license/notice posture, support/SLA/RACI, release notes, distribution channel, and cadence. | `v*` tag through release lane, approved license/notice files or internal policy, published/approved artifact, release notes, and support ownership. |
| Further product proof | Not complete | Prove at least one second downstream product and any graph-provider claims beyond CRUD scope. | Golden downstream evidence for a non-CRM product; separate Neo4j graph certification before graph support is marketed as release-ready. |

## Priority List

| Priority | Work item | Owner | Exit criteria |
| --- | --- | --- | --- |
| P0 | Execute the remote release lane from the final merged SHA. | CI/CD + platform | `LOCALSTACK_AUTH_TOKEN` is configured; the wrapper starts all provider backends, exports all four provider URLs, and retained release artifacts come from the final merged SHA. |
| P0 | Regenerate release-authoritative provider parity and security certification. | CI/CD + security | Remote `provider-parity.json` and `security-certification.json` match the green local shape and are referenced by a fresh `release-check.json`. |
| P0 | Retain PDS Security Baseline r4.5 release-authority evidence. | Release authority + security | LIVE-015 through LIVE-021 have retained evidence or approved risk acceptances with owner, approver, timestamp, scope, rationale, compensating controls, and follow-up. |
| P0 | Complete release identity/licensing/distribution approval. | Release authority + legal + framework owners | Approved `v*` tag, license/notice posture, package license metadata, release notes, distribution artifact, and retained `release-identity.json` with `release_ready:true`. |
| P0 | Produce one coherent release bundle. | Release engineering | Bitbucket wrapper, release-check, release-evidence-check, supply-chain, provider parity, security certification, PDS baseline, release identity, ops/performance when required, and handoff all reference the same release SHA. |
| P1 | Add live ops and performance proof when promotion scope requires it. | SRE/platform + performance | Live OTLP/SIEM/alert evidence, readiness/metrics proof, load-test evidence, and provider-performance certification are retained and green. |
| P1 | Operate PR release-lite approvals when sensitive paths change. | CI/CD + platform + security | Sensitive PRs retain `release-lite-guard.json` plus approved evidence URL, approver, and reason. |
| P1 | Prove a second downstream product lifecycle. | Framework + product team | A second non-CRM product is created, generated, tested, upgraded, and included in golden-downstream evidence. |
| P1 | Keep Neo4j claims scoped. | Runtime/provider owners | Either keep Neo4j outside CRUD release claims or create a separate graph certification lane and retained evidence bundle. |
| P2 | Publish security-critical coverage metrics. | Framework + security | Coverage is published for access filters, tenant isolation, IDOR negatives, route construction, provider query compilation, and audit behavior. |
| P2 | Complete product support wrapper. | Product owner + support + legal | Support owner, SLA targets, disclosure path, release cadence, RACI, license, and EOL policy are approved. |

## Reassessment Trigger

Re-score after all of the following are true in one retained remote release
bundle:

1. `release-check.json` has `ok:true` and `release_ready:true`.
2. Provider parity and security certification are release-authoritative and
   green for postgres, mongo, mssql, and snowflake.
3. PDS baseline evidence is live-required and release-ready.
4. Release identity, license, tag, release notes, and distribution evidence are
   approved, retained, and hash/byte-bound.
5. Supply-chain, Bitbucket wrapper, release-evidence-check, and handoff
   artifacts are regenerated by the same credentialed lane for the same release
   SHA.

At that point, enterprise release readiness should move from the current
6.7/10 range into the 7.8-8.7 range, depending on whether live ops/performance
and second-product adoption evidence are also complete.
