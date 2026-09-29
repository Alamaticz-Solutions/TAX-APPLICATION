# Support Policy

App Framework is currently pre-release framework software. Product teams may use
it for controlled internal pilots, but enterprise release support starts only
after a tagged release candidate passes the required release gate and retained
evidence is approved.

## Current Support Posture

| Audience | Current Status | Support Expectation |
| --- | --- | --- |
| Framework stewards | Supported for active development | Use `scripts/appfw framework handoff --json`, release artifacts, and branch/PR review. |
| Internal product pilots | Supported with explicit risk awareness | Pin framework provenance with `appfw.lock`, retain validation/generation/test/handoff evidence, and escalate framework defects through the owning team. |
| External or regulated production consumers | Not yet supported | Requires tagged release, live provider/security evidence, strict supply-chain evidence, support owner, security response path, license decision, and release notes. |

## Release-Line Support

No supported release line exists yet.

The first supported release line must define:

- version identifier and tag;
- supported provider matrix;
- supported product workspace contract;
- frontend scaffold and PDS compatibility;
- minimum Rust and Node versions;
- security-response target;
- patch/backport policy;
- deprecation/removal policy;
- support owner and escalation path.

Record that policy in
[`docs/release/versioning-and-compatibility.md`](docs/release/versioning-and-compatibility.md)
and the release notes before declaring enterprise support.

## Product Team Responsibilities

Product teams consuming App Framework remain responsible for product-specific
modeling, handlers, services, policy fixtures, deployment overlays, data
classification, secrets, operational runbooks, and release evidence.

Use:

- `scripts/appfw product validate --json`
- `scripts/appfw product generate --check --json`
- `scripts/appfw product test --fast --json`
- `scripts/appfw product handoff --json`
- `scripts/appfw product upgrade --json`

When live services are in scope, add product API, migration, frontend,
performance, and release checks as documented in `docs/product/README.md`.
