import { useMemo, useState } from 'react';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { TextField } from '@appfw/pds-health-components/forms';
import { Surface } from '@appfw/pds-health-components/surfaces';
import { CasesGrid } from '../../components/CasesGrid';
import { SingleSelectField } from '../../components/ui';
import { ALL_STATUSES } from '../shared/config/statuses';
import { useStaffDirectory } from '../shared/hooks/useStaffDirectory';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { matchesCase } from '../shared/utils/caseNumber';
import type { RecordStatus } from '../shared/types';

const STATUS_OPTIONS = ALL_STATUSES.map((s) => ({ value: s, label: s }));

/**
 * The one cases list. Staff see the records assigned to them. Anyone holding the
 * `routing_record.view_assignee` grant (Tax Admin) sees every record across all staff, with the
 * Assigned To column and an Assigned to filter (set it to yourself for "just mine"). Which rows
 * arrive is decided by the backend's row scoping; this screen only chooses the columns.
 */
export function MyCasesScreen() {
  const { cases, user, hasPermission } = useTaxRouting();
  const seesEveryone = hasPermission('routing_record.view_assignee');
  const { staff } = useStaffDirectory();
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState<RecordStatus | ''>('');
  const [owner, setOwner] = useState('');

  const ownerOptions = useMemo(() => staff.map((s) => ({ value: s.id, label: s.name })), [staff]);

  const rows = useMemo(() => {
    const term = query.trim().toLowerCase();
    return cases
      .filter((c) => seesEveryone || c.assignedTo === user.id)
      .filter((c) => !owner || c.assignedTo === owner)
      .filter((c) => !status || c.status === status)
      .filter((c) => !term || matchesCase(term, c) || c.client.fullName.toLowerCase().includes(term))
      .sort((a, b) => b.updated.localeCompare(a.updated));
  }, [cases, seesEveryone, user.id, query, status, owner]);

  return (
    <>
      <PageHeader
        title={seesEveryone ? 'Request Management' : 'My Cases'}
        subtitle={seesEveryone ? 'Every routing record across all staff.' : 'Records assigned to you.'}
      />
      <Surface density="compact">
        <div className={seesEveryone ? 'tax-filters tax-filters-3' : 'tax-filters'}>
          <TextField label="Search by case number or client name" value={query} onChange={(e) => setQuery(e.target.value)} />
          <SingleSelectField label="Status" value={status} onValueChange={(v) => setStatus(v as RecordStatus | '')} options={STATUS_OPTIONS} placeholder="All statuses" />
          {seesEveryone ? (
            <SingleSelectField label="Assigned to" value={owner} onValueChange={setOwner} options={ownerOptions} placeholder="Everyone" />
          ) : null}
        </div>
        <CasesGrid
          cases={rows}
          ariaLabel={seesEveryone ? 'All requests' : 'My cases'}
          pageSize={seesEveryone ? 6 : 8}
          options={seesEveryone ? { assignedTo: true } : { task: true, emails: true }}
          emptyTitle="No cases found"
          emptyDetail="Try a different search or filter."
        />
      </Surface>
    </>
  );
}
