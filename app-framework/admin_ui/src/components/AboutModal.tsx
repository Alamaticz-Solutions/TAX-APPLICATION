import { Dialog } from "@appfw/pds-health-components";
import type { AdminModel, EntityType } from "../types";
import { PdsLogo } from "./Brand";
import { AboutStat } from "./Stats";

type AboutModalProps = {
  model: AdminModel | null;
  entity: EntityType | null;
  version: string;
  onClose: () => void;
};

export function AboutModal({ model, entity, version, onClose }: AboutModalProps) {
  return (
    <Dialog
      open
      className="admin-about-dialog"
      title="PDS Health Admin"
      description="Runtime model-driven operations console."
      size="sm"
      onClose={onClose}
    >
        <div className="admin-about-brand">
          <PdsLogo compact />
          <div>
            <p className="eyebrow">About</p>
            <strong>PDS Health Admin</strong>
          </div>
        </div>

        <div className="about-grid">
          <AboutStat label="Admin UI" value={`v${version}`} />
          <AboutStat label="Backend" value={`v${model?.backend_version ?? "-"}`} />
          <AboutStat label="Schemas" value={String(model?.schemas.length ?? "-")} />
          <AboutStat label="Entity Types" value={String(model?.entity_types.length ?? "-")} />
        </div>

        <dl className="about-details">
          <div>
            <dt>Mode</dt>
            <dd>Runtime model-driven SPA</dd>
          </div>
          <div>
            <dt>Model Source</dt>
            <dd>/admin/model</dd>
          </div>
          <div>
            <dt>Current Entity</dt>
            <dd>{entity ? `${entity.schema_name}.${entity.pascal_1}` : "-"}</dd>
          </div>
        </dl>
    </Dialog>
  );
}
