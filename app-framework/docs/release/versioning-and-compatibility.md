# Versioning And Compatibility

This document defines the framework release contract that downstream product
teams and agents should use when deciding whether a framework checkout is safe
to adopt, upgrade, or support.

App Framework is currently `0.1.1` and has no production-certified tagged
release. Until the first approved `v*` release candidate exists, downstream
apps should treat framework consumption as pinned internal pilot usage through
`appfw.lock`, not as a supported enterprise product release.

## SemVer Policy

App Framework uses SemVer for tagged framework releases:

- `MAJOR` changes may break generated output, product extension APIs, provider
  behavior, CLI JSON contracts, or release evidence schemas.
- `MINOR` changes may add generated capabilities, provider areas, CLI commands,
  optional runtime modules, scaffolds, or evidence artifacts while preserving
  documented compatibility.
- `PATCH` changes should preserve generated interfaces and focus on bug fixes,
  security fixes, docs corrections, and non-breaking evidence improvements.
- Release candidates use `-rc.N` suffixes and are not production-certified until
  the release authority approves the retained evidence bundle.

Before `1.0.0`, `0.x` releases may still change APIs, but every breaking change
must be called out in the changelog and compatibility matrix.

## Compatibility Matrix

| Surface | Current Contract | Compatibility Rule | Release Evidence |
| --- | --- | --- | --- |
| Framework version | `0.1.1` / no supported tag | Product apps pin `appfw.lock`; no external support promise until first `v*` release candidate. | `appfw.lock`, release notes, tag build artifacts |
| CLI namespace | `scripts/appfw product ...`; `scripts/appfw framework ...` | Product/framework namespaces are canonical; flat commands remain compatibility aliases. | `scripts/appfw framework cli-test --json`; `scripts/appfw framework docs-check --json` |
| GraphQL schema HTTP route | `appfw_codegen::schema_route_segment` | Schema names stay snake_case (`nexus_work`); HTTP mounts are Inflector 0.11 kebab after a leading-slash trim (`nexus-work`). One package-resolvable crate function used by `appfw_introspect` and `appfw-test`; additive `schema_model.graphql_http_route` on model-status. | `cargo test --locked -p appfw-test -- graphql_client`; `cargo test --locked --bin appfw_introspect cli_contract_schema_route_segment` |
| Product workspace | Split-root app owns `.appfw/model`, generated output, handlers, services, frontend, deployment overlays | Generated files are replaceable; human-owned extension points survive regeneration. | `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; `scripts/appfw product handoff --json` |
| Codegen API | `appfw-codegen` crate root and compatibility binaries | Public crate-root API is stable within a release line; internal `app_gen/src` modules are framework-owned. | `scripts/appfw framework validate --json`; `scripts/appfw framework generate --check --json` |
| Runtime API | `appfw-runtime` contracts for ingress, operation, provider-neutral data, auth, observability, admin/MCP shells | Product code should import stable runtime/product API surfaces, not framework internals. | `scripts/appfw framework feature-check --json`; boundary checks |
| Providers | PostgreSQL, MongoDB, MS SQL Server, Snowflake CRUD parity; Neo4j graph-read posture is outside CRUD parity | CRUD providers must pass declared live-certified areas; graph providers need separate posture and certification before release claims. | `scripts/appfw framework provider-test --all --json`; provider capability matrix |
| Product frontend | Product-owned `frontend/`, generated UI contract placeholder, PDS Health components/tokens | Product screens must not fork admin UI or copy CRM-specific UX as the product model. | `scripts/appfw product frontend-test --json`; PDS component check |
| Release evidence | Local checks plus live provider/security/ops/perf/supply-chain evidence when in scope | Local evidence cannot replace provider-backed release evidence. | `scripts/appfw framework release-check --json`; CI retained artifacts |
| PDS Health | `@appfw/pds-health-components` has its own package changelog and lifecycle policy | PDS package compatibility must be named in framework release notes when frontend scaffold behavior changes. | `scripts/check-pds-components.mjs --json`; PDS changelog |

## Breaking Change Rules

A framework change is breaking when it requires downstream product teams to
change any of these without an automated migration or clear compatibility
adapter:

- `.appfw/model` config contract;
- `.appfw/manifest.yaml` topology contract;
- generated handler/service extension signatures;
- product API/runtime import paths;
- provider capability semantics;
- CLI JSON fields used by agents or CI;
- release evidence artifact schema;
- frontend scaffold ownership or generated UI contract shape.

Breaking changes must include:

1. changelog entry;
2. upgrade note;
3. migration or manual remediation steps;
4. affected product surfaces;
5. verification commands;
6. compatibility impact in release notes.

## Release Notes Template

Each release candidate should include:

```text
Version:
Git tag:
Release date:
Release type: rc | minor | patch | major
Supported consumers:
Supported providers:
Supported frontend/PDS versions:
Compatibility changes:
Breaking changes:
Security fixes:
Generated drift expectations:
Upgrade steps:
Required product verification:
Release evidence bundle:
Known limitations:
Risk acceptances:
Live-environment work items closed:
Live-environment work items deferred:
```

## Open Productization Decisions

These decisions are required before external or regulated production adoption:

- license selection and `LICENSE` file;
- support owner and response targets;
- external vulnerability reporting channel;
- support window for the first release line;
- package distribution target for framework crates, CLI, and PDS components;
- signed artifact and provenance system of record.

Track environment-owned or release-authority-owned items in
[`live-environment-work-items.md`](live-environment-work-items.md).
