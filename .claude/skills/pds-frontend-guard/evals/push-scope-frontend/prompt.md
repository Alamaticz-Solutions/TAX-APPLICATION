---
description: A push that changes the frontend runs the short frontend verification, not the database or Rust build.
tags: [push]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Skill]
---

I only changed a React screen under `tax-doc-routing/frontend/src/features/` and committed it. Before I push it, what does our pds-frontend-guard make me run, and roughly how long does it take? Do I need the database or the Rust backend built first?
