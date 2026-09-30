import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { TextField } from '@appfw/pds-health-components/forms';
import { PageHeader, Tooltip } from '@appfw/pds-health-components/layout';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import { InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import { SingleSelectField } from '../../components/ui';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';

type ExceptionRow = { id: string; client: string; error: string; detail: string; document: string; age: string; action: string };

/** The short category before the first colon (e.g. "File Transfer Failed" from "File Transfer
 * Failed: the K-1 file could not be moved…") — what the PDS Badge is designed to hold; the full
 * sentence stays available as the row's tooltip and on the record's own details page. */
function errorCategory(message: string): string {
  const [head] = message.split(':');
  return head.trim();
}

/**
 * Shared queue: every staff member sees every exception (business spec 5.3). Retrying is limited
 * to the record owner or an admin, which the details screen enforces.
 */
export function ExceptionQueueScreen() {
  const { exceptionQueue } = useTaxRouting();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [error, setError] = useState('');

  const errorOptions = useMemo(
    () => [...new Set(exceptionQueue.map((c) => (c.exception ? errorCategory(c.exception.error) : '')))].filter(Boolean).map((e) => ({ value: e, label: e })),
    [exceptionQueue]
  );

  const rows = useMemo<ExceptionRow[]>(() => {
    const term = query.trim().toLowerCase();
    return exceptionQueue
      .filter((c) => !error || (c.exception && errorCategory(c.exception.error) === error))
      .filter((c) => !term || c.id.toLowerCase().includes(term) || c.client.fullName.toLowerCase().includes(term))
      .map((c) => ({
        id: c.id,
        client: c.client.fullName || '—',
        error: c.exception ? errorCategory(c.exception.error) : '',
        detail: c.exception?.error ?? '',
        document: c.exception?.document ?? '—',
        age: c.exception?.age ?? '',
        action: c.id
      }));
  }, [exceptionQueue, query, error]);

  const columns: PdsDataGridColumn<ExceptionRow>[] = [
    { key: 'id', header: 'Record ID', width: 150, render: (row) => <Link to={`/records/${row.id}`}>{row.id}</Link> },
    { key: 'client', header: 'Client', width: 170 },
    {
      key: 'error',
      header: 'Error',
      width: 210,
      render: (row) => (
        <Tooltip content={row.detail}>
          <Badge tone="danger">{row.error}</Badge>
        </Tooltip>
      )
    },
    { key: 'document', header: 'Failed Step', width: 150 },
    { key: 'age', header: 'Age', width: 90 },
    {
      key: 'action',
      header: 'Action',
      width: 100,
      align: 'end',
      render: (row) => (
        <Button size="sm" variant="secondary" aria-label={`Open ${row.id}`} onClick={() => navigate(`/records/${row.id}`)}>
          Open
        </Button>
      )
    }
  ];

  return (
    <div className="tax-stack">
      <PageHeader title="Exception Queue" subtitle="Records that need staff attention." />
      {exceptionQueue.length > 0 ? (
        <InlineAlert
          tone="warning"
          title={`${exceptionQueue.length} record${exceptionQueue.length === 1 ? '' : 's'} require immediate attention`}
          detail="Open a record to see what failed and retry the step."
        />
      ) : null}
      <Surface density="compact">
        <div className="tax-filters">
          <TextField label="Search by record ID or client" value={query} onChange={(e) => setQuery(e.target.value)} />
          <SingleSelectField label="Error type" value={error} onValueChange={setError} options={errorOptions} placeholder="All errors" />
        </div>
        <DataGridShell<ExceptionRow>
          ariaLabel="Exception queue"
          columns={columns}
          rows={rows}
          rowKey="id"
          density="comfortable"
          emptyTitle={exceptionQueue.length ? 'No exceptions match your filters' : 'No exceptions'}
          emptyDetail={exceptionQueue.length ? 'Try clearing a filter.' : 'Everything is processing normally.'}
        />
      </Surface>
    </div>
  );
}
