export const TRUSTED_TASK_CONTRACT_VERSION = "trusted-task/1.0.0" as const;
export const TRUSTED_TASK_FIXTURE_VERSION = "trusted-task-fixtures/1.0.0" as const;
export const TRUSTED_TASK_PREVIEW_VERSION = "trusted-task-preview/1.0.0" as const;

export type TrustedTaskState =
  | "loading"
  | "empty"
  | "permitted-preview"
  | "denied"
  | "stale"
  | "partial"
  | "offline"
  | "unauthorized"
  | "pending"
  | "success-receipt"
  | "failed"
  | "error"
  | "recovery";

export const TRUSTED_TASK_STATES = [
  "loading",
  "empty",
  "permitted-preview",
  "denied",
  "stale",
  "partial",
  "offline",
  "unauthorized",
  "pending",
  "success-receipt",
  "failed",
  "error",
  "recovery"
] as const satisfies readonly TrustedTaskState[];

export type PermissionResult = Readonly<{
  result: "permitted" | "denied" | "unavailable";
  reason: string;
  evaluatedAt: string;
  policyReference: string;
}>;

export type UndoPosture = "not_supported" | "available" | "manual_recovery_required";

export type TrustedTaskWork = Readonly<{
  id: string;
  workToken: string;
  resumeToken: string;
  title: string;
  summary: string;
  ownerLabel: string;
  statusLabel: string;
  reasonForAttention: string;
  consequence: string;
  evidenceSource: string;
  evidenceSummary: string;
  evidenceObservedAt: string;
  freshnessLabel: string;
  timeline: readonly Readonly<{ at: string; label: string }>[];
}>;

export type TrustedTaskPreview = Readonly<{
  version: typeof TRUSTED_TASK_PREVIEW_VERSION;
  workToken: string;
  posture: "preview_only";
  nextStepLabel: string;
  consequence: string;
  disclosure: string;
  permission: PermissionResult;
}>;

export type TrustedTaskReceipt = Readonly<{
  receiptId: string;
  workToken: string;
  resumeToken: string;
  fixtureVersion: typeof TRUSTED_TASK_FIXTURE_VERSION;
  previewVersion: typeof TRUSTED_TASK_PREVIEW_VERSION;
  correlationId: string;
  operationId: string;
  operationState: "simulated_complete" | "simulated_failed";
  auditPosture: "fixture_record_retained";
  undoPosture: UndoPosture;
  recordedAt: string;
}>;

export type TrustedTaskFixture = Readonly<{
  contractVersion: typeof TRUSTED_TASK_CONTRACT_VERSION;
  fixtureVersion: typeof TRUSTED_TASK_FIXTURE_VERSION;
  snapshotIdentity: "n0-snapshot/92f87be0";
  state: TrustedTaskState;
  sourceLabel: string;
  message: string;
  recoveryGuidance: string;
  work: TrustedTaskWork | null;
  preview: TrustedTaskPreview | null;
  receipt: TrustedTaskReceipt | null;
}>;

export function isTrustedTaskState(value: string | null): value is TrustedTaskState {
  return value !== null && TRUSTED_TASK_STATES.some((state) => state === value);
}

export function receiptResolvesWork(
  receipt: TrustedTaskReceipt,
  work: TrustedTaskWork
): boolean {
  return receipt.workToken === work.workToken && receipt.resumeToken === work.resumeToken;
}

export function isKnownWorkToken(token: string | null, work: TrustedTaskWork | null): boolean {
  return token !== null && work?.workToken === token;
}

export function isKnownResumeToken(token: string | null, work: TrustedTaskWork | null): boolean {
  return token !== null && work?.resumeToken === token;
}
