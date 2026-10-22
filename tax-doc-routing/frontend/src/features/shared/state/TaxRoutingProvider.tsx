import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useSession } from '../../../app/SessionProvider';
import { fetchMyPermissions, hasPermission as grantsHavePermission, hasScope as grantsHaveScope, type Grant } from '../../../lib/permissions';
import {
  cancelRoutingRecord,
  createRoutingRecord,
  discardRequest,
  fetchAllRecords,
  fetchAppUserName,
  fetchExceptionQueue,
  fetchMyRecords,
  fetchRecordById,
  reassignRoutingRecord,
  retryFailedStep,
  submitRequest,
  uploadDocumentFile,
  type UploadedFile
} from '../../../lib/routingRecords';
import { STATUS, TERMINAL_STATUSES } from '../config/statuses';
import type { AppUser, Client, RoutingCase, RoutingDraft, Role, TaxDocument } from '../types';
import { delay } from '../services/delay';

// Routing records, their documents and exceptions are read from and written to the real
// `TaxRoutingRecord` / `TaxDocument` / `ExceptionTask` tables (lib/routingRecords.ts) — see
// docs/architecture/rbac-design.md. A request is created as soon as the user leaves screen 1 (so it
// has its TD number and its temporary storage folder before anything is uploaded); uploads go to
// that folder; Submit hands the documents to the server, which files them and reports progress on
// the record. The browser only follows that progress (`watchCase`), so processing continues
// whether or not this tab stays open. Permissions are database-driven grants (lib/permissions.ts).

