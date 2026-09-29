import {
  TRUSTED_TASK_CONTRACT_VERSION,
  TRUSTED_TASK_FIXTURE_VERSION,
  TRUSTED_TASK_PREVIEW_VERSION,
  type PermissionResult,
  type TrustedTaskFixture,
  type TrustedTaskPreview,
  type TrustedTaskReceipt,
  type TrustedTaskState,
  type TrustedTaskWork
} from "./contract";

export const TRUSTED_TASK_SOURCE_LABEL = "Deterministic product-neutral fixture";

const WORK = {
  id: "neutral-attention-01",
  workToken: "work-neutral-attention-01",
  resumeToken: "resume-neutral-attention-01-v1",
  title: "Review evidence freshness",
  summary: "Inspect why this neutral item needs attention before choosing a next step.",
  ownerLabel: "Shared operations queue",
  statusLabel: "Attention needed",
  reasonForAttention: "One retained evidence reference is older than the local freshness target.",
  consequence: "Continuing with stale evidence could produce an incomplete review record.",
  evidenceSource: "Fixture evidence register",
  evidenceSummary: "Two current references and one last-known reference are available.",
  evidenceObservedAt: "2026-07-17T21:40:00.000Z",
  freshnessLabel: "Last evaluated 20 minutes ago",
  timeline: [
    { at: "2026-07-17T21:20:00.000Z", label: "Fixture evidence retained" },
    { at: "2026-07-17T21:40:00.000Z", label: "Freshness posture evaluated" }
  ]
} as const satisfies TrustedTaskWork;

const PERMITTED = {
  result: "permitted",
  reason: "Fixture policy permits previewing the bounded next step.",
  evaluatedAt: "2026-07-17T21:40:00.000Z",
  policyReference: "fixture-policy/read-preview/1"
} as const satisfies PermissionResult;

const DENIED = {
  result: "denied",
  reason: "Fixture policy denies this preview posture; no protected detail is disclosed.",
  evaluatedAt: "2026-07-17T21:40:00.000Z",
  policyReference: "fixture-policy/read-preview/1"
} as const satisfies PermissionResult;

const UNAVAILABLE = {
  result: "unavailable",
  reason: "A fixture permission result is unavailable, so preview remains closed.",
  evaluatedAt: "2026-07-17T21:40:00.000Z",
  policyReference: "fixture-policy/read-preview/1"
} as const satisfies PermissionResult;

function preview(permission: PermissionResult): TrustedTaskPreview {
  return {
    version: TRUSTED_TASK_PREVIEW_VERSION,
    workToken: WORK.workToken,
    posture: "preview_only",
    nextStepLabel: "Prepare a refreshed evidence review",
    consequence: "A later governed flow could request refreshed evidence; this fixture sends nothing.",
    disclosure: "Preview only. No action is sent and no source record is changed.",
    permission
  };
}

const SUCCESS_RECEIPT = {
  receiptId: "receipt-neutral-0001",
  workToken: WORK.workToken,
  resumeToken: WORK.resumeToken,
  fixtureVersion: TRUSTED_TASK_FIXTURE_VERSION,
  previewVersion: TRUSTED_TASK_PREVIEW_VERSION,
  correlationId: "corr-fixture-7f31a2",
  operationId: "operation-fixture-success-0001",
  operationState: "simulated_complete",
  auditPosture: "fixture_record_retained",
  undoPosture: "not_supported",
  recordedAt: "2026-07-17T21:42:00.000Z"
} as const satisfies TrustedTaskReceipt;

const FAILED_RECEIPT = {
  ...SUCCESS_RECEIPT,
  receiptId: "receipt-neutral-failed-0001",
  correlationId: "corr-fixture-91c0e4",
  operationId: "operation-fixture-failed-0001",
  operationState: "simulated_failed",
  undoPosture: "manual_recovery_required"
} as const satisfies TrustedTaskReceipt;

function fixture(
  state: TrustedTaskState,
  values: Partial<Omit<TrustedTaskFixture, "contractVersion" | "fixtureVersion" | "snapshotIdentity" | "state" | "sourceLabel">> = {}
): TrustedTaskFixture {
  return {
    contractVersion: TRUSTED_TASK_CONTRACT_VERSION,
    fixtureVersion: TRUSTED_TASK_FIXTURE_VERSION,
    snapshotIdentity: "n0-snapshot/92f87be0",
    state,
    sourceLabel: TRUSTED_TASK_SOURCE_LABEL,
    message: "Trusted-task fixture context is available.",
    recoveryGuidance: "Return to the permitted local preview fixture.",
    work: WORK,
    preview: preview(PERMITTED),
    receipt: null,
    ...values
  };
}

export const TRUSTED_TASK_FIXTURES = {
  loading: fixture("loading", { message: "Loading retained fixture context.", work: null, preview: null }),
  empty: fixture("empty", { message: "No trusted task is available in this fixture state.", work: null, preview: null }),
  "permitted-preview": fixture("permitted-preview"),
  denied: fixture("denied", {
    message: "Preview is denied by the fixture-owned permission result.",
    preview: preview(DENIED)
  }),
  stale: fixture("stale", {
    message: "Last-known evidence is shown with an explicit freshness warning.",
    preview: preview(UNAVAILABLE)
  }),
  partial: fixture("partial", {
    message: "The evidence summary is available, but one reference is unavailable.",
    preview: preview(UNAVAILABLE)
  }),
  offline: fixture("offline", {
    message: "Offline fixture posture: retained context is visible and preview is closed.",
    preview: preview(UNAVAILABLE)
  }),
  unauthorized: fixture("unauthorized", {
    message: "Session context is unavailable. Trusted-task fields remain hidden.",
    work: null,
    preview: null
  }),
  pending: fixture("pending", {
    message: "A simulated operation is pending; this page cannot dispatch or advance it."
  }),
  "success-receipt": fixture("success-receipt", {
    message: "A deterministic simulated receipt is retained for exact resume.",
    receipt: SUCCESS_RECEIPT
  }),
  failed: fixture("failed", {
    message: "The simulated operation failed before any source change.",
    recoveryGuidance: "Use the exact receipt and correlation identifier for manual recovery.",
    receipt: FAILED_RECEIPT
  }),
  error: fixture("error", {
    message: "Fixture evidence could not be resolved.",
    recoveryGuidance: "Open the bounded recovery fixture; no network retry is attempted.",
    work: null,
    preview: null
  }),
  recovery: fixture("recovery", {
    message: "Exact work and receipt context have been restored locally.",
    recoveryGuidance: "Review the retained receipt before returning to preview.",
    receipt: SUCCESS_RECEIPT
  })
} as const satisfies Record<TrustedTaskState, TrustedTaskFixture>;

export function fixtureForTrustedTaskState(state: TrustedTaskState): TrustedTaskFixture {
  return TRUSTED_TASK_FIXTURES[state];
}

export function trustedTaskWork(): TrustedTaskWork {
  return WORK;
}
