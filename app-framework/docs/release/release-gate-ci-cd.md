# Release Gate, Bitbucket, and ArgoCD

This framework treats provider parity as release evidence, not a best-effort
test. A framework release is eligible for promotion only after Bitbucket
Pipelines runs the release gate against live provider-backed backend instances.

## Release Gate Contract

The required release command is:

```bash
scripts/appfw release-check --json
```

The command runs:

1. `scripts/appfw validate --json`
2. `scripts/appfw mcp-posture --json`
3. `scripts/appfw boundary-check --json`
4. `scripts/appfw feature-check --json`
5. `scripts/appfw docs-check --json`
6. `scripts/appfw generate --check --json`
7. `scripts/appfw test`
8. `scripts/appfw policy-test --json`
9. `scripts/appfw ops-certification --json`
10. `scripts/appfw pds-baseline --json`
11. `scripts/appfw provider-test --all --json`
12. `scripts/appfw security-certification --json`
13. `scripts/appfw handoff --json`

`provider-test --all --json` writes the provider certification report to the
framework-root certification crate:

```text
api_tests/target/provider-parity.json
```

`release-check --json` also copies that report into the release artifact
directory:

```text
target/appfw/provider-parity.json
```

The retained provider parity report must include a fresh `generated_at_utc`
timestamp. Final evidence validation rejects stale provider parity, non-full
provider runs, missing or out-of-bundle `provider-test-*.log` files, and provider
base URLs that do not match the retained provider URL preflight snapshot.
Provider, provider-area, and live-security contract identities must be recorded
once; duplicate identities fail the gate. `security-certification.json` retained
artifact paths, byte sizes, and SHA-256 hashes are revalidated as part of the
same final evidence check. The gate also requires security certification
introspection source/runtime-log references, provider parity references, and
provider log references to resolve to the same retained files validated by the
semantic evidence blocks and `provider-parity.json`; the introspection source
must be the canonical runtime routing source. Security certification must carry
a fresh timezone-aware `generated_at_utc` timestamp and must not predate the
retained provider parity report it consumes. Retained security check names must
be unique, `failure_count` must match both retained failures and failing checks,
and a report cannot claim `ok:true` or `release_ready:true` while checks,
failures, or release blockers remain.

Generated validation, topology, config-contract, and boundary reports are also
copied from `.appfw/target/appfw` into the release artifact directory so a
release can be reviewed from a single Bitbucket artifact bundle:

```text
target/appfw/app-gen/
target/appfw/boundary-check.json
```

Every provider entry includes every certification area. A `LiveCertified` area
passes only when its live contract executed and reported `passed`. `failed`,
`listed`, and `not-run` fail the release gate.

## Bitbucket Pipelines

The repository includes `bitbucket-pipelines.yml` and the orchestration script:

```text
scripts/ci/bitbucket-release-gate.sh
```

The release gate step starts PostgreSQL, MongoDB, MS SQL Server, and LocalStack
Snowflake services, runs migrations once per provider, applies versioned expand
migrations for relational providers, launches one backend instance per provider,
then runs:

```bash
scripts/appfw release-check --json
```

The Bitbucket release wrapper has two lanes:

- Focused provider-backed evidence: `main` branch builds and the custom
  `release-check` pipeline run `release-gate-focused`. This still starts the
  four provider services, runs migrations, launches one backend per provider,
  and executes `release-check --json` with live provider/performance evidence,
  but it explicitly disables production-attestation, live-ops, PDS baseline,
  release-identity, and security-assurance authority requirements.
- Strict production release authority: `v*` tag builds run `release-gate` with
  the default strict `APPFW_RELEASE_REQUIRE_*` posture. This lane is the one
  that can produce a promotable `bitbucket-release-gate.json` with
  `release_ready:true` for production claims.

Pull requests run the faster validation and local test loop. The pull-request
path runs `scripts/ci/release-lite-guard.sh` before the expensive PR gates,
then a cache-free `PR preflight`, and runs the guard again after retained PR
evidence is produced. The guard compares the PR diff
against provider/runtime/security/release-gate sensitive paths. If no sensitive
paths changed, it writes `target/appfw/release-lite-guard.json` with
`release_lite_required:false` and passes. If sensitive paths changed, the guard
fails closed unless the PR pipeline is rerun with
`APPFW_RELEASE_LITE_EVIDENCE_URL`, `APPFW_RELEASE_LITE_APPROVER`, and
`APPFW_RELEASE_LITE_REASON` pointing at an approved custom `release-check` run
or provider-backed release-lite lane. This keeps provider/security regressions
from merging silently, fails missing approval evidence before spending the long
PR lane, and still retains a final release-lite artifact for merge readiness.
The release-lite guard contract is also covered by a focused docs-check:

```bash
scripts/appfw framework docs-check --subcheck release-lite-guard --json
```

The `PR preflight` treats `BITBUCKET_COMMIT` as the triggering source and
resolved `HEAD` as the tested checkout because Bitbucket may integrate the
destination before scripts execute. It binds those commits to the exact
`BITBUCKET_PR_DESTINATION_COMMIT` and freshly fetched destination branch. The
destination variable may be Bitbucket's 12-character hexadecimal prefix; the
runner independently resolves the named remote branch, requires the provider
prefix to match it, requires 40/64-character provider identities to equal it
exactly, fetches that branch, and retains only the full destination SHA after
exact equality with the remote head. The
tested commit must equal source with destination as ancestor, equal destination
with source as ancestor, or be an exact two-parent merge of source and
destination. The effective merge base and all changed-content checks use
destination versus tested. Missing, malformed, unresolved, advanced, or
mismatched identity, unfetchable bounded history, or a missing merge base fail
as infrastructure; CI never falls back to an invented range. Local parity
accepts an explicit base through
`scripts/ci/local-pre-push-gates.sh --base <ref-or-sha>` and records source equal
to tested explicitly.

Within a 240-second runner deadline, the preflight retains report-only change
impact, runs `git diff --check`, requires a coherent conflict-marker block
before reporting `conflict_marker`, syntax-checks changed shell Git blobs
without dereferencing worktree symlinks, and runs
`cargo fmt --all --check`. Rustfmt absence or startup failure is
`infrastructure`, not `rust_format`; the cache-free Bitbucket step bounds its
availability probe at three seconds, makes one 45-second rustup
component-bootstrap attempt, gives both GNU timeouts a two-second
TERM-to-KILL escalation, and has a five-minute step backstop. Changed Git blobs
are size-checked before materialization and fail closed above 32 MiB with
complete evidence instead of being truncated or skipped. The preceding
release-lite step already proves `python3` exists in the same declared build
image, and the preflight repeats an explicit Python availability check before
starting the evidence-owning runner.

