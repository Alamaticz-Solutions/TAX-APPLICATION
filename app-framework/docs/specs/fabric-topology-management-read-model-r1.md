# Fabric Topology And Management Read Model R1

Status: selected dormant Framework contract; implementation and independent
review required; no adapter, Product, release, or risk credit

## Outcome

Establish a provider-neutral, deterministic in-memory foundation for describing
Framework-local application topology and for projecting only the portion an
authorized caller may see. The foundation gives future control-plane consumers
typed contracts without prematurely choosing a route, store, identity system,
event transport, health service, dashboard, or Product experience.

## Closed runtime records

`appfw.fabric.component@1` identifies an application, user interface, service,
database, event topic, integration, or artificial-intelligence service. Each
record binds stable identity, owner, version, revision, environment,
classification, lifecycle, provenance, typed dependencies, typed evidence,
and health/availability posture.

`appfw.fabric.topology@1` reduces revision histories to the latest valid record,
sorts nodes and relationships deterministically, and binds the closed result to
a stable SHA-256 digest. It rejects duplicate revisions, stable-identity
conflicts, lifecycle regression, dangling or kind-invalid relationships,
cross-owner containment, duplicate dependencies/evidence, malformed
provenance, open payloads, and sensitive metadata.

The registry preserves composition, dependency, invocation, read/write, and
publish/consume relationships as distinct types. It does not collapse domain
stores or event topics into ambiguous shared edges.

## Scoped management projection

`appfw.fabric.management_overview@1` and
`appfw.fabric.component_detail@1` are deterministic read models over a
validated registry. The caller supplies a pre-authorized `FabricReadScope`.
Environment and classification are required; owner and component filters may
only narrow through intersection. Invalid, empty, excessive, or disjoint
scope fails closed.

Hidden components are removed before summaries and digests are computed.
Relationships require both visible endpoints. Detail lookup returns the same
`NotAvailable` result for missing and unauthorized identifiers. The read model
therefore does not reveal a hidden identifier through edges, counts, digests,
or distinguishable errors.

Evidence coverage reports typed present and absent kinds without treating
absence as success. Health preserves its declared/local/observed source and
evidence binding. Observed health is valid only when it references verified
health evidence, and the closed health/availability matrix rejects optimistic
or contradictory combinations.

## Telemetry boundary

`appfw.fabric.telemetry@1` contains exactly five event kinds: registry loaded,
topology projected, validation failed, component health changed, and evidence
posture changed. Events retain bounded request and correlation identifiers and
only event-specific typed fields. Unknown fields, open payloads, sensitive
identifiers, and incoherent health shapes fail closed.

## Framework-local fixture

The test fixture is freshly authored at
`appfw_runtime/tests/fixtures/fabric/framework-local-topology.v1.json`. It uses
generic Framework-local systems, owners, and references. It contains no
tenant data, personal data, health data, credentials, tokens, provider URLs,
private keys, or Product claim. Its 13 records and 23 typed relationships
exercise every component kind and the separation of composition, shared
dependency, database flow, event flow, integration, and AI-service edges.

## Ownership and nonclaims

This lane owns only the twelve reviewed runtime, test, fixture, spec, and index
paths. Root/runtime manifests, features, dependencies, `Cargo.lock`, the
accepted private sensitive-metadata helper, generated output, provider code,
and Product code remain unchanged.

The module is dormant. It provides no route, persistence adapter, identity or
authorization provider, approval workflow, live probe, Grafana/UI integration,
deployment, release, security certification, Product adoption, or stable
public compatibility promise. A future consumer must obtain its own ownership,
security, environment, proof, and release decisions.

## Acceptance evidence

- all 13 registry scenarios and six management-read-model scenarios pass with
  none ignored or required-filtered;
- all accepted sensitive-metadata helper tests pass unchanged;
- topology and management digests and ordering are deterministic;
- lifecycle, relationship, provenance, closed-shape, sensitive-data, scope,
  hidden-edge, evidence, and health negative cases fail closed;
- the provider-neutral fixture and this specification match the exact path
  envelope and explicit nonclaims;
- manifests, lock, dependencies, helper, and all out-of-envelope bytes remain
  accepted-base exact; and
- Framework validation, docs, generation-check, fast tests, handoff,
  comprehensive independent review, Integration exact-SHA/currentness, PR CI,
  Wayne-only merge, and destination-green evidence close in order.
