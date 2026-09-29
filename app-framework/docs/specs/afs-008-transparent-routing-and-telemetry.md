# AFS-008 Transparent Routing And Telemetry

Status: **Prototype contract**
Semantic contract: `appfw_model_route@1`
Scope: deterministic route selection and metadata-only evidence; no dispatch

## Purpose

AFS-008 makes an agent task's governing instructions, capability route,
independent review route, provider mapping, and outcome economics explicit.
It is deliberately smaller than an orchestration system. The router cannot
admit work, spawn an agent, change WIP, mutate an assignment, select Product
priority, approve a fallback, accept an outcome, or merge a change.

The durable contract is provider-neutral. Codex and Claude details live only in
replaceable adapter documents. An unavailable or unsupported mapping fails
visibly; it never silently changes provider, profile, effort, or execution
target.

## Files And Authority

| File | Responsibility | Authority |
| --- | --- | --- |
| `appfw-model-route.v1.schema.json` | Closed route and telemetry record | Semantic validation contract |
| `route-policy.v1.json` | Instruction and capability-profile selection | Deterministic interpretation only |
| `codex-adapter.v1.json` | Codex profile mapping | Replaceable provider configuration |
| `claude-adapter.v1.json` | Claude profile mapping | Replaceable provider configuration |
| `routing-cases.v1.json` | Seven canonical and negative fixtures | Hermetic proof input |
| `appfw-model-route.mjs` | Read-only selector and validator | Derived output only |
| `appfw-model-route.test.mjs` | Semantic, privacy, and CLI proof | Test evidence only |

Tracked Product outcomes, Architecture contracts, repository instructions,
role cards, Git objects, and human decisions remain authoritative. A route
record points to those sources; it does not copy or supersede them.

## Request Contract

`--task` accepts one closed JSON object with discriminator
`appfw_model_route@1` and five sections:

- `subject`: Outcome, Product Increment, Deliverable, Task, Assignment,
  workstream, role card, stage, and explicit parent run;
- `task_shape`: role, exact roots/tools/network boundary, classification,
  change class, ambiguity, consequence, sensitive surfaces, signature
  significance, execution target, and delegation depth/type;
- `routing_context`: separately named implementer and reviewer identities,
  authorized bounds, provider availability, required instruction IDs, and any
  named human decision;
- `privacy`: classification and the mandatory metadata-only retention posture;
- `schema`: the semantic discriminator.

Material child work is valid only at depth one with an explicit parent run and
an independent reviewer. Implementer and reviewer assignment, actor, and run
IDs must all differ. Requested roots, tools, and network posture must remain
inside the assigned authorization.

Provider/model/effort fields, prompt bodies, completions, secrets,
credentials, tenant/user identity, PHI/PII, raw tool output, and absolute home
paths are forbidden from the provider-neutral request.

Machine identities are context-specific authority references, never generic
caller labels: `actor:<uuid>`, `assignment:<uuid>`, `run:<uuid>`,
`task:<uuid>`, `route:<uuid>`, and `evidence:<uuid>`. UUIDs are lowercase,
canonical, and type-prefixed; a valid reference of one kind cannot stand in
for another. Route IDs are deterministically issued from the authoritative
request, policy, and adapter identities.

Business, workstream, role-card, instruction, policy, profile, capability,
tool, adapter, and failure-code identifiers have separate bounded contracts.
Business and assignment-scope values are bound by the separately retained PFC
`assignment_authority@1` envelope. Instruction, policy, profile, capability,
and adapter values must resolve from tracked policy, source, or adapter
registries. The envelope binds the route decision, exact provider, adapter
identity/version/digest, both resolved profiles/models/efforts/capabilities,
source-bound instructions, authorized request and scope, network posture, and
privacy contract. A caller substitution remains invalid after recomputing all
mutable record digests because the separately retained envelope does not
change.

