---
description: A push with no frontend change has no PDS checks to run and is an ordinary push.
tags: [push]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Skill]
---

I only changed a Rust file under `tax-doc-routing/backend/src/services/` and committed it, nothing under `tax-doc-routing/frontend/`. With our pds-frontend-guard in place, do I have to run any verification before I push it, and will the pre-push hook block me?
