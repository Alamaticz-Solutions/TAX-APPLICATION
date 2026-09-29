# Release, Deployment, And Evidence

Use this layer when a change needs production-readiness proof.

| Topic | Guide |
| --- | --- |
| Current readiness numbers and priorities | [Roadmap](roadmap.md) |
| Release gate and Bitbucket evidence | [Release Gate CI/CD](release-gate-ci-cd.md) |
| Sanitized measured PR pipeline baseline and exact successor proof boundary | [PR Pipeline Performance Baseline — 2026-08-23](pr-pipeline-performance-baseline-20260823.md) |
| Deployment environments, secrets, observability, handoff | [Deployment Reference](deployment-reference.md) |
| ProGet-ready framework toolchain, gated CI publish, and pilot manual Cargo publish | [ProGet Distribution](proget-distribution.md); manual steps below |
| Release evidence and maturity matrix | [Evidence Matrix](evidence-matrix.md) |
| Framework SemVer, release notes, and compatibility matrix | [Versioning And Compatibility](versioning-and-compatibility.md) |
| Live-environment work items that cannot be closed in repo code alone | [Live Environment Work Items](live-environment-work-items.md) |
| Canonical App Fabric stages, effort, dependencies, decision gates, acceleration rules, and proof outcomes | [App Fabric Master Implementation Plan](app-fabric-master-implementation-plan.md) |
| PDS security baseline traceability for production-readiness claims | [PDS Security Baseline Traceability](pds-security-baseline-traceability.md) |
| PDS baseline release-authority evidence bundle | [PDS Baseline Evidence Kit](pds-baseline-evidence-kit.md) |
| Release tag, license, notes, and distribution evidence bundle | [Release Identity Evidence Kit](release-identity-evidence-kit.md) |

Release evidence should be retained as CLI JSON/artifacts, not prose-only
claims.

## Local CI Gates Before Commit And Push

Run the substantive locally reproducible subset of Bitbucket's gates, fix
failures, and only then commit/push. Destination freshness, Bitbucket's tested
checkout identity, artifact upload/capture behavior, fail-fast cancellation,
and producer overlap remain remote-only proof. Canonical detail lives in
[Release Gate CI/CD](release-gate-ci-cd.md) and `bitbucket-pipelines.yml`.

### What Bitbucket runs

| Lane | Trigger | Gates (in order) | Authority |
| --- | --- | --- | --- |
| **PR / branch merge gate** | `pull-requests: "**"` | `release-lite-guard` → cache-free PR preflight → fail-fast parallel `fast-framework-check` ∥ `supply-chain-gate` ∥ `secret-scan` → closing `release-lite-guard` + destination-freshness check | Merge readiness for the PR. The preflight is early feedback, not production release authority or gate routing. The closing check rejects evidence if `main` advanced during the long producers. |
| **Main CI** | push/`main` | parallel `supply-chain-gate` ∥ `secret-scan` → **focused** `release-gate` (`release-gate-focused`) | Provider-backed **CI readiness** only (`focused_evidence`). Not ProGet/production authority. |
| **Strict release** | tag `v*` | parallel `supply-chain-gate` ∥ `secret-scan` → **strict** `release-gate` → manual `publish-proget` | Production release authority + gated ProGet publish. |
| **Custom focused release** | custom `release-check` | same as main focused lane | Used to produce release-lite evidence URLs for sensitive PRs. |

PR **fast framework check** and the surrounding PR lane are defined once in
scripts (not re-listed here):

- `scripts/ci/pr-preflight.py` — exact source/tested/destination relation and
  effective-merge-base T0 signal; retains report-only change impact, diff
  hygiene, coherent conflict-marker, changed-shell syntax, and Rust formatting
  evidence under a 240-second runner deadline, with changed Git blobs capped at
  32 MiB before materialization
- `scripts/ci/pr-preflight.test.sh` — disposable-Git fixtures for successful,
  failed, timeout, infrastructure, path-safety, and evidence-finalization cases
- `scripts/ci/pr-fast-framework-check.sh` — Fast framework check command sequence;
  Bitbucket runs `bash scripts/ci/pr-fast-framework-check.sh --ci-bootstrap`
- `scripts/ci/package-pr-fast-evidence.py` — stages the bounded evidence
  allowlist outside the upload glob, publishes it atomically only after the
  complete size ceiling passes, and binds source, tested checkout, and
  destination identities, including Bitbucket's exact two-parent synthetic
  merge checkout
- `scripts/ci/pr-destination-freshness.py` — closing remote identity check that
  fails when the destination branch advanced after pipeline start
- `scripts/ci/local-pre-push-gates.sh` — substantive local PR-gate subset:
  release-lite → PR preflight → fast framework → supply-chain → secret-scan +
  local-fixture evidence check → release-lite again; it cannot establish the
  remote-only proof named above

The preflight's change-impact result is telemetry only. A zero-exit, exact-
identity, typed report contract remains usable when generic candidate annotation
makes operational `ok` false solely because
`delivery_profile.gate_execution_projection.ok` is the boolean `false` before
required gates run. The original report is retained unchanged, while preflight
evidence keeps report-contract validity distinct from operational profile
status. This does not independently certify classifier semantics. Arbitrary
false, nonboolean or incoherent projection, Git-error surface, malformed, or
identity-mismatched reports fail. This never selects, skips, or weakens the
fast, supply-chain, secret-scan, or final release-lite gates. Its scoped artifact
does not download prior-step artifacts and is not shared with downstream steps.
Bitbucket retains the exact allowlist and discovers
`test-results/pr-preflight.xml` as native test evidence:

