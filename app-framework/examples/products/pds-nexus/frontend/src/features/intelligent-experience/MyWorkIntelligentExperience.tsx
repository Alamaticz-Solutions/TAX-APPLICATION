/**
 * My Work intelligent experience journey (B_PRODUCT implementation).
 *
 * The journey consumes only the receipted PDS packages through node_modules:
 * `@appfw/pds-ix-presentation-contract` validates every presentation payload
 * before anything renders, and `@appfw/pds-health-components/ix-recipes`
 * renders the registered attention-stewardship recipe. All lifecycle facts
 * arrive as canonical Framework events over the real transport wire; nothing
 * here fabricates, intercepts, or replays fixture responses.
 *
 * Live execution of this journey (SSE stream/resume, idempotent cancel
 * through the transport, signed-JWT ingress, restart/replay) requires the
 * chat-gated IX runtime mounted by the B_HOST leaf; until that leaf lands,
 * the transport's absence renders the honest unavailable posture
 * (DEFERRED_TO_B_HOST).
 */

import React, {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState
} from 'react';
import { Badge, Button, Surface } from '@appfw/pds-health-components';
import { PdsIxRecipePresentation } from '@appfw/pds-health-components/ix-recipes';
import {
  validatePdsIxPresentation,
  type PdsIxPresentationEnvelope
} from '@appfw/pds-ix-presentation-contract';

import { cancelRun, resumeRunStream, startRunStream, type IxStreamHandle } from './ix-client';
import { myWorkRecipeRegistration, NEXUS_MY_WORK_INTENT_KEY } from './registration';
import {
  initialJourneyState,
  reduceJourney,
  type IxJourneyState
} from './run-state';
import {
  IX_RUN_REQUEST_SCHEMA_VERSION,
  type IxCallerAuth,
  type IxRunRequestWire
} from './types';

const AUTH_TOKEN_KEY = 'nexus.ix.local.bearer';
const AUTH_TENANT_KEY = 'nexus.ix.local.tenant';
const RESUME_KEY = 'nexus.ix.my-work.resume';

type StoredResume = {
  runId: string;
  cursor: string;
  snapshot: IxJourneyState;
};

function readStoredAuth(): IxCallerAuth {
  return {
    bearerToken: window.sessionStorage.getItem(AUTH_TOKEN_KEY) ?? '',
    tenantId: window.sessionStorage.getItem(AUTH_TENANT_KEY) ?? 'tenant_a'
  };
}

function readStoredResume(): StoredResume | null {
  const raw = window.sessionStorage.getItem(RESUME_KEY);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as StoredResume;
    if (!parsed.runId || !parsed.cursor) {
      return null;
    }
    if (parsed.snapshot?.runId === parsed.runId && parsed.snapshot.resumeCursor) {
      return parsed;
    }
    return {
      runId: parsed.runId,
      cursor: parsed.cursor,
      snapshot: {
        ...initialJourneyState,
        runId: parsed.runId,
        resumeCursor: parsed.cursor,
        posture: 'streaming'
      }
    };
  } catch {
    return null;
  }
}

function myWorkRequest(): IxRunRequestWire {
  return {
    schemaVersion: IX_RUN_REQUEST_SCHEMA_VERSION,
    intentKey: NEXUS_MY_WORK_INTENT_KEY,
    focus: { kind: 'nexus.work-queue', id: 'my-work' }
  };
}

type ValidatedPresentation =
  | { kind: 'none' }
  | { kind: 'valid'; envelope: PdsIxPresentationEnvelope }
  | { kind: 'contract_violation'; errors: readonly string[] };

function validateLatestPresentation(state: IxJourneyState): ValidatedPresentation {
  if (!state.latestArtifact) {
    return { kind: 'none' };
  }
  const validation = validatePdsIxPresentation(state.latestArtifact.presentation);
  if (validation.ok) {
    return { kind: 'valid', envelope: validation.value };
  }
  return { kind: 'contract_violation', errors: validation.errors };
}

