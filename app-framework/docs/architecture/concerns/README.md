# Architecture Concerns

This folder holds cross-cutting architecture concerns that need durable
visibility but should not clutter lifecycle flow.

| Concern | Guide |
| --- | --- |
| Threat model and security risk posture | [Threat Model](threat-model.md) |
| Agentic development control system | [Agentic Development Control System](agentic-development-control-system.md) |
| Agentic threat model and sandbox posture | [Agentic Threat Model](agentic-threat-model.md) |
| Maintainability and documentation IA | [Maintainability](maintainability.md) |
| Packaging and product boundary | [Packaging](packaging.md) |
| Runtime modularity and ingress convergence | [Runtime Modularity](runtime-modularity.md) |
| North-Star Wave 0 contract freeze | [Wave 0 Contract Freeze](north-star-wave-0.md) |
| Wave 0 spec artifact contracts | [Wave 0 Spec Artifact Contracts](north-star-wave-0-specs.md) |

If a concern becomes a historical investigation rather than current operating
guidance, move it to `docs/archive/`.
