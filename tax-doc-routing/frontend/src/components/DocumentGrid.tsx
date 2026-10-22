import { Lock, Trash2, Unlock } from 'lucide-react';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import type { DocumentFileStatus, TaxDocument, Tone } from '../features/shared/types';
import { formatBytes } from '../features/shared/utils/format';

type DocumentRow = {
  id: string;
  type: string;
  year: string;
  entity: string;
  file: string;
  fileName: string;
  protection: boolean;
  status: DocumentFileStatus;
  duplicate: boolean;
  actions: string;
};

const FILE_STATUS_TONE: Record<DocumentFileStatus, Tone> = {
  Ready: 'neutral',
  Uploaded: 'neutral',
  Filed: 'success',
  Failed: 'danger'
};

function toRow(d: TaxDocument): DocumentRow {
  const size = formatBytes(d.fileSize);
  return {
    id: d.id,
    type: d.type,
    // A string so the grid does not format 2025 as "2,025".
    year: String(d.year),
    entity: d.entity?.name ?? '—',
    file: size ? `${d.fileName} · ${size}` : d.fileName,
    fileName: d.fileName,
    protection: d.passwordProtected,
    status: d.status,
    duplicate: Boolean(d.isDuplicate),
    actions: d.id
  };
}

/** Read-only unless `onRemove` is supplied (record details / confirmation stay read-only). */
export function DocumentGrid({
  documents,
  onRemove,
  onOpen,
  emptyTitle = 'No documents',
  emptyDetail
}: {
  documents: TaxDocument[];
  onRemove?: (id: string) => void;
  /** When supplied, the file name opens the document (preview and download). */
  onOpen?: (id: string) => void;
  emptyTitle?: string;
  emptyDetail?: string;
}) {
  const columns: PdsDataGridColumn<DocumentRow>[] = [
    { key: 'type', header: 'Document Type', width: 150 },
    { key: 'year', header: 'Year', width: 90 },
    { key: 'entity', header: 'Entity', width: 260 },
    {
      key: 'file',
      header: 'File',
      width: 300,
      render: (row) =>
        onOpen ? (
          <Button size="sm" variant="quiet" aria-label={`Open ${row.fileName}`} onClick={() => onOpen(row.id)}>
            {row.file}
          </Button>
        ) : (
          row.file
        )
    },
    {
      key: 'protection',
      header: 'Protection',
      width: 130,
      render: (row) =>
        row.protection ? (
          <span className="tax-inline">
            <Lock size={14} aria-hidden="true" /> Yes
          </span>
        ) : (
          <span className="tax-inline tax-muted">
            <Unlock size={14} aria-hidden="true" /> No
          </span>
        )
    },
    {
      key: 'status',
      header: 'Status',
      width: 220,
      render: (row) => (
        <span className="tax-inline">
          <Badge tone={FILE_STATUS_TONE[row.status]}>{row.status}</Badge>
          {row.duplicate ? <Badge tone="warning">Duplicate</Badge> : null}
        </span>
      )
    }
  ];
  if (onRemove) {
    columns.push({
      key: 'actions',
      header: 'Actions',
      align: 'end',
      render: (row) => (
        <Button size="sm" variant="quiet" aria-label={`Remove ${row.type} ${row.year}`} onClick={() => onRemove(row.id)}>
          <Trash2 size={15} aria-hidden="true" />
        </Button>
      )
    });
  }
  return (
    <DataGridShell<DocumentRow>
      ariaLabel="Documents on this routing record"
      columns={columns}
      rows={documents.map(toRow)}
      rowKey="id"
      density="comfortable"
      emptyTitle={emptyTitle}
      emptyDetail={emptyDetail}
    />
  );
}