```text
target/appfw/pr-preflight-change-impact.json
target/appfw/pr-preflight.json
target/appfw/pr-preflight-progress.jsonl
target/appfw/pr-preflight-manifest.json
target/appfw/pr-preflight-logs/*.log
test-results/pr-preflight.xml
```

### Minimum local loop (every framework branch before commit/push)

```bash
scripts/ci/install-local-git-hooks.sh   # once per clone
bash scripts/ci/local-pre-push-gates.sh --base origin/main
scripts/appfw framework handoff --json
scripts/appfw framework review-brief --auto-depth --json
# Invoke /framework-pr-review (or --comprehensive) as the brief requires.
# Push only on GO / GO WITH CONDITIONS with zero blocker/critical.
```

If `release-lite-guard` reports `release_lite_required:true`, obtain approved
custom `release-check` / focused-lane evidence and set the secured CI vars
before expecting the PR to merge.

For product-owned work in a product checkout, use the product namespace
(`product validate` / `product test` / `product handoff` /
`product review-brief`) and `/product-pr-review` instead.

### Before opening or expecting a green provider/runtime/security PR

Sensitive paths need live provider confidence before merge. Locally:

```bash
scripts/appfw framework local-live-preflight --json
# Treat local-live-release-preflight.json ci_ready:true as branch confidence.
# It stays release_ready:false / local-preflight-only — not production authority.
```

Then ensure CI has a current focused release-check artifact if
`release-lite-guard` requires it (custom `release-check` pipeline or main
focused lane URL), and that the PR is re-run with the secured evidence
variables.

### Before claiming main CI readiness (merge destination)

Main runs supply-chain, secret-scan, and **focused**
`bash scripts/ci/bitbucket-release-gate.sh` (provider-backed `release-check`
with production-attestation / live-ops / PDS / identity / security-assurance
requirements disabled). Locally approximate with a green
`local-live-preflight` plus supply-chain/secret-scan; the remote focused gate
remains the merge-to-main CI proof.

### Before a `v*` production/ProGet claim

Strict tag lane only. Do **not** treat focused main evidence as enough. Follow
[Release Gate CI/CD](release-gate-ci-cd.md), identity/PDS kits, and
[ProGet Distribution](proget-distribution.md). Local rehearsal:

```bash
# Schema-only rehearsal (not release authority):
APPFW_RELEASE_ARTIFACT_DIR=target/appfw/local-fixture \
  bash scripts/ci/release-evidence-check.sh --local-fixture
# Full strict authority still requires the managed v* Bitbucket gate + retained
# bitbucket-release-gate.json with release_ready:true and focused_evidence:false.
```

### Common skip → remote fail patterns

| Skipped locally | Typical remote failure |
| --- | --- |
| `pr-preflight.py` through `local-pre-push-gates.sh` | whitespace, unresolved conflict block, changed-shell syntax, or rustfmt failure reported only after broader checks |
| `supply-chain-gate.sh` | fmt/clippy/audit/deny/PHI lint on PR or main |
| `secret-scan.sh` | gitleaks on full history |
| `docs-check` / wave3 PR gates | docs budget, composition, fork, governance, SPA build |
| `release-lite-guard` awareness | sensitive PR blocked without evidence URL/approver/reason |
| `local-live-preflight` | focused main/`release-check` provider or security failures |
| handoff + Framework PR Review + pre-push hook | push blocked or review stale vs handoff |

## Manual Cargo Publish (Pilot / Ungated)

Use this path only for internal pilot publishes to the ProGet Cargo feed when
the gated `v*` pipeline publisher is not in play. It does **not** satisfy
strict release-authority evidence. Production-bound releases still use the
Bitbucket `Publish to ProGet` step documented in
[ProGet Distribution](proget-distribution.md).

1. Package from a clean framework checkout at the intended commit:

```bash
scripts/appfw framework package --json
```

2. Export publish credentials (never commit them; map from secured Bitbucket
   vars or a local gitignored `.env`):

```bash
export PROGET_API_KEY='...'   # publish-scoped for pds-app-framework-crates
# Optional: PROGET_SERVER=https://proget.pdsconnect.com
```

3. Publish crates with the generated script (plan order, from the packaged
   source archive):

```bash
bash target/appfw/proget/publish-crates-from-source.sh
```

That script is regenerated under `target/appfw/proget/` by `framework package`.
It requires `PROGET_API_KEY` and runs
`cargo publish --locked --registry pds-app-framework-crates -p <crate>` for
each crate in `app-framework-crates-publish-plan-<version>.json`.

### Practical constraints

- ProGet crate versions are immutable. If a crate@version is already on the
  index, `cargo publish` fails and the stock script stops. Resume by publishing
  only remaining crates, or bump the crate version first.
- Dependent crates must resolve against the **registry**, not local path deps.
  If `appfw-cli` needs APIs that are not in the published `appfw-runtime`
  version it declares, bump and publish `appfw-runtime` first, then publish
  the CLI (each crate may version independently of the toolchain /
  `appfw-cli` version).
- Prefer a committed tree before publish. If you must include uncommitted
  version bumps, `cargo publish --allow-dirty` is required; commit afterward so
  git matches what ProGet received.
- Universal (upack) toolchain upload remains the gated CI publisher path unless
  an operator performs an equivalent ProGet upack upload outside this script.
