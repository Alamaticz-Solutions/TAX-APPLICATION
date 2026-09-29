# ADR 0012: Frontend Agent Enablement

## Status

Accepted (provisional — refined alongside the CRM reference frontend).

## Context

The scaffold exists so that Claude agents can turn a self-contained HTML artifact
plus its data into an enterprise frontend quickly and safely. That requires the
same agent-first affordances the backend has: explicit edit-surfaces, intent
routing, examples, and fast feedback. The architectural decisions are what make
the work fast — they collapse an open-ended port into a bounded mapping.

## Decision

Agent enablement is a first-class part of the frontend scaffold:

- A frontend `CLAUDE.md`/`AGENTS.md` and an agent task map routing intent to the
  exact file ("add a form field", "customize a list", "new custom screen", "wire
  a chart").
- Golden example screens as few-shot references.
- The generated contract, ownership manifest, and drift/typecheck loop give
  agents typed surfaces and fast, safe feedback.
- The conversion stages — extract the model, logic, and UI from the artifact;
  generate; lift styles; verify fidelity — are codified as reusable skills.

## Consequences

- Converting a vibe artifact becomes a bounded, repeatable, largely mechanical
  agent run against a known target.
- Agents edit human-owned surfaces with confidence; the ownership boundary and
  drift check protect them.
- Documentation and examples are part of the product and are maintained with the
  scaffold.
- Numeric/visual fidelity and data governance remain deliberately human-gated
  (see ADR 0010 and ADR 0011).
