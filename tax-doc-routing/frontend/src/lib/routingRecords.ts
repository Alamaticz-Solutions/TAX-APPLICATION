import { createAppfwClient } from './appfwClient';
import type { ExceptionInfo, RecordStatus, RoutingCase, RoutingRecordClient } from '../features/shared/types';

// Routing records and exceptions (business spec 4/5/5.3), read/written live from the database
// instead of frontend/src/features/shared/data/cases.ts's hardcoded `seedCases`. Same local-auth
// mechanism as lib/formEngine.ts / lib/clientDirectory.ts. Document/file-content persistence is
// explicitly out of scope for now (pending SharePoint integration) — `documents` on a RoutingCase
// only ever comes from this session's in-memory draft, never from a query below.

function localAuthHeader(userName: string, tenantId = 't1'): string {
  return `Bearer appfw-local:user=${userName};tenant=${tenantId};roles=n/a`;
}

function client(userName: string) {
  return createAppfwClient({ authorization: localAuthHeader(userName), tenantId: 't1' });
}

// RoutingRecordStatus (DB) <-> RecordStatus (frontend label). A bijective mapping so a status
// read from the database and one about to be written back always round-trip losslessly. The
// GraphQL enum serializes each snake_case model value (collecting_info, cleaning_up, ...) as
// PascalCase on the wire (confirmed live: CollectingInfo, CleaningUp, Processing, Exception,
// Resolved, Cancelled) — this must match that wire format exactly, not the model's own spelling.
const STATUS_TO_DB: Record<RecordStatus, string> = {
  Open: 'CollectingInfo',
  'In Progress': 'CleaningUp',
  Processing: 'Processing',
  Exception: 'Exception',
  'Resolved - Completed': 'Resolved',
  'Resolved - Cancelled': 'Cancelled'
};
const STATUS_FROM_DB: Record<string, RecordStatus> = Object.fromEntries(Object.entries(STATUS_TO_DB).map(([k, v]) => [v, k as RecordStatus]));

const TASK_LABEL: Record<RecordStatus, string> = {
  Open: 'Document Collection',
  'In Progress': 'Confirmation',
  Processing: 'Processing',
  Exception: 'Resolve Exception',
  'Resolved - Completed': 'Closed',
  'Resolved - Cancelled': 'Closed'
};

type RawTaxRoutingRecord = {
  id: string;
  status: string | null;
  created_at: string | null;
  updated_at: string | null;
  assigned_user_id: string | null;
  client_first_name: string | null;
  client_last_name: string | null;
  client_full_name: string | null;
  office_location: string | null;
  pds_email: string | null;
  personal_email: string | null;
  additional_email: string | null;
  notification_flag: boolean | null;
  internal_folder: string | null;
  folder_name: string | null;
  read_write_password: string | null;
  client_reference_id: string | null;
  progress_step: number | null;
  version: number | null;
};

/** Derives the fixed activity trail from a record's own real fields (no free-text log is
 * persisted yet — see docs/architecture/rbac-design.md's open-items list). */
function deriveActivity(row: RawTaxRoutingRecord, status: RecordStatus, exception: ExceptionInfo | null): { at: string; text: string; by: string }[] {
  const created = row.created_at ?? '';
  const updated = row.updated_at ?? '';
  const by = row.assigned_user_id ?? 'system';
  const list = [
    { at: created, text: 'Record created', by },
    { at: created, text: `Client selected: ${row.client_full_name ?? ''}`, by }
  ];
  if (status === 'Processing' || status === 'Exception' || status === 'Resolved - Completed' || status === 'In Progress') {
    list.push({ at: updated, text: 'Submitted for processing', by: 'system' });
  }
  if (exception) list.push({ at: updated, text: `Exception: ${exception.error}`, by: 'system' });
  if (status === 'Resolved - Completed') {
    list.push({ at: updated, text: 'Documents filed and client notified', by: 'system' }, { at: updated, text: 'Record resolved - completed', by: 'system' });
  }
  if (status === 'Resolved - Cancelled') list.push({ at: updated, text: 'Record cancelled', by: 'system' });
  return list;
}

