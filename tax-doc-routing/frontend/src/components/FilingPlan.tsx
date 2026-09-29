import { Lock } from 'lucide-react';
import { Badge } from '@appfw/pds-health-components/primitives';
import type { Client, TaxDocument } from '../features/shared/types';
import { buildFileName } from '../features/shared/utils/filenameRules';
import { destinationsFor } from '../features/shared/utils/routingRules';

/**
 * Shows exactly where each document will be filed and which copies are encrypted. Driven by
 * config/documentTypes and utils/routingRules, so it always matches the routing rules.
 */
export function FilingPlan({ client, documents }: { client: Client; documents: TaxDocument[] }) {
  return (
    <ul className="tax-plan-list">
      {documents.map((d) => (
        <li key={d.id}>
          <div>
            <strong>
              {d.year} {d.type}
            </strong>{' '}
            <span className="tax-muted">{d.entity ? `· ${d.entity.name}` : '· Personal'}</span>
          </div>
          <div className="tax-subtle tax-mono">{buildFileName(client, d)}</div>
          <div className="tax-inline tax-wrap">
            {destinationsFor(d).map((dest) => (
              <Badge key={dest.key} tone="neutral">
                {dest.encrypted ? <Lock size={12} aria-hidden="true" /> : null}
                {dest.label}: {dest.folder}
                {dest.encrypted ? ' (encrypted)' : ''}
              </Badge>
            ))}
          </div>
        </li>
      ))}
    </ul>
  );
}
