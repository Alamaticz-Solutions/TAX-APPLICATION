# Architectural Decision Records

ADRs capture framework-level decisions that affect generated applications,
provider behavior, or release gates. Keep them short, durable, and explicit
about consequences.

| ADR | Decision |
| --- | --- |
| [0001 Query IR](0001-query-ir.md) | Shared query, selection, mutation, aggregation, and pagination intent before provider compilation. |
| [0002 Provider Certification](0002-provider-certification.md) | Provider support is release-gated by explicit executable contracts and status tiers. |
| [0003 Relationship Source Of Truth](0003-relationship-source-of-truth.md) | Schema-level relationships generate directional navigation fields. |
| [0004 Audit Design](0004-audit-design.md) | Audit is append-only, scoped, redacted, and provider-certified before being claimed enterprise-ready. |
| [0005 Frontend Scaffold Stack](0005-frontend-scaffold-stack.md) | Enterprise product frontends are React + TypeScript + Vite SPAs over GraphQL (Chart.js; SSR deferred). |
| [0006 Frontend/Backend Contract](0006-frontend-backend-contract.md) | A typed UI contract and GraphQL client are generated from the backend model and drift-checked. |
| [0007 Frontend Generation And Ownership](0007-frontend-generation-and-ownership.md) | Hybrid: generated overwrite-safe scaffolds plus preserved human-owned customization, via split roots, manifest, and drift check. |
| [0008 PDS Health Frontend Design System](0008-frontend-design-system.md) | PDS Health branded tokens and source-in-repo primitives, with CRM as reference/E2E fixture only. |
| [0009 Frontend Shell And Archetypes](0009-frontend-shell-and-archetypes.md) | One standardized app shell plus CRUD, analytical-report, and document/workflow archetype kits. |
| [0010 Frontend Security And Data Governance](0010-frontend-security-and-data-governance.md) | Default-on Okta/JWT and role gating; synthetic data in code, real data only via governed feeds and row-level policy. |
| [0011 Frontend Test Backbone](0011-frontend-test-backbone.md) | Typecheck, contract-drift, component, and e2e tests as CI gates; the CRM frontend is the first fixture. |
| [0012 Frontend Agent Enablement](0012-frontend-agent-enablement.md) | Task map, examples, generated surfaces, and conversion skills make the scaffold agent-driven. |
| [0013 Docs Information Architecture](0013-docs-information-architecture.md) | Documentation is lifecycle-first, with skills as procedure, Markdown as durable knowledge, CLI as execution, and artifacts as proof. |
| [0014 PDS Health Signature Visual Language](0014-pds-signature-visual-language.md) | Precision Daylight, Chromatic Signal, and Connected Fabric define the bounded PDS signature visual language. |
| [0015 PDS Experience System Dimensions And Layers](0015-pds-experience-system-dimensions.md) | One experience system separates doctrine, foundations, adaptation, primitives, components, patterns, floorplans, and journey proof. |
| [0016 PDS Web Interaction Substrate](0016-pds-web-interaction-substrate.md) | React Aria Components supplies commodity web interaction behind PDS-owned APIs and visual semantics. |
| [0017 Web/Native Experience Projection Boundary](0017-web-native-experience-projection-boundary.md) | Shared experience semantics project separately into real Web, native iOS, and native Android renderers with explicit size class, applicability, and readiness evidence. |
| [0018 Provider Auth-Mode Plurality](0018-provider-auth-mode-plurality.md) | A provider may carry multiple permanent co-equal auth mechanisms selected by per-environment `auth_mode`; connection-level auth modes are certified by dedicated hermetic auth/connection tests (not a second full semantic `provider-parity` suite), mirroring TLS-mode. |

ADRs 0005–0012 are the enterprise frontend scaffold decision set. They are
accepted but **provisional**: the CRM reference frontend is their forcing
function, and analytical/reporting and document-generation archetypes
must confirm or correct them before the abstractions are trusted (rule of three).
