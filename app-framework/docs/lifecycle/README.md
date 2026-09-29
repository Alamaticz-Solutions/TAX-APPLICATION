# Product Delivery Lifecycle

The lifecycle is the spine of App Framework documentation. Every product or
framework change should be understood as one or more lifecycle phases with
inputs, edit surfaces, commands, evidence, and exit criteria.

| Phase | Purpose | Primary Guide |
| --- | --- | --- |
| 1. Intake and discovery | Turn PoC artifacts or legacy systems into product intent. | [PoC Intake](intake-and-discovery.md), [Legacy Modernization](legacy-modernization.md) |
| 2. Product bootstrap | Create a product-shaped repo with clean topology. | [Product Golden Path](product-golden-path.md) |
| 3. Model and contracts | Define schemas, relationships, data sources, and generated contracts. | [Model Docs](../model/README.md) |
| 4. Generate and extend | Generate artifacts and add human-owned product behavior. | [Application Lifecycle](application-lifecycle.md) |
| 5. Run local | Start providers, backend, admin UI, and product UI. | [Getting Started](../start/getting-started.md) |
| 6. Test and certify | Prove unit, API, provider, frontend, performance, security, and docs surfaces. | [Application Lifecycle](application-lifecycle.md) |
| 7. Release and deploy | Produce retained evidence, release gates, deployment, and rollback proof. | [Release Docs](../release/README.md) |
| 8. Operate and observe | Run with logs, metrics, readiness, traces, and diagnostics. | [Observability](../runtime/observability.md) |
| 9. Upgrade and evolve | Cascade framework changes into product repos safely. | [Application Lifecycle](application-lifecycle.md) |
| 10. Handoff | Preserve machine-readable continuation state. | [Agent Task Map](../start/agent-task-map.md) |

Agent skills in `agent_skills/` are procedural cards for these phases. The
Markdown docs in this directory and its sibling reference folders are the
knowledge source of truth.
