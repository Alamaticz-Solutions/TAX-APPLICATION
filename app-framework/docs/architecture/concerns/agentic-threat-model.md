# Agentic Threat Model

This concern defines the framework posture for coding agents, product-agent
harnesses, MCP clients, Kafka service actors, and conversational/action UI
flows. It complements the backend [Security Threat Model](threat-model.md):
that document covers runtime API security, while this one covers agentic
control-flow risks and the safeguards that later Wave 1 lanes must encode.

The roadmap's G3 lane is complete only when this threat model stays discoverable
through `scripts/appfw framework docs-check --json` and downstream harnesses
translate it into concrete command, filesystem, network, review, and handoff
limits.

## Threat Classes

| Area | Threat | Framework Posture |
| --- | --- | --- |
| ASI01 instruction/control-flow hijacking | A model, prompt, retrieved document, tool result, PoC artifact, or product data row steers the agent away from the approved task or into unsafe commands. | Agents must follow the repository task map, generated-boundary rules, and namespaced CLI path. Product commands operate inside the product workspace; framework commands operate inside the framework workspace. Handoff evidence must record what changed and which checks ran. |
| ASI02 tool misuse | An agent invokes a powerful tool outside the allowed task surface, runs release/live commands in a local lane, edits generated output instead of source, or bypasses provider/policy gates. | CLI commands are split into `scripts/appfw product ...` and `scripts/appfw framework ...`. Reserved Wave 0 commands start as `--plan --json` until their evidence contracts exist. Release, provider, MCP, Kafka, and governed-write paths must retain JSON evidence before promotion. |
| ASI03 identity and privilege abuse | An agent or agentic ingress acts as the wrong principal, reuses a user token across tenants/providers, performs a write without delegated authorization, or masks service-vs-user identity. | Runtime policy input must preserve principal type, tenant, ingress, provider, scopes, and optional on-behalf-of identity. SaaS governed writes remain unsupported until G1 evidence proves delegated actor context, token-store isolation, named mutation registry, idempotency, policy/scope enforcement, and audit. |

## Sandbox Posture

The framework assumes two layers of sandboxing and does not treat either one as
optional for enterprise product work.

| Layer | Required Posture | Evidence Hook |
| --- | --- | --- |
| Filesystem sandbox | Product agents write only inside the product app root and declared writable outputs. Framework agents write only inside framework-owned source, docs, scripts, or templates. Generated-looking files are edited through their source of generation unless ownership docs say otherwise. | `scripts/appfw product handoff --json` or `scripts/appfw framework handoff --json`; U2 `harness-check.json` records allowed write roots. |
| Network sandbox | Local product work is offline by default except declared development services. Live provider, SaaS, IdP, MCP, Kafka, release, ProGet, and cloud endpoints require an explicit lane and retained evidence. | Provider/live/release commands retain JSON artifacts; U2 `harness-check.json` records allowed network classes and live-service approval. |

The exact U2 retained artifact is `.appfw/target/appfw/harness-check.json`; it
records the product-agent `filesystem sandbox`, `network sandbox`, allowed
commands, sensitive-capability posture, review checkpoints, and handoff
requirements. Runtime `principal_type`, `tenant`, and `on_behalf_of` controls
belong to the G1/G3 policy and ingress evidence paths, and product-agent
profiles must not imply those runtime identities unless that evidence exists.

## Human Review Checkpoints

Human review is required before an agent:

- promotes a connector capability from unsupported or emulator-limited to
  certified;
- enables MCP, Kafka, or SaaS governed writes in release scope;
- accepts dependency, vulnerability, DAST, SAST, ASVS, provenance, or signing
  risk;
- commits or publishes generated product artifacts that contain customer,
  tenant, PHI, or PoC data;
- changes release gates, policy evaluation, tenant isolation, token storage, or
  delegated-write behavior.

The review artifact should name the actor, scope, command lane, risk decision,
expiry when applicable, and retained evidence path.

## Agent Harness Requirements

The U2 least-privilege product-agent profile must encode this threat model as
machine-readable policy. At minimum it records:

- allowed commands and whether each command is product or framework scoped;
- writable paths and generated-boundary rules;
- network and live-service access policy;
- whether MCP, Kafka, SaaS write-back, or release commands are allowed;
- required review checkpoints;
- required handoff artifact and evidence commands.

The profile must fail closed: absence of a field is treated as "not allowed,"
not as inherited permission.

## Runtime And UI Implications

G1 and G2 use this threat model together. G1 proves the backend can honor a
governed write. G2 provides the user-facing controls: intent preview,
confirm-before-act, action audit, undo/compensation state, denied-policy state,
and persistent activity context. UI controls must not imply a write is safe
until the backend evidence exists for that provider and operation.
