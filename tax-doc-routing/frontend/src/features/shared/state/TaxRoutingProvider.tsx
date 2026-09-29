import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { fetchMyPermissions, hasPermission as grantsHavePermission, hasScope as grantsHaveScope, type Grant } from '../../../lib/permissions';
import {
  cancelRoutingRecord,
  createRoutingRecord,
  fetchAllRecords,
  fetchExceptionQueue,
  fetchMyRecords,
  retryFailedStep,
  updateRoutingRecordProgress
} from '../../../lib/routingRecords';
import { PROCESSING_STEPS, STEP_DURATION_MS } from '../config/processingSteps';
import { STATUS, TERMINAL_STATUSES } from '../config/statuses';
import { users } from '../data/users';
import type { AppUser, Client, RoutingCase, RoutingDraft, Role, TaxDocument } from '../types';
import { delay } from '../services/delay';

// Routing records and exceptions are read from and written to the real `TaxRoutingRecord` /
// `ExceptionTask` tables (lib/routingRecords.ts) — see docs/architecture/rbac-design.md and the
// dynamic-form-engine design doc. Documents stay session-only (never written to `TaxDocument`)
// until the SharePoint integration replaces the in-memory `DocumentStore`; everything else about
// a routing record now survives a reload. Permissions are likewise real, database-driven grants
// (lib/permissions.ts), not prototype state.

const ROLE_KEY = 'tax-doc-routing.frontend.role';

// The role switcher (Tax Staff / Tax Admin) maps to the two bootstrap users seeded by
// .appfw/model/schemas/tax_routing/seeds/04_app_user.yaml, so switching roles fetches a real,
// different identity's grants and records from the backend rather than a canned constant.
const BACKEND_USER_NAME: Record<Role, string> = { standard: 'alex', admin: 'dana' };

const emptyDraft = (): RoutingDraft => ({ client: null, additionalEmail: '', notifyClient: true, documents: [], recordId: null });

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
  role: Role;
  setRole: (role: Role) => void;
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
  submitDraft: () => Promise<RoutingCase>;
  retryException: (id: string) => void;
  cancelCase: (id: string, resolution: string, comments: string) => void;
  /** Live, database-driven grants for the current role's backend identity. */
  grants: Grant[];
  permissionsStatus: 'loading' | 'ready' | 'error';
  hasPermission: (code: string) => boolean;
  hasScope: (code: string, scope: string) => boolean;
};

const TaxRoutingContext = createContext<TaxRoutingContextValue | null>(null);

function readRole(): Role {
  try {
    return window.sessionStorage.getItem(ROLE_KEY) === 'admin' ? 'admin' : 'standard';
  } catch {
    return 'standard';
  }
}

