# IX Runtime image-size GHSA Acceptance

Status: human-risk-accepted-for-poc

Spec depth: lightweight intent note

Owner roles:

- Human risk owner: named GO below (option 2)
- Coding: record the accept in `dependency-check.toml` only
- Integration Branch Manager: successor SHA on `integrate/ix-runtime-convergence-r2`

Related: [IX Runtime RUSTSEC-2023-0071 Remediation](ix-runtime-rsa-advisory-remediation.md)
records a different GO and must not be used to accept Marvin or these npm
advisories.

## Business Value

Unblock the Supply chain & lint gate for the IX integration train's PoC
architecture-validation moment without downgrading React Native, rewriting the
native stack, or widening accepted risk to other npm packages.

## Problem

Pipeline 422 still fails `dependency-check --strict` on
`appfw_ui/pds_health/native-components` because Metro (React Native) locks
`image-size@1.2.1`. The published advisories are:

- `GHSA-5p2g-fcmc-qvqq`
- `GHSA-w3rx-r6r6-pgpr`

There is no published advisory-clean metro or image-size. npm's only advertised
fix is a React Native 0.72.17 downgrade, which is forbidden. This note records
leftover risk on paper. It does not claim the GHSAs are false or fixed.

## Named Human GO

`GO human-risk: accept only native-components image-size GHSA-5p2g-fcmc-qvqq
and GHSA-w3rx-r6r6-pgpr in dependency-check until a published metro/image-size
exists — do not accept nanoid/ajv`

Date: 2026-08-18. Human chose option 2.

## Goals

- Record the two GHSAs in the existing `dependency-check.toml` `[[osv.accepted]]`
  contract, scoped to `package = "image-size"`, `ecosystem = "npm"`,
  `version = "1.2.1"` (the tightest match the scanner supports; it has no
  lockfile-path field).
- Honor those same accepted leaf GHSAs when classifying native-components
  `npm audit` ancestor rows (`metro` / React Native). Do not add a second
  exception file or a lockfile-wide npm audit ignore.
- Keep `blocking_finding_count` 0 under `scripts/appfw dependency-check --json --strict`.
- Expire the accepts on `2026-09-30` so the gate fails closed if they are not
  reviewed.

## Non-Goals

- Accepting `GHSA-2v37-7h3g-55p8` (nanoid), `GHSA-2g4f-4pwh-qvx6` (ajv),
  `RUSTSEC-2023-0071`, dotenv, or any other advisory.
- Downgrading React Native or starting an RN rewrite.
- Adding npm audit exceptions that hide other packages.
- Claiming the GHSAs are false, fixed, or production-accepted beyond this PoC
  paper accept.
- SRA/CAB approval or extra accepted-risk.

## Retirement Criterion

Remove both `[[osv.accepted]]` entries when a published metro or image-size
release drops `GHSA-5p2g-fcmc-qvqq` and `GHSA-w3rx-r6r6-pgpr` from the locked
native-components graph. Do not roll the accepts forward without a new named
human GO.

## Acceptance Evidence

- `dependency-check.toml` contains only those two new npm accepts, each pinned
  to `image-size` `1.2.1`.
- `scripts/appfw dependency-check --json --strict` reports
  `blocking_finding_count` 0.
- nanoid and ajv GHSAs remain unaccepted.
