---
type: llm
---

PASS if the answer says a push that touches the frontend runs `verify-all.sh`, which does only the frontend checks (the frontend gate with the PDS rule tests, then the Playwright suite on the mocked API), takes on the order of a few minutes (about 3), and needs no database and no Rust/backend build.
FAIL if the answer says the database or the Rust backend must be built or running, says the run takes tens of minutes or hours, or does not name the verification script or the frontend checks.
