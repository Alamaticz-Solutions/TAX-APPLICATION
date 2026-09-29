import { useState } from 'react';
import { Link, useParams } from 'react-router';
import { Ban, RotateCcw } from 'lucide-react';
import { PageHeader, Tabs } from '@appfw/pds-health-components/layout';
import { Button } from '@appfw/pds-health-components/primitives';
import { EmptyState, ForbiddenState, InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import { DefinitionList } from '../../components/DefinitionList';
import { DocumentGrid } from '../../components/DocumentGrid';
import { FilingPlan } from '../../components/FilingPlan';
import { ProcessingTimeline } from '../../components/ProcessingTimeline';
import { StatusBadge } from '../../components/StatusBadge';
import { PROCESSING_STEPS } from '../shared/config/processingSteps';
import { STATUS } from '../shared/config/statuses';
import { useStaffDirectory } from '../shared/hooks/useStaffDirectory';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import type { Client, RoutingCase } from '../shared/types';
import { formatDate, formatDateTime } from '../shared/utils/format';
import { progressPercent } from '../shared/utils/timeline';
import { CancelRecordDialog } from './CancelRecordDialog';

function stagingState(c: RoutingCase): string {
  if (c.progress >= 7) return 'Deleted';
  return c.progress >= 2 ? 'Created' : 'Pending';
}

function clientFolderState(c: RoutingCase): string {
  if (c.exception?.error === 'Storage Folder Error') return 'Not found';
  return c.progress >= 4 ? 'Connected' : 'Not yet verified';
}

export function RecordDetailsScreen() {
  const { recordId = '' } = useParams();
  const { role, user, getCase, canActOn, canCancel, cancelCase, retryException } = useTaxRouting();
  const { staffName } = useStaffDirectory();
  const [tab, setTab] = useState('overview');
  const [cancelling, setCancelling] = useState(false);
  const record = getCase(recordId);

  if (!record) {
    return <EmptyState title="Record not found" detail={`No routing record has the ID ${recordId}.`} />;
  }
  // Standard users only see their own records; the shared exception queue is the one exception (business spec 5.3).
  const visible = role === 'admin' || record.assignedTo === user.id || record.status === STATUS.EXCEPTION;
  if (!visible) {
    return <ForbiddenState title="You don't have access to this record" detail="Standard users can only open records assigned to them." />;
  }

  const client = record.client;
  // The record's own denormalised snapshot doesn't carry every Client field (e.g. `active`); the
  // Filing Plan tab only ever reads firstName/lastName from it, so the rest are inert placeholders.
  const filingClient: Client = { ...client, refId: client.id, passwordOnFile: true, active: true, readWritePassword: '' };
  const canRetry = record.status === STATUS.EXCEPTION && canActOn(record);
  const back = role === 'admin' ? { to: '/admin/cases', label: 'Back to All Cases' } : { to: '/dashboard', label: 'Back to Dashboard' };

  const overview = (
    <div className="tax-stack">
      <Surface title="Case Summary" subtitle="Who raised this request and where it stands." density="compact">
        <DefinitionList
          columns={3}
          items={[
            { label: 'Case No.', value: record.id },
            { label: 'Status', value: <StatusBadge status={record.status} /> },
            { label: 'Client', value: client.fullName },
            { label: 'Created By', value: staffName(record.createdBy) },
            { label: 'Created On', value: formatDateTime(record.created) },
            { label: 'Assigned To', value: staffName(record.assignedTo) },
            { label: 'Last Updated', value: formatDateTime(record.updated) },
            { label: 'Progress', value: `${progressPercent(record)}% (${Math.min(record.progress, PROCESSING_STEPS.length)}/${PROCESSING_STEPS.length} steps)` }
          ]}
        />
      </Surface>
      <div className="tax-two-col">
        <Surface title="Client Information" density="compact">
          <DefinitionList
            columns={2}
            items={[
              { label: 'Name', value: client.fullName },
              { label: 'Location', value: client.officeLocation },
              { label: 'PDS Email', value: client.pdsEmail },
              { label: 'Personal Email', value: client.personalEmail },
              { label: 'Notification', value: record.notifyClient ? 'Yes' : 'No' },
              { label: 'Client Reference ID', value: client.id }
            ]}
          />
        </Surface>
        <Surface title="Storage Status" density="compact">
          <DefinitionList
            columns={2}
            items={[
              { label: 'Staging Folder', value: stagingState(record) },
              { label: 'Client Folder', value: clientFolderState(record) }
            ]}
          />
        </Surface>
      </div>
      <Surface title={`Documents (${record.documents.length})`} density="compact">
        <DocumentGrid documents={record.documents} emptyTitle="No documents" emptyDetail="No documents added yet." />
      </Surface>
    </div>
  );

  const documents = (
    <div className="tax-stack">
      <Surface density="compact">
        <DocumentGrid documents={record.documents} emptyTitle="No documents" emptyDetail="Documents added to this record will appear here." />
      </Surface>
      {record.documents.length > 0 ? (
        <Surface title="Filing plan" density="compact">
          <FilingPlan client={filingClient} documents={record.documents} />
        </Surface>
      ) : null}
    </div>
  );

  const processing = (
    <Surface title="Processing timeline" density="compact">
      <ProcessingTimeline record={record} />
    </Surface>
  );

  const activity = (
    <Surface title="Activity log" density="compact">
      <ol className="tax-history-list">
        {[...record.activity]
          .sort((a, b) => a.at.localeCompare(b.at))
          .map((a, i) => (
            <li key={`${a.at}-${i}`}>
              <span className="tax-muted tax-mono">{formatDateTime(a.at)}</span>
              <span>{a.text}</span>
              <span className="tax-subtle">{staffName(a.by)}</span>
            </li>
          ))}
      </ol>
    </Surface>
  );

  return (
    <div className="tax-stack">
      <p>
        <Link to={back.to}>← {back.label}</Link>
      </p>
      <PageHeader
        title={record.id}
        subtitle={
          <span className="tax-inline">
            <StatusBadge status={record.status} />
            {`${client.fullName} · Created by ${staffName(record.createdBy)} on ${formatDate(record.created)}${role === 'admin' ? ` · Assigned to ${staffName(record.assignedTo)}` : ''}`}
          </span>
        }
        actions={
          <>
            {record.status === STATUS.EXCEPTION ? (
              <Button variant="secondary" disabled={!canRetry} title={canRetry ? undefined : 'Only the assigned operator or an admin can retry'} onClick={() => retryException(record.id)}>
                <RotateCcw size={16} aria-hidden="true" /> Retry failed step
              </Button>
            ) : null}
            {canCancel(record) ? (
              <Button variant="danger" onClick={() => setCancelling(true)}>
                <Ban size={16} aria-hidden="true" /> Cancel / Withdraw
              </Button>
            ) : null}
          </>
        }
      />

      {record.exception ? <InlineAlert tone="danger" title={record.exception.error} detail={record.exception.detail} /> : null}
      {record.status === STATUS.CANCELLED ? <InlineAlert tone="neutral" title="Record closed" detail={record.resolutionComment ?? undefined} /> : null}

      <Tabs
        ariaLabel="Record sections"
        selectedId={tab}
        onChange={setTab}
        items={[
          { id: 'overview', label: 'Overview', content: overview },
          { id: 'documents', label: 'Documents', content: documents },
          { id: 'processing', label: 'Processing', content: processing },
          { id: 'activity', label: 'Activity', content: activity }
        ]}
      />

      {cancelling ? (
        <CancelRecordDialog
          record={record}
          onClose={() => setCancelling(false)}
          onConfirm={(resolution, comments) => {
            cancelCase(record.id, resolution, comments);
            setCancelling(false);
          }}
        />
      ) : null}
    </div>
  );
}
