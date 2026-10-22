import { useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { ArrowLeft, Eye, Pencil, Send, Trash2 } from 'lucide-react';
import { WorkingSpinner } from '../../components/WorkingSpinner';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { FileUpload } from '@appfw/pds-health-components/forms';
import { Badge, Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import { useDocumentTypeCatalogue } from '../shared/hooks/useDocumentTypeCatalogue';
import { useRoutingRecord } from '../shared/hooks/useRoutingRecord';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { mockAIService } from '../shared/services/mockAIService';
import type { TaxDocument } from '../shared/types';
import { formatCaseNumber } from '../shared/utils/caseNumber';
import { buildFileName } from '../shared/utils/filenameRules';
import { formatBytes } from '../shared/utils/format';
import { EditDocumentDialog, type EditedDocument } from './EditDocumentDialog';
import { PdfPreviewDialog } from './PdfPreviewDialog';

type Upload = { id: string; fileName: string; fileSize: number; storageFileId?: string };

type GridRow = {
  id: string;
  n: number;
  type: string;
  year: string;
  entityType: string;
  entityNumber: string;
  entityName: string;
  uploaded: string;
  size: string;
  named: string;
  protection: string;
  status: string;
  actions: string;
};

const AI_CONFIDENT = 60;
const isPdf = (f: File) => f.type === 'application/pdf' || f.name.toLowerCase().endsWith('.pdf');
const dash = (v: string | number | undefined | null) => (v === undefined || v === null || v === '' ? '—' : String(v));

/**
 * Screen 2 of "Create New Request". Each PDF is stored in the request's temporary storage folder as
 * soon as it is added; the (mock) AI assistant reads it and prefills one grid row per file. Nothing is final until the user reviews the rows: every value
 * can be corrected in the Review dialog, and password protection can be flipped inline.
 */
export function UploadDocumentsStep() {
  const navigate = useNavigate();
  const { draft, addDocument, updateDocument, removeDocument, submit, discard, uploadFile } = useRoutingRecord();
  const { getDocumentType } = useDocumentTypeCatalogue();
  const { getCase } = useTaxRouting();
  const { client, documents } = draft;
  // The request already exists (it was created on screen 1), so its number is shown here.
  const created = draft.recordId ? getCase(draft.recordId) : null;
  const requestLabel = created ? formatCaseNumber(created.caseNumber, created.id) : '';
  const [analyzing, setAnalyzing] = useState<Upload[]>([]);
  const [unclassified, setUnclassified] = useState<Upload[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [rejected, setRejected] = useState<string[]>([]);
  const [uploadErrors, setUploadErrors] = useState<string[]>([]);
  const [submitError, setSubmitError] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [tried, setTried] = useState(false);
  const [previewId, setPreviewId] = useState<string | null>(null);
  // The files added in this browser, kept so the user can open one to check it is the right document.
  const files = useRef(new Map<string, File>());
  const counter = useRef(0);

  const nextId = () => `doc-${Date.now()}-${counter.current++}`;
  const passwordDefault = client?.passwordOnFile ?? false;

  const processFile = async (file: File) => {
    const upload: Upload = { id: nextId(), fileName: file.name, fileSize: file.size };
    files.current.set(upload.id, file);
    setAnalyzing((list) => [...list, upload]);
    let suggestion;
    try {
      // The file goes to the request's temporary folder while the AI reads its name.
      const [read, stored] = await Promise.all([mockAIService.analyzeDocument(file.name), uploadFile(file)]);
      suggestion = read;
      upload.storageFileId = stored.fileId;
    } catch (err) {
      setAnalyzing((list) => list.filter((u) => u.id !== upload.id));
      files.current.delete(upload.id);
      setUploadErrors((list) => [...list, err instanceof Error && err.message ? err.message : `${file.name} could not be uploaded.`]);
      return;
    }
    setAnalyzing((list) => list.filter((u) => u.id !== upload.id));
    if (!suggestion.documentType || suggestion.confidence < AI_CONFIDENT) {
      setUnclassified((list) => [...list, upload]);
      return;
    }
    const cfg = getDocumentType(suggestion.documentType);
    addDocument({
      id: upload.id,
      type: suggestion.documentType,
      year: suggestion.year,
      name: upload.fileName,
      fileName: upload.fileName,
      fileSize: upload.fileSize,
      entity: cfg?.allowsEntity && suggestion.entity ? { type: suggestion.entity.type, name: suggestion.entity.name, number: suggestion.entity.number } : null,
      passwordProtected: Boolean(cfg?.supportsPasswordProtection && passwordDefault),
      attachments: {},
      status: 'Ready',
      aiConfidence: suggestion.confidence,
      storageFileId: upload.storageFileId
    });
  };

  const onFiles = (list: FileList | null) => {
    if (!list?.length) return;
    const files = Array.from(list);
    setRejected(files.filter((f) => !isPdf(f)).map((f) => f.name));
    setUploadErrors([]);
    files.filter(isPdf).forEach((f) => void processFile(f));
  };

  const saveEdit = (id: string, values: EditedDocument) => {
    updateDocument(id, { ...values, aiConfidence: undefined });
    setEditingId(null);
  };

  const classify = (upload: Upload, values: EditedDocument) => {
    addDocument({ id: upload.id, fileName: upload.fileName, fileSize: upload.fileSize, status: 'Ready', storageFileId: upload.storageFileId, ...values });
    setUnclassified((list) => list.filter((u) => u.id !== upload.id));
  };

  const submitAll = async () => {
    setTried(true);
    if (!client || documents.length === 0 || analyzing.length > 0) return;
    setSubmitError('');
    setSubmitting(true);
    try {
      await submit();
      navigate('/new/processing');
    } catch (err) {
      setSubmitError(err instanceof Error && err.message ? err.message : 'The request could not be submitted.');
      setSubmitting(false);
    }
  };

  const isDuplicate = (doc: TaxDocument, index: number) =>
    documents.some((d, i) => i < index && d.type === doc.type && d.year === doc.year && (d.entity?.number ?? '') === (doc.entity?.number ?? ''));

  const rows: GridRow[] = [
    ...documents.map((d, i) => ({
      id: d.id,
      n: i + 1,
      type: d.type,
      year: String(d.year),
      entityType: d.entity?.type ?? '',
      entityNumber: d.entity?.number ?? '',
      entityName: d.entity?.name ?? '',
      uploaded: d.fileName,
      size: formatBytes(d.fileSize),
      named: client ? buildFileName(client, d) : '',
      protection: d.id,
      status: d.id,
      actions: d.id
    })),
    ...analyzing.map((u, i) => ({
      id: u.id,
      n: documents.length + i + 1,
      type: '',
      year: '',
      entityType: '',
      entityNumber: '',
      entityName: '',
      uploaded: u.fileName,
      size: formatBytes(u.fileSize),
      named: '',
      protection: '',
      status: u.id,
      actions: ''
    }))
  ];

  const docById = (id: string) => documents.find((d) => d.id === id);

  const columns: PdsDataGridColumn<GridRow>[] = [
    { key: 'n', header: '#', width: 48 },
    { key: 'type', header: 'Document Type', width: 130, render: (r) => dash(r.type) },
    { key: 'year', header: 'Year', width: 72, render: (r) => dash(r.year) },
    { key: 'entityType', header: 'Entity Type', width: 150, render: (r) => dash(r.entityType) },
    { key: 'entityNumber', header: 'Entity Number', width: 130, render: (r) => dash(r.entityNumber) },
    { key: 'entityName', header: 'Entity Name', width: 220, render: (r) => dash(r.entityName) },
    {
      key: 'uploaded',
      header: 'Document Uploaded',
      width: 240,
      render: (r) => (
        <span className="tax-stack-tight">
          <span>{r.uploaded}</span>
          {r.size ? <span className="tax-muted">{r.size}</span> : null}
        </span>
      )
    },
    { key: 'named', header: 'Document Will Be Named', width: 270, render: (r) => dash(r.named) },
    {
      key: 'protection',
      header: 'Password Protect?',
      width: 200,
      render: (r) => {
        const doc = docById(r.id);
        if (!doc) return '—';
        const supported = Boolean(getDocumentType(doc.type)?.supportsPasswordProtection);
        return (
          <SegmentedControl<'yes' | 'no'>
            ariaLabel={`Password protect ${doc.fileName}`}
            size="sm"
            disabled={!supported}
            value={doc.passwordProtected ? 'yes' : 'no'}
            onValueChange={(v) => updateDocument(doc.id, { passwordProtected: v === 'yes' })}
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
      header: 'AI Status',
      width: 210,
      render: (r) => {
        const doc = docById(r.id);
        if (!doc) {
          return (
            <span className="tax-inline tax-muted">
              <WorkingSpinner size={14} /> Analyzing…
            </span>
          );
        }
        const index = documents.indexOf(doc);
        return (
          <span className="tax-inline">
            {doc.aiConfidence ? <Badge tone={doc.aiConfidence >= 90 ? 'success' : 'warning'}>AI · {doc.aiConfidence}%</Badge> : <Badge tone="neutral">Reviewed</Badge>}
            {isDuplicate(doc, index) ? <Badge tone="warning">Possible duplicate</Badge> : null}
          </span>
        );
      }
    },
    {
      key: 'actions',
      header: 'Actions',
      width: 140,
      align: 'end',
      render: (r) =>
        docById(r.id) ? (
          <span className="tax-inline">
            <Button size="sm" variant="quiet" aria-label={`Preview ${r.uploaded}`} disabled={!files.current.has(r.id)} onClick={() => setPreviewId(r.id)}>
              <Eye size={15} aria-hidden="true" />
            </Button>
            <Button size="sm" variant="quiet" aria-label={`Review ${r.uploaded}`} onClick={() => setEditingId(r.id)}>
              <Pencil size={15} aria-hidden="true" />
            </Button>
            <Button size="sm" variant="quiet" aria-label={`Remove ${r.uploaded}`} onClick={() => {
                files.current.delete(r.id);
                removeDocument(r.id);
              }}>
              <Trash2 size={15} aria-hidden="true" />
            </Button>
          </span>
        ) : null
    }
  ];

  const editing = editingId ? docById(editingId) ?? null : null;
  const manual = unclassified[0];
  const previewFile = previewId ? files.current.get(previewId) : undefined;

  return (
    <div className="tax-stack">
      <Surface
        title={requestLabel ? `Upload Documents · ${requestLabel}` : 'Upload Documents'}
        subtitle={
          client
            ? `${requestLabel ? `Request ${requestLabel}` : 'Request'} for ${client.fullName}. AI reads each file and prefills the table below.`
            : 'AI reads each file and prefills the table below.'
        }
        density="compact"
      >
        <FileUpload
          label="Import Files"
          multiple
          accept="application/pdf,.pdf"
          hint="Drop PDF files here or browse. Only PDF files are accepted."
          onChange={(e) => {
            onFiles(e.target.files);
            e.target.value = '';
          }}
        />
        {rejected.length > 0 ? (
          <InlineAlert tone="warning" title="Some files were skipped" detail={`Only PDF files (.pdf) are allowed: ${rejected.join(', ')}`} />
        ) : null}
        {uploadErrors.map((message) => (
          <InlineAlert key={message} tone="danger" title="A file could not be uploaded" detail={message} />
        ))}
        {tried && documents.length === 0 ? (
          <InlineAlert tone="warning" title="Upload at least one document to submit" detail="Import a PDF above." />
        ) : null}
        {submitError ? <InlineAlert tone="danger" title="The request could not be submitted" detail={submitError} /> : null}
      </Surface>

      <Surface
        title={`Extracted Documents (${documents.length})`}
        subtitle="Review the AI-prefilled values. Use the pencil to correct a row before submitting."
        density="compact"
      >
        <DataGridShell<GridRow>
          ariaLabel="Extracted documents"
          columns={columns}
          rows={rows}
          rowKey="id"
          density="comfortable"
          emptyTitle="No documents yet"
          emptyDetail="Uploaded files appear here with the details AI extracted."
        />
      </Surface>

      <div className="tax-actions tax-spread">
        <Button variant="secondary" disabled={submitting} onClick={() => navigate('/new/client')}>
          <ArrowLeft size={16} aria-hidden="true" /> Back to Setup
        </Button>
        <div className="tax-inline">
          <Button
            variant="quiet"
            disabled={submitting}
            onClick={() => {
              // Cancelling removes the request and its temporary folder, with the files uploaded so far.
              void discard().then(() => navigate('/dashboard'));
            }}
          >
            Cancel
          </Button>
          <Button variant="primary" disabled={submitting || analyzing.length > 0} onClick={() => void submitAll()}>
            {submitting ? (
              <>
                <WorkingSpinner /> Submitting…
              </>
            ) : (
              <>
                <Send size={16} aria-hidden="true" /> Submit
              </>
            )}
          </Button>
        </div>
      </div>

      {previewFile ? <PdfPreviewDialog file={previewFile} onClose={() => setPreviewId(null)} /> : null}
      {editing ? (
        <EditDocumentDialog doc={editing} fileName={editing.fileName} onSave={(v) => saveEdit(editing.id, v)} onClose={() => setEditingId(null)} />
      ) : null}
      {!editing && manual ? (
        <EditDocumentDialog
          key={manual.id}
          doc={null}
          fileName={manual.fileName}
          unclassified
          onSave={(v) => classify(manual, v)}
          onClose={() => setUnclassified((list) => list.filter((u) => u.id !== manual.id))}
        />
      ) : null}
    </div>
  );
}
