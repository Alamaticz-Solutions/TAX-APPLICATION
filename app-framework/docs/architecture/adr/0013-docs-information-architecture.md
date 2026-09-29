# ADR 0013: Lifecycle-First Documentation Information Architecture

## Status

Accepted.

## Context

App Framework documentation had grown into a flat numbered list plus several
root-level reference files. That made it harder for agents and developers to
know where to start, which document owned durable knowledge, and which command
proved a lifecycle phase was complete.

The framework is now explicitly agentic-first and audience-split:

- skills provide concise workflow procedure;
- Markdown docs provide durable knowledge;
- `scripts/appfw` provides execution;
- JSON artifacts provide proof.
- product developers use App Framework to build product apps;
- framework stewards evolve App Framework itself.

The docs structure needs to express that model.

## Decision

Adopt an audience-aware, lifecycle-first documentation architecture.

The durable seam is:

```text
product developer path  -> scripts/appfw product ...
framework steward path  -> scripts/appfw framework ...
```

Keep one CLI binary, `appfw`, with two first-class namespaces. Flat commands may
remain as compatibility aliases, but new docs, skills, prompts, and CI examples
should use the namespace that matches the work.

Organize docs around the product delivery lifecycle first:

```text
docs/lifecycle/
docs/start/
docs/model/
docs/reference/
docs/runtime/
docs/frontend/
docs/release/
docs/architecture/
docs/archive/
```

Lifecycle docs are the delivery spine. Reference, model, runtime, frontend,
release, and architecture folders provide deeper knowledge for each phase.
Architecture concerns, ADRs, and the threat model live under
`docs/architecture/` so they stay visible without becoming the first screen for
product developers.

Root-level compatibility stubs are not retained. Update agents, scripts,
downstream repos, and prompts to the canonical foldered paths.

Follow these design decisions for future documentation work:

- Start by deciding audience: product developer or framework steward. Product
  docs teach teams how to build with the framework. Framework docs teach
  stewards how to evolve the framework.
- Start from reader intent, not implementation chronology. A new doc must serve
  a clear job in start, lifecycle, model, reference, runtime, frontend, release,
  architecture, or archive.
- Skills are procedural overlays and must declare their audience, lifecycle
  phase, CLI namespace, proof commands, and artifacts. They should route to
  canonical docs and artifacts instead of becoming a second knowledge base.
- Lifecycle docs own the path from intake through release and upgrade. Reference
  docs own stable contracts. Architecture docs own framework intent and ADRs.
- Concern docs under `docs/architecture/concerns/` own cross-cutting posture
  such as security, maintainability, and future architecture concerns.
- ADRs own durable decisions. They should not become implementation diaries or
  release-status ledgers.
- Archive owns obsolete branch notes, scratch plans, and historical reviews.
  Live docs should describe what to do now.
- Every command contract or agent-facing example should be guarded by
  `scripts/appfw docs-check --json` unless it needs live services or release CI.
- Canonical docs, skills, scripts, and prompts should point at the foldered
  paths; do not create root-level redirect stubs.

## Consequences

- `docs/README.md` becomes the only top-level docs router.
- `docs/product/README.md` and `docs/framework/README.md` are the audience
  routers beneath the top-level docs index.
- Agent skills point to audience/lifecycle/reference docs instead of
  duplicating them.
- New docs must declare which lifecycle phase or reference domain owns them.
- New CLI examples should use `scripts/appfw product ...` or
  `scripts/appfw framework ...`; flat commands are compatibility aliases.
- `scripts/appfw docs-check --json` must guard the hierarchy and key links.
- Historical branch notes and obsolete planning detail stay in `docs/archive/`.
- Root-level docs stay sparse: `docs/README.md` is the router, not a growing
  wall of Markdown files or redirect stubs.