function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return '?';
  return (parts.length === 1 ? parts[0].slice(0, 2) : parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

const emptyDraft = (): RoutingDraft => ({ client: null, additionalEmail: '', notifyClient: true, documents: [], recordId: null, submitted: false });

// How often, and for how long, a request being filed is re-read.
const POLL_MS = 1500;
const POLL_LIMIT_MS = 10 * 60 * 1000;
// A request still shown as Processing is followed after a reload only if it moved recently, so old
// records that were never finished do not keep the browser polling.
const RECENT_MS = 15 * 60 * 1000;

export type DraftApi = {
  setClient: (client: Client | null) => void;
  setAdditionalEmail: (value: string) => void;
  setNotifyClient: (value: boolean) => void;
  addDocument: (doc: TaxDocument) => void;
  updateDocument: (id: string, patch: Partial<TaxDocument>) => void;
  removeDocument: (id: string) => void;
  reset: () => void;
};

export type TaxRoutingContextValue = {
  /** Derived from the signed-in user's real grants, never chosen in the UI. */
  role: Role;
  /** The signed-in user name (null when signed out). */
  userName: string | null;
  user: AppUser;
  cases: RoutingCase[];
  casesStatus: 'loading' | 'ready' | 'error';
  visibleCases: RoutingCase[];
  exceptionQueue: RoutingCase[];
  getCase: (id: string) => RoutingCase | null;
  canActOn: (c: RoutingCase) => boolean;
  canCancel: (c: RoutingCase) => boolean;
  draft: RoutingDraft;
  draftApi: DraftApi;
  /** Creates the request (number + temporary folder) when the user leaves screen 1; reuses it afterwards. */
  createDraftRecord: () => Promise<RoutingCase>;
  /** Stores one PDF in the request's temporary folder. */
  uploadFile: (file: File) => Promise<UploadedFile>;
  /** Saves the documents and starts filing on the server. */
  submitDraft: () => Promise<RoutingCase>;
  /** Abandons a request that was never submitted, removing its temporary folder. */
  discardDraft: () => Promise<void>;
  retryException: (id: string) => void;
  cancelCase: (id: string, resolution: string, comments: string) => void;
  reassignCase: (id: string, newAssignedUserId: string) => Promise<void>;
  /** Live, database-driven grants for the current role's backend identity. */
  grants: Grant[];
  permissionsStatus: 'loading' | 'ready' | 'error';
  hasPermission: (code: string) => boolean;
  hasScope: (code: string, scope: string) => boolean;
};

const TaxRoutingContext = createContext<TaxRoutingContextValue | null>(null);

export function TaxRoutingProvider({ children }: { children: ReactNode }) {
  const { userName } = useSession();
  const [role, setRole] = useState<Role>('standard');
  const [displayName, setDisplayName] = useState<string | null>(null);
  const [cases, setCases] = useState<RoutingCase[]>([]);
  const [casesStatus, setCasesStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [exceptionQueue, setExceptionQueue] = useState<RoutingCase[]>([]);
  const [draft, setDraft] = useState<RoutingDraft>(emptyDraft);

  const casesRef = useRef(cases);
  useEffect(() => {
    casesRef.current = cases;
  }, [cases]);
  const watching = useRef(new Set<string>());

  const [grants, setGrants] = useState<Grant[]>([]);
  const [userId, setUserId] = useState<string | null>(null);
  const [permissionsStatus, setPermissionsStatus] = useState<'loading' | 'ready' | 'error'>('loading');

  // The real AppUser.id for the role's backend identity — what `assigned_user_id` on a
  // TaxRoutingRecord actually stores, replacing the placeholder `u-alex` / `u-dana` ids that used
  // to live in features/shared/data/users.ts.
  const user: AppUser = useMemo(() => {
    const name = displayName ?? userName ?? '';
    return { id: userId ?? '', name, initials: initialsOf(name), role, roleLabel: role === 'admin' ? 'Tax Admin' : 'Tax Staff' };
  }, [displayName, userName, userId, role]);

  useEffect(() => {
    let cancelled = false;
    setPermissionsStatus('loading');
    setUserId(null);
    setGrants([]);
    setDisplayName(null);
    setRole('standard');
    if (!userName) {
      // Signed out: drop everything the previous user could see.
      setCases([]);
      setExceptionQueue([]);
      setCasesStatus('loading');
      setDraft(emptyDraft());
      return undefined;
    }
    fetchMyPermissions(userName)
      .then((principal) => {
        if (cancelled) return;
        setGrants(principal.grants);
        setUserId(principal.user_id);
        // Role is what the database says this user can do (All Cases is the admin-only grant).
        setRole(grantsHavePermission(principal.grants, 'routing_record.view_assignee') ? 'admin' : 'standard');
        setPermissionsStatus(principal.user_id ? 'ready' : 'error');
        if (principal.user_id) {
          fetchAppUserName(userName, principal.user_id)
            .then((name) => {
              if (!cancelled) setDisplayName(name);
            })
            .catch(() => undefined);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setGrants([]);
          setPermissionsStatus('error');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [userName]);

  const refreshCases = useCallback(async () => {
    if (!userId || !userName) return;
    setCasesStatus('loading');
    try {
      const loaded = role === 'admin' ? await fetchAllRecords(userName) : await fetchMyRecords(userName, userId);
      setCases(loaded);
      setCasesStatus('ready');
      const queue = await fetchExceptionQueue(userName, loaded);
      setExceptionQueue(queue);
    } catch {
      setCasesStatus('error');
    }
  }, [role, userId, userName]);

  useEffect(() => {
    void refreshCases();
  }, [refreshCases]);

  const hasPermission = useCallback((code: string) => grantsHavePermission(grants, code), [grants]);
  const hasScope = useCallback((code: string, scope: string) => grantsHaveScope(grants, code, scope), [grants]);

  const updateCaseInPlace = useCallback((updated: RoutingCase) => {
    casesRef.current = casesRef.current.map((c) => (c.id === updated.id ? updated : c));
    setCases((list) => list.map((c) => (c.id === updated.id ? updated : c)));
  }, []);

  // Standard users see only records assigned to them; admins see everything (business spec BR-19/20).
  // `cases` is already scoped by `refreshCases` (fetchMyRecords vs fetchAllRecords + Rego's own row
  // filter), so this is a defensive pass-through, not the primary enforcement.
  const visibleCases = useMemo(() => (role === 'admin' ? cases : cases.filter((c) => c.assignedTo === user.id)), [cases, role, user.id]);
  const getCase = useCallback(
    (id: string) => cases.find((c) => c.id === id) ?? exceptionQueue.find((c) => c.id === id) ?? null,
    [cases, exceptionQueue]
  );
  const canActOn = useCallback((c: RoutingCase) => role === 'admin' || c.assignedTo === user.id, [role, user.id]);
  // Cancel/Withdraw visibility is driven by the real `routing_record.cancel` grant fetched from
  // the backend, not a role literal (business spec 9.1; docs/architecture/rbac-design.md).
  const canCancel = useCallback(
    (c: RoutingCase) => hasPermission('routing_record.cancel') && !TERMINAL_STATUSES.includes(c.status),
    [hasPermission]
  );

  const draftApi = useMemo<DraftApi>(
    () => ({
      setClient: (client) => setDraft((d) => ({ ...d, client })),
      setAdditionalEmail: (additionalEmail) => setDraft((d) => ({ ...d, additionalEmail })),
      setNotifyClient: (notifyClient) => setDraft((d) => ({ ...d, notifyClient })),
      addDocument: (doc) => setDraft((d) => ({ ...d, documents: [...d.documents, doc] })),
      updateDocument: (id, patch) => setDraft((d) => ({ ...d, documents: d.documents.map((x) => (x.id === id ? { ...x, ...patch } : x)) })),
      removeDocument: (id) => setDraft((d) => ({ ...d, documents: d.documents.filter((x) => x.id !== id) })),
      reset: () => setDraft(emptyDraft())
    }),
    []
  );

  // Follows a request the server is filing: re-reads it until it stops processing (done, failed or
  // cancelled). The work itself runs on the server, so closing the tab does not stop it.
  const watchCase = useCallback(
    (id: string) => {
      if (!userName || watching.current.has(id)) return;
      watching.current.add(id);
      void (async () => {
        const started = Date.now();
        try {
          while (watching.current.has(id) && Date.now() - started < POLL_LIMIT_MS) {
            await delay(POLL_MS);
            if (!watching.current.has(id)) return;
            const fresh = await fetchRecordById(userName, id);
            if (!fresh) return;
            updateCaseInPlace(fresh);
            if (fresh.status !== STATUS.PROCESSING) {
              if (fresh.status === STATUS.EXCEPTION) setExceptionQueue(await fetchExceptionQueue(userName, casesRef.current));
              return;
            }
          }
        } catch (err) {
          console.error('Could not follow the request', err);
        } finally {
          watching.current.delete(id);
        }
      })();
    },
    [userName, updateCaseInPlace]
  );

  // After a reload, pick up requests that are still being filed.
  useEffect(() => {
    for (const c of cases) {
      if (c.status === STATUS.PROCESSING && c.updated && Date.now() - new Date(c.updated).getTime() < RECENT_MS) watchCase(c.id);
    }
  }, [cases, watchCase]);
  useEffect(() => {
    const active = watching.current;
    return () => active.clear();
  }, [userName]);

  const createDraftRecord = useCallback(async () => {
    if (!draft.client) throw new Error('A client is required to create a request.');
    if (!userId) throw new Error('Your identity has not resolved yet — try again in a moment.');
    const existing = draft.recordId ? casesRef.current.find((c) => c.id === draft.recordId) : undefined;
    if (existing) return existing;
    const created = await createRoutingRecord(userName ?? '', {
      assignedUserId: userId,
      client: {
        refId: draft.client.refId,
        firstName: draft.client.firstName,
        lastName: draft.client.lastName,
        fullName: draft.client.fullName,
        officeLocation: draft.client.officeLocation,
        pdsEmail: draft.client.pdsEmail,
        personalEmail: draft.client.personalEmail,
        internalFolder: draft.client.internalFolder,
        folderName: draft.client.folderName,
        readWritePassword: draft.client.readWritePassword
      },
      additionalEmail: draft.additionalEmail || null,
      notifyClient: draft.notifyClient
    });
    casesRef.current = [created, ...casesRef.current];
    setCases((list) => [created, ...list]);
    setDraft((d) => ({ ...d, recordId: created.id }));
    return created;
  }, [draft.client, draft.recordId, draft.additionalEmail, draft.notifyClient, userName, userId]);

  const uploadFile = useCallback(
    async (file: File) => {
      if (!draft.recordId) throw new Error('Create the request before uploading documents.');
      return uploadDocumentFile(userName ?? '', draft.recordId, file);
    },
    [draft.recordId, userName]
  );

  const submitDraft = useCallback(async () => {
    const recordId = draft.recordId;
    if (!recordId) throw new Error('The request has not been created yet.');
    await submitRequest(userName ?? '', recordId, draft.documents);
    const current = casesRef.current.find((c) => c.id === recordId);
    if (!current) throw new Error('The request could not be found.');
    const record: RoutingCase = {
      ...current,
      status: STATUS.PROCESSING,
      task: 'Processing',
      progress: Math.max(current.progress, 3),
      documents: draft.documents.map((d) => ({ ...d, status: 'Uploaded' as const }))
    };
    updateCaseInPlace(record);
    setDraft((d) => ({ ...d, submitted: true }));
    watchCase(recordId);
    return record;
  }, [draft.recordId, draft.documents, userName, updateCaseInPlace, watchCase]);

  const discardDraft = useCallback(async () => {
    const recordId = draft.recordId;
    if (recordId && !draft.submitted) {
      try {
        await discardRequest(userName ?? '', recordId);
        casesRef.current = casesRef.current.filter((c) => c.id !== recordId);
        setCases((list) => list.filter((c) => c.id !== recordId));
      } catch (err) {
        // The request stays open in My Cases (and its folder in storage) if this fails.
        console.error('Could not discard the request', err);
      }
    }
    setDraft(emptyDraft());
  }, [draft.recordId, draft.submitted, userName]);

  const retryException = useCallback(
    (id: string) => {
      const current = casesRef.current.find((c) => c.id === id) ?? exceptionQueue.find((c) => c.id === id);
      if (!current?.exception) return;
      void retryFailedStep(userName ?? '', current.exception.id).then(() => {
        const resumed: RoutingCase = { ...current, status: STATUS.PROCESSING, task: 'Processing', exception: null };
        updateCaseInPlace(resumed);
        setExceptionQueue((list) => list.filter((c) => c.id !== id));
        watchCase(id);
      });
    },
    [userName, exceptionQueue, updateCaseInPlace, watchCase]
  );

  const cancelCase = useCallback(
    (id: string, resolution: string, comments: string) => {
      void cancelRoutingRecord(userName ?? '', id, resolution, comments).then(() => {
        const current = casesRef.current.find((c) => c.id === id);
        if (!current) return;
        updateCaseInPlace({ ...current, status: STATUS.CANCELLED, task: 'Closed', exception: null, resolutionComment: comments });
      });
    },
    [userName, updateCaseInPlace]
  );

  const reassignCase = useCallback(
    async (id: string, newAssignedUserId: string) => {
      await reassignRoutingRecord(userName ?? '', id, newAssignedUserId);
      const current = casesRef.current.find((c) => c.id === id);
      if (current) updateCaseInPlace({ ...current, assignedTo: newAssignedUserId });
    },
    [userName, updateCaseInPlace]
  );

  const value: TaxRoutingContextValue = {
    role, userName, user,
    cases, casesStatus, visibleCases, exceptionQueue, getCase,
    canActOn, canCancel,
    draft, draftApi,
    createDraftRecord, uploadFile, submitDraft, discardDraft, retryException, cancelCase, reassignCase,
    grants, permissionsStatus, hasPermission, hasScope
  };
  return <TaxRoutingContext.Provider value={value}>{children}</TaxRoutingContext.Provider>;
}

export function useTaxRouting(): TaxRoutingContextValue {
  const ctx = useContext(TaxRoutingContext);
  if (!ctx) throw new Error('useTaxRouting must be used inside TaxRoutingProvider');
  return ctx;
}