Change impact is retained as telemetry only after a zero child exit plus typed
report-contract and exact-identity validation. Generic candidate delivery
annotation may honestly make the embedded report's operational `ok` false, but
only when `delivery_profile.gate_execution_projection.ok` is the boolean
`false`. The runner retains that payload unchanged and records report-contract
validity and operational profile status separately. It rejects arbitrary false
values, nonboolean or incoherent projections, Git-error surfaces, malformed
structure, and identity mismatch. Contract validity does not independently
recompute or certify the classifier's semantic policy. The report does not
select, omit, narrow, or satisfy any downstream gate; the existing assurance
sequence still runs in full.

The stable result categories are `change_classification`, `diff_hygiene`,
`conflict_marker`, `shell_syntax`, `rust_format`, `timeout`,
`infrastructure`, and `unknown`. JSON, JSONL, bounded logs, JUnit, and a
SHA-256/byte-size manifest are finalized for successful, failed, timed-out,
and infrastructure outcomes. Result and manifest provenance includes source,
tested, tested-commit relation, destination, and effective merge-base identity.
The manifest explicitly excludes its own hash because a manifest cannot
contain a stable cryptographic digest of itself.
Its named Bitbucket artifact is `scoped`, downloads no prior artifacts, and is
not consumed by later steps. The existing fast, supply-chain, secret-scan, and
final release-lite gates remain unchanged in scope. After preflight, the three
independent producers run concurrently; the final release-lite/freshness guard
still waits for all three terminal outcomes.

The pull-request fast framework path also runs the PR framework gate wrapper,
currently `scripts/ci/wave3-pr-gates.sh` for compatibility. That wrapper writes
`target/appfw/wave3-pr-gates/progress.jsonl` and prints the same JSONL progress
events to the CI log before, during, and after each sub-gate. Child stdout and
stderr remain in retained per-gate log files instead of being streamed into the
main pipeline log. The Bitbucket step installs Node 22 through
`scripts/ci/install-node.sh` before the product SPA build because the frontend
Tailwind/Vite toolchain requires Node 20 or newer; do not replace that with
Debian's default Node 18 package. The default per-gate timeout is controlled by
`APPFW_WAVE3_GATE_TIMEOUT_SECS`; the product SPA install and build gates can be
tuned separately with `APPFW_WAVE3_NPM_CI_TIMEOUT_SECS` and
`APPFW_WAVE3_NPM_BUILD_TIMEOUT_SECS`.

Bitbucket caches Rust build output under `target/cargo` through
`CARGO_TARGET_DIR`; it must not cache the whole `target/` directory. Retained
CI and release evidence lives under `target/appfw`, and that evidence must be
fresh for the current commit, pipeline, and step. Producer steps that write
framework evidence run `scripts/ci/prepare-appfw-evidence-root.sh` before their
gate commands. The helper clears only the canonical `target/appfw` evidence
root, refuses non-canonical artifact paths, and writes
`target/appfw/ci-evidence-root.json` with the current Git/Bitbucket context and
cache-boundary metadata. Aggregating release steps that intentionally consume
prior step artifacts must not call the cleanup helper before reading those
artifacts.

The PR Fast producer does not upload the whole evidence root. Its exit
finalizer runs `scripts/ci/package-pr-fast-evidence.py`, which stages the
root-level framework JSON/log/text reports and Wave 3 child diagnostics under
`target/appfw/pr-fast-evidence/`. CRM validation remains a required local green
predicate, but no product-owned CRM machine reports are staged or uploaded.
The retained
`target/appfw/pr-fast-evidence-manifest.json` records the exact triggering
source, tested checkout, and destination Git SHAs, their permitted direct,
fast-forward, or exact two-parent synthetic-merge relation, and each
source/staged path, byte size, and SHA-256. A successful bundle fails closed if
a required report is absent or non-green, identity is incomplete or unrelated,
a staged hash changes, or uncompressed evidence exceeds 100 MiB. The bundle is
built outside Bitbucket's upload glob and atomically published only after every
copy verifies and the complete manifest-inclusive size passes. An over-limit
failure retains its small diagnostic manifest but deletes the unpublished
payload, so `capture-on: always` cannot upload the oversized evidence.
Build output, npm caches, generated workspaces, tool
installations, and package archives are outside this allowlist.

Bitbucket may supply the destination commit as a 12-character abbreviation.
Only that documented short form may prefix-match the resolved commit; supplied
40- or 64-character object IDs must match the complete resolved identity.

Bitbucket publishes that bundle as the named `pr-fast-evidence` shared
artifact with `capture-on: always`. The PR Fast, supply-chain, secret-scan, and
release-lite producer/guard steps set `artifacts.download: false`: Fast and the
two security producers initialize their own evidence roots, while the opening
release-lite guard reads Git/environment inputs and the closing guard also
checks the destination branch head directly from `origin`. The focused/strict
release gates and ProGet publisher still download inherited artifacts because
those steps intentionally aggregate or consume them. The 100 MiB ceiling is a
local structural control; compressed size and the target upload duration of no
more than 30 seconds must be confirmed by an exact-SHA Bitbucket run rather
than inferred from local execution.

On pull requests, the opening release-lite guard remains the first fail-closed
step. The cache-free exact-SHA preflight runs next. Fast, supply-chain/lint,
and secret scan then run in one fail-fast parallel group; each is a
self-contained evidence producer and none consumes another member's artifact.
The closing release-lite guard remains after the group, therefore waits for all
three terminal outcomes, and runs
`scripts/ci/pr-destination-freshness.py`. Bitbucket does not automatically
restart an already-running PR pipeline when its destination advances, so this
cheap closing query rejects an otherwise green but stale source/destination
proof. Pipeline `#437` measured the prior
warm-cache shape as Fast 52m26s followed by supply-chain 10m58s and secret scan
2m16s in parallel, so the expected critical-path change removes approximately
the supply-chain duration rather than removing assurance. The exact candidate
pipeline must still prove overlap, cache behavior, terminal outcomes, and total
wall time; parallel cold-cache compilation or concurrent cache writes may
reduce the modeled gain.

That proof keeps the no-sensitive-change pass case, sensitive fail-closed case,
and secured-approval pass case executable without starting live providers.
The pull-request path also runs `scripts/appfw docs-check --changed-only --json
--progress --enforce-budget`, the supply-chain gate, dependency-check evidence,
and the secret scan so agent-facing CLI examples, upgrade/advisory evidence,
and security proof do not drift from the implementation. The changed-only lane
records the selected tier and timing artifacts, enforcing the accepted PR
budgets while preserving full docs-check for command-contract, generator,
agent-skill, and release-grade documentation changes.

Required secured Bitbucket repository or workspace variable:

```text
LOCALSTACK_AUTH_TOKEN
```

The release wrapper fails before installing CI prerequisites or waiting for
provider services when `LOCALSTACK_AUTH_TOKEN` is missing or set to an obvious
placeholder. This token is only the infrastructure credential that starts the
LocalStack Snowflake emulator used for provider certification; keep it masked as
a secured CI variable and never print it in logs.

