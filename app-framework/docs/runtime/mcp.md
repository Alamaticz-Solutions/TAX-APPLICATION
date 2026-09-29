# Agentic MCP Server

App Framework exposes an optional Model Context Protocol endpoint for coding
agents and enterprise developer tools that need a model-driven way to inspect
and query generated applications.

The MCP server is a transport adapter, not a second backend. Tool execution
uses the same runtime path as generated GraphQL resolvers:

```text
MCP JSON-RPC request
  -> auth and request context
  -> model-driven MCP adapter
  -> generated public operation dispatcher
  -> handler implementation
  -> DataAccess
  -> QueryIR, policy, tenant isolation, audit, metrics
  -> provider client
```

The MCP operation catalog is generated from the same entity model as GraphQL.
It exposes public handler-level operation names such as `find_account`,
`query_accounts`, `account_health`, and `pipeline_forecast`. It does not expose
internal `_impl` names, and app-defined methods appear beside standard methods
only when their `custom_methods[]` entry sets `mcp_enabled: true`.

## Enable Locally

MCP is disabled by default.

```bash
APP_MCP_ENABLED=true scripts/appfw serve
```

Current release posture excludes MCP. Keep `APP_MCP_ENABLED=false` in release
pipelines unless a dedicated certified MCP release lane includes signed
capability manifests, per-tool authorization, token audience/resource checks,
mandatory audit evidence, and HITL evidence.

The endpoint is:

```text
POST /mcp
```

For local development with `ENV_NAME=local`, the backend accepts the same local
auth behavior used by GraphQL. Outside local development, MCP requires normal
JWT/OIDC authentication. MCP also has its own front-door role/scope gate.
By default, the authenticated user must have the `admin` role:

```bash
APP_MCP_REQUIRED_ROLES=admin
APP_MCP_REQUIRED_SCOPES=
```

Deployments that prefer OAuth scopes can set, for example:

```bash
APP_MCP_REQUIRED_ROLES=
APP_MCP_REQUIRED_SCOPES=appfw:mcp.read
```

When a browser-based MCP client sends an `Origin` header, set allowed origins:

```bash
APP_MCP_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000
```

## Security Posture

- `APP_MCP_ENABLED=false` by default.
- All MCP requests are authenticated.
- MCP access requires an allowed role or OAuth scope via
  `APP_MCP_REQUIRED_ROLES` or `APP_MCP_REQUIRED_SCOPES`.
- Privileged MCP diagnostics, such as disabled-operation inspection and
  `appfw_explain_access`, require `APP_MCP_PRIVILEGED_TOOL_ROLES` or
  `APP_MCP_PRIVILEGED_TOOL_SCOPES`.
- Browser origins are checked for HTTP clients that send `Origin`.
- Tools use model metadata from `AppConfig`; they do not guess provider shapes.
- Handler operation tools are generated from `app_gen` beside the dispatcher.
  Generic data-call wrappers are not exposed. Operation tools execute through
  the generated handler dispatcher, then
  `DataAccess`, `QueryIR`, policy, tenant isolation, pagination limits, provider
  execution, redaction, entity audit for audited entities, tracing, and provider
  metrics.
- Tool results are bounded by `APP_MCP_MAX_RESULT_BYTES`, default `262144`.
- Resource results are bounded by `APP_MCP_MAX_RESOURCE_BYTES`, default
  `262144`.
- JSON-RPC batches are bounded by `APP_MCP_MAX_BATCH_ITEMS`, default `20`.
- Mutation tools are intentionally not available yet; startup rejects
  `APP_MCP_MUTATIONS_ENABLED=true` until write-safety certification exists.

## Tools

`tools/list` returns framework support tools plus generated handler operation
tools. Handler operation tools use the actual public operation names generated
from the entity model, such as `find_account`, `query_accounts`,
`aggregate_accounts`, and app-defined names such as `account_health`.

| Tool | Purpose |
| --- | --- |
| `appfw_list_schemas` | List configured schemas and entity counts. |
| `appfw_list_operations` | List public handler-level operations reflected into MCP. |
| `appfw_describe_operation` | Describe one operation's schema, entity, return type, args, selection behavior, and MCP status. |
| `appfw_describe_entity` | Describe fields, relationships, facets, primary key, and default fields. |
| `appfw_explain_access` | Return the current user's policy decision and access filter. |
| Generated handler tools | Public generated/app-defined operations, for example `query_accounts` or `account_health`. |

Prefer `appfw_list_operations`, `appfw_describe_operation`, and
`tools/list` when building agentic clients. Call the reflected handler tool
directly after discovery.

App-defined methods require `mcp_enabled: true` in the entity model before they
appear in MCP discovery. Disabled operations are hidden by default. Pass
`include_disabled: true` to `appfw_list_operations` or
`appfw_describe_operation` when you need to inspect MCP-enabled mutation names
and why they are not currently callable through MCP. Disabled-operation
inspection is privileged.

Example app-defined method config:

```yaml
custom_methods:
- name: account_health
  kind: Query
  mcp_enabled: true
  args:
  - name: account_id
    arg_type: String
  return_type: serde_json::Value
```

## Resources

Current model resources:

```text
appfw://schemas
appfw://schemas/{schema}
appfw://schemas/{schema}/entities
appfw://schemas/{schema}/entities/{entity}
```

These resources intentionally omit provider credentials and environment
secrets.

## Example Requests

Initialize:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "clientInfo": {
      "name": "local-agent",
      "version": "dev"
    }
  }
}
```

List tools:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

Describe `crm.Account`:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "appfw_describe_entity",
    "arguments": {
      "schema_name": "crm",
      "type_name": "Account"
    }
  }
}
```

List public operations for `crm.Account`:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "appfw_list_operations",
    "arguments": {
      "schema_name": "crm",
      "type_name": "Account"
    }
  }
}
```

Describe an app-defined operation:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "appfw_describe_operation",
    "arguments": {
      "schema_name": "crm",
      "type_name": "Account",
      "name": "account_health"
    }
  }
}
```

Call a standard public operation:

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "query_accounts",
    "arguments": {
      "fields": ["id", "name", "website", "phone"],
      "filter": {
        "name": {
          "_contains": "Test"
        }
      },
      "sort": {
        "name": "asc"
      },
      "skip": 0,
      "limit": 10
    }
  }
}
```

Call an app-defined operation:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "account_health",
    "arguments": {
      "account_id": "<account-id>"
    }
  }
}
```

Inspect disabled mutations:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "appfw_list_operations",
    "arguments": {
      "schema_name": "crm",
      "type_name": "Account",
      "include_disabled": true
    }
  }
}
```

Explain access:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "appfw_explain_access",
    "arguments": {
      "schema_name": "crm",
      "type_name": "Account",
      "action": "read"
    }
  }
}
```

Read the entity resource:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "resources/read",
  "params": {
    "uri": "appfw://schemas/crm/entities/Account"
  }
}
```

## Next Certification Work

Before enabling writes through MCP, add certification for:

- create/update/delete denial and audit behavior.
- tenant-isolation proof across all providers.
- redaction of tool results, diagnostics, and error payloads.
- request/result size limits under load.
- provider consistency for JSON-native DataAccess mutation paths.
