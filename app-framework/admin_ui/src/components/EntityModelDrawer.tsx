import { SlidersHorizontal } from "lucide-react";
import { Drawer } from "@appfw/pds-health-components";
import type { EntityType, MetaGroup } from "../types";
import { fieldDescription } from "../lib/entityModel";
import { MetaChipGroup } from "./MetaChipGroup";

type EntityModelDrawerProps = {
  entity: EntityType;
  metaGroups: MetaGroup[];
  onClose: () => void;
};

export function EntityModelDrawer({ entity, metaGroups, onClose }: EntityModelDrawerProps) {
  return (
    <Drawer
      open
      className="model-drawer"
      title={entity.caption_1}
      description="Entity Model"
      onClose={onClose}
    >
        <div className="model-drawer-body">
          <div className="model-head">
            <SlidersHorizontal size={18} />
            <div>
              <strong>{entity.schema_name}.{entity.pascal_1}</strong>
              <span>{entity.caption_n}</span>
            </div>
          </div>
          <div className="model-summary">
            {metaGroups.map((group) => (
              <MetaChipGroup group={group} key={group.label} />
            ))}
          </div>
          <div className="field-list">
            {entity.props.map((prop) => (
              <div className="field-card" key={prop.id}>
                <div>
                  <strong>{prop.caption || prop.name}</strong>
                  <span>{fieldDescription(prop)}</span>
                </div>
                <em>{prop.data_type}</em>
              </div>
            ))}
          </div>
        </div>
    </Drawer>
  );
}
