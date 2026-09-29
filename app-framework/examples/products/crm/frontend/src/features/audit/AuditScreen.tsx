import { Badge, PageHeader, Surface } from "../../components/ui";

// Human-owned (ADR 0009). The audit workflow is a read-only timeline backed by
// the backend audit query rather than a standard entity list, so it is built
// out as a dedicated screen in a later step.
export function AuditScreen() {
  return (
    <>
      <PageHeader title="Audit" subtitle="Read-only audit timeline with request correlation and policy context" />
      <Surface title="Audit timeline" actions={<Badge>Planned</Badge>}>
        <p className="crm-audit-placeholder-copy">
          This screen will render the hash-chained audit events for audited entities, surfacing
          request/correlation IDs and policy context. It consumes the backend audit query, not a
          standard entity list, and is implemented after the generated contract exposes it.
        </p>
      </Surface>
    </>
  );
}