function toCase(row: RawTaxRoutingRecord, exception: ExceptionInfo | null): RoutingCase {
  const status = STATUS_FROM_DB[row.status ?? ''] ?? 'Open';
  const clientSnapshot: RoutingRecordClient = {
    id: row.client_reference_id ?? '',
    firstName: row.client_first_name ?? '',
    lastName: row.client_last_name ?? '',
    fullName: row.client_full_name ?? '',
    officeLocation: row.office_location ?? '',
    pdsEmail: row.pds_email ?? '',
    personalEmail: row.personal_email ?? '',
    internalFolder: row.internal_folder ?? '',
    folderName: row.folder_name ?? '',
    readWritePassword: row.read_write_password ?? ''
  };
  return {
    id: row.id,
    client: clientSnapshot,
    status,
    task: TASK_LABEL[status],
    assignedTo: row.assigned_user_id ?? '',
    createdBy: row.assigned_user_id ?? '',
    created: row.created_at ?? '',
    updated: row.updated_at ?? '',
    documents: [],
    exception,
    progress: row.progress_step ?? 0,
    notifyClient: row.notification_flag ?? false,
    additionalEmail: row.additional_email ?? null,
    notificationSent: status === 'Resolved - Completed' && Boolean(row.notification_flag),
    resolutionComment: null,
    activity: deriveActivity(row, status, exception),
    version: row.version ?? 0
  };
}

function recordFields(): string {
  return `
    id status created_at updated_at assigned_user_id
    client_first_name client_last_name client_full_name office_location pds_email personal_email
    additional_email notification_flag internal_folder folder_name read_write_password
    client_reference_id progress_step version
  `;
}

type RawExceptionTask = {
  id: string;
  routing_record_id: string | null;
  failure_reason: string | null;
  failed_step: string | null;
  opened_at: string | null;
  resolved_at: string | null;
};