The release wrapper also installs the same exact checksum-verified Node.js
runtime as the pull-request Fast gate before building the CRM product frontend.
Both paths delegate to `scripts/ci/install-node.sh`, whose default is Node.js
22.17.0. This avoids runtime-dependent test-runner behavior while satisfying
the Tailwind 4 native `@tailwindcss/oxide` requirement for Node.js 20 or newer;
do not substitute Debian's default Node.js 18 package or an unpinned NodeSource
runtime.

Provider service defaults used by the pipeline are intentionally local-only:

```text
PG_SERVICE_ACCOUNT_NAME=postgres
PG_SERVICE_ACCOUNT_PASS=postgres
MONGO_SERVICE_ACCOUNT_NAME=mongo
MONGO_SERVICE_ACCOUNT_PASS=mongo
MSSQL_SERVICE_ACCOUNT_NAME=sa
MSSQL_SERVICE_ACCOUNT_PASS=YourStrong!Passw0rd
SNOWFLAKE_SERVICE_ACCOUNT_NAME=test
SNOWFLAKE_ACCESS_TOKEN=test
SNOWFLAKE_AUTH_TOKEN_TYPE=OAUTH
```

The pipeline sets:

```text
ENV_NAME=compose
API_TEST_AUTH_MODE=local_dev
APP_ENABLE_LOCAL_TEST_AUTH=true
APP_PROVIDER_CERTIFICATION_CI=true
```

For each provider lane, the wrapper sets `APP_CRM_DATA_SOURCE_NAME` to the
provider-specific data source before migration and backend startup. Avoid using
the global `APP_DATA_SOURCE_NAME` for release certification unless the whole app
really should move every schema, including system-owned surfaces, to one data
source.

`release-check --json` fails fast when the four provider-specific backend URLs
are missing or malformed. The Bitbucket release wrapper exports
`API_TEST_BASE_URL_POSTGRES`, `API_TEST_BASE_URL_MONGO`,
`API_TEST_BASE_URL_MSSQL`, and `API_TEST_BASE_URL_SNOWFLAKE` after starting the
provider-specific backends; ad hoc release lanes must do the same before running
`scripts/appfw release-check --json`. Values must be distinct provider-specific
HTTP(S) base URLs without credentials, query strings, or fragments. Distinctness
is enforced by scheme, host, and effective port, so path aliases on one backend
do not count as provider-specific URLs. The preflight also probes each
`${base_url}/health/ready`; dead or stale backend URLs are retained as
`invalid_provider_base_urls` instead of falling through to later provider-test
logs. The preflight writes
`target/appfw/release-check-provider-url-preflight.json`; the top-level
missing/invalid provider URL arrays in `release-check.json` are derived from
that single snapshot. A passing snapshot must also retain one valid URL proof
for each release-certified provider, including the redacted health URL, passing
HTTP status, probe timestamp, and timeout. The snapshot and each retained
health probe timestamp must be generated within the final evidence check's
24-hour freshness window.
The JSON release-check command exits successfully only when its retained report
is `release_ready:true`; `release_ready:false` is a nonzero release gate result
even when the report schema is valid.

## Local Live Preflight

Before pushing a provider/runtime/release-gate branch for review, run:

```bash
scripts/appfw framework local-live-preflight --json
```

This is a local release-certification preflight, not the production release
gate. It starts or verifies local PostgreSQL, MongoDB, MS SQL Server, and
LocalStack Snowflake services, runs each provider migration, builds the CRM
backend once, launches four provider-specific backend instances, verifies the
four local provider URLs, runs full `provider-test --all --json`, and then runs
`security-certification --json` against the retained provider parity report.
The backend build uses `APPFW_CERTIFICATION_BACKEND_FEATURES`, defaulting to
`http,provider-postgres,provider-mongo,provider-mssql,provider-snowflake`, so
the release lane certifies the provider surface explicitly instead of relying
on the product backend's everyday default feature set.

The primary evidence is:

```text
target/appfw/local-live-release-preflight.json
target/appfw/provider-parity.json
target/appfw/security-certification.json
target/appfw/local-live-preflight/provider-url-preflight.json
target/appfw/local-live-preflight/backend-*.log
target/appfw/local-live-preflight/migrate-*.json
```

Treat `local-live-release-preflight.json` with `ci_ready:true` as the branch
precondition for opening a PR that should pass the remote release lane. The
same artifact intentionally keeps `release_ready:false` and
`release_authority:"local-preflight-only"` because local proof cannot supply
the release-authoritative PDS baseline, release identity, operations,
performance, security-assurance, production-attestation, and final
release-evidence bundle required by the remote release gate. In short: codebase
work must make the local live preflight green; the live environment must still
collect and retain the release-authoritative evidence from the Bitbucket lane.
The artifact also records `release_authority_boundary`, whose
`local_preflight_satisfies_release:false` value is intentional. Wave 2 status
uses the same rule: P1-P4 is not release-ready unless the retained managed
`bitbucket-release-gate.json` is `ok:true`, `release_ready:true`, not
`focused_evidence:true`, and backed by the strict managed release requirements.
Focused branch evidence may prove CI readiness, but only the strict managed
release lane can satisfy production release authority.
Each `external_release_authority_gates` entry in the local preflight plan
records an explicit `authority` of `managed-release-ci` or `release-authority`
and `local_preflight_satisfies:false`; local preflight output must not be
promoted into those release-authoritative gates.

`APP_ENABLE_LOCAL_TEST_AUTH=true` lets compose-style CI use explicit
`Bearer appfw-local:...` test tokens for policy-scoped API contracts. It does
not allow missing-auth local admin behavior; that remains limited to
`ENV_NAME=local`. Runtime validation requires
`APP_PROVIDER_CERTIFICATION_CI=true` before this mode can run in compose.

## Required Artifacts

The early PR preflight producer retains only this scoped allowlist; these are
feedback and review artifacts, not production-release authority:

```text
target/appfw/pr-preflight-change-impact.json
target/appfw/pr-preflight.json
target/appfw/pr-preflight-progress.jsonl
target/appfw/pr-preflight-manifest.json
target/appfw/pr-preflight-logs/*.log
test-results/pr-preflight.xml
```

The normal `target/appfw/change-impact.json` report remains command-owned and
may be refreshed by later handoff or review commands. The preflight upload uses
the immutable snapshot above so its manifest remains self-consistent after
those normal refreshes.

The PR Fast producer retains a separate named, non-release-authoritative
artifact only after manifest-inclusive size validation succeeds. On a failed or
over-limit packager run, Bitbucket retains only the small red manifest and no
payload directory:

```text
target/appfw/pr-fast-evidence-manifest.json
target/appfw/pr-fast-evidence/**
```

Bitbucket stores these artifacts on release gate runs:

