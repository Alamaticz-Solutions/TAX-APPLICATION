---
description: A hand-built empty message with a raw colour must be flagged (use EmptyState with one action, tokens only).
tags: [review]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Agent]
---

Can you review this list screen against our PDS frontend rules? I want to be sure it's compliant before I push.

```tsx
// tax-doc-routing/frontend/src/features/risks/RiskList.tsx (new)
import { List, ListItem } from '@appfw/pds-health-components/experience';
import { Badge } from '@appfw/pds-health-components/primitives';

type Risk = { id: string; title: string; level: 'High' | 'Medium' | 'Low' };

export function RiskList({ risks, query }: { risks: Risk[]; query: string }) {
  const rows = risks.filter((r) => r.title.toLowerCase().includes(query.toLowerCase()));
  if (rows.length === 0) {
    return <p style={{ color: '#888', textAlign: 'center', padding: 24 }}>Nothing here.</p>;
  }
  return (
    <List aria-label="Risks">
      {rows.map((r) => (
        <ListItem key={r.id} headline={r.title} trailing={<Badge tone="neutral">{r.level}</Badge>} />
      ))}
    </List>
  );
}
```
