# ProGet Distribution

App Framework product teams should not need a local framework checkout or local
framework crate builds for normal product validation, generation, handoff, and
upgrade work. The framework release pipeline produces ProGet-ready artifacts;
the enterprise CI system publishes those artifacts to the approved ProGet feed.

## Artifact Contract

Run from the framework checkout:

```bash
scripts/appfw framework package --json
```

For a fast contract preview that does not build binaries or tarballs:

```bash
scripts/appfw framework package --plan --json
```

The command writes:

```text
target/appfw/proget/app-framework-<version>-<platform>.tar.gz
target/appfw/proget/app-framework-binaries-<version>-<platform>.tar.gz
target/appfw/proget/app-framework-crates-publish-plan-<version>.json
target/appfw/proget/app-framework-product-docs-<version>.tar.gz
target/appfw/proget/app-framework-proget-manifest.json
```

The manifest records the artifact paths, SHA-256 hashes, platform, package
version, and ProGet posture. CI should retain this manifest, publish the Cargo
crates in the plan order, and upload the tarballs it names.

## Cargo Crate Publishing

Generated product backends and generated product test crates should not depend
on a local framework checkout. The package command creates an ordered publish
plan for the product-facing framework modules:

```text
appfw-saas-core
appfw-runtime
appfw-mssql-auth
appfw-provider-postgres
appfw-provider-mongo
appfw-provider-mssql
appfw-provider-snowflake
appfw-provider-neo4j
appfw-provider-salesforce
appfw-provider-workday
appfw-provider-anaplan
appfw-provider-oracle-financials
appfw-provider-servicenow
appfw-provider-icims
appfw-cli
appfw-codegen
appfw-test
```

CI should run the plan with `cargo publish --locked --registry
pds-app-framework-crates -p <package>` in the listed order. The ordering matters
because `appfw-runtime` and SaaS provider crates depend on `appfw-saas-core`,
database provider crates depend on `appfw-runtime`, `appfw-provider-mssql` and
`appfw-codegen` depend on `appfw-mssql-auth`, `appfw-codegen` depends on
`appfw-cli`, and `appfw-test` depends on `appfw-codegen`. The plan is a closed
topological order of the published crate graph: every publishable workspace
normal dependency of an in-plan crate is also in-plan and precedes it. A clean
feed cannot resolve a later crate until its in-plan dependencies are already
published. Product scaffolds generated from a packaged toolchain create
registry dependencies and a `.cargo/config.toml` pointing at the approved
ProGet Cargo endpoint. Product CI should authenticate Cargo with
`cargo login --registry pds-app-framework-crates <api-key>` or
`CARGO_REGISTRIES_PDS_APP_FRAMEWORK_CRATES_TOKEN`.

Committed product `Cargo.toml` files should point at the registry, not at a
local framework checkout. A typical generated product backend shape is:

```toml
appfw_runtime = { package = "appfw-runtime", version = "0.1.1", registry = "pds-app-framework-crates", default-features = false }
appfw_provider_mssql = { package = "appfw-provider-mssql", version = "0.1.1", registry = "pds-app-framework-crates" }
```

Generated product test crates follow the same rule:

```toml
appfw_test = { package = "appfw-test", version = "0.1.1", registry = "pds-app-framework-crates" }
```

Published `appfw-codegen` owns the official GraphQL schema route converter
`appfw_codegen::schema_route_segment`. Packaged `appfw_introspect` and
`appfw-test` both depend on that crate module via Cargo.
`appfw_introspect` emits `schema_model.graphql_http_route` from it. Publish
scripts must not invent a second kebab/ASCII algorithm, add a `#[path]`
include, or treat a test-file copy as a separate contract.

Temporary local path overrides are acceptable only as an uncommitted
troubleshooting aid while developing framework changes. They must be removed
before product branches are pushed, because product developers and CI should be
able to build, test, and deploy without downloading or compiling the framework
checkout.

## Toolchain Bundle

The framework toolchain bundle contains:

```text
bin/appfw
bin/app_gen
bin/appfw_introspect
bin/database
scripts/appfw
scripts/check_app_gen_backend_equivalence.sh
app_gen/_templates
app_gen/_golden
app_gen/_config
app_gen/src
appfw_runtime/src
appfw_cli/src
product-docs/
```

The packaged `appfw` binary auto-discovers sibling binaries under `bin/` and the
packaged `scripts/appfw` wrapper. Product commands use the prebuilt
`app_gen`, `appfw_introspect`, and `database` binaries when available, and the
bundle carries helper scripts needed by drift checks. This avoids local
framework crate builds for the common product path.

## Product Docs Pack

The product docs pack intentionally excludes framework-owner internals and
historical extraction notes. It includes the product lifecycle docs, model docs,
frontend docs, release/deployment docs, product reference contracts, start
guides, generated product `AGENTS.md`, the Product PR Review Agent harness, and
`product-*` agent skills.

The generated product `AGENTS.md` tells product developers and coding agents to
run `scripts/appfw product review-brief --auto-depth --json`, invoke
`/product-pr-review` or `/product-pr-review --comprehensive` as indicated by
the retained brief, show the structured review output and artifact path to the
human, and push only when explicit human approval is recorded or the standing
push approval policy applies. Bare `review-brief --json` remains focused by
default for smaller local reviews.

