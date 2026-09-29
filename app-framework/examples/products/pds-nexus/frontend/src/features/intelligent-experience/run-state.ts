/**
 * Pure reducer folding canonical Framework IX events into the journey view
 * state. The reducer derives presentation state only; it never re-derives or
 * re-authors lifecycle facts, and the latest artifact presentation is passed
 * through verbatim for the PDS renderer plus contract validation.
 */

import type {
  IxArtifactRevisionWire,
  IxCancellationStageWire,
  IxContextSummaryWire,
  IxEventWire,
  IxPhaseWire,
  IxStreamFrame,
  IxTerminalOutcomeWire
} from './types';

export type IxJourneyPosture =
  | 'idle'
  | 'starting'
  | 'streaming'
  | 'waiting_for_user'
  | 'cancelling'
  | 'finished'
  | 'unavailable';

export type IxTimelineEntry = {
  key: string;
  sequence: number;
  occurredAt: string;
  actorSubject: string;
  summary: string;
};

export type IxJourneyState = {
  posture: IxJourneyPosture;
  runId: string | null;
  intentLabel: string | null;
  phase: IxPhaseWire | null;
  context: IxContextSummaryWire | null;
  artifacts: Record<string, IxArtifactRevisionWire>;
  /** Latest artifact revision for the journey's registered artifact type. */
  latestArtifact: IxArtifactRevisionWire | null;
  waitingPrompt: string | null;
  cancellationStage: IxCancellationStageWire | null;
  terminalOutcome: IxTerminalOutcomeWire | null;
  terminalMessage: string | null;
  /** Commit-boundary cursor for reload/resume. */
  resumeCursor: string | null;
  lastSequence: number;
  timeline: IxTimelineEntry[];
  unavailableReason: string | null;
};

export const initialJourneyState: IxJourneyState = {
  posture: 'idle',
  runId: null,
  intentLabel: null,
  phase: null,
  context: null,
  artifacts: {},
  latestArtifact: null,
  waitingPrompt: null,
  cancellationStage: null,
  terminalOutcome: null,
  terminalMessage: null,
  resumeCursor: null,
  lastSequence: 0,
  timeline: [],
  unavailableReason: null
};

export type IxJourneyAction =
  | { kind: 'start_requested' }
  | { kind: 'frame'; frame: IxStreamFrame }
  | { kind: 'cancel_requested' }
  | { kind: 'transport_unavailable'; reason: string }
  | { kind: 'hydrate_from_stored'; snapshot: IxJourneyState }
  | { kind: 'resume_rejected'; reason: string }
  | { kind: 'reset' };

export function reduceJourney(
  state: IxJourneyState,
  action: IxJourneyAction
): IxJourneyState {
  switch (action.kind) {
    case 'reset':
      return initialJourneyState;
    case 'hydrate_from_stored':
      return { ...action.snapshot };
    case 'resume_rejected':
      return {
        ...initialJourneyState,
        posture: 'unavailable',
        unavailableReason: action.reason
      };
    case 'start_requested':
      return { ...initialJourneyState, posture: 'starting' };
    case 'cancel_requested':
      if (state.posture === 'finished' || state.posture === 'unavailable') {
        return state;
      }
      return { ...state, posture: 'cancelling' };
    case 'transport_unavailable':
      return {
        ...state,
        posture: 'unavailable',
        unavailableReason: action.reason
      };
    case 'frame':
      return applyFrame(state, action.frame);
    default:
      return state;
  }
}

function applyFrame(state: IxJourneyState, frame: IxStreamFrame): IxJourneyState {
  const event = frame.event;
  // Replayed frames below the observed sequence are already folded in.
  if (event.sequence <= state.lastSequence && state.runId === event.runId) {
    return withCursor(state, frame);
  }
  const next: IxJourneyState = {
    ...state,
    runId: event.runId,
    lastSequence: event.sequence,
    timeline: [...state.timeline, timelineEntry(event)].slice(-64)
  };
  const applied = applyPayload(next, event);
  return withCursor(applied, frame);
}

function withCursor(state: IxJourneyState, frame: IxStreamFrame): IxJourneyState {
  if (!frame.commitCursor) {
    return state;
  }
  return { ...state, resumeCursor: frame.commitCursor };
}

function applyPayload(state: IxJourneyState, event: IxEventWire): IxJourneyState {
  const payload = event.payload;
  switch (payload.type) {
    case 'run_acknowledged':
      return {
        ...state,
        posture: 'streaming',
        intentLabel: payload.intent_label,
        phase: 'acknowledged'
      };
    case 'context_resolved':
      return { ...state, posture: 'streaming', context: payload.context };
    case 'phase_changed':
      return { ...state, posture: 'streaming', phase: payload.phase };
    case 'artifact_revision': {
      const artifacts = {
        ...state.artifacts,
        [payload.artifact.artifactId]: payload.artifact
      };
      return {
        ...state,
        posture: 'streaming',
        artifacts,
        latestArtifact: payload.artifact
      };
    }
    case 'waiting_for_user':
      return {
        ...state,
        posture: 'waiting_for_user',
        waitingPrompt: payload.prompt
      };
    case 'user_input_accepted':
      return { ...state, posture: 'streaming', waitingPrompt: null };
    case 'cancel_requested':
      return { ...state, posture: 'cancelling', cancellationStage: 'accepted' };
    case 'cancellation_progress':
      return {
        ...state,
        posture: payload.stage === 'confirmed' ? state.posture : 'cancelling',
        cancellationStage: payload.stage
      };
    case 'run_finished':
      return {
        ...state,
        posture: 'finished',
        terminalOutcome: payload.outcome,
        terminalMessage: payload.message
      };
    case 'context_reconciliation_requested':
    case 'context_reconciled':
      return state;
    default:
      return state;
  }
}

function timelineEntry(event: IxEventWire): IxTimelineEntry {
  return {
    key: event.eventId,
    sequence: event.sequence,
    occurredAt: event.occurredAt,
    actorSubject: event.actor.subject,
    summary: summarize(event)
  };
}

function summarize(event: IxEventWire): string {
  const payload = event.payload;
  switch (payload.type) {
    case 'run_acknowledged':
      return `Run acknowledged: ${payload.intent_label}`;
    case 'context_resolved':
      return `Context resolved: ${payload.context.focusLabel}`;
    case 'context_reconciliation_requested':
      return 'Context reconciliation requested';
    case 'context_reconciled':
      return 'Context reconciled';
    case 'phase_changed':
      return `Phase: ${payload.phase} (${payload.reason})`;
    case 'artifact_revision':
      return `Artifact ${payload.artifact.artifactId} revision ${payload.artifact.revision} (${payload.artifact.status})`;
    case 'waiting_for_user':
      return `Waiting for you: ${payload.prompt}`;
    case 'user_input_accepted':
      return 'Your input was accepted';
    case 'cancel_requested':
      return 'Cancel requested';
    case 'cancellation_progress':
      return `Cancellation ${payload.stage}`;
    case 'run_finished':
      return `Run finished (${payload.outcome}): ${payload.message}`;
    default:
      return 'Framework event';
  }
}
