# Fabric Contract Schemas (v1)

The nine App Fabric contract schemas ratified from the held
[Hosted Product Factory Spec](../../docs/specs/hosted-product-factory.md)
(Contracts Touched, Registration and composition, Mobile platform surfaces).
Their integrated delivery sequence and any proposed contract-delta work are
tracked in the [App Fabric Master Implementation
Plan](../../docs/release/app-fabric-master-implementation-plan.md). Style
follows `scripts/agent-routing/`: closed JSON Schema
2020-12 documents, a dependency-free validator, fixture cases, and `node:test`.

| Contract | File | Carries |
| --- | --- | --- |
| `fabric_app_registration@1` | `fabric-app-registration.v1.schema.json` | Fabric-minted identity, origination path, lifecycle, declared platform surfaces, CMDB cross-reference |
| `app_component_snapshot@1` | `app-component-snapshot.v1.schema.json` | Platform-qualified desired/built/deployed/running composition; store-release and version-skew states for native mobile |
| `fabric_observation_event@1` | `fabric-observation-event.v1.schema.json` | Append-only source-attributed observations, including framework-release, whole-cost (TCO), value-realization, satisfaction, and mobile kinds |
| `fabric_command_proposal@1` | `fabric-command-proposal.v1.schema.json` | Typed, context-bound change proposals with idempotency and disposition |
| `factory_run_manifest@1` | `factory-run-manifest.v1.schema.json` | Exact source, profile, commands, executed/deferred gates, runner and cost identity, review, result |
| `factory_required_action@1` | `factory-required-action.v1.schema.json` | Human-owned prerequisites (including CMDB matriculation) with evidence-backed fulfillment |
| `product_work_model@1` | `product-work-model.v1.schema.json` | Per-app purpose, strategy, product increment, task — the bounded product profile |
| `portfolio_model@1` | `portfolio-model.v1.schema.json` | One-level themes (journey/process/experience), portfolio strategies, app participation |
| `framework_request@1` | `framework-request.v1.schema.json` | Product-to-framework demand signals with fleet-dedup-friendly dispositions |

Business rules the schemas cannot express structurally (idea-stage apps stay
out of the CMDB, candidate runs defer no gates, fulfilled actions carry
evidence, satisfaction-class payloads exclude free text, version skew stays
first-class, and so on) live in `fabric-contracts.mjs` and run with every
validation. Every record carries a `record_digest` — the sha256 of the
canonical JSON of the record minus the digest field.

One recorded openness: `fabric_observation_event@1.payload` is an open object.
Per-kind payload minimums are enforced for the economics kinds (`whole_cost`,
`value_realization`, `satisfaction`, `store_rating`) and the mobile kinds
(`store_release`, `version_adoption`, `crash_free`); the remaining kinds'
payload shapes deliberately ratify with the FAB-A5 source adapters, whose
real authorities (Bitbucket, ArgoCD, analytics) own those field names.

Run the checks:

```bash
node scripts/check-fabric-contracts.mjs --json
```

```bash
node --test scripts/fabric-contracts/fabric-contracts.test.mjs
```

The checker retains `target/appfw/fabric-contracts-check.json`. Fixture cases
in `fabric-contract-cases.v1.json` double as worked examples — the `crm` and
`pds-nexus` registrations there are the seed records the fabric console
registers first. The Framework common PR gate now runs both checks. Any future
Fabric Console must also run them in its own CI. Conformance is not
deployed-console or live authority proof.