```text
target/appfw/release-check.json
target/appfw/release-check-output.json
target/appfw/release-mcp-posture.json
target/appfw/ops-certification.json
target/appfw/pds-security-baseline.json
target/appfw/app-gen/validation.json
target/appfw/app-gen/app_topology.json
target/appfw/app-gen/config_contract.json
target/appfw/boundary-check.json
target/appfw/supply-chain-gate.json
target/appfw/dependency-check.json
target/appfw/sbom-manifest.json
target/appfw/sbom-rust-workspace.cdx.json
target/appfw/sbom-frontend-packages.cdx.json
target/appfw/sbom-deployable-image.cdx.json (conditional: required when an image source or image-SBOM release policy applies)
target/appfw/pds-component-check.json
target/appfw/phi-log-lint.json
target/appfw/secret-scan.json
target/appfw/gitleaks-report.json
target/appfw/security-assurance-decision.json
target/appfw/security-risk-acceptance.json
target/appfw/dast-evidence.json
target/appfw/sast-evidence.json or target/appfw/sast.sarif
target/appfw/asvs-traceability.json
target/appfw/release-provenance.intoto.jsonl
target/appfw/artifact-signing.json
target/appfw/release-evidence-check.json
target/appfw/bitbucket-release-gate.json
target/appfw/provider-parity.json
target/appfw/security-certification.json
target/appfw/agent-handoff.json
target/appfw/release-check-*.log
target/appfw/provider-test-*.log
target/appfw/backend-*.log
target/appfw/migrate-*.json
target/appfw/migrate-*.log
api_tests/target/provider-parity.json
api_tests/target/provider-test-*.log
```

`target/appfw/bitbucket-release-gate.json` includes top-level `ok`,
`release_ready`, `release_requirements`, `release_blockers`, and
`failure_summary.root_causes` fields. Downstream promotion jobs must key on
`release_ready:true`, and `ok:true` must match the same no-blocker release
decision. The retained `release_requirements` block proves the wrapper required
security assurance, production attestations, operations certification, live ops
evidence, PDS baseline evidence, and performance evidence before the artifact
can be considered promotable.
Focused wrapper artifacts carry `focused_evidence:true` and
`release_authority:"focused-ci-only"`; they keep `release_ready:false` and use
`ci_ready:true` for the focused pass signal. Aggregate Wave 2 status treats those
as CI readiness only, even when the focused release-check slice passed.
After the wrapper reaches release-gate execution, failures in provider waits,
migration, backend startup, `release-check`, security-assurance, evidence
collation, or final artifact writing also retain this file with `ok:false`,
`release_ready:false`, `failed_stage`, `exit_status`, and the available blocker
details. The wrapper aggregates retained `release_blockers`, raw `failures`,
and nested release evidence `failure_summary` entries with a root-cause-first
policy: when `release-evidence-check.json` already contains structured
`failure_summary.root_causes`, the wrapper promotes those canonical causes
instead of duplicating child-report status lines or raw assertion labels.
Prior wrapper self-check causes, such as stale `bitbucket-release-gate.json`
validation failures, are not propagated when the wrapper regenerates; the new
artifact reports the current failed stage and child evidence instead of
preserving an obsolete validation loop.
The final `release-evidence-check` pass validates that a retained non-ready
Bitbucket wrapper includes those canonical release-evidence causes, so a
promotion-facing failure artifact cannot omit the final gate's root causes.
If the wrapper claims `release_ready:true`, the final checker also requires
the current release-evidence run to have no canonical root causes and no
retained failure-stage fields. The wrapper's own ready assertion also requires
stable `command:"bitbucket-release-gate"`, a fresh timezone-aware
`generated_at_utc`, `release_blockers` and `failure_summary.root_causes` to be
present, made only of non-empty strings, and equal, with no retained
`failed_stage` or `exit_status` keys.
When both artifacts are retained, the wrapper timestamp must not predate the
`release-check.json` evidence it summarizes. A ready wrapper must also retain
green child summaries for `release_check.ok` and
`release_check.release_ready`, provider certification for PostgreSQL, MongoDB,
MS SQL Server, and Snowflake, live operations certification, live PDS baseline
evidence, security-assurance decision evidence, MCP release posture, and the
static/security gate summary. The wrapper's ready assertion also reloads the
retained child JSON artifacts and requires those artifacts to prove the same
green release posture, with fresh timestamps that do not postdate the wrapper;
the retained `release-check.json` must also carry the same required green check
set and canonical artifact references, and the retained `provider-parity.json`
must prove full-mode live certification with non-empty live-certified areas and
passed live contract records for every release-certified provider. Required
performance evidence must also retain a populated `load-test-suite` summary with
hash-bound passing scenario artifacts plus a live-required `provider-performance`
matrix tied to hash-bound retained `provider-parity.json`,
`performance_recommendations.json`, and QueryIR budget-cap source evidence;
hollow `ok:true` stubs are rejected. Security, operations, PDS baseline, and
final evidence-check child artifacts must also carry green internal checks, zero
failures and blockers, and their required live authority/evidence structures
before a wrapper can promote. The wrapper also recomputes retained
security-certification artifacts, live-ops artifacts, PDS baseline
decision/evidence artifacts, and SBOM manifest CycloneDX artifact SHA-256
digests and byte sizes before treating those child reports as
release-authoritative. Supply-chain child artifact references for
dependency-check, PHI lint, SBOM manifest, and retained CycloneDX files must
also carry matching SHA-256 digests and byte sizes. SBOM manifest entries must
also match the retained CycloneDX JSON `specVersion` and component count, so a
hash-consistent manifest cannot overstate generated SBOM coverage. SBOM
manifest source lists must also match the tracked package lockfiles and
deployable image sources that made each SBOM applicable. Static security
evidence is also reloaded from the retained PHI lint, supply-chain,
dependency-check, SBOM
manifest, secret-scan, and gitleaks reports so embedded `security_gates`
summaries cannot mask hollow retained static artifacts. Dependency-check child
checks marked `required:true` must also remain green with zero exit status, and
`blocking_findings` must stay empty. The retained security-assurance decision must prove production
attestation requirements, release-authoritative category dispositions, and
retained evidence artifacts whose SHA-256 digests and byte sizes still match the
retained files. Risk-accepted security-assurance categories must be explicitly
accepted, unexpired, bounded by the configured max-days window, and carry owner,
approver, release scope, rationale, compensating controls, and follow-up. The
retained MCP posture must be fresh, excluded, and match the wrapper summary.
Provider summaries must name each release-certified
provider exactly once, retain data source and base URL, and show at least one
live-certified area with all live-certified areas passed. Both the live wrapper
assertion and final evidence checker reject boolean or missing live-certified
counts; the summary counts must be real integers. The final checker validates
the embedded wrapper summaries directly: `release_check.checks` must include
the release-ready check set, each retained check must be green, artifact-bearing
checks must retain artifact paths that resolve to the expected files inside the
configured release evidence directory, operations and PDS summaries must require live
evidence, and operations, PDS, security-assurance, and MCP summaries must be
green before a retained wrapper can promote. The `security_gates.checks` block must name each
expected release security gate once, all gate checks must be green, release-ready
gate entries must carry `release_ready:true`, and the ops/PDS gate entries must
carry `live_evidence_required:true`.

