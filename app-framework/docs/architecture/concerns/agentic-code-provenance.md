# Agentic Code Provenance And AIBOM

This concern defines the App Framework posture for code, tests, docs, and
release artifacts produced with AI assistance. It supports the Wave 4
`SEC-AIBOM` lane in the roadmap.

## Why This Exists

Agentic development is part of the framework operating model. That does not
make AI-generated code automatically unsafe, but it does mean release evidence
must answer four questions:

1. Which agent/model materially contributed to the change?
2. Which prompts, source artifacts, and generated outputs influenced it?
3. Which human reviewed and accepted the change?
4. Which retained evidence proves the code, docs, generated boundaries, and
   release posture were checked?

The AIBOM is the machine-readable evidence bundle for those answers. It is
parallel to an SBOM: an SBOM describes software components; an AIBOM describes
AI-assisted production of the change.

## Local Report Command

Use the framework command:

```bash
scripts/appfw framework aibom-check --json
```

The command writes:

```text
target/appfw/aibom-check.json
```

The report is a local posture check. It records:

- current Git HEAD;
- required PR trailer contract;
- release AIBOM attestation contract;
- whether the release attestation is present and tied to HEAD;
- source documents and scripts that define the posture;
- next steps for managed CI/release authority.

## Enforced Mode

Use enforced mode only when a branch or release claims SEC-AIBOM readiness:

```bash
scripts/appfw framework aibom-check --enforce --json
```

Enforced mode writes:

```text
target/appfw/aibom-check-enforced.json
```

It fails closed unless both conditions are true:

- the PR/commit attribution trailers required by the report are present;
- `target/appfw/aibom-release-attestation.json` is valid, release-ready, and
  bound to the current HEAD.

Local report evidence does not satisfy production release authority. Managed CI
must produce the release attestation when the enterprise AIBOM/provenance
toolchain is selected.

## Release Attestation Shape

The release attestation schema is:

```json
{
  "schema": "appfw.aibom.release-attestation.v1",
  "ok": true,
  "release_ready": true,
  "head_sha": "<current git head sha>",
  "agentic_changes": [
    {
      "commit": "<commit sha>",
      "agent": "codex|claude|other",
      "model": "<model family or approved alias>",
      "prompt_context_hash": "sha256:<hash>",
      "changed_surfaces": ["docs", "runtime", "frontend"],
      "evidence_artifacts": ["target/appfw/docs-check-timing.json"]
    }
  ],
  "human_review": {
    "reviewer": "<person or group>",
    "approved": true
  },
  "trailers_verified": true,
  "aibom_artifact": {
    "path": "target/appfw/aibom.json",
    "sha256": "sha256:<artifact hash>"
  }
}
```

`commit_shas` may be used instead of a single `head_sha` when the attestation
covers a PR range, but it must include the exact HEAD being released.

## PR Trailer Contract

The current required trailers are:

```text
Agent-Assisted: true
AIBOM-Artifact: target/appfw/aibom-release-attestation.json
Human-Review: <reviewer or group>
Evidence: <primary retained artifact>
```

The framework does not require these trailers for every local commit yet. They
are enforced only when `aibom-check --enforce` is used. This lets normal agent
iteration stay fast while giving release branches a concrete fail-closed gate.

## Boundary

Framework owns:

- the `aibom-check` command contract;
- the attestation schema and fail-closed validation;
- docs-check coverage for the command and concern doc;
- release guidance for CI/PR trailer integration.

Product teams own:

- product-specific evidence artifacts referenced by the AIBOM;
- human review and acceptance;
- any regulated-data redaction needed before prompts, logs, or artifacts leave
  the product boundary.

