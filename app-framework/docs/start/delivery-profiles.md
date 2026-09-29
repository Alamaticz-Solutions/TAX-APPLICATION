# Delivery Profiles

The tracked policy in `docs/start/delivery-profiles.json` defines the local
delivery profiles. Use the root wrapper only:

```bash
scripts/appfw mode status --json
scripts/appfw mode set accelerated --json
scripts/appfw mode set candidate --json
```

`accelerated` records focused routing and its deferred candidate gates.
`candidate` records enforce-candidate routing for the same gates. The state and
ledger live below each worktree's Git directory, so linked worktrees keep
independent local modes without tracking workstation state.

With neither state file present, status uses the tracked `accelerated` default
side-effect-free and writes nothing in fresh primary or linked worktrees.
`mode set` is the only writer: it requires a clean checkout and records the
exact source SHA. Half-present, malformed, and noncanonical persisted state fail
closed.

Status and report-only annotations describe dirty or stale source binding.
Handoff, review-brief, and change-impact JSON stdout equals the final retained
artifact including `delivery_profile`; this annotation does not weaken handoff,
review, or pre-push freshness and does not approve release, risk, merge, or
review.