export function TaxRoutingProvider({ children }: { children: ReactNode }) {
  const [role, setRoleState] = useState<Role>(readRole);
  const [cases, setCases] = useState<RoutingCase[]>([]);
  const [casesStatus, setCasesStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [exceptionQueue, setExceptionQueue] = useState<RoutingCase[]>([]);
  const [draft, setDraft] = useState<RoutingDraft>(emptyDraft);

  const casesRef = useRef(cases);
  useEffect(() => {
    casesRef.current = cases;
  }, [cases]);
  const running = useRef(new Set<string>());

  const [grants, setGrants] = useState<Grant[]>([]);
  const [userId, setUserId] = useState<string | null>(null);
  const [permissionsStatus, setPermissionsStatus] = useState<'loading' | 'ready' | 'error'>('loading');

  // The real AppUser.id for the role's backend identity — what `assigned_user_id` on a
  // TaxRoutingRecord actually stores, replacing the placeholder `u-alex` / `u-dana` ids that used
  // to live in features/shared/data/users.ts.
  const user: AppUser = useMemo(() => ({ ...users[role], id: userId ?? '' }), [role, userId]);

  useEffect(() => {
    let cancelled = false;
    setPermissionsStatus('loading');
    setUserId(null);
    fetchMyPermissions(BACKEND_USER_NAME[role])
      .then((principal) => {
        if (cancelled) return;
        setGrants(principal.grants);
        setUserId(principal.user_id);
        setPermissionsStatus(principal.user_id ? 'ready' : 'error');
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
  }, [role]);

  const refreshCases = useCallback(async () => {
    if (!userId) return;
    setCasesStatus('loading');
    try {
      const loaded = role === 'admin' ? await fetchAllRecords(BACKEND_USER_NAME[role]) : await fetchMyRecords(BACKEND_USER_NAME[role], userId);
      setCases(loaded);
      setCasesStatus('ready');
      const queue = await fetchExceptionQueue(BACKEND_USER_NAME[role], loaded);
      setExceptionQueue(queue);
    } catch {
      setCasesStatus('error');
    }
  }, [role, userId]);

  useEffect(() => {
    void refreshCases();
  }, [refreshCases]);

  const hasPermission = useCallback((code: string) => grantsHavePermission(grants, code), [grants]);
  const hasScope = useCallback((code: string, scope: string) => grantsHaveScope(grants, code, scope), [grants]);

  const setRole = useCallback((next: Role) => {
    setRoleState(next);
    try {
      window.sessionStorage.setItem(ROLE_KEY, next);
    } catch {
      /* the demo role just won't be remembered */
    }
  }, []);

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

  // Animates the 9-step processing timeline (business spec 5-8) and persists each completed step
  // to the database in turn — sequentially, never in parallel, since each write's optimistic
  // `version` depends on the previous write having already landed.
  const startProcessing = useCallback(
    (id: string) => {
      if (running.current.has(id)) return;
      running.current.add(id);
      void (async () => {
        let current = casesRef.current.find((c) => c.id === id);
        if (!current) {
          running.current.delete(id);
          return;
        }
        for (let step = current.progress; step < PROCESSING_STEPS.length; step += 1) {
          await delay(STEP_DURATION_MS);
          if (!running.current.has(id)) return; // cancelled or retried away mid-animation
          const isLast = step + 1 === PROCESSING_STEPS.length;
          try {
            current = await updateRoutingRecordProgress(BACKEND_USER_NAME[role], current, {
              progress: step + 1,
              status: isLast ? STATUS.COMPLETED : STATUS.PROCESSING
            });
          } catch (err) {
            // eslint-disable-next-line no-console -- surfaced instead of silently freezing the timeline
            console.error('Failed to persist processing step', err);
            break;
          }
          updateCaseInPlace(current);
        }
        running.current.delete(id);
      })();
    },
    [role, updateCaseInPlace]
  );

  const submitDraft = useCallback(async () => {
    if (!draft.client) throw new Error('A client is required to create a routing record.');
    if (!userId) throw new Error('Your identity has not resolved yet — try again in a moment.');
    const created = await createRoutingRecord(BACKEND_USER_NAME[role], {
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
    // Documents are session-only (deferred pending SharePoint — see module comment); attach the
    // draft's uploads to the in-memory case so Completed/Record Details can still show them for
    // the rest of this session, without ever writing a TaxDocument row.
    const record: RoutingCase = { ...created, progress: 3, documents: draft.documents.map((d) => ({ ...d, status: 'Uploaded' as const })) };
    casesRef.current = [record, ...casesRef.current];
    setCases((list) => [record, ...list]);
    setDraft((d) => ({ ...d, recordId: record.id }));
    startProcessing(record.id);
    return record;
  }, [draft, role, userId, startProcessing]);

  const retryException = useCallback(
    (id: string) => {
      const current = casesRef.current.find((c) => c.id === id) ?? exceptionQueue.find((c) => c.id === id);
      if (!current?.exception) return;
      void retryFailedStep(BACKEND_USER_NAME[role], current.exception.id).then(() => {
        const resumed: RoutingCase = { ...current, status: STATUS.PROCESSING, task: 'Processing', exception: null };
        updateCaseInPlace(resumed);
        setExceptionQueue((list) => list.filter((c) => c.id !== id));
        startProcessing(id);
      });
    },
    [role, exceptionQueue, updateCaseInPlace, startProcessing]
  );

  const cancelCase = useCallback(
    (id: string, resolution: string, comments: string) => {
      void cancelRoutingRecord(BACKEND_USER_NAME[role], id, resolution, comments).then(() => {
        const current = casesRef.current.find((c) => c.id === id);
        if (!current) return;
        updateCaseInPlace({ ...current, status: STATUS.CANCELLED, task: 'Closed', exception: null, resolutionComment: comments });
      });
    },
    [role, updateCaseInPlace]
  );

  const value: TaxRoutingContextValue = {
    role, setRole, user,
    cases, casesStatus, visibleCases, exceptionQueue, getCase,
    canActOn, canCancel,
    draft, draftApi,
    submitDraft, retryException, cancelCase,
    grants, permissionsStatus, hasPermission, hasScope
  };
  return <TaxRoutingContext.Provider value={value}>{children}</TaxRoutingContext.Provider>;
}

export function useTaxRouting(): TaxRoutingContextValue {
  const ctx = useContext(TaxRoutingContext);
  if (!ctx) throw new Error('useTaxRouting must be used inside TaxRoutingProvider');
  return ctx;
}
