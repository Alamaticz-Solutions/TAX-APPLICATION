import { useMemo, useState } from 'react';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { TextField } from '@appfw/pds-health-components/forms';
import { Surface } from '@appfw/pds-health-components/surfaces';
import { CasesGrid } from '../../components/CasesGrid';
import { SingleSelectField } from '../../components/ui';
import { ALL_STATUSES } from '../shared/config/statuses';
import { matchesCaseSearch } from '../shared/utils/caseNumber';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import type { RecordStatus } from '../shared/types';

const STATUS_OPTIONS = ALL_STATUSES.map((s) => ({ value: s, label: s }));

/** "My Cases" = records assigned to the signed-in user, regardless of role. */
export function MyCasesScreen() {
  const { cases, user } = useTaxRouting();
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState<RecordStatus | ''>('');

  const mine = useMemo(() => {
    return cases
      .filter((c) => c.assignedTo === user.id)
      .filter((c) => !status || c.status === status)
      .filter((c) => matchesCaseSearch(c, query))
      .sort((a, b) => b.updated.localeCompare(a.updated));
  }, [cases, user.id, query, status]);

  return (
    <>
      <PageHeader title="My Cases" subtitle="Records assigned to you." />
      <Surface density="compact">
        <div className="tax-filters">
          <TextField label="Search by case number or client name" value={query} onChange={(e) => setQuery(e.target.value)} />
          <SingleSelectField label="Status" value={status} onValueChange={(v) => setStatus(v as RecordStatus | '')} options={STATUS_OPTIONS} placeholder="All statuses" />
        </div>
        <CasesGrid
          cases={mine}
          ariaLabel="My cases"
          pageSize={8}
          options={{ task: true, emails: true }}
          emptyTitle="No cases found"
          emptyDetail="Try a different search or status filter."
        />
      </Surface>
    </>
  );
}
