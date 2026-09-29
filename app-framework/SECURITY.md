# Security Policy

App Framework is designed for secure-by-default generated enterprise
applications, but this repository is not yet production-certified as a
versioned external product.

## Supported Versions

| Version | Security Support |
| --- | --- |
| `0.1.x` / unreleased checkout | Internal pilot and release-candidate review only. Security fixes are handled on active branches until the first tagged release exists. |

Tagged release support starts with the first approved `v*` release candidate.
The support window and backport policy for that line must be recorded in
[`SUPPORT.md`](SUPPORT.md) and the release notes before external adoption.

## Reporting Vulnerabilities

Do not open public issues or commit proof-of-concept exploit details for
vulnerabilities, secrets, tenant data, PHI/ePHI, or credential material.

For internal use, report suspected vulnerabilities through the approved
security-response process for the owning organization and include:

- affected commit, branch, or release candidate;
- affected product app or framework surface;
- reproduction steps or retained artifact paths;
- data classification and tenant impact, if known;
- whether the issue affects generated output, runtime/provider behavior,
  product-owned code, CI/CD, deployment, or observability.

Before external distribution, this file must be updated with the approved
external contact channel, expected initial response time, disclosure process,
and supported release lines.

## Security Evidence

Release candidates must retain the relevant evidence described in:

- [`docs/release/release-gate-ci-cd.md`](docs/release/release-gate-ci-cd.md)
- [`docs/release/deployment-reference.md`](docs/release/deployment-reference.md)
- [`docs/release/live-environment-work-items.md`](docs/release/live-environment-work-items.md)
- [`docs/architecture/concerns/threat-model.md`](docs/architecture/concerns/threat-model.md)

Missing DAST, SAST/ASVS, provenance, signing, live provider security, live ops,
or deployable-image evidence requires formal, time-boxed risk acceptance in the
release bundle.
