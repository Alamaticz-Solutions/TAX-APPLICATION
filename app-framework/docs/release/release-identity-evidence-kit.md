# Release Identity Evidence Kit

This guide turns the release identity blocker into a concrete retained evidence
bundle. It covers the approval path for release tag, license/notice posture,
release notes, package license metadata, and consumable distribution artifacts.

Do not use this kit to pick a license or publish a distribution artifact without
legal, product leadership, and release authority approval. The repository can
validate that the evidence exists and is self-consistent; it cannot approve the
business/legal decision.

## Bundle Contract

Start from the fail-safe template:

```text
docs/release/templates/release-identity-decision.template.json
```

Copy it into the release artifact directory as:

```text
target/appfw/release-identity-decision.json
```

The template is deliberately not release-ready. It keeps `ok:false`,
`release_ready:false`, placeholder approval metadata, and placeholder release
fields so it cannot accidentally satisfy the production gate.

Before promotion, release authority must replace the placeholders with:

- `ok:true` and `release_ready:true`;
- release owner, approver, and timezone-aware `approved_at_utc`;
- the current `v*` release tag attached to the release SHA;
- an approved root license/notice file retained in the repository;
- release notes retained in the bundle or a release notes URL;
- at least one consumable distribution artifact with a name and HTTPS URL or
  retained file path; and
- package license metadata in the framework manifests.

The decision file must stay inside the release artifact directory. When present,
`release-identity.json` records the decision file SHA-256 digest and byte size;
strict evidence recomputes both so a changed approval file cannot ride along
with stale release evidence.

## Expected Evidence

| Evidence area | Owner | Evidence to retain |
| --- | --- | --- |
| Release tag | Release management + CI/CD | `BITBUCKET_TAG` or a git `v*` tag attached to the release SHA, plus tag-build evidence from the release lane. |
| License/notice posture | Legal + product leadership | Approved root `LICENSE*`, `NOTICE*`, or `COPYING*` file and matching package manifest license metadata. |
| Release notes | Framework owners + release management | Versioned release notes, changelog section for the tag, or approved release notes URL. |
| Distribution artifact | Release engineering | Package, image, archive, or approved internal artifact URL/path that downstream consumers can pin. |
| Release authority decision | Release authority | Final `target/appfw/release-identity-decision.json` with owner, approver, approval timestamp, matching tag, license file, release notes, and distribution artifacts. |

## Validation Command

Run the release identity gate in release-required mode:

```bash
APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY=true \
APPFW_RELEASE_IDENTITY_DECISION_FILE=target/appfw/release-identity-decision.json \
scripts/appfw framework release-identity --json
```

For production release promotion, the resulting
`target/appfw/release-identity.json` must show:

```json
{
  "ok": true,
  "release_ready": true,
  "release_identity_required": true
}
```

The report must also show a non-empty `git.release_tag`, retained
`license_files`, package license metadata without missing entries, and a
release-ready `decision_artifact`.

## What This Does Not Prove

The repository cannot approve the license choice, support model, release notes,
or artifact distribution channel. Those remain product/legal/release authority
decisions. The gate only proves that the chosen identity evidence is present,
structured, and tied to the release SHA.
