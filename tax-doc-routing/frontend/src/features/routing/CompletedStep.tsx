import { useNavigate } from 'react-router';
import { FileText, Hash, Mail, User } from 'lucide-react';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import { FeedbackState, KpiTile, Surface } from '@appfw/pds-health-components/surfaces';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import { StatusBadge } from '../../components/StatusBadge';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { buildFileName } from '../shared/utils/filenameRules';
import { destinationsFor } from '../shared/utils/routingRules';

type ResultRow = { id: string; type: string; year: string; entity: string; filedAs: string; location: string; status: string };

/** Final stage: success summary, the record's key facts, and what happened to each document. */
export function CompletedStep({ recordId }: { recordId: string }) {
  const { getCase, draftApi } = useTaxRouting();
  const navigate = useNavigate();
  const record = getCase(recordId);
  if (!record) return null;
  const client = record.client;

  const rows: ResultRow[] = record.documents.map((d) => ({
    id: d.id,
    type: d.type,
    year: String(d.year),
    entity: d.entity?.name ?? 'Personal',
    filedAs: buildFileName(client, d),
    location: destinationsFor(d)
      .map((dest) => dest.folder)
      .filter(Boolean)
      .join(', '),
    status: d.status
  }));

  const columns: PdsDataGridColumn<ResultRow>[] = [
    { key: 'type', header: 'Document Type', width: 140 },
    { key: 'year', header: 'Year', width: 80 },
    { key: 'entity', header: 'Entity', width: 240 },
    { key: 'filedAs', header: 'Filed As', width: 420 },
    { key: 'location', header: 'Folders', width: 240 },
    { key: 'status', header: 'Status', width: 110, render: () => <Badge tone="success">Filed</Badge> }
  ];

  return (
    <div className="tax-stack">
      <Surface density="compact">
        <FeedbackState
          kind="success"
          title="Routing Completed"
          detail="All documents were filed and the temporary folder was cleaned up."
          action={
            <>
              <Button variant="secondary" onClick={() => navigate(`/records/${record.id}`)}>
                View Record
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  draftApi.reset();
                  navigate('/dashboard');
                }}
              >
                Back to Dashboard
              </Button>
            </>
          }
        />
      </Surface>

      <div className="tax-kpi-grid" role="list" aria-label="Record summary">
        <KpiTile as="div" role="listitem" label="Record ID" value={record.id} icon={<Hash size={20} aria-hidden="true" />} />
        <KpiTile as="div" role="listitem" label="Client" value={client.fullName ? `${client.lastName}, ${client.firstName}` : '—'} icon={<User size={20} aria-hidden="true" />} />
        <KpiTile as="div" role="listitem" label="Documents Filed" value={record.documents.length} icon={<FileText size={20} aria-hidden="true" />} tone="success" />
        <KpiTile
          as="div"
          role="listitem"
          label="Client Notification"
          value={record.notificationSent ? 'Sent' : 'Not sent'}
          detail={record.notificationSent ? undefined : 'Notification was off for this request'}
          icon={<Mail size={20} aria-hidden="true" />}
        />
      </div>

      <Surface
        title="Document results"
        subtitle="Where each document was filed."
        density="compact"
        actions={<StatusBadge status={record.status} />}
      >
        <DataGridShell<ResultRow> ariaLabel="Document results" columns={columns} rows={rows} rowKey="id" density="comfortable" emptyTitle="No documents" />
      </Surface>
    </div>
  );
}
