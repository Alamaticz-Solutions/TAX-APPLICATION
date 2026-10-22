import { clientFor } from './routingRecords';

export type BulkDoc = {
  file_name: string;
  size: number;
  document_type: string;
  year: number | null;
  entity_name: string;
  entity_type: string;
  entity_number: string;
  password_protected: boolean;
  source: 'filename' | 'manual' | 'none';
  status: 'ready' | 'error';
  errors: string[];
};

export type BulkClient = {
  first_name: string;
  last_name: string;
  full_name: string;
  personal_email: string;
  pds_email: string;
  additional_email: string;
  office_location: string;
  internal_folder: string;
  client_folder: string;
  password_on_file: boolean;
  notify: boolean;
};

export type BulkItemStatus = 'ready' | 'error' | 'queued' | 'processing' | 'created' | 'failed' | 'skipped';

export type BulkItem = {
  client_key: string;
  client: BulkClient;
  source: 'existing' | 'new';
  differences: string[];
  errors: string[];
  docs: BulkDoc[];
  status: BulkItemStatus;
  record_id: string | null;
  case_number: number | null;
  message: string | null;
  record_status?: string | null;
  progress_step?: number | null;
};

export type BulkBatch = {
  id: string;
  file_name: string;
  status: 'checking' | 'check_failed' | 'staged' | 'processing' | 'completed' | 'completed_with_errors' | 'cancelled';
  /** Why the files could not be checked (status check_failed). */
  error: string | null;
  /** What the checking is doing now, and how far it has got. */
  progress_label: string;
  progress_done: number;
  progress_total: number;
  notes: string[];
  items: BulkItem[];
};

export type BulkClientValues = {
  firstName: string;
  lastName: string;
  fullName: string;
  personalEmail: string;
  pdsEmail: string;
  additionalEmail: string;
  officeLocation: string;
  internalFolder: string;
  clientFolder: string;
  /** Empty keeps the password the client already has. */
  password: string;
  notify: boolean;
};

export type BulkDocumentValues = {
  documentType: string;
  year: number;
  entityType: string;
  entityName: string;
  entityNumber: string;
  passwordProtected: boolean;
};

function toBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const text = String(reader.result ?? '');
      resolve(text.slice(text.indexOf(',') + 1));
    };
    reader.onerror = () => reject(new Error(`${file.name} could not be read.`));
    reader.readAsDataURL(file);
  });
}

async function call<T>(userName: string, operationName: string, query: string, variables: Record<string, unknown>, field: string): Promise<T> {
  const result = await clientFor(userName).graphql<Record<string, T>, Record<string, unknown>>({ schemaName: 'tax_routing', operationName, query, variables });
  return result.data[field];
}

/** Reads and checks the two files. Nothing is created until `confirmBulk`. */
export async function previewBulk(userName: string, excel: File, zip: File): Promise<BulkBatch> {
  const [excelBase64, zipBase64] = await Promise.all([toBase64(excel), toBase64(zip)]);
  return call<BulkBatch>(
    userName,
    'bulk_preview',
    `mutation($fileName: String!, $excelBase64: String!, $zipBase64: String!) { bulkPreview(fileName: $fileName, excelBase64: $excelBase64, zipBase64: $zipBase64) }`,
    { fileName: zip.name, excelBase64, zipBase64 },
    'bulkPreview'
  );
}

export function updateBulkClient(userName: string, batchId: string, clientKey: string, values: BulkClientValues): Promise<BulkBatch> {
  return call<BulkBatch>(
    userName,
    'bulk_update_client',
    `mutation($batchId: String!, $clientKey: String!, $firstName: String!, $lastName: String!, $fullName: String!, $personalEmail: String!, $pdsEmail: String!, $additionalEmail: String!, $officeLocation: String!, $internalFolder: String!, $clientFolder: String!, $password: String!, $notify: Boolean!) {
      bulkUpdateClient(batchId: $batchId, clientKey: $clientKey, firstName: $firstName, lastName: $lastName, fullName: $fullName, personalEmail: $personalEmail, pdsEmail: $pdsEmail, additionalEmail: $additionalEmail, officeLocation: $officeLocation, internalFolder: $internalFolder, clientFolder: $clientFolder, password: $password, notify: $notify)
    }`,
    { batchId, clientKey, ...values },
    'bulkUpdateClient'
  );
}

export function updateBulkDocument(userName: string, batchId: string, clientKey: string, fileName: string, values: BulkDocumentValues): Promise<BulkBatch> {
  return call<BulkBatch>(
    userName,
    'bulk_update_document',
    `mutation($batchId: String!, $clientKey: String!, $fileName: String!, $documentType: String!, $year: Int!, $entityKind: String!, $entityName: String!, $entityNumber: String!, $passwordProtected: Boolean!) {
      bulkUpdateDocument(batchId: $batchId, clientKey: $clientKey, fileName: $fileName, documentType: $documentType, year: $year, entityKind: $entityKind, entityName: $entityName, entityNumber: $entityNumber, passwordProtected: $passwordProtected)
    }`,
    { batchId, clientKey, fileName, documentType: values.documentType, year: values.year, entityKind: values.entityType, entityName: values.entityName, entityNumber: values.entityNumber, passwordProtected: values.passwordProtected },
    'bulkUpdateDocument'
  );
}

/** Reads one staged PDF so it can be previewed or downloaded before the requests are created. */
export async function getBulkDocument(userName: string, batchId: string, clientKey: string, fileName: string): Promise<File> {
  const doc = await call<{ file_name: string; content_base64: string }>(
    userName,
    'bulk_get_document',
    `mutation($batchId: String!, $clientKey: String!, $fileName: String!) { bulkGetDocument(batchId: $batchId, clientKey: $clientKey, fileName: $fileName) }`,
    { batchId, clientKey, fileName },
    'bulkGetDocument'
  );
  const bytes = Uint8Array.from(atob(doc.content_base64), (c) => c.charCodeAt(0));
  return new File([bytes], doc.file_name, { type: 'application/pdf' });
}

/** Drops one document, or the whole client when `fileName` is empty. */
export function removeFromBulk(userName: string, batchId: string, clientKey: string, fileName: string): Promise<BulkBatch> {
  return call<BulkBatch>(
    userName,
    'bulk_remove',
    `mutation($batchId: String!, $clientKey: String!, $fileName: String!) { bulkRemove(batchId: $batchId, clientKey: $clientKey, fileName: $fileName) }`,
    { batchId, clientKey, fileName },
    'bulkRemove'
  );
}

export function confirmBulk(userName: string, batchId: string): Promise<BulkBatch> {
  return call<BulkBatch>(userName, 'bulk_confirm', `mutation($batchId: String!) { bulkConfirm(batchId: $batchId) }`, { batchId }, 'bulkConfirm');
}

export function fetchBulkStatus(userName: string, batchId: string): Promise<BulkBatch> {
  return call<BulkBatch>(userName, 'bulk_status', `mutation($batchId: String!) { bulkStatus(batchId: $batchId) }`, { batchId }, 'bulkStatus');
}

/** Runs the clients that failed again. Nothing is created twice. */
export function retryBulkFailed(userName: string, batchId: string): Promise<BulkBatch> {
  return call<BulkBatch>(userName, 'bulk_retry_failed', `mutation($batchId: String!) { bulkRetryFailed(batchId: $batchId) }`, { batchId }, 'bulkRetryFailed');
}

export async function cancelBulk(userName: string, batchId: string): Promise<void> {
  await call<unknown>(userName, 'bulk_cancel', `mutation($batchId: String!) { bulkCancel(batchId: $batchId) }`, { batchId }, 'bulkCancel');
}
