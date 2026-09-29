import type {
  ActivityDecisionSignalState,
  ActivityQueueFixtureItem
} from "./activityWorkQueueFixture";

export type ActivityExperienceState =
  | "ready"
  | "loading"
  | "empty"
  | "partial"
  | "stale"
  | "offline"
  | "unauthorized"
  | "error";

export type ActivityLocalReviewState = "idle" | "inspected" | "receipted";

export type ActivityDecisionStageId =
  | "relationship"
  | "evidence"
  | "dependency"
  | "consequence"
  | "receipt";

export type ActivityDecisionStageTone =
  | "clear"
  | "attention"
  | "blocked"
  | "limited"
  | "pending"
  | "recorded";

export type ActivityDecisionStage = {
  id: ActivityDecisionStageId;
  label: "Relationship" | "Evidence" | "Dependency" | "Consequence" | "Receipt";
  stateLabel: string;
  detail: string;
  tone: ActivityDecisionStageTone;
};

type ActivityFeedbackKind = "loading" | "empty" | "error" | "denied" | "info";

export type ActivityExperienceSurface = {
  showWorkItem: boolean;
  feedbackKind?: ActivityFeedbackKind;
  title?: string;
  detail?: string;
  correlationId?: string;
  recoverable: boolean;
  masksDecisionPath: boolean;
  retainedNotice?: {
    title: string;
    detail: string;
  };
};

export type ActivityDecisionSpineViewModel = {
  itemId: string;
  subject: string;
  experienceState: ActivityExperienceState;
  localReviewState: ActivityLocalReviewState;
  effectiveFreshness: ActivityQueueFixtureItem["freshness"];
  stages: readonly ActivityDecisionStage[];
  consequence: ActivityQueueFixtureItem["decisionPath"]["consequence"];
  constraint: string | null;
  canInspect: boolean;
  canRecordReceipt: boolean;
  surface: ActivityExperienceSurface;
};

export const activityExperienceStateLabels: Record<ActivityExperienceState, string> = {
  ready: "Ready",
  loading: "Loading",
  empty: "Empty",
  partial: "Partial",
  stale: "Stale",
  offline: "Offline",
  unauthorized: "Unauthorized",
  error: "Error"
};

export function activityLocalReviewPostureKey(
  item: ActivityQueueFixtureItem,
  experienceState: ActivityExperienceState
) {
  const effectiveFreshness = experienceState === "stale" ? "Stale" : item.freshness;
  return [
    item.id,
    experienceState,
    effectiveFreshness,
    item.decisionPath.relationship.state,
    item.decisionPath.evidence.state,
    item.decisionPath.dependency.state
  ].join(":");
}

const adverseSurfaces: Record<
  Exclude<ActivityExperienceState, "ready" | "partial" | "stale">,
  Omit<ActivityExperienceSurface, "correlationId">
> = {
  loading: {
    showWorkItem: false,
    feedbackKind: "loading",
    title: "Loading activity context",
    detail: "The deterministic local fixture is preparing the queue. No provider request is in progress.",
    recoverable: false,
    masksDecisionPath: true
  },
  empty: {
    showWorkItem: false,
    feedbackKind: "empty",
    title: "No activities need attention",
    detail: "The deterministic local fixture contains no work items in this state. There is nothing to review or send.",
    recoverable: false,
    masksDecisionPath: true
  },
  offline: {
    showWorkItem: false,
    feedbackKind: "info",
    title: "Local source unavailable",
    detail: "The local fixture is unavailable. No network recovery or provider call was attempted.",
    recoverable: true,
    masksDecisionPath: true
  },
  unauthorized: {
    showWorkItem: false,
    feedbackKind: "denied",
    title: "You do not have access to this activity context",
    detail: "Relationship details, the decision path, and local receipts stay masked. Request access before continuing.",
    recoverable: false,
    masksDecisionPath: true
  },
  error: {
    showWorkItem: false,
    feedbackKind: "error",
    title: "The activity view could not be prepared",
    detail: "The local fixture failed before activity context was shown. Use the correlation-safe identifier when reporting it.",
    recoverable: true,
    masksDecisionPath: true
  }
};

export function deriveActivityDecisionSpine(
  item: ActivityQueueFixtureItem,
  experienceState: ActivityExperienceState,
  localReviewState: ActivityLocalReviewState
): ActivityDecisionSpineViewModel {
  const surface = deriveSurface(experienceState);
  const effectiveFreshness = experienceState === "stale" ? "Stale" : item.freshness;
  const relationship = deriveRelationshipStage(item, experienceState);
  const evidence = deriveEvidenceStage(item, effectiveFreshness);
  const dependency = deriveSignalStage(
    "dependency",
    "Dependency",
    item.decisionPath.dependency
  );
  const constraint = deriveConstraint(experienceState, effectiveFreshness, dependency.tone);
  const consequence = deriveConsequenceStage(item, localReviewState, constraint);
  const receipt = deriveReceiptStage(item, localReviewState, constraint);

  return {
    itemId: item.id,
    subject: item.subject,
    experienceState,
    localReviewState,
    effectiveFreshness,
    stages: [relationship, evidence, dependency, consequence, receipt],
    consequence: item.decisionPath.consequence,
    constraint,
    canInspect: surface.showWorkItem && localReviewState === "idle",
    canRecordReceipt: surface.showWorkItem && localReviewState === "inspected",
    surface
  };
}

