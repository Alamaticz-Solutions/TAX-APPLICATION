# Runtime Modularity

This concern owns the durable runtime modularity direction for App Framework.

HTTP/GraphQL, MCP, Kafka, admin diagnostics, and future ingress services must be
independently loadable while converging on one governed runtime operation path.

## Durable Rules

- Transport authentication gets a request or message into the runtime. It does
  not bypass policy, tenant isolation, audit, QueryIR budgets, provider
  certification, metrics, tracing, or redaction.
- Runtime operation invocation flows through shared actor, tenant,
  correlation/request context, generated contract validation, dispatcher,
  DataAccess, provider, audit, metrics, tracing, and normalized result/error
  semantics.
- Runtime modules should compile and load independently. Worker-only artifacts
  must not implicitly start HTTP/admin routes, and HTTP artifacts must not
  implicitly start broker consumers.
- Product handlers and services should depend on stable runtime/product API
  contracts, not provider clients or transport-local request shapes.
- Optional ingress surfaces such as MCP and Kafka need explicit release posture
  and certification before production inclusion.

## Current Reference Docs

- [Architecture Overview](../overview.md)
- [MCP Runtime](../../runtime/mcp.md)
- [Provider Certification](../../runtime/provider-certification.md)
- [Maintainability Command Center](maintainability.md)

Historical extraction coordination and runtime inventory ledgers live in
`docs/archive/` and should not be used as current operating guidance.
