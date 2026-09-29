# Architecture

App Framework is a config-driven backend generation system. The durable
architecture is the boundary between config, generator, generated code, and
human-owned extension points.

## System Flow

```text
.appfw/model
        |
        v
app_gen validation and templates
        |
        v
generated backend, database package, API tests
        |
        v
human-owned handler impls and services
```

## Main Crates

| Path | Role |
| --- | --- |
| `app_gen` | Validates config and generates code/artifacts. |
| `backend` | Axum + async-graphql API server. |
| `database` | Executes generated database package scripts. |
| `api_tests` | Runs generated API scenarios. |
| `rego_test` | Product policy fixtures and assertions that run through the framework-owned verifier harness in `appfw-test`. |

## Runtime Shape

```text
backend/src/main.rs
backend/src/routes/
backend/src/handlers/
backend/src/schemas/
backend/src/data/
backend/src/config/
```

Generated route modules expose schema-specific GraphQL endpoints. Generated
handler modules wire GraphQL operations. Human-owned handler impl files contain
custom behavior and are created only when missing.

## Data Access

`DataAccess` is the application-facing data boundary. Provider clients live
under:

```text
backend/src/data/clients/
```

Provider implementations should preserve the same filter, sort, pagination,
projection, and access-control semantics unless a documented provider limitation
requires otherwise.

## Runtime Ingress And Operation Invocation

Every external entrypoint should adapt into the same runtime operation path.
GraphQL/HTTP, MCP, admin diagnostics, and Kafka worker ingress must not bypass
shared auth, tenant, policy, query-budget, audit, metrics, tracing, redaction,
or provider certification behavior.

Runtime ingress is now independently loaded through the runtime host module
boundary:

```text
APPFW_RUNTIME_MODE=http|mcp|consumers|all
APPFW_MODULES=http,mcp,kafka
```

The code contract starts in `appfw_runtime::host` and is re-exported through
`appfw_runtime::ingress`:

```text
RuntimeMode
RuntimeIngressKind
RuntimeIngressDescriptor
RuntimeHostPlan
```

These types make ingress services explicit and independently loadable. HTTP,
MCP, and Kafka can each be compiled, selected, and hosted without forcing the
other ingress modules into the process. A server binary compiled with HTTP
defaults to the HTTP listener only; worker modules such as Kafka must be
selected explicitly unless the artifact is worker-only. This prevents optional
feature flags from accidentally starting broker consumers in an API
deployment.

Target shape:

```text
RuntimeIngress
        |
        v
runtime actor + tenant + request/correlation context
        |
        v
RuntimeOperationDispatcher
        |
        v
generated operation dispatcher or product extension handler
        |
        v
DataAccess -> QueryIR -> policy/access filter -> provider client
        |
        v
audit + metrics + tracing + normalized result/error envelope
```

Ingress adapters own transport-specific work:

- HTTP/GraphQL owns route mounting, JWT/header extraction, request limits,
  GraphiQL posture, and response headers.
- MCP owns JSON-RPC envelopes, tool/resource/prompt catalog shape, MCP
  front-door gates, result bounds, and release posture.
- Kafka owns runtime worker ingress config, service-principal actor mapping,
  tenant derivation, broker-auth config validation, operation bindings,
  idempotency/retry/DLQ/readiness policy validation, and dispatch from
  messages into generated runtime operations. Concrete broker client loops
  remain behind product/platform binding until release certification chooses a
  broker implementation.

`RuntimeOperationDispatcher` in `appfw_runtime::operation` is the shared
operation seam. It owns operation identity, generated contract lookup, auth
context, tenant context, validation, policy-aware dispatch, audit, metrics,
tracing, redaction, and normalized result/error envelopes.

The operation code contract is:

```text
RuntimeOperationRequest
RuntimeOperation
RuntimeOperationCatalog
RuntimeOperationDispatcher
```

Use the host, ingress, and operation contracts when adding or extracting an
ingress path. HTTP/GraphQL, MCP, admin, and Kafka work should share these
runtime semantics rather than creating transport-local auth, tenant, audit, or
provider invocation shapes.

Guardrail: a trusted ingress only admits a request or message into the runtime.
It does not grant data access. Policy, tenant isolation, QueryIR budgets,
provider contracts, and audit still apply.

## Config Contract

The config contract is code-owned:

```text
app_gen/src/config_contract.rs
app_gen/src/validation.rs
```

Generated outputs:

```text
.appfw/model/_specs/CONFIG_CONTRACT.md
.appfw/target/appfw/config_contract.json
```

Agents should read the JSON contract when authoring or repairing config.

## Generated Ownership

Generation emits:

```text
.appfw/target/appfw/artifacts.json
```

Ownership values:

- `generated`: controlled by config/templates and overwrite-safe.
- `human_owned`: created when missing, then preserved.

Use:

```bash
scripts/appfw manifest --json
```

If the manifest is missing, use `docs/start/generated-ownership.md` instead of running
generation only to discover ownership.

## Architectural Rules

- Config changes should go through validation.
- Generated behavior should be changed in templates or generator code.
- App-specific behavior should live in human-owned handler impls or services.
- Provider-specific behavior should stay behind provider clients.
- Access-control behavior must be preserved across custom data access paths.
- Observability should use `tracing`.

## Enterprise Extension Pattern

For complex applications:

```text
generated route -> generated handler -> human-owned impl -> service -> DataAccess
```

This gives teams stable generated APIs while keeping domain behavior readable,
testable, and safe from regeneration.
