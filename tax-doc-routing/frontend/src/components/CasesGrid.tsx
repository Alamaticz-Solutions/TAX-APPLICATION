import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { DataGridPagination, DataGridShell } from '@appfw/pds-health-components/data';
import { Button } from '@appfw/pds-health-components/primitives';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import { useStaffDirectory } from '../features/shared/hooks/useStaffDirectory';
import type { RecordStatus, RoutingCase } from '../features/shared/types';
import { formatDate } from '../features/shared/utils/format';
import { StatusBadge } from './StatusBadge';

type CaseRow = {
  id: string;
  task: string;
  client: string;
  status: RecordStatus;
  pdsEmail: string;
  personalEmail: string;
  assignedTo: string;
  created: string;
  updated: string;
  action: string;
};

export type CasesGridOptions = {
  /** Show the Task column (work queues). */
  task?: boolean;
  /** Show PDS + personal email columns. */
  emails?: boolean;
  /** Show Assigned To (admin-only, business spec 12.3). */
  assignedTo?: boolean;
};

function toRow(c: RoutingCase, staffName: (id: string) => string): CaseRow {
  return {
    id: c.id,
    task: c.task,
    client: c.client.fullName || '—',
    status: c.status,
    pdsEmail: c.client.pdsEmail || '—',
    personalEmail: c.client.personalEmail || '—',
    assignedTo: staffName(c.assignedTo),
    created: formatDate(c.created),
    updated: formatDate(c.updated),
    action: c.id
  };
}

/**
 * One grid definition reused by Dashboard, My Cases and All Cases. Paging is client-side over the
 * rows it is given; a server-backed list would pass the page in and drop the local state.
 */
export function CasesGrid({
  cases,
  options = {},
  ariaLabel,
  pageSize,
  emptyTitle = 'No cases found',
  emptyDetail
}: {
  cases: RoutingCase[];
  options?: CasesGridOptions;
  ariaLabel: string;
  pageSize?: number;
  emptyTitle?: string;
  emptyDetail?: string;
}) {
  const navigate = useNavigate();
  const { staffName } = useStaffDirectory();
  const [page, setPage] = useState(0);

  const columns = useMemo<PdsDataGridColumn<CaseRow>[]>(() => {
    const cols: PdsDataGridColumn<CaseRow>[] = [
      { key: 'id', header: 'Record ID', width: 130, render: (row) => <Link to={`/records/${row.id}`}>{row.id}</Link> }
    ];
    if (options.task) cols.push({ key: 'task', header: 'Task', width: 130 });
    cols.push({ key: 'client', header: 'Client', width: 170 });
    cols.push({ key: 'status', header: 'Status', width: 230, render: (row) => <StatusBadge status={row.status} /> });
    if (options.emails) {
      cols.push({ key: 'pdsEmail', header: 'PDS Email', width: 260 }, { key: 'personalEmail', header: 'Personal Email', width: 260 });
    }
    if (options.assignedTo) cols.push({ key: 'assignedTo', header: 'Assigned To', width: 170 });
    cols.push({ key: 'created', header: 'Created', width: 130 }, { key: 'updated', header: 'Last Updated', width: 140 });
    cols.push({
      key: 'action',
      header: 'Action',
      width: 100,
      align: 'end',
      render: (row) => (
        <Button size="sm" variant="secondary" aria-label={`View ${row.id}`} onClick={() => navigate(`/records/${row.id}`)}>
          View
        </Button>
      )
    });
    return cols;
  }, [options.task, options.emails, options.assignedTo, navigate]);

  const rows = useMemo(() => cases.map((c) => toRow(c, staffName)), [cases, staffName]);
  const size = pageSize ?? Math.max(rows.length, 1);
  const pageCount = Math.max(1, Math.ceil(rows.length / size));
  const pageIndex = Math.min(page, pageCount - 1);
  const visible = pageSize ? rows.slice(pageIndex * size, (pageIndex + 1) * size) : rows;

  return (
    <>
      <DataGridShell<CaseRow>
        ariaLabel={ariaLabel}
        columns={columns}
        rows={visible}
        rowKey="id"
        density="comfortable"
        emptyTitle={emptyTitle}
        emptyDetail={emptyDetail}
      />
      {pageSize && rows.length > pageSize ? (
        <DataGridPagination
          ariaLabel={`${ariaLabel} pagination`}
          pageSize={size}
          pageIndex={pageIndex + 1}
          pageCount={pageCount}
          startRow={pageIndex * size + 1}
          endRow={Math.min((pageIndex + 1) * size, rows.length)}
          totalRows={rows.length}
          onPreviousPage={() => setPage(pageIndex - 1)}
          onNextPage={() => setPage(pageIndex + 1)}
          canPrevious={pageIndex > 0}
          canNext={pageIndex < pageCount - 1}
        />
      ) : null}
    </>
  );
}