export function MyWorkIntelligentExperience(): React.JSX.Element {
  const [state, dispatch] = useReducer(reduceJourney, initialJourneyState);
  const [auth, setAuth] = useState<IxCallerAuth>(() => readStoredAuth());
  const [storedResume, setStoredResume] = useState<StoredResume | null>(() =>
    readStoredResume()
  );
  const streamRef = useRef<IxStreamHandle | null>(null);
  const cancelCommandRef = useRef<string | null>(null);

  useEffect(() => {
    return () => streamRef.current?.abort();
  }, []);

  // Reload/resume: persist the product snapshot plus the latest exclusive
  // cursor so a finished-run reload can hydrate history when replay is empty.
  useEffect(() => {
    if (state.runId && state.resumeCursor) {
      const record: StoredResume = {
        runId: state.runId,
        cursor: state.resumeCursor,
        snapshot: state
      };
      window.sessionStorage.setItem(RESUME_KEY, JSON.stringify(record));
      setStoredResume(record);
    }
  }, [state]);

  const applyAuth = useCallback((next: IxCallerAuth) => {
    window.sessionStorage.setItem(AUTH_TOKEN_KEY, next.bearerToken);
    window.sessionStorage.setItem(AUTH_TENANT_KEY, next.tenantId);
    setAuth(next);
  }, []);

  const openStream = useCallback(
    (open: () => IxStreamHandle) => {
      streamRef.current?.abort();
      streamRef.current = open();
    },
    []
  );

  const handleStart = useCallback(() => {
    dispatch({ kind: 'start_requested' });
    cancelCommandRef.current = `nexus-cancel-${crypto.randomUUID()}`;
    openStream(() =>
      startRunStream(myWorkRequest(), auth, {
        onFrame: (frame) => dispatch({ kind: 'frame', frame }),
        onStreamError: (error) =>
          dispatch({ kind: 'transport_unavailable', reason: error.message })
      })
    );
  }, [auth, openStream]);

  const handleResume = useCallback(() => {
    const resume = storedResume;
    if (!resume) {
      return;
    }
    openStream(() =>
      resumeRunStream(resume.cursor, auth, {
        onTransportAccepted: ({ status, lastEventId }) => {
          if (status < 200 || status >= 300 || lastEventId !== resume.cursor) {
            window.sessionStorage.removeItem(RESUME_KEY);
            setStoredResume(null);
            dispatch({
              kind: 'resume_rejected',
              reason: 'Resume was not authorized for the stored cursor.'
            });
            return;
          }
          dispatch({ kind: 'hydrate_from_stored', snapshot: resume.snapshot });
        },
        onFrame: (frame) => dispatch({ kind: 'frame', frame }),
        onStreamError: (error) => {
          window.sessionStorage.removeItem(RESUME_KEY);
          setStoredResume(null);
          dispatch({ kind: 'resume_rejected', reason: error.message });
        }
      })
    );
  }, [auth, openStream, storedResume]);

  const handleCancel = useCallback(() => {
    if (!state.runId) {
      return;
    }
    // Idempotent by contract: retries reuse the same command id.
    const commandId =
      cancelCommandRef.current ?? `nexus-cancel-${crypto.randomUUID()}`;
    cancelCommandRef.current = commandId;
    dispatch({ kind: 'cancel_requested' });
    void cancelRun(state.runId, commandId, auth).catch((error: Error) =>
      dispatch({ kind: 'transport_unavailable', reason: error.message })
    );
  }, [auth, state.runId]);

  const presentation = useMemo(() => validateLatestPresentation(state), [state]);

  return (
    <section className="ix-journey" id="my-work-ix" aria-label="My Work intelligent experience">
      <Surface
        title="My Work intelligent experience"
        subtitle="Attention-stewardship recipe over the governed IX foreground lifecycle"
        density="compact"
      >
        <div className="ix-journey__controls">
          <Button variant="primary" onClick={handleStart} data-testid="ix-start">
            Start My Work brief
          </Button>
          <Button
            variant="quiet"
            onClick={handleCancel}
            disabled={!state.runId || state.posture === 'finished'}
            data-testid="ix-cancel"
          >
            Stop safely
          </Button>
          <Button
            variant="quiet"
            onClick={handleResume}
            disabled={!storedResume}
            data-testid="ix-resume"
          >
            Resume after reload
          </Button>
          <span className="ix-journey__posture" data-testid="ix-posture" data-posture={state.posture}>
            <Badge
              tone={
                state.posture === 'finished'
                  ? 'success'
                  : state.posture === 'unavailable'
                    ? 'danger'
                    : state.posture === 'idle'
                      ? 'neutral'
                      : 'accent'
              }
            >
              {state.posture}
            </Badge>
          </span>
        </div>

        <details className="ix-journey__auth">
          <summary>Local session identity</summary>
          <p>
            Local exploration only: the bearer token is verified by the backend JWT
            ingress; the browser never decides authorization.
          </p>
          <label>
            Bearer token
            <input
              type="password"
              data-testid="ix-auth-token"
              value={auth.bearerToken}
              onChange={(event) =>
                applyAuth({ ...auth, bearerToken: event.currentTarget.value })
              }
            />
          </label>
          <label>
            Tenant
            <input
              type="text"
              data-testid="ix-auth-tenant"
              value={auth.tenantId}
              onChange={(event) =>
                applyAuth({ ...auth, tenantId: event.currentTarget.value })
              }
            />
          </label>
        </details>
      </Surface>

      {state.posture === 'unavailable' || state.terminalOutcome === 'failed' ? (
        <Surface
          title="This experience is unavailable"
          subtitle="Honest posture: nothing is fabricated when the governed runtime or your context is absent"
          density="compact"
        >
          <div className="ix-journey__unavailable" data-testid="ix-unavailable">
            <Badge tone="danger">Unavailable</Badge>
            <p>{state.unavailableReason ?? state.terminalMessage}</p>
            <p>
              The My Work brief renders only from canonical Framework lifecycle
              events over the real transport. When the IX runtime is not mounted
              or no authorized work context exists, this posture is shown instead
              of synthetic content.
            </p>
          </div>
        </Surface>
      ) : null}

      {state.terminalOutcome ? (
        <div className="ix-journey__terminal" data-testid="ix-terminal" data-outcome={state.terminalOutcome}>
          <Badge tone={state.terminalOutcome === 'completed' ? 'success' : 'warning'}>
            {state.terminalOutcome}
          </Badge>
          <span>{state.terminalMessage}</span>
        </div>
      ) : null}

      {state.waitingPrompt ? (
        <div className="ix-journey__waiting" data-testid="ix-waiting">
          <Badge tone="accent">Waiting for you</Badge>
          <span>{state.waitingPrompt}</span>
        </div>
      ) : null}

      {presentation.kind === 'valid' ? (
        <div data-testid="ix-presentation">
          <PdsIxRecipePresentation
            registration={myWorkRecipeRegistration}
            presentation={presentation.envelope}
          />
        </div>
      ) : null}

      {presentation.kind === 'contract_violation' ? (
        <Surface
          title="Presentation refused"
          subtitle="The payload failed the pds.ix.presentation@1 contract and will not render"
          density="compact"
        >
          <ul className="ix-journey__violations" data-testid="ix-contract-violation">
            {presentation.errors.slice(0, 8).map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
        </Surface>
      ) : null}

      <Surface
        title="Run timeline"
        subtitle="Canonical Framework lifecycle events, newest last"
        density="compact"
      >
        <div className="ix-journey__meta">
          <span data-testid="ix-run-id">{state.runId ?? 'No active run'}</span>
          <span data-testid="ix-phase">{state.phase ?? 'no phase yet'}</span>
          <span
            data-testid="ix-cursor"
            data-final-tip={state.posture === 'finished' ? 'true' : undefined}
          >
            {state.resumeCursor ?? 'no commit cursor yet'}
          </span>
        </div>
        <ol className="ix-journey__timeline" data-testid="ix-timeline">
          {state.timeline.map((entry) => (
            <li key={entry.key}>
              <span className="ix-journey__timeline-seq">#{entry.sequence}</span>
              <span className="ix-journey__timeline-actor">{entry.actorSubject}</span>
              <span>{entry.summary}</span>
            </li>
          ))}
        </ol>
        {state.context ? (
          <div className="ix-journey__context" data-testid="ix-context">
            <Badge tone="accent">{state.context.freshness}</Badge>
            <strong>{state.context.focusLabel}</strong>
            <span>{state.context.detail}</span>
          </div>
        ) : null}
      </Surface>
    </section>
  );
}
