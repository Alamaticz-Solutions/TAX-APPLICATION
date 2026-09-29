# IX Runtime RUSTSEC-2023-0071 Remediation

Status: accepted-for-implementation

Spec depth: lightweight

Owner roles:

- Architect / Coding: this assignment
- Integration Branch Manager: successor SHA on `integrate/ix-runtime-convergence-r2`
- Human risk owner: required only to baseline the historical test-key
  fingerprint on `272620942`, or to authorize a separately named history rewrite

## Business Value

Clear the Supply chain & lint and Secret scan gates on PR 477 without accepting
RUSTSEC-2023-0071 (Marvin), without committing a private key, and without
weakening production IX JWT or stored-run seal contracts.

## Problem

`appfw-runtime` first added a direct `rsa 0.9` dev-dependency so IX tests could
mint ephemeral RS256 keys and serve a matching JWKS. `cargo audit` failed
because `rsa 0.9.10` has no fixed upgrade (pipeline 421 / RUSTSEC-2023-0071).

Commit `272620942` then replaced that crate with a committed PKCS#1 PEM plus
companion JWKS `n`/`e`. That cleared the advisory and kept RS256 coverage, but
pipeline 422 Secret scan (`gitleaks detect --full-history`) correctly flagged
the PEM as a new `private-key` leak. Production IX stored-run sealing already
uses HMAC (`hmac` / `sha2`). Production JWT verification already uses
`okta-jwt-verifier` / `jsonwebtoken` (`ring`), not `rsa`.

## Goals

- `cargo audit` exits 0 under the existing allowed-warning policy only.
- Keep IX JWT tests on RS256 + JWKS.
- Remint RS256 test keys at runtime so the tip contains no private-key material.
- Do not ignore RUSTSEC-2023-0071.
- Do not add `chat` to default features.
- Do not baseline or rewrite the historical fingerprint in this GO.

## Non-Goals

- Changing production JWT algorithms or the HMAC stored-run sealer.
- Accepting Marvin as residual risk.
- Retrying pipelines 418/419/420/421.
- Broad dependency upgrades.
- Baselining the historical PEM fingerprint or rewriting git history.

## Options Considered

| Option | Decision |
| --- | --- |
| Ignore RUSTSEC-2023-0071 in `deny.toml` / cargo-audit | Rejected; no human risk GO |
| Switch tests to HS256 | Rejected; would stop exercising the RS256 JWKS path |
| Keep `rsa` 0.9.x for ephemeral keygen | Rejected; no fixed upgrade |
| Static test-only RS256 PEM + JWKS `n`/`e`; drop `rsa` | Rejected after pipeline 422; committed private key is a true positive |
| Baseline the historical fingerprint while the PEM remains at tip | Rejected; scanner suppression needs a separate human risk GO |
| Runtime remint via OpenSSL CLI already present in CI/dev; drop `rsa` | Chosen |
| History rewrite to excise `272620942` | Out of scope; separately named GO only |

## Security, Privacy, And Governance

Runtime-minted keys stay in process memory for the test binary. They must not
be persisted, committed, or reused as a production secret. Removing `rsa`
eliminates the Marvin crate from the lockfile. This change does not accept
residual risk and does not authorize history rewrite or baseline updates.

Secret scan is `gitleaks detect --log-opts=--full-history HEAD`. After the tip
is clean, commit `272620942` still contains the PEM. The next named GO is either
a human risk GO to baseline that one historical test-key fingerprint, or a
separately named history-rewrite GO.

## Acceptance Evidence

- Tip has no committed PKCS#1/PKCS#8 PEM or static private-key companion `n`/`e`.
- `cargo audit` has no RUSTSEC-2023-0071 finding.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo fmt --all --check`
- Focused `appfw-runtime` IX JWT / transport tests with `--features chat`
- `bash scripts/ci/secret-scan.sh` from the integrate worktree
- `scripts/appfw framework validate --json` and framework handoff from the
  integrate worktree
