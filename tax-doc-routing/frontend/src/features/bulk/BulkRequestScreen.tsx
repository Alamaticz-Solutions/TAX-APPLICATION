import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { ArrowLeft, Download, Eye, Pencil, Send, Trash2 } from 'lucide-react';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { FileUpload } from '@appfw/pds-health-components/forms';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { Badge, Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import {
  cancelBulk,
  confirmBulk,
  fetchBulkStatus,
  getBulkDocument,
  previewBulk,
  removeFromBulk,
  retryBulkFailed,
  updateBulkClient,
  updateBulkDocument,
  type BulkBatch,
  type BulkClientValues,
  type BulkDoc,
  type BulkDocumentValues,
  type BulkItem,
  type BulkItemStatus
} from '../../lib/bulkRequests';
import { PdfPreviewDialog } from '../routing/PdfPreviewDialog';
import { useDocumentTypeCatalogue } from '../shared/hooks/useDocumentTypeCatalogue';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { formatCaseNumber } from '../shared/utils/caseNumber';
import { buildFileName } from '../shared/utils/filenameRules';
import { formatBytes } from '../shared/utils/format';
import { BulkClientDialog } from './BulkClientDialog';
import { BulkProgress } from './BulkProgress';
import { WorkingSpinner } from '../../components/WorkingSpinner';
import { BulkDocumentDialog } from './BulkDocumentDialog';

type Tone = 'neutral' | 'success' | 'warning' | 'danger';

const ITEM_LABEL: Record<BulkItemStatus, { label: string; tone: Tone }> = {
  ready: { label: 'Ready', tone: 'success' },
  error: { label: 'Needs attention', tone: 'warning' },
  queued: { label: 'Waiting', tone: 'neutral' },
  processing: { label: 'Creating', tone: 'neutral' },
  created: { label: 'Created', tone: 'success' },
  failed: { label: 'Failed', tone: 'danger' },
  skipped: { label: 'Skipped', tone: 'neutral' }
};

const dash = (v: string | number | null | undefined) => (v === null || v === undefined || v === '' ? '—' : String(v));

type ClientRow = {
  id: string;
  key: string;
  name: string;
  first: string;
  last: string;
  personal: string;
  pds: string;
  additional: string;
  office: string;
  internal: string;
  folder: string;
  password: string;
  notify: string;
  docs: string;
  source: string;
  status: BulkItemStatus;
  note: string;
  caseNumber: number | null;
  recordId: string;
  step: string;
};

type DocRow = {
  id: string;
  n: number;
  key: string;
  client: string;
  recordId: string;
  caseNumber: number | null;
  type: string;
  year: string;
  entityType: string;
  entityNumber: string;
  entityName: string;
  uploaded: string;
  named: string;
  status: string;
  source: string;
  note: string;
  actions: string;
};

function clientNote(i: BulkItem): string {
  const all = [...i.errors, ...(i.message ? [i.message] : [])];
  if (all.length > 0) return all.join(' ');
  return i.differences.join(' ');
}

function saveFile(file: File) {
  const url = URL.createObjectURL(file);
  const a = document.createElement('a');
  a.href = url;
  a.download = file.name;
  a.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Bulk request creation: an Excel file of client details and a ZIP with one folder per Client Key
 * become one request per client. Every client detail and every document is shown first (and can be
 * corrected); nothing is created until the user confirms.
 */
export function BulkRequestScreen() {
  const navigate = useNavigate();
  const { userName } = useTaxRouting();
  const { getDocumentType } = useDocumentTypeCatalogue();
  const who = userName ?? '';
  const [excel, setExcel] = useState<File | null>(null);
  const [zip, setZip] = useState<File | null>(null);
  const [batch, setBatch] = useState<BulkBatch | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [editingClient, setEditingClient] = useState<string | null>(null);
  const [editingDoc, setEditingDoc] = useState<{ key: string; doc: BulkDoc } | null>(null);
  const [preview, setPreview] = useState<File | null>(null);

  const message = (err: unknown, fallback: string) => (err instanceof Error && err.message ? err.message : fallback);

  const [startedAt, setStartedAt] = useState(() => Date.now());
  const [connectionTrouble, setConnectionTrouble] = useState(false);
  const failures = useRef(0);
  const batchId = batch?.id;
  const working = batch?.status === 'checking' || batch?.status === 'processing';

  // While the server is checking the files or creating the requests, follow its real progress. The
  // spinner stays until the server says it has finished, never because a request merely returned.
  useEffect(() => {
    if (!batchId || !working) return undefined;
    let alive = true;
    const timer = window.setInterval(() => {
      fetchBulkStatus(who, batchId)
        .then((next) => {
          if (!alive) return;
          failures.current = 0;
          setConnectionTrouble(false);
          setBatch(next);
        })
        .catch(() => {
          // A missed answer is not the end of the work: keep asking, and say so after a few misses.
          failures.current += 1;
          if (alive && failures.current >= 3) setConnectionTrouble(true);
        });
    }, 1500);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [batchId, working, who]);

  const run = async (work: () => Promise<void>) => {
    setError('');
    setBusy(true);
    try {
      await work();
    } catch (err) {
      setError(message(err, 'Something went wrong. Try again.'));
    } finally {
      setBusy(false);
    }
  };

  const check = () =>
    run(async () => {
      if (!excel || !zip) {
        setError('Choose both the Excel file and the ZIP file.');
        return;
      }
      setStartedAt(Date.now());
      failures.current = 0;
      setConnectionTrouble(false);
      setBatch(await previewBulk(who, excel, zip));
    });

  const saveClient = (key: string, values: BulkClientValues) =>
    run(async () => {
      setBatch(await updateBulkClient(who, batch!.id, key, values));
      setEditingClient(null);
    });

  const saveDoc = (key: string, doc: BulkDoc, values: BulkDocumentValues) =>
    run(async () => {
      setBatch(await updateBulkDocument(who, batch!.id, key, doc.file_name, values));
      setEditingDoc(null);
    });

  const flipProtection = (key: string, doc: BulkDoc, on: boolean) =>
    saveDoc(key, doc, {
      documentType: doc.document_type,
      year: doc.year ?? 0,
      entityType: doc.entity_type,
      entityName: doc.entity_name,
      entityNumber: doc.entity_number,
      passwordProtected: on
    });

  const remove = (key: string, fileName: string) => run(async () => setBatch(await removeFromBulk(who, batch!.id, key, fileName)));
  const open = (key: string, fileName: string, then: (f: File) => void) => run(async () => then(await getBulkDocument(who, batch!.id, key, fileName)));
  const create = () =>
    run(async () => {
      setStartedAt(Date.now());
      failures.current = 0;
      setConnectionTrouble(false);
      setBatch(await confirmBulk(who, batch!.id));
    });
  const retry = () =>
    run(async () => {
      setStartedAt(Date.now());
      failures.current = 0;
      setConnectionTrouble(false);
      setBatch(await retryBulkFailed(who, batch!.id));
    });
  const discard = () =>
    run(async () => {
      if (batch) await cancelBulk(who, batch.id).catch(() => undefined);
      navigate('/dashboard');
    });

  const checking = batch?.status === 'checking';
  if (!batch || checking || batch.status === 'check_failed') {
    return (
      <div className="tax-stack">
        <PageHeader title="Bulk Request" subtitle="Create one request per client from an Excel file and a ZIP." />
        <Surface title="Upload Files" subtitle="Nothing is created until you review and confirm." density="compact">
          <FileUpload
            label="Client Details (Excel)"
            accept=".xlsx"
            hint="One row per client: Client Key, names, emails, Internal Folder ID, Client Folder ID, Read Write Password, Send Notification."
            onChange={(e) => setExcel(e.target.files?.[0] ?? null)}
          />
          {excel ? <p className="tax-muted">{excel.name}</p> : null}
          <FileUpload
            label="Documents (ZIP)"
            accept=".zip"
            hint="One folder per client, named with the Client Key, holding that client's PDFs. Name each PDF like K-1_2025_EntityName.pdf."
            onChange={(e) => setZip(e.target.files?.[0] ?? null)}
          />
          {zip ? <p className="tax-muted">{zip.name}</p> : null}
          {error ? <InlineAlert tone="danger" title="The files could not be checked" detail={error} /> : null}
          {batch?.status === 'check_failed' ? (
            <InlineAlert tone="danger" title="The files could not be checked" detail={`${batch.error ?? 'Something went wrong.'} Nothing was created. Correct the files and press Check Files again.`} />
          ) : null}
        </Surface>
        {checking && batch ? (
          <Surface density="compact">
            <BulkProgress
              title="Processing documents, please wait…"
              step={batch.progress_label}
              done={batch.progress_done}
              total={batch.progress_total}
              startedAt={startedAt}
              note="Nothing is created yet. The results appear here when the check is complete."
            />
            {connectionTrouble ? <InlineAlert tone="warning" title="The connection to the server is slow" detail="The check is still running on the server. Trying again automatically." /> : null}
          </Surface>
        ) : null}
        <div className="tax-actions tax-spread">
          <Button variant="quiet" onClick={() => navigate('/dashboard')}>
            Cancel
          </Button>
          <Button variant="primary" disabled={busy || checking} onClick={() => void check()}>
            {busy || checking ? (
              <>
                <WorkingSpinner /> Checking Files…
              </>
            ) : (
              'Check Files'
            )}
          </Button>
        </div>
      </div>
    );
  }

  const staged = batch.status === 'staged';
  const readyCount = batch.items.filter((i) => i.status === 'ready').length;
  const docCount = batch.items.reduce((n, i) => n + i.docs.length, 0);
  const issueCount = batch.items.filter((i) => i.status === 'error').length;

  // ---------------------------------------------------------------------- client details
  const clientRows: ClientRow[] = batch.items.map((i) => ({
    id: i.client_key,
    key: i.client_key,
    name: i.client.full_name,
    first: i.client.first_name,
    last: i.client.last_name,
    personal: i.client.personal_email,
    pds: i.client.pds_email,
    additional: i.client.additional_email,
    office: i.client.office_location,
    internal: i.client.internal_folder,
    folder: i.client.client_folder,
    password: i.client.password_on_file ? '••••••••••' : '',
    notify: i.client.notify ? 'Yes' : 'No',
    docs: String(i.docs.length),
    source: i.source === 'existing' ? 'In client directory' : 'New client',
    status: i.status,
    note: clientNote(i),
    caseNumber: i.case_number,
    recordId: i.record_id ?? '',
    step: i.progress_step !== undefined && i.progress_step !== null ? `${i.progress_step} of 9` : ''
  }));

  const text = (key: keyof ClientRow): PdsDataGridColumn<ClientRow>['render'] => (r) => dash(r[key] as string);
  const clientColumns: PdsDataGridColumn<ClientRow>[] = [
    { key: 'key', header: 'Client Key', width: 110 },
    { key: 'name', header: 'Client Name', width: 210, render: text('name') },
    { key: 'first', header: 'First Name', width: 120, render: text('first') },
    { key: 'last', header: 'Last Name', width: 120, render: text('last') },
    { key: 'personal', header: 'Client Personal Email', width: 220, render: text('personal') },
    { key: 'pds', header: 'Client PDS Email', width: 220, render: text('pds') },
    { key: 'additional', header: 'Additional Email', width: 200, render: text('additional') },
    { key: 'office', header: 'Office Location', width: 160, render: text('office') },
    { key: 'internal', header: 'Internal Folder ID', width: 160, render: text('internal') },
    { key: 'folder', header: 'Client Folder ID', width: 160, render: text('folder') },
    { key: 'password', header: 'Read Write Password', width: 160, render: text('password') },
    { key: 'notify', header: 'Send Notification', width: 130 },
    { key: 'docs', header: 'Documents', width: 100 },
    { key: 'source', header: 'Client Record', width: 160 },
    { key: 'status', header: 'Status', width: 150, render: (r) => <Badge tone={ITEM_LABEL[r.status].tone}>{ITEM_LABEL[r.status].label}</Badge> },
    ...(staged
      ? []
      : ([
          {
            key: 'caseNumber',
            header: 'Request',
            width: 110,
            render: (r) => (r.recordId ? <Link to={`/records/${r.recordId}`}>{formatCaseNumber(r.caseNumber, r.recordId)}</Link> : '—')
          },
          { key: 'step', header: 'Filing Step', width: 100, render: text('step') }
        ] as PdsDataGridColumn<ClientRow>[])),
    { key: 'note', header: 'Details', width: 380, render: (r) => <span className="tax-muted">{dash(r.note)}</span> },
    ...(staged
      ? ([
          {
            key: 'id',
            header: 'Actions',
            width: 100,
            align: 'end',
            render: (r) => (
              <span className="tax-inline">
                <Button size="sm" variant="quiet" aria-label={`Edit client ${r.key}`} onClick={() => setEditingClient(r.key)}>
                  <Pencil size={15} aria-hidden="true" />
                </Button>
                <Button size="sm" variant="quiet" aria-label={`Remove client ${r.key}`} onClick={() => void remove(r.key, '')}>
                  <Trash2 size={15} aria-hidden="true" />
                </Button>
              </span>
            )
          }
        ] as PdsDataGridColumn<ClientRow>[])
      : [])
  ];

  // ---------------------------------------------------------------------- documents
  const flatDocs = batch.items.flatMap((i) => i.docs.map((d) => ({ item: i, doc: d })));
  const find = (id: string) => flatDocs.find(({ item, doc }) => `${item.client_key}\u0000${doc.file_name}` === id);
  const docRows: DocRow[] = flatDocs.map(({ item, doc }, index) => ({
    id: `${item.client_key}\u0000${doc.file_name}`,
    n: index + 1,
    key: item.client_key,
    client: `${item.client_key}${item.client.full_name ? ` · ${item.client.full_name}` : ''}`,
    recordId: item.record_id ?? '',
    caseNumber: item.case_number,
    type: doc.document_type,
    year: doc.year ? String(doc.year) : '',
    entityType: doc.entity_type,
    entityNumber: doc.entity_number,
    entityName: doc.entity_name,
    uploaded: `${doc.file_name} · ${formatBytes(doc.size)}`,
    named: doc.status === 'ready' ? buildFileName({ firstName: item.client.first_name, lastName: item.client.last_name, fullName: item.client.full_name }, { fileName: doc.file_name }) : '',
    status: doc.status,
    source: doc.source,
    note: doc.errors.join(' '),
    actions: ''
  }));

  const docColumns: PdsDataGridColumn<DocRow>[] = [
    { key: 'n', header: '#', width: 48 },
    { key: 'client', header: 'Client', width: 220 },
    {
      key: 'caseNumber',
      header: 'Request',
      width: 120,
      render: (r) => (r.recordId ? <Link to={`/records/${r.recordId}`}>{formatCaseNumber(r.caseNumber, r.recordId)}</Link> : <span className="tax-muted">Not created yet</span>)
    },
    { key: 'type', header: 'Document Type', width: 130, render: (r) => dash(r.type) },
    { key: 'year', header: 'Year', width: 72, render: (r) => dash(r.year) },
    { key: 'entityType', header: 'Entity Type', width: 140, render: (r) => dash(r.entityType) },
    { key: 'entityNumber', header: 'Entity Number', width: 120, render: (r) => dash(r.entityNumber) },
    { key: 'entityName', header: 'Entity Name', width: 220, render: (r) => dash(r.entityName) },
    { key: 'uploaded', header: 'Document Uploaded', width: 260 },
    { key: 'named', header: 'Document Will Be Named', width: 300, render: (r) => dash(r.named) },
    {
      key: 'id',
      header: 'Password Protect?',
      width: 190,
      render: (r) => {
        const found = find(r.id);
        if (!found) return '—';
        const { item, doc } = found;
        const supported = Boolean(doc.document_type && getDocumentType(doc.document_type)?.supportsPasswordProtection) && item.client.password_on_file;
        return (
          <SegmentedControl<'yes' | 'no'>
            ariaLabel={`Password protect ${doc.file_name}`}
            size="sm"
            disabled={!staged || !supported || doc.status !== 'ready'}
            value={doc.password_protected ? 'yes' : 'no'}
            onValueChange={(v) => void flipProtection(item.client_key, doc, v === 'yes')}
            options={[
              { value: 'yes', label: 'Yes' },
              { value: 'no', label: 'No' }
            ]}
          />
        );
      }
    },
    {
      key: 'status',
      header: 'Status',
      width: 200,
      render: (r) =>
        r.status === 'ready' ? (
          <Badge tone="neutral">{r.source === 'manual' ? 'Reviewed' : 'Read from file name'}</Badge>
        ) : (
          <span className="tax-inline">
            <Badge tone="warning">Needs details</Badge>
          </span>
        )
    },
    { key: 'note', header: 'Details', width: 300, render: (r) => <span className="tax-muted">{dash(r.note)}</span> },
    ...(staged
      ? ([
          {
            key: 'actions',
            header: 'Actions',
            width: 190,
            align: 'end',
            render: (r) => {
              const found = find(r.id);
              if (!found) return null;
              const { item, doc } = found;
              return (
                <span className="tax-inline">
                  <Button size="sm" variant="quiet" aria-label={`Preview ${doc.file_name}`} onClick={() => void open(item.client_key, doc.file_name, setPreview)}>
                    <Eye size={15} aria-hidden="true" />
                  </Button>
                  <Button size="sm" variant="quiet" aria-label={`Download ${doc.file_name}`} onClick={() => void open(item.client_key, doc.file_name, saveFile)}>
                    <Download size={15} aria-hidden="true" />
                  </Button>
                  <Button size="sm" variant="quiet" aria-label={`Set details for ${doc.file_name}`} onClick={() => setEditingDoc({ key: item.client_key, doc })}>
                    <Pencil size={15} aria-hidden="true" />
                  </Button>
                  <Button size="sm" variant="quiet" aria-label={`Remove ${doc.file_name}`} onClick={() => void remove(item.client_key, doc.file_name)}>
                    <Trash2 size={15} aria-hidden="true" />
                  </Button>
                </span>
              );
            }
          }
        ] as PdsDataGridColumn<DocRow>[])
      : [])
  ];

  const finished = batch.status === 'completed' || batch.status === 'completed_with_errors';
  const created = batch.items.filter((i) => i.status === 'created').length;
  const failed = batch.items.filter((i) => i.status === 'failed').length;
  const processing = batch.status === 'processing';
  const attempted = batch.items.filter((i) => ['queued', 'processing', 'created', 'failed'].includes(i.status)).length;
  const settled = created + failed;
  const filed = batch.items.filter((i) => i.status === 'created' && (i.record_status === 'Resolved' || i.record_status === 'Exception')).length;
  const subtitle = staged
    ? `${batch.file_name}: ${batch.items.length} clients, ${docCount} documents. Review and correct them, then create the requests.`
    : finished
      ? `${created} request${created === 1 ? '' : 's'} created${failed ? `, ${failed} failed` : ''}.`
      : 'Creating the requests. You can leave this page; filing continues on the server.';
  const editedItem = batch.items.find((i) => i.client_key === editingClient) ?? null;

  return (
    <div className="tax-stack">
      <PageHeader title="Bulk Request" subtitle={subtitle} />
      {error ? <InlineAlert tone="danger" title="Something went wrong" detail={error} /> : null}
      {processing ? (
        <Surface density="compact">
          <BulkProgress
            title="Creating requests, please wait…"
            step={`${created} of ${attempted} requests created${failed ? `, ${failed} failed` : ''}`}
            done={settled}
            total={attempted}
            startedAt={startedAt}
            note={`Filing in Box continues in the background: ${filed} of ${created} created requests are filed so far. You can follow each one in the table below.`}
          />
          {connectionTrouble ? <InlineAlert tone="warning" title="The connection to the server is slow" detail="The requests are still being created on the server. Trying again automatically." /> : null}
        </Surface>
      ) : null}
      {finished && failed === 0 ? (
        <InlineAlert tone="success" title={`${created} request${created === 1 ? '' : 's'} created`} detail="Filing in Box continues in the background. Open a request from the table to follow it." />
      ) : null}
      {finished && failed > 0 ? (
        <InlineAlert
          tone="danger"
          title={`${created} request${created === 1 ? '' : 's'} created, ${failed} failed`}
          detail="Retrying carries on with each failed client and does not create anything twice."
          action={
            <Button size="sm" variant="secondary" disabled={busy} onClick={() => void retry()}>
              {busy ? (
                <>
                  <WorkingSpinner size={14} /> Retrying…
                </>
              ) : (
                'Retry Failed'
              )}
            </Button>
          }
        />
      ) : null}
      {staged && issueCount > 0 ? (
        <InlineAlert
          tone="warning"
          title={`${issueCount} client${issueCount === 1 ? ' needs' : 's need'} attention`}
          detail="Fix or remove them. Clients that still have problems are skipped when you create the requests."
        />
      ) : null}
      {batch.notes.map((note) => (
        <InlineAlert key={note} tone="warning" title="A file was ignored" detail={note} />
      ))}

      <Surface title={`Client Details (${batch.items.length})`} subtitle={staged ? 'As given in the Excel file. Use the pencil to correct a client before creating the requests.' : undefined} density="compact">
        <DataGridShell<ClientRow>
          ariaLabel="Client details from the Excel file"
          columns={clientColumns}
          rows={clientRows}
          rowKey="id"
          density="comfortable"
          emptyTitle="No clients"
          emptyDetail="Every client was removed."
        />
      </Surface>

      <Surface title={`Documents (${docCount})`} subtitle="Every document in the ZIP, with the request it belongs to. Preview and download them before you create the requests." density="compact">
        <DataGridShell<DocRow>
          ariaLabel="Documents in this bulk upload"
          columns={docColumns}
          rows={docRows}
          rowKey="id"
          density="comfortable"
          emptyTitle="No documents"
          emptyDetail="There are no documents in the ZIP."
        />
      </Surface>

      <div className="tax-actions tax-spread">
        {staged || processing ? (
          <>
            <Button variant="secondary" disabled={busy || processing} onClick={() => void discard()}>
              <ArrowLeft size={16} aria-hidden="true" /> Cancel
            </Button>
            <Button variant="primary" disabled={busy || processing || readyCount === 0} onClick={() => void create()}>
              {busy || processing ? (
                <>
                  <WorkingSpinner /> Creating Requests…
                </>
              ) : (
                <>
                  <Send size={16} aria-hidden="true" /> Create Requests
                </>
              )}
            </Button>
          </>
        ) : (
          <>
            <span />
            <div className="tax-inline">
              <Button variant="quiet" onClick={() => navigate('/cases')}>
                View My Cases
              </Button>
              {finished ? (
                <Button variant="primary" onClick={() => setBatch(null)}>
                  Start Another Bulk Request
                </Button>
              ) : null}
            </div>
          </>
        )}
      </div>

      {editedItem ? <BulkClientDialog item={editedItem} onSave={(values) => void saveClient(editedItem.client_key, values)} onClose={() => setEditingClient(null)} /> : null}
      {editingDoc ? (
        <BulkDocumentDialog
          doc={editingDoc.doc}
          canProtect={Boolean(batch.items.find((i) => i.client_key === editingDoc.key)?.client.password_on_file)}
          onSave={(values) => void saveDoc(editingDoc.key, editingDoc.doc, values)}
          onClose={() => setEditingDoc(null)}
        />
      ) : null}
      {preview ? <PdfPreviewDialog file={preview} onClose={() => setPreview(null)} /> : null}
    </div>
  );
}
