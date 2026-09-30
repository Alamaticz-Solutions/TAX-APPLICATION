import type { ReactNode } from 'react';

export type DefinitionItem = { label: string; value: ReactNode };

/** Label/value pairs for read-only summaries (client info, record facts). */
export function DefinitionList({ items, columns = 1 }: { items: DefinitionItem[]; columns?: 1 | 2 | 3 }) {
  return (
    <dl className="tax-definition-list" data-columns={columns}>
      {items.map(({ label, value }) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value === null || value === undefined || value === '' ? <span className="tax-subtle">—</span> : value}</dd>
        </div>
      ))}
    </dl>
  );
}
