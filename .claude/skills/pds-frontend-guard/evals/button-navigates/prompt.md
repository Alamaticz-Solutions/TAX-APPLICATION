---
description: A Button whose handler only navigates must be flagged (use ButtonLink).
tags: [review]
runs: 1
max_turns: 12
allowed_tools: [Read, Glob, Grep, Agent]
---

Before I merge, can you check this frontend change against our PDS design-system rules? It's React + TypeScript on `@appfw/pds-health-components`.

```tsx
// tax-doc-routing/frontend/src/features/projects/ProjectCard.tsx (new)
import { useNavigate } from 'react-router';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import { PageSection } from '../../components/ui';

export function ProjectSummary({ id, name, stage }: { id: string; name: string; stage: string }) {
  const navigate = useNavigate();
  return (
    <PageSection title={name} actions={<Badge tone="neutral">{stage}</Badge>}>
      <Button variant="secondary" onClick={() => navigate(`/projects/${id}`)}>
        Open project
      </Button>
    </PageSection>
  );
}
```