function deriveSurface(experienceState: ActivityExperienceState): ActivityExperienceSurface {
  if (experienceState === "ready") {
    return {
      showWorkItem: true,
      recoverable: false,
      masksDecisionPath: false
    };
  }

  if (experienceState === "partial") {
    return {
      showWorkItem: true,
      recoverable: true,
      masksDecisionPath: false,
      retainedNotice: {
        title: "Partial relationship context retained",
        detail: "The decision path remains inspectable, but the relationship signal is explicitly constrained until the local fixture is restored."
      }
    };
  }

  if (experienceState === "stale") {
    return {
      showWorkItem: true,
      recoverable: true,
      masksDecisionPath: false,
      retainedNotice: {
        title: "Last-known evidence retained",
        detail: "The decision path remains inspectable, but its evidence is marked stale and cannot imply provider readiness."
      }
    };
  }

  return {
    ...adverseSurfaces[experienceState],
    correlationId: `crm-demo-activity-state-${experienceState}`
  };
}

function deriveRelationshipStage(
  item: ActivityQueueFixtureItem,
  experienceState: ActivityExperienceState
): ActivityDecisionStage {
  if (experienceState === "partial") {
    return {
      id: "relationship",
      label: "Relationship",
      stateLabel: "Partial",
      detail: `Retained: ${item.decisionPath.relationship.detail}`,
      tone: "limited"
    };
  }

  return deriveSignalStage(
    "relationship",
    "Relationship",
    item.decisionPath.relationship
  );
}

function deriveEvidenceStage(
  item: ActivityQueueFixtureItem,
  effectiveFreshness: ActivityQueueFixtureItem["freshness"]
): ActivityDecisionStage {
  if (effectiveFreshness === "Stale") {
    return {
      id: "evidence",
      label: "Evidence",
      stateLabel: "Stale",
      detail: `Last known: ${item.decisionPath.evidence.detail}`,
      tone: "limited"
    };
  }

  if (effectiveFreshness === "Refreshing") {
    return {
      id: "evidence",
      label: "Evidence",
      stateLabel: "Refreshing",
      detail: item.decisionPath.evidence.detail,
      tone: "attention"
    };
  }

  return deriveSignalStage("evidence", "Evidence", item.decisionPath.evidence);
}

function deriveSignalStage(
  id: "relationship" | "evidence" | "dependency",
  label: "Relationship" | "Evidence" | "Dependency",
  signal: { detail: string; state: ActivityDecisionSignalState }
): ActivityDecisionStage {
  const presentation: Record<
    ActivityDecisionSignalState,
    Pick<ActivityDecisionStage, "stateLabel" | "tone">
  > = {
    ready: { stateLabel: id === "dependency" ? "Clear" : "Available", tone: "clear" },
    "needs-attention": { stateLabel: "Needs review", tone: "attention" },
    blocked: { stateLabel: "Blocked", tone: "blocked" },
    unavailable: { stateLabel: "Unavailable", tone: "limited" }
  };

  return {
    id,
    label,
    detail: signal.detail,
    ...presentation[signal.state]
  };
}

function deriveConsequenceStage(
  item: ActivityQueueFixtureItem,
  localReviewState: ActivityLocalReviewState,
  constraint: string | null
): ActivityDecisionStage {
  if (localReviewState === "idle") {
    return {
      id: "consequence",
      label: "Consequence",
      stateLabel: "Ready to inspect",
      detail: constraint ?? item.decisionPath.consequence.title,
      tone: constraint ? "attention" : "pending"
    };
  }

  return {
    id: "consequence",
    label: "Consequence",
    stateLabel: "Inspected",
    detail: "The fixture-backed consequence is visible in the local tool below.",
    tone: constraint ? "attention" : "clear"
  };
}

function deriveReceiptStage(
  item: ActivityQueueFixtureItem,
  localReviewState: ActivityLocalReviewState,
  constraint: string | null
): ActivityDecisionStage {
  if (localReviewState === "receipted") {
    return {
      id: "receipt",
      label: "Receipt",
      stateLabel: "Recorded locally",
      detail: `Attached only to ${item.subject}. No provider action was sent.`,
      tone: "recorded"
    };
  }

  if (localReviewState === "inspected") {
    return {
      id: "receipt",
      label: "Receipt",
      stateLabel: "Ready to record",
      detail: constraint
        ? "A local review receipt may record the constraint; it does not clear it."
        : "The inspected consequence can be acknowledged in this local fixture.",
      tone: constraint ? "attention" : "pending"
    };
  }

  return {
    id: "receipt",
    label: "Receipt",
    stateLabel: "Not recorded",
    detail: "Inspect the consequence before recording a local review receipt.",
    tone: "pending"
  };
}

function deriveConstraint(
  experienceState: ActivityExperienceState,
  effectiveFreshness: ActivityQueueFixtureItem["freshness"],
  dependencyTone: ActivityDecisionStageTone
) {
  const constraints: string[] = [];

  if (experienceState === "partial") {
    constraints.push(
      "Relationship context is partial. Inspect retained context only; this local review cannot establish readiness."
    );
  }
  if (effectiveFreshness === "Stale") {
    constraints.push(
      "Evidence is stale. Inspect the last-known consequence only; this local review cannot establish freshness."
    );
  } else if (effectiveFreshness === "Refreshing") {
    constraints.push(
      "Evidence is refreshing. Inspect retained context only; this local review does not complete the refresh."
    );
  }
  if (dependencyTone === "blocked") {
    constraints.push(
      "A dependency is blocked. Inspect the consequence only; this local review does not clear the dependency."
    );
  } else if (dependencyTone === "attention") {
    constraints.push(
      "A dependency needs review. Inspect the consequence before recording a local receipt."
    );
  }

  return constraints.length > 0 ? constraints.join(" ") : null;
}