The provider certification JSON is the canonical release evidence. Release
notes should reference the Bitbucket build number and retain the artifact link.
Source-controlled dependency risk acceptances live in `dependency-check.toml`.
The dependency-check report records matched OSV findings, package/version,
aliases, reason, owner, source file, and expiry; expired or invalid acceptances
make strict dependency-check evidence fail.
Set `APPFW_DEPENDENCY_CHECK_CARGO_HOME` to a writable CI cache path when
RustSec/cargo-deny advisory databases must not use the runner's default Cargo
home. If unset, the supply-chain gate and dependency checker use writable
`CARGO_HOME`/`$HOME/.cargo` when available and fall back to
`target/appfw/cargo-home`; the selected path is retained in both supply-chain
and dependency-check artifacts.
The Bitbucket release wrapper runs seed data by default because the live
provider-certification contracts and generated CRM load suite validate seeded
lookup rows and relationship projections. Override
`APPFW_MIGRATE_SKIP_SEED=true` only for a narrower schema-readiness lane that is
not claiming provider certification or load-test release evidence.
For PostgreSQL and MS SQL Server, the wrapper also applies versioned expand
migrations after bootstrap and retains
`target/appfw/migrate-<provider>-versioned-expand.json` plus the matching log.
That keeps generated table bootstrap and provider-routine DDL in the same live
certification environment without rerunning seed backfills.
The Bitbucket wrapper also writes `target/appfw/bitbucket-release-gate.json`,
which records the release-check and provider-parity artifact hashes, the
security gate artifact hashes, SBOM artifact hashes, the MCP release-posture
artifact, the operations certification artifact, the PDS baseline artifact, the
security-assurance decision, the Bitbucket build metadata when present, each
release subcheck, each security subcheck, and the live-certified area count for
PostgreSQL, MongoDB, MS SQL Server, and Snowflake. The wrapper fails if any
strict `release_requirements` flag is not enabled, if any expected provider entry is missing from
`target/appfw/provider-parity.json`, if MCP is enabled while it is excluded from
the release posture, if operations certification is not ok, if PDS baseline
evidence is missing or not ok, if the supply-chain, dependency-check, SBOM, PHI
log lint, secret-scan, or redacted gitleaks JSON
evidence is missing or not ok, or if DAST/SAST/ASVS/provenance/signing release
evidence is neither present nor formally risk-accepted. Before the wrapper writes
`target/appfw/bitbucket-release-gate.json`, it runs
`scripts/ci/security-assurance-decision.sh` and
`scripts/ci/release-evidence-check.sh` to verify the retained security JSON
schema, top-level `release-check.json` contract, required fields, artifact
references, dependency-check shape, SBOM shape, redaction claims, PDS baseline
posture, and security-assurance decision shape. Retained PHI log-lint,
supply-chain, dependency-check, SBOM, and secret-scan reports must also carry
fresh timezone-aware `generated_at_utc` timestamps; stale, malformed, or future
static evidence fails the final gate. Static evidence artifact entries must
resolve to the canonical retained files in the release artifact directory. The
supply-chain report hashes its retained dependency-check, PHI log-lint, SBOM
manifest, and CycloneDX artifact references, and SBOM manifest hashes are
rechecked against the retained CycloneDX files.
Release-ready identity evidence is reloaded from
`target/appfw/release-identity.json`; retained license/notice files must still
exist and be non-empty, package metadata must still record license metadata, and
retained distribution artifact paths must resolve under the release artifact
directory with matching SHA-256 digests and byte sizes.
Retained dependency-check reports must keep every `required:true` child check
green with `status:"passed"` and exit code 0, and must not retain blocking
findings.
If a previous or external `bitbucket-release-gate.json` is already retained in
the release artifact directory, `release-evidence-check` validates it as well:
`ok` must match `release_ready`, `failure_summary.root_causes` must mirror
`release_blockers`, the wrapper timestamp must be fresh, and
`release_requirements` must show every strict production evidence requirement,
including release identity evidence, enabled before any ready claim can be
accepted.
When a retained `agent-handoff.json` is present, the same final evidence check
also validates that the handoff timestamp is fresh, the recorded branch and SHA
match the checkout, drift counts match changed surfaces, and handoff
verification summaries for release check, provider certification, and
operations certification match the retained evidence reports.
Secret-scan summary counts are recalculated from the retained redacted gitleaks
report plus the configured git-tracked baseline, and any unbaselined fingerprint
fails the final gate. The retained `target/appfw/release-evidence-check.json`
carries `release_ready`, raw
`release_blockers`, and structured `failure_summary.root_causes` so promotion
jobs and audit tooling can read the same blocker set. If `release-check.json`
is structurally valid but `release_ready:false`, the evidence checker preserves
that as a release blocker rather than hiding the top-level gate state. Strict
mode exits nonzero whenever retained release blockers remain; `--local-fixture`
is the only schema-only mode allowed to return success while not release-ready.
The retained `release-check.json` must include a fresh timezone-aware
`generated_at_utc` timestamp; stale or future top-level release reports fail the
final evidence check and production lanes must rerun `release-check`.
Timestamped artifacts referenced by retained `release-check` check entries must
also have been generated no later than the top-level `release-check` timestamp,
within the standard evidence clock-skew allowance, so a promotion bundle cannot
mix a release decision with later regenerated check artifacts.
The retained `provider_scope` must keep release-certified CRUD providers pinned
to PostgreSQL, MongoDB, MS SQL Server, and Snowflake. Neo4j is recorded only as
`release_certified:false` graph-provider scope with certification posture in
`docs/runtime/graph-read-providers.md#certification-posture`; graph support must
not be promoted through the CRUD release evidence path.
External API providers are also retained in `provider_scope` with
`release_certified:false` and `governed_write_certified:false` until the
governed-write lane produces `target/appfw/governed-write-evidence.json`. If a
provider scope claims `governed_write_certified:true`, final evidence validation
requires that artifact and checks delegated actor, token-store isolation, named
mutation, idempotency, policy/scope, and audit fields.
`release-check` also retains `wave2-readiness.json` from
`framework wave2-status --json` as a report-only CI artifact. It lets release
reviewers see the current North-Star Wave 2 posture beside the release bundle.
For non-default release artifact directories, the rollup is written into the
active bundle, prefers artifacts already retained there, and falls back to
canonical `target/appfw` posture evidence for lane artifacts produced outside
the bundle. It is not a promotion shortcut while the G1, U5, G4, and P1-P4
live evidence remains externally gated, or while U2 and G2 still depend on G1
live governed-write evidence. The rollup also records `external_evidence_plan`
entries for G1, U2, G2, U5, G4, and P1-P4: retained artifacts, commands,
environment hooks, and owner boundaries that release authority can use to
collect the missing evidence. `release-evidence-check` validates that plan when
`wave2-readiness.json` is retained, but the plan itself is not release
evidence.
CH6 chat-eval uses the same split: `target/appfw/wave4/ch6-chat-eval.json`
is deterministic local-fixture posture and must remain `release_ready:false`;
managed judge/live proof belongs in
`target/appfw/chat-eval-judge-evidence.json`. The final evidence checker
validates that managed artifact when retained and requires it when
`APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true` or
`APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true`, so release lanes can
fail closed until an approved AI gateway/search provider has produced live
judge evidence.
Connector graduation evidence is retained separately in
`target/appfw/provider-graduation.json`. It is optional until the U4 connector
graduation lane is in release scope, but `release-evidence-check` validates the
report whenever it is present and rejects promoted capability claims that lack
the required compiler or live evidence.
The final evidence check also validates `missing_provider_base_urls` and
`invalid_provider_base_urls` directly; provider URL preflight gaps fail the
final gate even if a stale or edited `release-check.json` claims readiness.
Provider URL entries must use canonical provider/env pairs, such as `postgres`
with `API_TEST_BASE_URL_POSTGRES`.
Invalid provider URL entries must retain a safe `redacted_url`; unparseable
values use `<redacted-invalid-url>`, and retained diagnostics must not include
credentials, query strings, or fragments.
The provider URL preflight check must retain the snapshot artifact, and final
evidence validation compares the snapshot's missing/invalid provider URL arrays
to the top-level `release-check.json` arrays. The snapshot must be retained in
the same release evidence directory being validated.
Across the snapshot's missing, invalid, and valid URL entries, each
release-certified provider must appear exactly once; duplicates, overlaps, or
omitted providers fail the final gate.
The retained `checks` array is also audited: each check must have a unique
name, boolean `ok` flag, retained log for failed checks, existing retained
artifact paths when recorded, and retained log/artifact paths must resolve
inside the release evidence directory so the archived bundle is self-contained.
Any `current_check` value must name the failed check that stopped the top-level
release check and be absent from a ready top-level report. Top-level `ok` and
`release_ready` cannot be true while any retained check has `ok:false`.
If a retained child artifact exposes `release_ready:false`, the corresponding
`release-check.checks[]` entry must also be `ok:false`; release-check treats
schema-valid but non-ready PDS or ops evidence as an explicit failed check.
When the top-level report claims `release_ready:true`, it must also retain
successful `pds-baseline`, `provider-url-preflight`, `ops-certification`,
`release-identity`, `provider-test`, and `security-certification` check entries;
artifact-bearing checks must point at the retained evidence files in the same
bundle. If performance evidence is required, the ready report must likewise
retain successful `load-test-suite` and `provider-performance` entries.
Provider URL preflight
blockers must retain the failed `provider-url-preflight` check log and a
matching provider URL root cause.
When provider preflight or PDS baseline root causes are present, the same check
requires the matching `remediation_work_items` entries, including LIVE-001 and
LIVE-002 for provider-backed release certification and LIVE-015 through
LIVE-021 for PDS baseline release-authority evidence. Release identity root
causes require LIVE-004 and LIVE-014 remediation entries.

