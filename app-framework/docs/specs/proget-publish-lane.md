# ProGet Publish Lane Contract

Status: Integration candidate

Owner: Framework release engineering

Review class: Class D — release, supply-chain, and credential boundary

## Outcome

Publish an immutable App Framework release to the approved ProGet Cargo and
universal-package feeds only after the same Bitbucket tag pipeline has produced
strict managed-release evidence. Product developers consume versioned
framework artifacts without a local framework checkout.

This contract does not authorize a release, create credentials, select a
release version, or replace the human-controlled manual production step.

## Trigger and authority

The `v*` Bitbucket tag lane runs supply-chain and secret checks, then the strict
release gate. `Publish to ProGet` remains a manual production-deployment step.
The publisher accepts only canonical `bitbucket-release-gate` evidence with:

- `ok:true`, `release_ready:true`, and `focused_evidence:false`;
- `release_authority:strict-managed-release`;
- satisfied strict-gate and managed-requirement boundary fields;
- every named release requirement enabled; and
- commit, tag, build number, and pipeline UUID equal to the current publisher
  step.

Missing, stale, copied, focused, malformed, or mismatched evidence fails closed.

## Publication contract

1. The tag, package manifest version, CLI version, and packaged commit agree.
2. Every manifest artifact exists and matches its recorded SHA-256.
3. Crates publish in the generated dependency order with `--locked`.
   The generated plan is a closed topological order of the published
   crate graph: every publishable workspace normal dependency of an
   in-plan crate is also in-plan and precedes its consumer
   (`appfw-mssql-auth` before `appfw-provider-mssql` and `appfw-codegen`;
   `appfw-cli` before `appfw-codegen` before `appfw-test`). A missing
   publishable workspace normal dependency is a plan defect, not a
   skipped edge.
4. Existing crate versions are skipped to permit recovery from a partial
   publish. A failed index check does not claim a crate exists.
5. A failed or malformed universal-package version probe blocks publication.
   An existing version is never overwritten.
6. Cargo publication extracts the one source archive named by the verified
   manifest. Unmanifested or stale archives are never selected.
7. The universal package contains the manifest artifacts and the required Rust
   CycloneDX SBOM. A missing SBOM blocks publication.
8. Dry-run mode executes every local gate and builds the package but performs
   no network publication.
9. `APPFW_PROGET_LIBRARY_ONLY` is test-only. Direct execution with that
   variable set fails rather than bypassing the lane.
10. Schema HTTP route identity is the official
    `appfw_codegen::schema_route_segment` converter. `appfw-test` GraphQL
    clients and `appfw_introspect` depend on that crate module via Cargo.
    The publish lane must not add a `#[path]` include, a second kebab/ASCII
    algorithm, or bump `appfw-test` version identity just to name that
    function.

## Evidence

The publisher writes `target/appfw/proget-publish.json` with the exact commit,
tag, profile, stage results, crate decisions, and universal-package identity.
The release pipeline retains this artifact and the package manifest.

Canonical verification:

```bash
bash -n scripts/ci/proget-publish.sh scripts/ci/proget-publish.test.sh
bash scripts/ci/proget-publish.test.sh
scripts/appfw framework cli-test --json
scripts/appfw framework package --plan --json
```

A release-candidate rehearsal additionally uses real strict release-gate
evidence and a generated Rust SBOM with `APPFW_PROGET_DRY_RUN=true`. Tests must
never use real credentials or publish artifacts.

## Failure and recovery

- Do not rerun a failed publish unchanged until its failed stage is understood.
- Never weaken the secret scan, release evidence, artifact hashes, SBOM, or
  immutability probes to make publication pass.
- A partial crate publish is resumed at the same version; existing versions are
  skipped and remaining crates continue in order.
- A universal-package collision requires a new release version. Overwrite is
  forbidden.
- Credential, network, malformed-response, or identity failures are terminal
  for that attempt and leave retained failed-stage evidence.
