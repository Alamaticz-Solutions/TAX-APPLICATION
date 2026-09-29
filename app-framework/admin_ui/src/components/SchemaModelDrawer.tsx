import { Activity, Database, GitBranch, Layers3, ShieldCheck } from "lucide-react";
import { Drawer } from "@appfw/pds-health-components";
import type { EntityType, ProviderCapabilityDetail, SchemaHealth, SchemaHealthItem, SchemaModel } from "../types";

type SchemaModelDrawerProps = {
  schema: SchemaModel;
  entities: EntityType[];
  onClose: () => void;
};

export function SchemaModelDrawer({ schema, entities, onClose }: SchemaModelDrawerProps) {
  const latestMigration = schema.latest_migration;
  const health = schema.health ?? fallbackHealth(schema, entities);
  const topCapabilities = health.provider_capabilities.slice(0, 6);
  const remainingCapabilityCount = Math.max(0, health.provider_capabilities.length - topCapabilities.length);

  return (
    <Drawer
      open
      className="model-drawer"
      title={schema.name}
      description="Schema Model"
      onClose={onClose}
    >
        <div className="model-drawer-body">
          <div className="model-head">
            <Database size={18} />
            <div>
              <strong>{schema.description || schema.name}</strong>
              <span>{schema.data_source_name}</span>
            </div>
          </div>

          <div className="schema-detail-grid">
            <DetailTile label="Data source" value={schema.data_source_name} />
            <DetailTile label="Type" value={formatValue(schema.data_source_type)} />
            <DetailTile label="Entity types" value={health.entity_count.toLocaleString()} />
            <DetailTile label="Table entities" value={health.table_entity_count.toLocaleString()} />
            <DetailTile label="Schema id" value={schema.id} />
          </div>

          <section className="schema-section">
            <div className="schema-section-head">
              <Activity size={16} />
              <strong>Schema Health</strong>
            </div>
            <div className="schema-health-grid">
              <HealthTile title="Migration status" item={health.migration_status} />
              <HealthTile title="Pending drift" item={health.pending_drift} />
              <HealthTile title="Connectivity" item={health.connectivity} />
              <div className="schema-health-tile">
                <span>Inventory</span>
                <strong>{health.entity_count.toLocaleString()} entities</strong>
                <small>
                  {health.table_entity_count.toLocaleString()} table-backed, {health.migration_count.toLocaleString()} migrations
                </small>
              </div>
            </div>
          </section>

          <section className="schema-section">
            <div className="schema-section-head">
              <GitBranch size={16} />
              <strong>Latest Migration</strong>
            </div>
            {latestMigration ? (
              <div className="migration-card">
                <div>
                  <strong>{latestMigration.name}</strong>
                  <span>{latestMigration.description || "No description"}</span>
                </div>
                <div className="migration-meta">
                  <span>{latestMigration.id}</span>
                  <span>{latestMigration.phase}</span>
                  <span>{latestMigration.dialect}</span>
                </div>
                <dl>
                  <div>
                    <dt>Scope</dt>
                    <dd>{latestMigration.schema || "All schemas"}</dd>
                  </div>
                  <div>
                    <dt>Data source</dt>
                    <dd>{latestMigration.data_source}</dd>
                  </div>
                  <div>
                    <dt>Path</dt>
                    <dd>{latestMigration.path}</dd>
                  </div>
                </dl>
              </div>
            ) : (
              <div className="empty-panel">No migration metadata found for this schema and data source.</div>
            )}
          </section>

          <section className="schema-section">
            <div className="schema-section-head">
              <ShieldCheck size={16} />
              <strong>Provider Capabilities</strong>
            </div>
            {topCapabilities.length > 0 ? (
              <div className="provider-capability-list">
                {topCapabilities.map((capability) => (
                  <CapabilityRow capability={capability} key={capability.area_key} />
                ))}
                {remainingCapabilityCount > 0 && (
                  <div className="capability-more">
                    {remainingCapabilityCount.toLocaleString()} more contract areas available from the provider profile
                  </div>
                )}
              </div>
            ) : (
              <div className="empty-panel">No provider capability profile is available for this data source type.</div>
            )}
          </section>

          <section className="schema-section">
            <div className="schema-section-head">
              <Layers3 size={16} />
              <strong>Entity Types</strong>
            </div>
            <div className="entity-mini-list">
              {entities.map((entity) => (
                <div className="field-card" key={entity.id}>
                  <div>
                    <strong>{entity.caption_1}</strong>
                    <span>{entity.schema_name}.{entity.pascal_1}</span>
                  </div>
                  <em>{entity.props.length} fields</em>
                </div>
              ))}
            </div>
          </section>
        </div>
    </Drawer>
  );
}

function HealthTile({ title, item }: { title: string; item: SchemaHealthItem }) {
  return (
    <div className="schema-health-tile">
      <span>{title}</span>
      <strong>
        <StatusDot status={item.status} />
        {item.label}
      </strong>
      <small>{item.message}</small>
    </div>
  );
}

function CapabilityRow({ capability }: { capability: ProviderCapabilityDetail }) {
  return (
    <div className="capability-row">
      <div>
        <strong>{titleCase(capability.area_label)}</strong>
        <span>{capability.reason || evidenceSummary(capability)}</span>
      </div>
      <em className={`status-pill ${statusTone(capability.status)}`}>{formatStatus(capability.status)}</em>
    </div>
  );
}

function StatusDot({ status }: { status: string }) {
  return <i className={`status-dot ${statusTone(status)}`} />;
}

function DetailTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="schema-detail-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "Not configured";
  return String(value);
}

function formatStatus(value: string) {
  return value.replace(/-/g, " ");
}

function statusTone(status: string) {
  if (status === "ok" || status === "ready" || status === "live-certified") return "ok";
  if (status === "warning" || status === "partial" || status === "implemented" || status === "compiler-contracted") return "warning";
  if (status === "error" || status === "unsupported") return "error";
  return "unknown";
}

function titleCase(value: string) {
  return value.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function evidenceSummary(capability: ProviderCapabilityDetail) {
  if (!capability.evidence.length) return "No certification evidence listed.";
  const live = capability.evidence.filter((item) => item.kind === "live-contract").length;
  const compiler = capability.evidence.filter((item) => item.kind === "compiler-contract").length;
  return [live ? `${live} live` : "", compiler ? `${compiler} compiler` : ""].filter(Boolean).join(", ");
}

function fallbackHealth(schema: SchemaModel, entities: EntityType[]): SchemaHealth {
  return {
    migration_status: {
      status: schema.latest_migration ? "ok" : "warning",
      label: schema.latest_migration ? "Migration package available" : "No migration metadata",
      message: schema.latest_migration
        ? `Latest: ${schema.latest_migration.name} (${schema.latest_migration.id}).`
        : "No migration entry was found for this schema and data source."
    },
    pending_drift: {
      status: "unknown",
      label: "Not evaluated by server",
      message: "Run `scripts/appfw generate --check --json` to detect pending generated drift."
    },
    connectivity: {
      status: "unknown",
      label: "Unknown",
      message: "This server did not include schema readiness metadata."
    },
    entity_count: entities.length,
    table_entity_count: entities.filter((entity) => entity.is_table).length,
    migration_count: schema.latest_migration ? 1 : 0,
    provider_capabilities: []
  };
}