Roots are normalized repository-relative paths. Email addresses,
patient/member identifiers, credential shapes, private-key material,
absolute/drive/UNC/userinfo paths, traversal, and raw payload fields fail
closed. Recursive privacy validation covers every retained string and object
key in requests, records, policy, adapter configuration, assignment authority,
and generated resolution. Provider model IDs use closed provider-specific
grammars and forbid whitespace. A bounded ASCII-whitespace-normalized copy is
structurally decoded to detect compact DER PKCS#8, PKCS#1, SEC1, Ed25519, and
OpenSSH private-key material; lexical token and envelope checks remain defense
in depth. Rejected values are never copied into error output.

## Route Record

Successful selection emits a closed `appfw_model_route@1` record containing:

1. immutable subject and task shape;
2. ordered instruction references with source commit and blob identity;
3. provider-neutral implementer and reviewer profiles;
4. adapter ID, version, digest, and provider-specific resolution;
5. a planned or actual metadata-only run record;
6. privacy posture;
7. the digest of the separately retained Program Flow Controller
   `assignment_authority@1` envelope; and
8. a canonical record digest.

Canonical digests use RFC 8785-compatible JSON key ordering and ECMAScript JSON
number serialization. Arrays retain declared order. Instruction references
contain paths, headings, commits, blobs, and rationale codes, never copied
instruction prose.

The authority-envelope and record digests are integrity evidence, not
self-authority. Validation requires the complete PFC-held envelope as a
separate input. It validates that envelope against tracked policy, instruction
sources, adapters, and provider-specific grammars, then compares every
corresponding immutable route field exactly. It never chooses a provider or
derives an authority premise from the mutable record being validated.

## Commands

```text
node scripts/agent-routing/appfw-model-route.mjs --task <json> --provider <codex|claude> --json
node scripts/agent-routing/appfw-model-route.mjs --validate-record <json> --assignment-authority <pfc-held-json> --json
node scripts/agent-routing/appfw-model-route.mjs --check --json
```

Standard output is exactly one JSON document. Exit status is zero only when
`ok:true`. Task routing returns `route` and `assignment_authority` as separate
objects; PFC retains the latter unchanged and record validation receives it by
file. The command performs no network access, source mutation, dispatch, or
agent spawn. Errors use stable codes and never echo task content.

The focused proof is:

```text
git diff --check
node --check scripts/agent-routing/appfw-model-route.mjs
node --test scripts/agent-routing/appfw-model-route.test.mjs
node scripts/agent-routing/appfw-model-route.mjs --check --json
```

## Provider-Neutral Profiles

Implementation profiles:

- `bounded_read@1`
- `routine_delivery@1`
- `complex_delivery@1`
- `consequential_decision@1`

Independent-review profiles:

- `focused_independent_review@1`
- `comprehensive_independent_review@1`
- `adversarial_independent_review@1`

Change class, profile, role, stage, provider, and execution target are separate
dimensions. Consequence, sensitivity, or signature significance may raise a
route. The router evaluates every matching specialized rule and combines the
strongest implementer profile, strongest reviewer profile, rationale set, and
human-decision obligation. It also unions the reviewer capabilities required
by every match. The selected profile and any fallback must provide that entire
capability union; profile rank alone cannot discard strategic challenge or
sensitive-boundary review. The always-match routine rule is used only when no
specialized rule matches. No rule or caller flag may lower a declared class or
reviewer requirement; writable and material work always requires a named
independent reviewer. Stage review floors read the canonical `subject.stage`:
integration and release candidates cannot be treated as prototypes.

## Fail-Visible Rules

Routing fails nonzero when any of these conditions is observed:

- required instruction source or heading is missing;
- a provider detail enters provider-neutral task semantics;
- a material child inherits implicitly or exceeds depth one;
- implementer and reviewer identity is not independent;
- requested roots, tools, or network posture exceed assignment authority;
- human judgment is required but no human decision is named;
- provider, profile, or execution target is unsupported/unavailable;
- a fallback was not predeclared and explicitly approved;
- the activated assignment or route was mutated;
- the PFC-held assignment-authority envelope is absent, invalid, or differs;
- provider, route, adapter, profile, model, effort, capability, scope, source,
  network, or privacy fields differ from external authority;
