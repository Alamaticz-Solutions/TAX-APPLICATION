# ADR 0002: Provider Certification

## Status

Accepted.

## Context

Multi-provider support is an enterprise promise only if the framework can prove
runtime behavior is equivalent where support is claimed. A coarse support
matrix is not enough because broad categories such as relationships and
many-to-many hide separate projection, filtering, mutation, and access-control
concerns.

## Decision

Provider support is represented by granular `ProviderContractArea` entries with
explicit status tiers:

- `Unsupported`
- `Partial`
- `Implemented`
- `CompilerContracted`
- `LiveCertified`
- `EmulatorLimited`

Capability declarations must carry evidence. Live-certified areas require a
live contract that executed and passed. Provider-test JSON reports are
area-driven so release gates can fail on a specific overclaim.

## Consequences

- PostgreSQL, MongoDB, MS SQL Server, and Snowflake can have honest support
  tiers without hiding provider limitations.
- Release CI can use `scripts/appfw provider-test --all --json` as the top
  semantic parity gate.
- Capability export logic belongs to the backend library; CLI binaries are thin
  adapters.
- New provider features should add or update executable contracts before being
  marked live-certified.
