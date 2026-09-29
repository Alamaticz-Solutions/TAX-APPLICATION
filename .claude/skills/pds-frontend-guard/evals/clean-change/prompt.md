---
description: A compliant change must not be failed with invented findings.
tags: [review]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Agent]
---

Quick PDS compliance check on this change, please — is it OK to merge? (`RouterButtonLink` is our documented thin wrapper that renders PDS `ButtonLink` with client-side routing; loading and error states are handled by the parent screen's `AsyncSection`.)

```tsx
// tax-doc-routing/frontend/src/features/risks/RiskList.tsx (changed)
import { List, ListItem } from '@appfw/pds-health-components/experience';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import { EmptyState } from '@appfw/pds-health-components/surfaces';
import { RouterButtonLink } from '../../components/ui';

type Risk = { id: string; title: string; level: 'High' | 'Medium' | 'Low' };

const LEVEL_TONE = { High: 'danger', Medium: 'warning', Low: 'neutral' } as const;

export function RiskList({ risks, query, onClearSearch, onAddRisk }: {
  risks: Risk[];
  query: string;
  onClearSearch: () => void;
  onAddRisk: () => void;
}) {
  if (risks.length === 0) {
    return (
      <EmptyState
        title="No risks recorded yet"
        detail="Record the first risk so the review committees can see it."
        action={<Button variant="primary" onClick={onAddRisk}>Add risk</Button>}
      />
    );
  }
  const rows = risks.filter((r) => r.title.toLowerCase().includes(query.toLowerCase()));
  if (rows.length === 0) {
    return (
      <EmptyState
        title="No risks match this search"
        detail="Try another word, or clear the search to see every risk on this project."
        action={<Button variant="outlined" onClick={onClearSearch}>Clear search</Button>}
      />
    );
  }
  return (
    <List aria-label="Risks">
      {rows.map((r) => (
        <ListItem
          key={r.id}
          headline={r.title}
          trailing={<Badge tone={LEVEL_TONE[r.level]}>{r.level}</Badge>}
          action={
            <RouterButtonLink variant="quiet" size="sm" to={`/risks/${r.id}`} aria-label={`Open risk: ${r.title}`}>
              Open
            </RouterButtonLink>
          }
        />
      ))}
    </List>
  );
}
```