Deployable image SBOM evidence is `not-applicable` when no release image ref or
retained image tar/OCI artifact exists. Set
`APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM=true` or
`APPFW_RELEASE_REQUIRE_IMAGE_SBOM=true` in release environments that build or
promote an image; then `scripts/ci/supply-chain-gate.sh` requires exactly one
image source and writes `target/appfw/sbom-deployable-image.cdx.json`. When the
image SBOM path is applicable and `syft` is not already on `PATH`, the
supply-chain gate installs the pinned Syft release from GitHub, verifies the
downloaded archive against the release checksum, and then generates CycloneDX
JSON. Override the default pin with `APPFW_SYFT_VERSION` when the release image
pipeline advances the approved scanner version.
If a separate image-build repository or deployment pipeline creates the
deployable image, that lane owns the image SBOM artifact and must retain its
link/hash with the release evidence. Do not treat image SBOM as
`not-applicable` for a release that promotes an image outside this repository;
either provide the external image SBOM evidence to this bundle or document the
system of record that retained it.

Security-assurance evidence is release-required for regulated promotion. Provide
real artifacts for each required category:

```text
target/appfw/dast-evidence.json
target/appfw/sast-evidence.json or target/appfw/sast.sarif
target/appfw/asvs-traceability.json
target/appfw/release-provenance.intoto.jsonl
target/appfw/artifact-signing.json
```

If a release lane is not regulated production or the control is formally
deferred, provide `target/appfw/security-risk-acceptance.json` instead. The risk
acceptance must include top-level approval, owner, approver, non-expired
`expires_on`, release scope, and one accepted item per missing known category
with a rationale, compensating controls, and follow-up/remediation plan. Owner,
approver, and release scope must be non-empty strings, unknown categories are
rejected, and acceptance expiry is capped at 90 days by default. Set
`APPFW_SECURITY_RISK_ACCEPTANCE_MAX_DAYS` only when the release authority has an
approved alternate expiry policy. The release wrapper writes the normalized
decision to `target/appfw/security-assurance-decision.json` and fails when any
category is missing both evidence and risk acceptance.

Production release gates also set
`APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS=true`, which means local
`security-assurance-local-evidence.sh` provenance and OpenSSL signing artifacts
cannot satisfy the `release-provenance` or `artifact-signing` categories. Use
SLSA/in-toto provenance, cosign or enterprise signing evidence, or a formal
time-boxed risk acceptance for those categories.
`release-evidence-check` treats production attestations as an implicit
security-assurance decision requirement, even when the separate
`APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION` flag is omitted.
Raw `scripts/appfw release-check --json` reports this requirement in
`evidence_mode.security_assurance`; when the retained
`security-assurance-decision.json` is missing, not release-ready, or not
generated with production attestations required, `release_ready:false` includes
LIVE-007 remediation. The final evidence check also revalidates each retained
security-assurance category: evidence dispositions must point at retained,
passing artifacts whose checksums and byte sizes still match the referenced
files, and risk-accepted dispositions must be explicitly accepted with
structured owner, approver, release scope, unexpired bounded expiry, rationale,
compensating controls, and follow-up fields. Retained security-assurance
decisions must also carry a fresh timezone-aware `generated_at_utc`; stale,
malformed, or future timestamps fail the final evidence check.

