// Domain types for the Tax Document Routing frontend. Kept as `type` aliases (not interfaces)
// so row objects are assignable to the PDS DataGrid's Record<string, unknown> constraint.

export type Role = 'standard' | 'admin';
export type Tone = 'neutral' | 'accent' | 'success' | 'danger' | 'warning';

export type AppUser = {
  id: string;
  name: string;
  initials: string;
  role: Role;
  roleLabel: string;
};

export type RecordStatus =
  | 'Open'
  | 'In Progress'
  | 'Processing'
  | 'Exception'
  | 'Resolved - Completed'
  | 'Resolved - Cancelled';

export type EntityType = 'Partnership' | 'S-Corp' | 'Personal Corporation';

export type Client = {
  id: string;
  /** TaxPayerRefData's own primary key — distinct from `id` (the CR-xxxxxx Box reference id
   * shown in the UI). This is what a TaxRoutingRecord's `client_reference_id` FK points at. */
  refId: string;
  firstName: string;
  lastName: string;
  fullName: string;
  officeLocation: string;
  pdsEmail: string;
  personalEmail: string;
  internalFolder: string;
  folderName: string;
  passwordOnFile: boolean;
  active: boolean;
  /** The Box read/write password, snapshotted onto a TaxRoutingRecord when a request is created.
   * Never rendered in the UI (ClientInfoStep shows a masked placeholder). */
  readWritePassword: string;
};

export type Entity = { number: string; name: string; type: EntityType };
export type DocEntity = { type: EntityType; name: string; number: string };

export type DocumentTypeName =
  | 'K-1'
  | 'Draft K-1'
  | 'Tax Return'
  | 'Income Statement'
  | 'Balance Sheet'
  | 'S-Corp Election'
  | 'S-Corp'
  | 'Form 8308'
  | 'Extensions';

export type DocumentTypeConfig = {
  type: DocumentTypeName;
  businessName: string;
  allowsEntity: boolean;
  k1Attachments: boolean;
  form8308Attachments: boolean;
  supportsPasswordProtection: boolean;
  externalOnly: boolean;
  externalFolder: string;
  internalFolder: string | null;
  note: string;
};

export type DocumentFileStatus = 'Ready' | 'Uploaded' | 'Filed' | 'Failed';

export type TaxDocument = {
  id: string;
  type: DocumentTypeName;
  year: number;
  name: string;
  fileName: string;
  fileSize?: number;
  entity: DocEntity | null;
  passwordProtected: boolean;
  attachments: Record<string, boolean>;
  status: DocumentFileStatus;
  isDuplicate?: boolean;
  /** Set when the AI assistant prefilled this row; cleared conceptually once the user edits it. */
  aiConfidence?: number;
};

export type ExceptionInfo = { id: string; error: string; document: string; age: string; detail: string };
export type ActivityEntry = { at: string; text: string; by: string };

/** Snapshot of the client's reference data as it was when the record was created (business spec
 * 5.4/9.1): TaxRoutingRecord denormalises these columns so a later change to TaxPayerRefData
 * cannot silently rewrite where an already-routed document was filed. */
export type RoutingRecordClient = {
  id: string;
  firstName: string;
  lastName: string;
  fullName: string;
  officeLocation: string;
  pdsEmail: string;
  personalEmail: string;
  internalFolder: string;
  folderName: string;
  /** Carried so `updateRoutingRecordProgress` can resend the required column unchanged; never rendered. */
  readWritePassword: string;
};

export type RoutingCase = {
  id: string;
  client: RoutingRecordClient;
  status: RecordStatus;
  task: string;
  assignedTo: string;
  /** Who created the request (the operator who submitted it). */
  createdBy: string;
  created: string;
  updated: string;
  documents: TaxDocument[];
  exception: ExceptionInfo | null;
  /** Number of completed processing steps (0..9). */
  progress: number;
  notifyClient: boolean;
  additionalEmail?: string | null;
  notificationSent: boolean;
  resolutionComment: string | null;
  activity: ActivityEntry[];
  /** Concurrency token for the next update mutation. */
  version: number;
};

export type RoutingDraft = {
  client: Client | null;
  additionalEmail: string;
  notifyClient: boolean;
  documents: TaxDocument[];
  recordId: string | null;
};

export type ProcessingStepState = 'done' | 'current' | 'failed' | 'pending' | 'skipped';
export type DestinationKey = 'quickView' | 'external' | 'internal';
export type Destination = { key: DestinationKey; label: string; folder: string | null; encrypted: boolean };

export type AiSuggestion = {
  documentType: DocumentTypeName | null;
  year: number;
  entity: Entity | null;
  confidence: number;
};
