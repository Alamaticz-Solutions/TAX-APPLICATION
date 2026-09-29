import type { MetaGroup } from "../types";

export function MetaChipGroup({ group }: { group: MetaGroup }) {
  return (
    <section className="meta-chip-group">
      <div>
        <strong>{group.label}</strong>
        <span>{group.description}</span>
      </div>
      <div className="chips">
        {group.items.length ? (
          group.items.map((item) => (
            <span key={item.label} title={item.description}>
              {item.label}
            </span>
          ))
        ) : (
          <span title={group.description}>None</span>
        )}
      </div>
    </section>
  );
}
