# Changelog

All notable framework-level changes should be recorded in this file.

This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
formatting and the versioning policy in
[`docs/release/versioning-and-compatibility.md`](docs/release/versioning-and-compatibility.md).

## Unreleased

### Added

- Added the provider-neutral Intelligent Experience lifecycle and PDS recipe
  packages to the unpublished `appfw-runtime 0.2.0` candidate, together with
  strict signed-JWT POST/SSE/cancel transport and durable repository extension
  contracts. This local candidate does not claim package publication, Product
  acceptance, live-provider readiness, security approval, release, or
  deployment.
- Added a live-environment work-item register for release tasks that require
  managed providers, CI/CD, observability, security tooling, or release
  authority.
- Added explicit product-intake starter-mode discovery to
  `appfw product new --list-profiles --json`.
- Added release-check JSON diagnostics for release readiness, evidence mode,
  missing provider base URLs, and root-cause summaries.
- Added framework-level security, support, and version compatibility policy
  documents.

### Changed

- `docs-check` now guards product-intake starter-mode discoverability.

### Release Status

- No production-certified framework release has been cut yet.
- Live provider-backed release evidence, strict supply-chain evidence,
  production security-assurance evidence, and release authority decisions remain
  required before the first enterprise release candidate.
