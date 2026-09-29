# CLI Quickstart

Use this page when you need the right command path in the first two minutes.
Use the full [CLI Reference](../reference/cli.md) when you need every option,
artifact, or JSON contract.

App Framework uses one CLI with two canonical namespaces:

```bash
scripts/appfw product <command>
scripts/appfw framework <command>
```

Use `product` when building, running, validating, testing, releasing,
upgrading, or handing off a downstream product app. Use `framework` when
changing App Framework itself: docs IA, CLI contracts, generator behavior,
runtime ingress, providers, dependency policy, release gates, packaging, or
reusable scaffolds.

Flat commands such as `scripts/appfw validate --json` remain compatibility
aliases. New docs, skills, prompts, and CI examples should use the explicit
namespace.

## Discover Context

```bash
scripts/appfw context --json
scripts/appfw lifecycle --json
```

`context` reports the framework root, app root, manifest, and recommended
namespace. `lifecycle` maps product and framework phases to skills, docs,
commands, and retained artifacts.

## Resolve Current-Task Instructions

Before material work, resolve the exact role card, skill, canonical references,
and independent review route selected for the assigned task and change class:

```bash
scripts/appfw framework instructions --task framework-docs-ia --role coding-agent --change-class C --json
```

Use the `product` namespace only for an explicitly supported `product-*` task,
such as `product-frontend`. The `appfw_instruction_route@1` response is
deterministic and source-linked. It does not copy the instructions, start work,
change WIP or authority, or allow the implementer to certify its own route.

## Set Delivery Mode

Use the root-only delivery controller for the current worktree's local profile;
mode state is stored below each worktree's Git directory:

```bash
scripts/appfw mode status --json
scripts/appfw mode set accelerated --json
scripts/appfw mode set candidate --json
```

It records focused or enforce-candidate routing and annotates evidence reports;
when no local state exists, status projects the tracked `accelerated` default
side-effect-free without writing state in primary or linked worktrees. Only
`mode set` writes state, and it requires a clean checkout bound to the exact
source SHA. Half-present, malformed, or noncanonical state fails closed. Dirty
or stale status and evidence annotations are descriptive; handoff, review-brief,
and change-impact JSON stdout is their final retained artifact including
`delivery_profile`. This does not change handoff/review freshness or review,
merge, and release authority.

## Product App Loop

```bash
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

Use this loop for downstream product model, generated artifact, frontend, or
release evidence work. For a mobile HTML mockup or React Native product app,
add:

```bash
scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json
```

When local services are running, add:

```bash
scripts/appfw product migrate
scripts/appfw product serve
scripts/appfw product api-test
scripts/appfw product frontend-test --json
scripts/appfw product release-check --json
```

To prove the product-intake/analyze/proposal path from the framework checkout
without creating a durable app repo, run:

```bash
scripts/appfw framework intake-proof --json
```

## Framework Steward Loop

```bash
scripts/appfw framework cli-test --json
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

Use this loop for docs, CLI, generator, runtime, provider, packaging, release,
dependency, or scaffold changes. `cli-test` is the fast command-routing lane;
`docs-check --changed-only` selects the faster static lane unless command or
agent-facing contracts changed. Use `docs-check --full --json` for release-grade
docs/example evidence. Add risk-specific certification when the surface changes:

```bash
scripts/appfw framework feature-check --json
scripts/appfw framework provider-test --all --json
scripts/appfw framework provider-performance --json --all
scripts/appfw framework release-check --json
scripts/appfw framework dependency-check --json --offline
```

## Agent Skills

Route work through the smallest skill that matches the task:

```bash
scripts/appfw product skills --json
scripts/appfw framework skills --json
```

Skills are procedural overlays on canonical docs. They should not duplicate
large reference material.