- a primitive type, timestamp, lifecycle transition, or evidence relationship
  violates the closed schema;
- running or dispatched failed telemetry differs from the approved
  provider/model/effort route;
- a cost source is not an authority-issued canonical
  `provider_measurement:pm_<uuid>` for actual cost or
  `declared_rate:dr_<uuid>` for estimated cost;
- privacy fields retain prompts, secrets, identity, or unknown telemetry; or
- a value claim lacks comparable baseline and Accepted-outcome evidence.

## Telemetry And Observability

Allowlisted telemetry covers route/run/review IDs, class/profile/stage/status,
adapter identity, actual provider/model/effort, context/token counts, timing,
retries, material corrections, rework, review findings, escaped defects,
measured cost, and Implemented/Accepted evidence references.

Actual-route fields are lifecycle exact. `not_started` records no actual route;
`running` and `completed` record the complete approved route; `failed` records
either no route for a pre-dispatch failure or the complete approved route.
An all-null route carries no timestamps, usage, timing, cost, retry/rework,
defect, or implementation/acceptance/value evidence. A failed pre-dispatch
record may retain only non-sensitive failure findings and its telemetry
completeness classification. Any execution evidence requires the complete
approved route. Cost-source references use authority-issued canonical UUID
forms only; arbitrary encoded or hexadecimal payloads are rejected without
being echoed.
Actual provider measurements must occur inside the execution interval.
Declared-rate estimates have distinct semantics and may predate execution but
never postdate completion. First-correct-action and review-wait durations may
not exceed elapsed time, and elapsed execution may not exceed its wall-clock
interval.
Accepted evidence and positive economic claims are completed-run outcomes
only. Failed work may retain partial Implemented evidence, but it earns no
Accepted or economic credit. `telemetry_complete:true` on a failed run means
only that failure telemetry is complete.

OTel names are:

- `appfw.agent.route.select`
- `appfw.agent.run`
- `appfw.agent.review`

Low-cardinality class/profile/stage/status values may be metric attributes.
Task/run/evidence IDs belong only on traces or logs. Prompt and source content
are never attributes. Missing telemetry remains visible as
`telemetry_complete:false` and cannot support an economic claim.

## Five-Task Anti-Fad Canary

The prototype does not claim economic value. Product acceptance requires at
least five representative tasks with comparable baselines and independent
reviews:

| Representative task | Required comparison |
| --- | --- |
| Read-heavy source discovery | Orientation/startup time, context size, corrections, findings |
| Routine program coordination | Elapsed time, retries, review wait, rework |
| Narrow Class A/B implementation | Cost, elapsed time, corrections, findings, Accepted result |
| Signature experience work | Human corrections, review findings, rework, Accepted result |
| Sensitive boundary or independent review | Defects, findings, elapsed time, Accepted result |

Each pair retains task class, route, context size, startup-to-first-correct
action, elapsed time, cost quality/source, retries, material human corrections,
review findings, rework, escaped defects, and terminal Implemented/Accepted
evidence.

The canary advances only with at least one of:

- 15% lower median startup-to-first-correct action, elapsed time, or cost per
  Accepted outcome; or
- 20% fewer combined retries, rework events, and material review findings per
  Accepted outcome.

It also requires no critical escaped defect, no lower Accepted-outcome rate,
no increase in material human corrections, and no more than 5% regression in
another primary time/cost measure. An inconclusive result narrows to bounded
evidence collection. A failed value threshold stops route optimization and
retains only useful deterministic disclosure and telemetry.

## Non-Claims

This prototype does not prove live provider availability, dispatch, hosted
execution, Product acceptance, release readiness, adaptive or learned routing,
economic savings, or P2 readiness. Provider IDs in the adapters are bounded
configuration snapshots, not durable product semantics or availability claims.