Operations evidence starts with `target/appfw/ops-certification.json`, which is
produced by `scripts/appfw ops-certification --json` during `release-check`.
The default command verifies the committed alert/dashboard/runbook bundle. For
local pre-CI confidence, run
`scripts/ci/ops-certification-ready-fixture.sh`; it creates synthetic retained
live-ops evidence in `target/appfw/ops-ready-fixture/` and runs the real
live-required ops gate so the branch proves the expected OTLP, readiness,
metrics, Prometheus, Alertmanager, Grafana, runbook-drill, path, digest, and
byte-size shape before CI. That fixture is structural evidence only and is not
live operations approval. For
production lanes that require live observability proof, set
`APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true`,
`APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true`, and optionally
`APPFW_RELEASE_REQUIRE_PROMTOOL=true`. Retain live OTLP, readiness, metrics,
Prometheus, Alertmanager, Grafana provisioning, and runbook-drill evidence under
`target/appfw` or pass paths through `APPFW_OPS_EVIDENCE_FILES`.
`release-evidence-check` treats live-ops evidence as an implicit
ops-certification requirement, even when the separate
`APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION` flag is omitted. When live-ops or
promtool policy is enabled, the retained ops-certification artifact must also
show it was generated with `live_evidence_required:true` or
`promtool_required:true`; stale local artifacts are rejected. Every retained
ops-certification report must carry a fresh timezone-aware `generated_at_utc`;
stale, malformed, or future timestamps fail the final evidence check.
If an ops report claims `ok:true` or `release_ready:true`, final evidence also
revalidates each retained check result and every required artifact's presence,
SHA-256 digest, and byte size, so changed observability or deployment evidence
cannot be promoted with stale ops certification metadata.
Release promotion must key off `release_ready:true`, not only `ok:true`; static
local bundles can be schema-valid while still retaining `release_blockers` and
LIVE-005 remediation, and strict evidence checking exits nonzero until the
live-ops blocker is resolved or formally accepted.

PDS production-readiness evidence starts with
`target/appfw/pds-security-baseline.json`, which is produced by
`scripts/appfw pds-baseline --json` during `release-check`. The default command
verifies that repo traceability covers PDS Security Baseline r4.5 and emits
LIVE-015 through LIVE-021 remediation when live evidence is not required. For
local pre-CI confidence, run `scripts/ci/pds-baseline-ready-fixture.sh`; it
creates a synthetic retained decision/evidence bundle in
`target/appfw/pds-ready-fixture/` and runs the real live-required PDS gate so
the branch proves the expected decision-file, evidence-file, path, digest,
byte-size, approval-metadata, and LIVE-015 through LIVE-021 coverage shape
before CI. That fixture is structural evidence only and is not a live PDS
approval. For
final release evidence, the retained PDS baseline report must carry a fresh
timezone-aware `generated_at_utc`; stale, malformed, or future timestamps fail
the gate before promotion. For
production lanes, set `APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true` and
provide `APPFW_PDS_BASELINE_DECISION_FILE` with a JSON decision artifact that
declares `ok:true`, `release_ready:true`, baseline `r4.5`, release-authority
owner/approver, an ISO-8601 approval timestamp with timezone that is not in the
future, and LIVE-015 through LIVE-021 coverage. Each mapped work item must point
at existing retained evidence or carry a time-boxed risk-acceptance record with
owner, approver, scope, `expires_on`, rationale, compensating controls, and
follow-up.
The `LIVE-*` labels are App Framework release-gate checkpoint IDs, not PDS
baseline section numbers. Retained decision artifacts should also include
`baseline_sections` so PDS reviewers can audit the bundle by the section names
from the IT Security Baseline Standard while automation checks stable
release-gate IDs.
Risk acceptances must expire within `risk_acceptance.max_days`, which defaults
to 365 days and can be tightened, but not raised above 365, with
`APPFW_PDS_BASELINE_RISK_ACCEPTANCE_MAX_DAYS`. Additional retained IdP,
secrets, SIEM, gateway/WAF/TLS,
backup/restore, vulnerability-SLA, SAST/DAST, and platform-hardening artifacts
can be listed in `APPFW_PDS_BASELINE_EVIDENCE_FILES`. Release promotion must
key off `release_ready:true`, not only `ok:true`; `release-evidence-check`
also revalidates the retained decision file's LIVE-015 through LIVE-021
work-item mappings, evidence file existence, retained SHA-256 digests, byte
sizes, risk-acceptance fields, and `decision_summary` consistency before
production promotion. Any retained PDS baseline report that claims
`release_ready:true` is treated as a production-readiness claim even when the
current lane did not require live PDS evidence; final evidence then requires
`live_evidence_required:true`, a live-required `release-check` evidence mode,
the retained decision artifact, and no PDS release blockers or failure-summary
root causes. Any retained PDS baseline report that claims `ok:true` also cannot
carry embedded failing checks. Traceability-only local bundles can be
schema-valid while still retaining PDS baseline `release_blockers` and LIVE-015
through LIVE-021 remediation; in the parent `release-check` report this appears
as `checks[].ok:false` for `pds-baseline`, and strict evidence checking exits
nonzero until the PDS blocker is resolved or formally accepted.
The Bitbucket release wrapper defaults
`APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true` so production CI fails closed
unless the retained PDS baseline decision artifact is present and release-ready.
Set `APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=false` only for an explicit
non-production dry run that must exercise the rest of the release gate without
claiming PDS production readiness. The wrapper records that override in
`release_requirements.pds_baseline_evidence_required` and refuses a promotable
`release_ready:true` result while the strict requirement is disabled.
The wrapper also asserts the final
`target/appfw/bitbucket-release-gate.json` `release_ready:true` signal before it
exits successfully. A completed wrapper run that writes `release_ready:false`
is therefore a failed release gate, not a promotable artifact.

## Release Identity Evidence

`scripts/appfw framework release-identity --json` writes
`target/appfw/release-identity.json`; `release-check` retains it as the
`release-identity` child check. A production-ready report requires
`APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY=true`, a `v*` tag for the release SHA,
root license/notice files, package license metadata, release notes, at least one
consumable distribution artifact, and a retained
`release-identity-decision.json` with owner, approver, approval timestamp, and a
matching tag. The repository owns the schema, checks, and fail-closed behavior;
legal, product leadership, release management, and distribution owners own the
approved license, release notes, distribution artifact, and decision evidence.
The decision file must be retained under `target/appfw`; `release-identity.json`
records the retained decision SHA-256 digest and byte size, and strict evidence
recomputes both before promotion.
Use [`release-identity-evidence-kit.md`](release-identity-evidence-kit.md) and
[`templates/release-identity-decision.template.json`](templates/release-identity-decision.template.json)
to prepare the release-authority decision bundle. The template is intentionally
non-release-ready and must be copied into the release artifact directory, filled
with approved values, and validated by the release lane before promotion.
Strict release evidence validates the retained `release-identity.json` directly,
including timestamp freshness, required child check coverage, release blockers,
failure-summary root causes, and LIVE-004/LIVE-014 remediation. This prevents a
parent `release-check` summary from being the only evidence that release
identity proof was evaluated.

