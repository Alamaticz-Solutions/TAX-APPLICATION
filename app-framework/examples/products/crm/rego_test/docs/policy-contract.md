# Policy Contract

Access policies are complete Rego rules named `access` under a package matching the backend policy key:

```rego
package scheduling.resource
```

The backend evaluates:

```text
data.scheduling.resource.access
```

Policies must deny by default:

```rego
default access = {"allow": false}
```

When a request is allowed, return `allow: true` plus a `filter` object. Use `{}` when the caller can see all rows for the entity. Use a field filter, such as `{"tenant_id": "180123"}`, when the data layer must scope the query.

Keep Rego comments focused on domain intent, especially security-sensitive choices like tenant bypasses. Avoid comments that restate syntax.
