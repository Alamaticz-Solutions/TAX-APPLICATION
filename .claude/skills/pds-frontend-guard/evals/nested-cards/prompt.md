---
description: Cards around each section and a card around KPI tiles must be flagged (containment, no nested cards).
tags: [review]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Agent]
---

Please review this dashboard change for PDS compliance before we merge it.

```tsx
// tax-doc-routing/frontend/src/features/dashboard/PortfolioSummary.tsx (new)
import { KpiTile, Surface } from '@appfw/pds-health-components/surfaces';

export function PortfolioSummary({ open, decided, overdue }: { open: number; decided: number; overdue: number }) {
  return (
    <div style={{ display: 'grid', gap: 'var(--pds-space-4)' }}>
      <Surface variant="elevated" title="Portfolio at a glance">
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 'var(--pds-space-3)' }}>
          <KpiTile label="Open projects" value={open} tone="neutral" />
          <KpiTile label="Decided this month" value={decided} tone="neutral" />
          <KpiTile label="Overdue steps" value={overdue} tone={overdue > 0 ? 'warning' : 'neutral'} />
        </div>
      </Surface>
      <Surface variant="elevated" title="About this page">
        <p>These figures come from the governance steps recorded for each project.</p>
      </Surface>
    </div>
  );
}
```