Product repos may copy this pack under their own documentation area, or CI may
publish it beside the CLI/toolchain artifact for agent retrieval.

## Manual Cargo Publish (Pilot / Ungated)

For internal pilot uploads that intentionally bypass the gated `v*` publisher,
package locally and run the generated Cargo publish helper:

```bash
scripts/appfw framework package --json
export PROGET_API_KEY='...'   # never commit; publish-scoped for the Cargo feed
bash target/appfw/proget/publish-crates-from-source.sh
```

`publish-crates-from-source.sh` is produced next to the package artifacts under
`target/appfw/proget/`. It extracts `app-framework-source-*.tar.gz`, sets
`CARGO_REGISTRIES_PDS_APP_FRAMEWORK_CRATES_TOKEN` from `PROGET_API_KEY`, and
publishes each crate in
`app-framework-crates-publish-plan-<version>.json` order.

This path does not run release-gate evidence checks and does not upload the
universal toolchain upack. Already-published crate versions fail closed
(immutability). When a newer consumer (for example `appfw-cli`) depends on
APIs missing from the registry copy of a dependency, bump and publish that
dependency first so `cargo publish` verification can resolve against ProGet.
See also the short procedure in [Release README](README.md).

## CI Publish Boundary

This repository does not store ProGet credentials and the package command does
not publish over the network. The production-bound publish implementation is
the Bitbucket `Publish to ProGet` step on the `v*` tag lane, which runs
`scripts/ci/proget-publish.sh` after the strict release gate:

```text
tags v*:  supply-chain ∥ secret-scan  →  release-gate  →  publish-proget (manual)
```

The step is `trigger: manual` — a release owner reviews the retained
`bitbucket-release-gate.json` evidence, then starts the publish — and runs
under the `production` deployment environment so Bitbucket scopes who may
trigger it and where the ProGet credentials live.

### Environment contract

- `PROGET_SERVER` / `PROGET_API_KEY` are mapped from the secured Bitbucket
  variables (`ProgetServer` / `ProgetApiKey`). The API key must be
  publish-scoped for the Cargo feed and the universal (upack) feed.
- `APPFW_PACKAGE_PROFILE=release` is exported by the CI step: distributed
  toolchain binaries are always optimized release builds, and the packaged
  `build_profile` is recorded in `app-framework-proget-manifest.json`.
- `APPFW_PROGET_DRY_RUN=true` runs every gate and builds the upack without
  publishing; use it to rehearse a release.

### Publish gates

`proget-publish.sh` fails closed, records every stage in
`target/appfw/proget-publish.json`, and enforces:

1. Release evidence: retained `bitbucket-release-gate.json` must be `ok:true`
   and `release_ready:true`, identify the canonical release-gate command and
   strict managed-release authority, prove every managed-release requirement,
   and bind the same commit, tag, build number, and pipeline UUID as the
   publisher step. Focused evidence and copied evidence from another pipeline
   are rejected.
2. Identity: `BITBUCKET_TAG` (`vMAJOR.MINOR.PATCH[-prerelease]`) must equal the
   packaged manifest version and `appfw_cli/Cargo.toml`, and the packaged
   commit must equal the pipeline commit.
3. Versions: the toolchain/tag version anchors to `appfw-cli`; every crate in
   the publish plan must resolve in `cargo metadata`, and each crate publishes
   at its own manifest version (crates such as `appfw-runtime` version
   independently of the toolchain version).
4. Integrity: artifact SHA-256s are recomputed against the manifest.
5. Immutability: an already-published upack version is a hard failure (a
   released version is never overwritten). Already-published crates are
   skipped with a prominent warning so a partially failed publish can be
   safely retried. A failed or malformed ProGet version response fails closed;
   the publisher never treats an unavailable immutability probe as absence.
6. Source binding: Cargo publication extracts only the single source archive
   named by the hash-verified package manifest. Other source archives left in
   the package directory are ignored.
7. SBOM: the Rust CycloneDX SBOM is mandatory. A missing SBOM fails the
   universal-package stage instead of producing an incomplete release.

The hermetic failure-path suite exercises these boundaries without credentials
or network access:

```bash
bash scripts/ci/proget-publish.test.sh
```

### What gets published

1. Cargo crates publish to the `pds-app-framework-crates` feed with
   `cargo publish --locked` in `app-framework-crates-publish-plan-<version>.json`
   order, waiting for each version to appear in the sparse index before its
   dependents publish.
2. The toolchain, binaries-only, product-docs, and source tarballs, the crates
   publish plan, the ProGet manifest, and the Rust CycloneDX SBOM upload as one
   universal package (`app-framework-toolchain@<version>`) with commit, build
   number, pipeline UUID, platform, and build-profile provenance metadata.
3. `target/appfw/proget-publish.json` and the package manifest are retained as
   release evidence on the pipeline step.

Feed creation, credential rotation, retention policy, signing policy, and
rc-to-stable promotion rules remain environment-owned in ProGet and Bitbucket
settings.