Local release-readiness runs can create the default live-ops artifact set from a
running backend with:

```bash
APPFW_OPS_BASE_URL=http://127.0.0.1:8080 \
APPFW_ALERTMANAGER_URL=http://127.0.0.1:9093 \
  bash scripts/ci/live-ops-evidence.sh

APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true \
APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true \
  scripts/appfw ops-certification --json
```

The collector writes `live-otel-evidence.json`, `health-ready.json`,
`metrics.prom`, `metrics.json`, `prometheus-targets.json`,
`prometheus-rules.json`, `alertmanager-status.json`,
`grafana-dashboard-provisioning.json`, and `runbook-drill.json`. The evidence is
truthful local proof: backend health and metrics are probed live, Alertmanager
status API semantics are probed live when `APPFW_ALERTMANAGER_URL` is set, and
Prometheus, Grafana, and runbook artifacts are retained from the committed
observability bundle.
Production CI can add managed OTLP collector, Prometheus, Alertmanager, and
Grafana API evidence through the same file names or `APPFW_OPS_EVIDENCE_FILES`.
When live ops evidence is required, JSON artifacts must report `ok=true`; OTLP
evidence must show export enabled with a service identity, protocol, and
endpoint; runbook drill evidence must include passing steps; and Alertmanager
evidence must prove a live Alertmanager status API endpoint rather than only
retained static config.
The Bitbucket release wrapper defaults
`APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true` and
`APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true`; it collects the default live-ops
evidence before `release-check` and passes both requirements into the final
release-evidence check. By default the wrapper uses the PostgreSQL-backed
runtime for `APPFW_OPS_BASE_URL`; set that variable when another runtime
instance should be the operations proof target. Set
`APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=false` only for an explicit
non-production dry run that does not claim enterprise operations readiness.

The Bitbucket release wrapper also defaults
`APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true` and points
`APPFW_LOAD_TEST_URL` at the PostgreSQL-backed CRM API it starts. That makes
`release-check` run the generated API load suite and live provider-performance
certification before the final evidence check. In strict performance mode,
`release-evidence-check` requires `load-test-suite` evidence and
provider-performance coverage for PostgreSQL, MongoDB, MS SQL Server, and
Snowflake; a single `load-test` artifact is not enough for enterprise
promotion. The suite summary must retain per-scenario artifact paths, SHA-256
digests, and byte sizes so the wrapper can reject edited load evidence.
Provider-performance evidence must also retain hash-bound provider-parity and
generated performance-recommendation inputs, plus the QueryIR budget-cap source
file. Override the load-test URL, request counts, concurrency, and thresholds
with the `APPFW_LOAD_TEST_*` variables when the release lane has a different
performance target. Set
`APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=false` only for an explicit
non-production dry run that cannot run backend load proof.

Local security-assurance evidence can be collected with:

```bash
APPFW_SECURITY_ASSURANCE_BASE_URL=http://127.0.0.1:8080 \
  bash scripts/ci/security-assurance-local-evidence.sh

APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION=true \
  bash scripts/ci/security-assurance-decision.sh
```

This writes `dast-evidence.json`, `sast-evidence.json`,
`asvs-traceability.json`, `release-provenance.json`, and
`artifact-signing.json`. These are local release-readiness artifacts, not a
claim that enterprise scanners or signing services ran. Regulated production CI
may replace them with ZAP/Burp DAST, CodeQL/Semgrep SARIF, formal ASVS
traceability, SLSA/in-toto provenance, and cosign or enterprise signing
evidence. The normalized decision in
`target/appfw/security-assurance-decision.json` is the release contract either
way.

For local, offline schema checks, agents may run:

```bash
APPFW_RELEASE_ARTIFACT_DIR=target/appfw/local-fixture \
bash scripts/ci/release-evidence-check.sh --local-fixture
```

This writes `target/appfw/local-fixture/release-evidence-check-local.json`. The
local fixture mode does not install gitleaks, does not use network access, and
is not release evidence. It creates synthetic retained PDS baseline, release
identity, security-certification, supply-chain, dependency-check, PHI lint, SBOM,
secret-scan, and gitleaks child artifacts plus fixture logs, then creates
synthetic retained PDS/release-identity decisions and security provider-parity
evidence under the fixture directory's `fixture-artifacts/` path only to
exercise retained artifact path, digest, byte-size, decision-file matching,
security cross-reference, supply-chain child-artifact hashing, and LIVE-015
through LIVE-021 evidence-file coverage checks; those fixture files are not
release approvals. Local fixture mode can pass schema validation while still
recording release blockers for synthetic/non-authoritative evidence and missing
live release-authority decisions. In CI, the same mode may intentionally use the
default `target/appfw` directory to validate real upstream artifacts plus
synthetic fallbacks for missing release-only artifacts. Inspect
`failure_summary.root_causes` for the combined blocker/assertion summary. Strict
mode writes
`target/appfw/release-evidence-check.json` and exits nonzero when any retained
release blocker keeps `release_ready:false`. Bitbucket and release promotion continue to
use the default strict mode, which requires all retained release/security
artifacts to be present and `ok=true`. When retained Wave 2 readiness includes
an `external_evidence_plan`, strict mode also rejects plan entries that carry
release-authoritative fields, local/focused satisfaction claims, local fixture
commands, focused CI commands, placeholder tokens, localhost evidence, or local
fixture artifact paths. The plan is guidance for collecting external evidence;
it is never a substitute for retained strict release artifacts. Plan
`required_artifacts` must name relative release-staged artifact locations under
`target/appfw`; raw product workspace outputs may feed those artifacts, but
they are not release-authority paths by themselves.

## ArgoCD Promotion Model

ArgoCD should deploy only artifacts that already passed the Bitbucket release
gate. It should not be the place where provider certification is first
attempted.

Use one of these promotion patterns:

- Image tag promotion: Bitbucket creates the release image tag only after the
  release gate passes. ArgoCD or ArgoCD Image Updater watches only release tags.
- GitOps manifest promotion: Bitbucket opens or merges a GitOps manifest change
  only after the release gate passes. ArgoCD syncs the manifest after review.

Recommended promotion metadata:

```yaml
metadata:
  annotations:
    app-framework/release-check-build: "<bitbucket-build-number>"
    app-framework/provider-certification-artifact: "target/appfw/provider-parity.json"
```

The ArgoCD application should enforce immutable image tags or digests. Rollback
means reverting to the previous certified image or manifest revision, not
rebuilding from an unverified branch.

## Agent Handoff

Before asking an agent to cut or promote a framework release, hand it this
minimum context:

```text
Run the Bitbucket custom pipeline `release-check`.
The release is blocked unless target/appfw/release-check.json has ok=true and
target/appfw/provider-parity.json shows every live-certified area as passed.
If the release passes, update the GitOps promotion target for ArgoCD with the
certified image tag or digest and include the Bitbucket build/artifact link.
```
