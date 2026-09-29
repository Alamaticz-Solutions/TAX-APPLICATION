/**
 * Wire types for the Framework IX foreground transport, mirrored from the
 * appfw_runtime serde contracts (camelCase structs; snake_case payload tags
 * with snake_case variant fields). These types describe the canonical
 * Framework events only; the product never authors lifecycle facts.
 */

export const IX_RUN_REQUEST_SCHEMA_VERSION = 'appfw.ix_run_request@1';
export const IX_EVENT_SCHEMA_VERSION = 'appfw.ix_event@1';
export const IX_CANCEL_REQUEST_SCHEMA_VERSION = 'appfw.ix_cancel@1';

/** POST target for start (JSON body) and resume (`last-event-id` header). */
export const IX_STREAM_PATH = '/chat/stream';
/** POST target for idempotent cancel; `:runId` is interpolated. */
export const ixCancelPath = (runId: string): string =>
  `/chat/runs/${encodeURIComponent(runId)}/cancel`;

export type IxFocusRef = {
  kind: string;
  id: string;
};

export type IxRunRequestWire = {
  schemaVersion: typeof IX_RUN_REQUEST_SCHEMA_VERSION;
  intentKey: string;
  focus: IxFocusRef;
  question?: string;
};

export type IxPrincipalBindingWire = {
  tenantId: string;
  subject: string;
  principalType: 'user' | 'service' | 'agent';
  onBehalfOf?: string;
};

export type IxExecutionAuthorityWire = {
  contextRevision: number;
  authorizationFingerprint: string;
};

export type IxContextSummaryWire = {
  focusLabel: string;
  detail: string;
  evaluatedAt: string;
  freshness: 'current' | 'stale' | 'mixed' | 'unknown';
  sources?: ReadonlyArray<{
    sourceRef: string;
    label: string;
    refreshedAt?: string;
    freshness: 'current' | 'stale' | 'mixed' | 'unknown';
    inspectable: boolean;
  }>;
  gaps?: readonly string[];
};

export type IxArtifactRevisionWire = {
  artifactId: string;
  artifactType: string;
  contentSchemaVersion: string;
  rendererKey: string;
  revision: number;
  status: 'partial' | 'ready' | 'revising' | 'stale' | 'failed';
  changedRegionIds?: readonly string[];
  presentation: unknown;
};

export type IxPhaseWire =
  | 'acknowledged'
  | 'understanding'
  | 'gathering'
  | 'resolving'
  | 'interpreting'
  | 'composing'
  | 'checking';

export type IxCancellationStageWire =
  | 'accepted'
  | 'stopping'
  | 'draining'
  | 'stopped'
  | 'confirmed';

export type IxTerminalOutcomeWire = 'completed' | 'partial' | 'cancelled' | 'failed';

/**
 * Canonical event payloads. The `type` tag and variant field names are
 * snake_case per the Framework contract.
 */
export type IxEventPayloadWire =
  | { type: 'run_acknowledged'; intent_label: string; focus: IxFocusRef }
  | {
      type: 'context_resolved';
      authority: IxExecutionAuthorityWire;
      context: IxContextSummaryWire;
    }
  | { type: 'context_reconciliation_requested'; proposal: unknown }
  | { type: 'context_reconciled'; proposal_id: string; context_revision: unknown }
  | {
      type: 'phase_changed';
      authority: IxExecutionAuthorityWire;
      phase: IxPhaseWire;
      reason: string;
    }
  | {
      type: 'artifact_revision';
      authority: IxExecutionAuthorityWire;
      artifact: IxArtifactRevisionWire;
    }
  | { type: 'waiting_for_user'; authority: IxExecutionAuthorityWire; prompt: string }
  | { type: 'user_input_accepted'; authority: IxExecutionAuthorityWire }
  | {
      type: 'cancel_requested';
      command_id: string;
      accepted_cursor: string;
      accepted_revision: number;
    }
  | { type: 'cancellation_progress'; command_id: string; stage: IxCancellationStageWire }
  | {
      type: 'run_finished';
      authority?: IxExecutionAuthorityWire;
      outcome: IxTerminalOutcomeWire;
      message: string;
    };

export type IxEventWire = {
  schemaVersion: typeof IX_EVENT_SCHEMA_VERSION;
  eventId: string;
  runId: string;
  sequence: number;
  occurredAt: string;
  actor: IxPrincipalBindingWire;
  payload: IxEventPayloadWire;
};

export type IxCancelReceiptWire = {
  disposition: 'accepted' | 'already_requested' | 'already_terminal' | 'not_found';
  runId: string;
  commandId: string;
  cursor: string | null;
  auditRetained: boolean;
  auditProjected: 'pending' | 'confirmed' | 'unknown';
  auditProjectionCursor?: string;
};

/** One parsed SSE frame from the canonical envelope stream. */
export type IxStreamFrame = {
  event: IxEventWire;
  /**
   * Commit-boundary replay cursor. Present only on the final event of each
   * envelope (the SSE `id:` field); it is the resume token.
   */
  commitCursor?: string;
};

/** Caller-supplied auth posture; verification is backend-owned (B_HOST). */
export type IxCallerAuth = {
  bearerToken: string;
  tenantId: string;
};