function ageFrom(openedAt: string | null): string {
  if (!openedAt) return '';
  const ms = Date.now() - new Date(openedAt).getTime();
  const mins = Math.max(0, Math.round(ms / 60000));
  if (mins < 60) return `${mins}m`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.round(hours / 24)}d`;
}

/** Open (unresolved) exceptions, keyed by their parent routing_record_id, for joining onto the
 * matching records client-side (business spec 5.3: the exception queue is one shared list). */
async function fetchOpenExceptions(userName: string): Promise<Map<string, ExceptionInfo>> {
  // No "is null" filter operator exists on the query surface; the exception queue is a small,
  // shared list (business spec 5.3), so filtering resolved_at client-side is cheap and correct.
  const query = `{
    queryExceptionTasks(limit: 200) {
      items { id routing_record_id failure_reason failed_step opened_at resolved_at }
    }
  }`;
  const result = await client(userName).graphql<{ queryExceptionTasks: { items: RawExceptionTask[] } }, Record<string, never>>({
    schemaName: 'tax_routing',
    operationName: 'query_exception_tasks',
    query,
    variables: {}
  });
  const map = new Map<string, ExceptionInfo>();
  for (const row of result.data.queryExceptionTasks.items) {
    if (!row.routing_record_id || row.resolved_at) continue;
    map.set(row.routing_record_id, {
      id: row.id,
      error: row.failure_reason ?? 'Exception',
      document: row.failed_step ?? '—',
      age: ageFrom(row.opened_at),
      detail: row.failure_reason ?? ''
    });
  }
  return map;
}

async function queryRecords(userName: string, filter: unknown): Promise<RoutingCase[]> {
  const query = `query($filter: JSON) {
    queryTaxRoutingRecords(filter: $filter, sort: { updated_at: "desc" }, limit: 200) {
      items { ${recordFields()} }
    }
  }`;
  const [result, exceptions] = await Promise.all([
    client(userName).graphql<{ queryTaxRoutingRecords: { items: RawTaxRoutingRecord[] } }, { filter: unknown }>({
      schemaName: 'tax_routing',
      operationName: 'query_tax_routing_records',
      query,
      variables: { filter }
    }),
    fetchOpenExceptions(userName)
  ]);
  return result.data.queryTaxRoutingRecords.items.map((row) => toCase(row, exceptions.get(row.id) ?? null));
}

/** Every record assigned to this user (My Cases / Work I Need To Do / Dashboard for a standard user). */
export async function fetchMyRecords(userName: string, assignedUserId: string): Promise<RoutingCase[]> {
  return queryRecords(userName, { assigned_user_id: { _eq: assignedUserId } });
}

/** Every record, admin-only (All Cases / admin Dashboard, business spec 12.3). */
export async function fetchAllRecords(userName: string): Promise<RoutingCase[]> {
  return queryRecords(userName, {});
}

/**
 * Every open exception (business spec 5.3: "every staff member sees every exception" — granted
 * as `exception_task.read` scope "all" for both roles). A standard user's own `routing_record.read`
 * grant is scope "own", so Rego will not return another operator's routing record even though
 * they can see that the exception exists; for those, this returns a minimal stand-in case with
 * the exception's own fields and blank client details rather than fabricating a name from
 * nothing. `knownCases` (whatever the caller already loaded — own records, or every record for an
 * admin) is used to fill in full detail wherever access allows it. */
export async function fetchExceptionQueue(userName: string, knownCases: RoutingCase[]): Promise<RoutingCase[]> {
  const byId = new Map(knownCases.map((c) => [c.id, c]));
  const exceptions = await fetchOpenExceptions(userName);
  const result: RoutingCase[] = [];
  for (const [recordId, info] of exceptions) {
    const known = byId.get(recordId);
    if (known) {
      result.push({ ...known, exception: info, status: 'Exception', task: TASK_LABEL.Exception });
      continue;
    }
    result.push({
      id: recordId,
      client: { id: '', firstName: '', lastName: '', fullName: 'Restricted', officeLocation: '', pdsEmail: '', personalEmail: '', internalFolder: '', folderName: '', readWritePassword: '' },
      status: 'Exception',
      task: TASK_LABEL.Exception,
      assignedTo: '',
      createdBy: '',
      created: '',
      updated: '',
      documents: [],
      exception: info,
      progress: 0,
      notifyClient: false,
      additionalEmail: null,
      notificationSent: false,
      resolutionComment: null,
      activity: [],
      version: 0
    });
  }
  return result;
}

export type CreateRoutingRecordInput = {
  assignedUserId: string;
  client: { refId: string; firstName: string; lastName: string; fullName: string; officeLocation: string; pdsEmail: string; personalEmail: string; internalFolder: string; folderName: string; readWritePassword: string };
  additionalEmail: string | null;
  notifyClient: boolean;
};

const CREATE_RECORD_MUTATION = `mutation($input: InputTaxRoutingRecord!) {
  createTaxRoutingRecord(input: $input) { ${recordFields()} }
}`;

/** Creates the routing record row (business spec 4/9.1) — the one write that must happen before
 * a new request can appear in My Cases. Starts life in `processing`/step 0, matching the previous
 * mock's `createRecord` (documents are already "uploaded" by the time this is called). */
export async function createRoutingRecord(userName: string, input: CreateRoutingRecordInput): Promise<RoutingCase> {
  const now = new Date().toISOString();
  const graphqlInput = {
    id: null,
    status: STATUS_TO_DB.Processing,
    created_at: now,
    updated_at: now,
    assigned_user_id: input.assignedUserId,
    client_first_name: input.client.firstName,
    client_last_name: input.client.lastName,
    client_full_name: input.client.fullName,
    office_location: input.client.officeLocation,
    pds_email: input.client.pdsEmail,
    personal_email: input.client.personalEmail,
    additional_email: input.additionalEmail,
    notification_flag: input.notifyClient,
    internal_folder: input.client.internalFolder,
    folder_name: input.client.folderName,
    read_write_password: input.client.readWritePassword,
    client_reference_id: input.client.refId,
    progress_step: 0,
    version: null
  };
  const result = await client(userName).graphql<{ createTaxRoutingRecord: RawTaxRoutingRecord }, { input: unknown }>({
    schemaName: 'tax_routing',
    operationName: 'create_tax_routing_record',
    query: CREATE_RECORD_MUTATION,
    variables: { input: graphqlInput }
  });
  return toCase(result.data.createTaxRoutingRecord, null);
}

const UPDATE_RECORD_MUTATION = `mutation($input: InputTaxRoutingRecord!) {
  updateTaxRoutingRecord(input: $input) { ${recordFields()} }
}`;

/** Persists a status/progress advance from the client-side processing animation. Carries every
 * other field forward unchanged, same "read the whole row, replace it" pattern the backend's
 * cancel_record/reassign_record custom mutations use for the columns they don't touch. */
export async function updateRoutingRecordProgress(userName: string, record: RoutingCase, patch: { status?: RecordStatus; progress?: number }): Promise<RoutingCase> {
  const nextStatus = patch.status ?? record.status;
  const graphqlInput = {
    id: record.id,
    status: STATUS_TO_DB[nextStatus],
    created_at: record.created,
    updated_at: new Date().toISOString(),
    assigned_user_id: record.assignedTo,
    client_first_name: record.client.firstName,
    client_last_name: record.client.lastName,
    client_full_name: record.client.fullName,
    office_location: record.client.officeLocation,
    pds_email: record.client.pdsEmail,
    personal_email: record.client.personalEmail,
    additional_email: record.additionalEmail ?? null,
    notification_flag: record.notifyClient,
    internal_folder: record.client.internalFolder,
    folder_name: record.client.folderName,
    read_write_password: record.client.readWritePassword,
    client_reference_id: record.client.id,
    progress_step: patch.progress ?? record.progress,
    version: record.version
  };
  const result = await client(userName).graphql<{ updateTaxRoutingRecord: RawTaxRoutingRecord }, { input: unknown }>({
    schemaName: 'tax_routing',
    operationName: 'update_tax_routing_record',
    query: UPDATE_RECORD_MUTATION,
    variables: { input: graphqlInput }
  });
  return toCase(result.data.updateTaxRoutingRecord, record.exception);
}

const CANCEL_RECORD_MUTATION = `mutation($recordId: String!, $resolutionStatus: String!, $comments: String!) {
  cancelRecord(recordId: $recordId, resolutionStatus: $resolutionStatus, comments: $comments)
}`;

/** Admin-only permanent close (business spec 9.1); delegates to the backend's `cancel_record`
 * custom mutation, which is the only place allowed to set status to cancelled. */
export async function cancelRoutingRecord(userName: string, recordId: string, resolutionStatus: string, comments: string): Promise<void> {
  await client(userName).graphql<{ cancelRecord: unknown }, { recordId: string; resolutionStatus: string; comments: string }>({
    schemaName: 'tax_routing',
    operationName: 'cancel_record',
    query: CANCEL_RECORD_MUTATION,
    variables: { recordId, resolutionStatus, comments }
  });
}

const REASSIGN_RECORD_MUTATION = `mutation($recordId: String!, $newAssignedUserId: String!) {
  reassignRecord(recordId: $recordId, newAssignedUserId: $newAssignedUserId)
}`;

/** Admin-only reassignment (business spec 12.3); delegates to the backend's `reassign_record`
 * custom mutation. */
export async function reassignRoutingRecord(userName: string, recordId: string, newAssignedUserId: string): Promise<void> {
  await client(userName).graphql<{ reassignRecord: unknown }, { recordId: string; newAssignedUserId: string }>({
    schemaName: 'tax_routing',
    operationName: 'reassign_record',
    query: REASSIGN_RECORD_MUTATION,
    variables: { recordId, newAssignedUserId }
  });
}

const RETRY_FAILED_STEP_MUTATION = `mutation($exceptionTaskId: String!) {
  retryFailedStep(exceptionTaskId: $exceptionTaskId)
}`;

/** Resolves the exception and resumes processing (business spec 5.3), via the backend's
 * `retry_failed_step` custom mutation. */
export async function retryFailedStep(userName: string, exceptionTaskId: string): Promise<void> {
  await client(userName).graphql<{ retryFailedStep: unknown }, { exceptionTaskId: string }>({
    schemaName: 'tax_routing',
    operationName: 'retry_failed_step',
    query: RETRY_FAILED_STEP_MUTATION,
    variables: { exceptionTaskId }
  });
}

type RawAppUser = { id: string; display_name: string | null; is_active: boolean | null };

const STAFF_QUERY = `{
  queryAppUsers(filter: { is_active: { _eq: true } }, limit: 100) {
    items { id display_name is_active }
  }
}`;

/** The active operator directory (business spec 12.3's Assigned To / reassignment lists),
 * replacing frontend/src/features/shared/data/users.ts's hardcoded `staffDirectory`. */
export async function fetchStaffDirectory(userName: string): Promise<{ id: string; name: string }[]> {
  const result = await client(userName).graphql<{ queryAppUsers: { items: RawAppUser[] } }, Record<string, never>>({
    schemaName: 'tax_routing',
    operationName: 'query_app_users',
    query: STAFF_QUERY,
    variables: {}
  });
  return result.data.queryAppUsers.items
    .filter((u) => u.display_name && u.display_name !== 'Local Admin')
    .map((u) => ({ id: u.id, name: u.display_name ?? u.id }));
}
