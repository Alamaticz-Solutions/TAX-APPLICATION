---
type: llm
---

PASS if the answer says a push with no frontend change has no PDS checks to run (only the frontend has a guard for now): the verification script stamps it straight away or lets it through, so the person does not need to run the frontend suite or build the backend for it, and the hook does not block it.
FAIL if the answer says the person must run the full verification, the database or Rust builds, or the frontend suite for this push, or that the hook will block it.
