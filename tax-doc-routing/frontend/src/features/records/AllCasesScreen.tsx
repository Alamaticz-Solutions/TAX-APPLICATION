import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { Plus } from 'lucide-react';
import { TextField } from '@appfw/pds-health-components/forms';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { Button } from '@appfw/pds-health-components/primitives';
import { Surface } from '@appfw/pds-health-components/surfaces';
import { CasesGrid } from '../../components/CasesGrid';
import { SingleSelectField } from '../../components/ui';
import { ALL_STATUSES } from '../shared/config/statuses';
import { useStaffDirectory } from '../shared/hooks/useStaffDirectory';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import type { RecordStatus } from '../shared/types';

const STATUS_OPTIONS = ALL_STATUSES.map((s) => ({ value: s, label: s }));

/** Admin-only: every record across all staff, including the Assigned To column. */
export function AllCasesScreen() {
  const { cases, draftApi } = useTaxRouting();
  const { staff } = useStaffDirectory();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState<RecordStatus | ''>('');
  const [owner, setOwner] = useState('');

  const ownerOptions = useMemo(() => staff.map((s) => ({ value: s.id, label: s.name })), [staff]);

  const rows = useMemo(() => {
    const term = query.trim().toLowerCase();
    return cases
      .filter((c) => !status || c.status === status)
      .filter((c) => !owner || c.assignedTo === owner)
      .filter((c) => !term || c.id.toLowerCase().includes(term) || c.client.fullName.toLowerCase().includes(term))
      .sort((a, b) => b.updated.localeCompare(a.updated));
  }, [cases, query, status, owner]);

  return (
    <>
      <PageHeader
        title="All Cases"
        subtitle="Every routing record across all staff."
        actions={
          <Button
            variant="primary"
            onClick={() => {
              draftApi.reset();
              navigate('/new/client');
            }}
          >
            <Plus size={16} aria-hidden="true" /> Create New Request
          </Button>
        }
      />
      <Surface density="compact">
        <div className="tax-filters tax-filters-3">
          <TextField label="Search by record ID or client name" value={query} onChange={(e) => setQuery(e.target.value)} />
          <SingleSelectField label="Status" value={status} onValueChange={(v) => setStatus(v as RecordStatus | '')} options={STATUS_OPTIONS} placeholder="All statuses" />
          <SingleSelectField label="Assigned to" value={owner} onValueChange={setOwner} options={ownerOptions} placeholder="Everyone" />
        </div>
        <CasesGrid
          cases={rows}
          ariaLabel="All cases"
          pageSize={6}
          options={{ assignedTo: true }}
          emptyTitle="No cases match your filters"
          emptyDetail="Try clearing a filter."
        />
      </Surface>
    </>
  );
}
