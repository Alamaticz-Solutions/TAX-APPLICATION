import {
  IX_REFERENCE_PLAYBACK_SCHEMA,
  type IxArtifact,
  type IxAmbientWorkstream,
  type IxAmbientTuning,
  type IxAmbientTuningCorpus,
  type IxArtifactRegion,
  type IxReferencePlaybackEvent,
  type IxReferenceRecipeArtifact,
  type IxStrategyChallenge,
  type IxResolvedContext,
  type ScheduledIxReferencePlaybackEvent
} from "./contract";

// Catalog-only display fixtures. These schedules do not reproduce or assert
// an application runtime, provider, authority decision, or durable lifecycle.

export type IxReferenceId =
  | "analyze-why"
  | "contextual-conversation"
  | "adaptive-composition"
  | "working-goal-plan"
  | "adaptive-information-lens"
  | "situation-to-strategy"
  | "attention-stewardship"
  | "ambient-agent-continuity";

export type IxReferenceDescriptor = {
  id: IxReferenceId;
  number: number;
  name: string;
  promise: string;
  nextLeap: string;
  availability: "interactive";
};

export const IX_REFERENCES: readonly IxReferenceDescriptor[] = [
  {
    id: "analyze-why",
    number: 1,
    name: "Analyze Why",
    promise: "Intelligence begins on the metric and turns the visible situation into editable work.",
    nextLeap: "Real context, revisioned artifacts, exact resume, and source-driven refresh.",
    availability: "interactive"
  },
  {
    id: "contextual-conversation",
    number: 2,
    name: "Contextual Conversation",
    promise: "Open-ended inquiry stays attached to the object, evidence, and artifact already in use.",
    nextLeap: "Typed claims update the workspace instead of ending as transcript prose.",
    availability: "interactive"
  },
  {
    id: "adaptive-composition",
    number: 3,
    name: "Adaptive Composition",
    promise: "Registered components form the most useful representation for the current intent.",
    nextLeap: "Preserve focus, edits, and pins while explaining what changed in the composition.",
    availability: "interactive"
  },
  {
    id: "working-goal-plan",
    number: 4,
    name: "Working Goal Plan",
    promise: "A stable goal can have a visible, revisable path without hiding required checkpoints.",
    nextLeap: "Durable plan revisions, explicit differences, pause, checkpoint, and resume.",
    availability: "interactive"
  },
  {
    id: "adaptive-information-lens",
    number: 5,
    name: "Adaptive Information Lens",
    promise: "Canonical records become purpose-shaped without losing the complete source view.",
    nextLeap: "Combine role, objective, record condition, and conversation; explain omissions.",
    availability: "interactive"
  },
  {
    id: "situation-to-strategy",
    number: 6,
    name: "Situation to Strategy",
    promise: "Signals become a falsifiable point of view and measurable intervention.",
    nextLeap: "Probe assumptions, compare alternatives, and feed measured outcomes back into the thesis.",
    availability: "interactive"
  },
  {
    id: "attention-stewardship",
    number: 7,
    name: "Attention Stewardship",
    promise: "The experience calmly identifies where this user can change an outcome now.",
    nextLeap: "Durable attention history with covered, not-mine, snooze, delegate, and challenge feedback.",
    availability: "interactive"
  },
  {
    id: "ambient-agent-continuity",
    number: 8,
    name: "Agents Helping You",
    promise: "Useful agents can watch, work, pause, and return results without pretending the user was present.",
    nextLeap: "Connect the same calm continuity surface to durable, authorized App Fabric agent operations.",
    availability: "interactive"
  }
] as const;

export const RECOVERY_CONTEXT: IxResolvedContext = {
  focusLabel: "Recovery readiness · current quarter",
  detail: "67% readiness · target 85% · four authorized operational signals",
  sourceCount: 4,
  evaluatedAt: "2026-08-09T09:20:00-07:00",
  freshness: "mixed",
  sources: [
    {
      sourceRef: "recovery-exercise-register",
      label: "Recovery exercise register",
      refreshedAt: "2026-08-09T09:12:00-07:00",
      freshness: "current",
      inspectable: true
    },
    {
      sourceRef: "continuity-scorecard",
      label: "Continuity scorecard",
      refreshedAt: "2026-07-17T17:00:00-07:00",
      freshness: "stale",
      inspectable: true
    },
    {
      sourceRef: "recovery-ownership-roster",
      label: "Recovery ownership roster",
      refreshedAt: "2026-08-08T16:30:00-07:00",
      freshness: "current",
      inspectable: true
    },
    {
      sourceRef: "external-handoff-status",
      label: "External handoff status",
      freshness: "unknown",
      inspectable: false
    }
  ],
  gaps: ["One readiness input is 23 days old", "One overdue exercise has no named owner"]
};

export const CALM_ATTENTION_CONTEXT: IxResolvedContext = {
  focusLabel: "Recovery advisory view · current quarter",
  detail: "Observer role · no continuity action is assigned to this user",
  sourceCount: 2,
  evaluatedAt: "2026-08-09T09:20:00-07:00",
  freshness: "mixed",
  sources: [
    {
      sourceRef: "recovery-ownership-roster",
      label: "Recovery ownership roster",
      refreshedAt: "2026-08-08T16:30:00-07:00",
      freshness: "current",
      inspectable: true
    },
    {
      sourceRef: "external-handoff-status",
      label: "External handoff status",
      freshness: "unknown",
      inspectable: false
    }
  ],
  gaps: ["External handoff detail remains unavailable and is outside this advisory role’s current ownership"]
};

const AUTHORIZED_SOURCE_REFS: Readonly<Record<string, string>> = {
  "recovery exercise register": "recovery-exercise-register",
  "exercise register": "recovery-exercise-register",
  "continuity scorecard": "continuity-scorecard",
  "scorecard": "continuity-scorecard",
  "recovery ownership roster": "recovery-ownership-roster",
  "ownership roster": "recovery-ownership-roster",
  "external handoff status": "external-handoff-status"
};

function sourceRefFor(label: string, observedAt: string) {
  const authorized = AUTHORIZED_SOURCE_REFS[label.toLowerCase()];
  if (authorized) {
    const resolved = RECOVERY_CONTEXT.sources.find(({ sourceRef }) => sourceRef === authorized);
    if (!resolved || resolved.freshness === "mixed") {
      throw new Error(`Authorized claim source ${authorized} has no individual context-ledger freshness.`);
    }
    return {
      sourceRef: resolved.sourceRef,
      label: resolved.label,
      kind: "authorized-context" as const,
      freshness: resolved.freshness,
      ...(resolved.refreshedAt ? { refreshedAt: resolved.refreshedAt } : {})
    };
  }
  return {
    sourceRef: `fixture:${label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")}`,
    label,
    kind: "deterministic-fixture" as const,
    freshness: "unknown" as const,
    observedAt: observedAt.toLowerCase().includes("unknown") ? "Observation time unknown" : observedAt
  };
}

function claimTrace(source: string, derivation: string, observedAt = "Aug 9, 2026 at 9:12 AM") {
  const sources = source.split(" + ").map((label) => sourceRefFor(label, observedAt));
  return { sources, derivation };
}

const BASE_REGIONS: readonly IxArtifactRegion[] = [
  {
    id: "changed",
    label: "Observation",
    title: "What changed",
    body: "Two recovery exercises moved past their planned dates this quarter. The most recent completed test remains within policy, but the operating picture weakened since the last readout.",
    kind: "observation",
    status: "ready",
    whyItMatters: "The timing changed during the current quarter.",
    evidence: {
      source: "Recovery exercise register",
      owner: "Service continuity",
      sourceUpdatedAt: "2026-08-09T09:12:00-07:00",
      retrievedAt: "2026-08-09T09:20:00-07:00",
      calculation: "Compare planned date with today where status is not complete.",
      limitations: ["This reference uses deterministic synthetic data."]
    },
    challengePrompt: "Could a recently completed exercise make this conclusion stale?"
  },
  {
    id: "attention",
    label: "Interpretation",
    title: "What needs attention",
    body: "The readiness input is 23 days old and one overdue exercise has no owner. Treat the 67% result as provisional until those two facts are refreshed.",
    kind: "interpretation",
    status: "stale",
    whyItMatters: "The score may not represent today’s operating conditions.",
    evidence: {
      source: "Continuity scorecard and ownership roster",
      owner: "Operations planning",
      sourceUpdatedAt: "2026-07-17T17:00:00-07:00",
      retrievedAt: "2026-08-09T09:20:00-07:00",
      calculation: "Weighted completion across four signals; two overdue of four exercises.",
      assumptions: ["The latest recorded statuses remain current."],
      limitations: ["One stale input", "One missing owner"]
    },
    challengePrompt: "What evidence would overturn this interpretation?"
  },
  {
    id: "move",
    label: "Recommendation",
    title: "Next useful move",
    body: "Confirm the missing owner, refresh the readiness input, and bring the two overdue tests into the next operating review as one coordinated recovery action.",
    kind: "recommendation",
    status: "ready",
    whyItMatters: "It converts a warning signal into bounded, reviewable work.",
    evidence: {
      source: "Same four visible operational signals",
      owner: "Service continuity",
      retrievedAt: "2026-08-09T09:20:00-07:00",
      assumptions: ["The current target remains 85%."],
      limitations: ["Suggestion only; no source action has been created."]
    },
    challengePrompt: "Is ownership really the most important intervention?"
  }
] as const;

const REDIRECT_REGIONS: Record<"continuity" | "roadblocks" | "decisions", readonly IxArtifactRegion[]> = {
  continuity: [
    {
      id: "continuity-refresh",
      label: "Rechecked observation",
      title: "The original continuity signal still holds",
      body: "The overdue exercises and stale readiness input remain the strongest explanation in the permitted source set. The redirected views stay in the brief as useful context.",
      kind: "observation",
      status: "ready",
      whyItMatters: "Returning to the original lens should recheck the conclusion without erasing useful work.",
      evidence: {
        source: "Recovery exercise register and continuity scorecard",
        owner: "Service continuity",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["This reference uses deterministic synthetic data."]
      }
    },
    {
      id: "continuity-next-test",
      label: "Rechecked recommendation",
      title: "Refresh the uncertain input before changing course",
      body: "Keep the current recovery move, but refresh the 23-day-old readiness input before treating the present 67% score as settled.",
      kind: "recommendation",
      status: "ready",
      whyItMatters: "It preserves valid work while testing the most important uncertainty.",
      evidence: {
        source: "Current working brief and continuity scorecard",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["No action has been created."]
      }
    }
  ],
  roadblocks: [
    {
      id: "roadblock-handoff",
      label: "New observation",
      title: "External handoff is now the schedule risk",
      body: "Three dependencies sit between the overdue exercises and completion. The external service handoff has no committed date, making it the strongest visible roadblock.",
      kind: "observation",
      status: "ready",
      whyItMatters: "Completing internal preparation alone will not recover the date.",
      evidence: {
        source: "Recovery dependency register",
        owner: "Service continuity",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["External team capacity is not available in this fixture."]
      }
    },
    {
      id: "roadblock-move",
      label: "New recommendation",
      title: "Secure the handoff before changing the forecast",
      body: "Name the handoff owner and obtain a committed date before adjusting the quarter forecast. The original three brief sections remain unchanged.",
      kind: "recommendation",
      status: "ready",
      whyItMatters: "It tests the binding constraint rather than adding general escalation.",
      evidence: {
        source: "Recovery dependency register",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["No action has been created."]
      }
    }
  ],
  decisions: [
    {
      id: "decision-choice",
      label: "New hypothesis",
      title: "One decision now shapes the quarter",
      body: "Recovery remains credible only if leadership either protects the current scope through the external handoff or deliberately moves that dependency into the next quarter.",
      kind: "hypothesis",
      status: "ready",
      whyItMatters: "The two paths have different operating and reporting consequences.",
      evidence: {
        source: "Quarter target and recovery dependency register",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        assumptions: ["The remaining internal work can finish this quarter."],
        limitations: ["External handoff timing is unresolved."]
      }
    },
    {
      id: "decision-move",
      label: "New recommendation",
      title: "Prepare a two-option decision note",
      body: "Show impact, accountable owner, evidence needed, and latest responsible date for both choices. Preserve the existing analysis as the evidence base.",
      kind: "recommendation",
      status: "ready",
      whyItMatters: "It turns ambiguity into a bounded choice without pretending approval exists.",
      evidence: {
        source: "Current working brief",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["Decision authority remains with the responsible leaders."]
      }
    }
  ]
};

type EventPayload<T extends IxReferencePlaybackEvent["type"]> =
  Extract<IxReferencePlaybackEvent, { type: T }> extends infer Event
    ? Event extends IxReferencePlaybackEvent
      ? Omit<Event, "schemaVersion" | "eventId" | "playbackId" | "sequence" | "occurredAt">
      : never
    : never;

function event<T extends IxReferencePlaybackEvent["type"]>(
  playbackId: string,
  sequence: number,
  value: EventPayload<T>
): Extract<IxReferencePlaybackEvent, { type: T }> {
  const payload = value as Record<string, unknown>;
  return {
    schemaVersion: IX_REFERENCE_PLAYBACK_SCHEMA,
    eventId: `${playbackId}:${sequence}`,
    playbackId,
    sequence,
    occurredAt: `2026-08-09T09:20:${String(sequence).padStart(2, "0")}-07:00`,
    ...payload
  } as Extract<IxReferencePlaybackEvent, { type: T }>;
}

function artifact(
  revision: number,
  regions: readonly IxArtifactRegion[],
  changedRegionIds: readonly string[],
  status: IxArtifact["status"] = "partial"
): IxArtifact {
  return {
    artifactId: "recovery-working-brief",
    artifactType: "pds.ix.working_brief@1",
    revision,
    status,
    title: "Recovery readiness working brief",
    changedRegionIds,
    regions
  };
}

function reconcileRegions(
  current: readonly IxArtifactRegion[],
  updates: readonly IxArtifactRegion[]
): readonly IxArtifactRegion[] {
  const order: string[] = [];
  const byId = new Map<string, IxArtifactRegion>();

  for (const region of current) {
    if (!byId.has(region.id)) order.push(region.id);
    byId.set(region.id, region);
  }

  for (const region of updates) {
    if (!byId.has(region.id)) order.push(region.id);
    byId.set(region.id, region);
  }

  return order.map((regionId) => byId.get(regionId)!);
}

export function buildAnalyzeWhySchedule(
  focus: "continuity" | "roadblocks" | "decisions",
  preservedArtifact?: IxArtifact
): ScheduledIxReferencePlaybackEvent[] {
  const playbackId = `analyze-${focus}-${preservedArtifact?.revision ?? 0}`;
  let sequence = 0;
  const scheduled: ScheduledIxReferencePlaybackEvent[] = [];
  const add = (afterMs: number, next: IxReferencePlaybackEvent) => scheduled.push({ afterMs, event: next });
  const nextSequence = () => ++sequence;

  add(0, event(playbackId, nextSequence(), {
    type: "playback_started",
    intentLabel: focus === "continuity" ? "Analyze recovery readiness" : `Redirect analysis to ${focus}`
  }));
  add(260, event(playbackId, nextSequence(), {
    type: "context_displayed",
    context: RECOVERY_CONTEXT
  }));
  add(340, event(playbackId, nextSequence(), {
    type: "display_phase_changed",
    phase: "resolving",
    label: "Using the metric already in view",
    detail: "Resolving four permitted operational signals."
  }));
  add(380, event(playbackId, nextSequence(), {
    type: "display_phase_changed",
    phase: "interpreting",
    label: focus === "continuity" ? "Finding what changed" : `Reconsidering ${focus}`,
    detail: "Comparing dates, ownership, freshness, and current status."
  }));

  if (!preservedArtifact) {
    BASE_REGIONS.forEach((region, index) => {
      const revision = index + 1;
      add(480, event(playbackId, nextSequence(), {
        type: "display_phase_changed",
        phase: index === 2 ? "checking" : "composing",
        label: index === 2 ? "Checking the proposed move" : `Building ${region.title.toLowerCase()}`,
        detail: index === 2 ? "Verifying source, derivation, and known limits." : "Publishing a usable section as soon as it is ready."
      }));
      add(160, event(playbackId, nextSequence(), {
        type: "fixture_revision_displayed",
        artifact: artifact(
          revision,
          BASE_REGIONS.slice(0, index + 1),
          [region.id],
          index === BASE_REGIONS.length - 1 ? "ready" : "partial"
        )
      }));
    });
  } else {
    const additions = REDIRECT_REGIONS[focus];
    additions.forEach((region, index) => {
      add(420, event(playbackId, nextSequence(), {
        type: "display_phase_changed",
        phase: index === additions.length - 1 ? "checking" : "composing",
        label: index === 0 ? `Adding the ${focus} view` : "Checking what changed",
        detail: "Keeping completed regions and adding only the new perspective."
      }));
      add(160, event(playbackId, nextSequence(), {
        type: "fixture_revision_displayed",
        artifact: artifact(
          preservedArtifact.revision + index + 1,
          reconcileRegions(preservedArtifact.regions, additions.slice(0, index + 1)),
          [region.id],
          index === additions.length - 1 ? "ready" : "revising"
        )
      }));
    });
  }

  add(260, event(playbackId, nextSequence(), {
    type: "playback_finished",
    outcome: "completed",
    message: focus === "continuity"
      ? "The brief is editable; sources and calculations remain one gesture away."
      : `The ${focus} view was added without replacing the earlier brief.`
  }));

  return scheduled;
}

export function buildCancelledReferenceSchedule(): ScheduledIxReferencePlaybackEvent[] {
  const schedule = buildAnalyzeWhySchedule("continuity");
  const firstArtifactIndex = schedule.findIndex(({ event: candidate }) =>
    candidate.type === "fixture_revision_displayed"
  );
  if (firstArtifactIndex < 0) {
    throw new Error("The cancelled reference requires one usable artifact revision.");
  }

  const partial = schedule.slice(0, firstArtifactIndex + 1);
  const last = partial[partial.length - 1]?.event;
  if (!last) {
    throw new Error("The cancelled reference requires an initialized run.");
  }
  return [
    ...partial,
    {
      afterMs: 0,
      event: event(last.playbackId, last.sequence + 1, {
        type: "playback_finished",
        outcome: "cancelled",
        message: "Stopped — the work already produced remains available."
      })
    }
  ];
}

function answerRegions(question: string): readonly IxArtifactRegion[] {
  const normalized = question.toLowerCase();
  const conclusion = normalized.includes("challenge") || normalized.includes("assumption")
    ? "The weakest assumption is that the 23-day-old readiness input still represents today. Refreshing it could move the score in either direction."
    : normalized.includes("outlook") || normalized.includes("change")
      ? "The outlook changes most if the two overdue exercises gain accountable owners and the stale readiness input is refreshed."
      : "The decline is mainly explained by two recovery exercises moving past their dates. Missing ownership makes recovery less credible; stale input adds uncertainty.";

  return [
    {
      id: "answer-conclusion",
      label: "Interpretation",
      title: "Direct answer",
      body: conclusion,
      kind: "interpretation",
      status: "ready",
      whyItMatters: "This is the current best explanation, not a hidden certainty score.",
      evidence: {
        source: "Recovery working brief and four resolved signals",
        retrievedAt: "2026-08-09T09:20:00-07:00",
        limitations: ["One stale input", "One missing owner"]
      },
      challengePrompt: "What would make this answer wrong?"
    },
    {
      id: "answer-calculation",
      label: "Calculation",
      title: "How the 67% was formed",
      body: "The score is weighted completion across four current-quarter continuity signals. Two overdue exercises account for most of the 15-point decline.",
      kind: "observation",
      status: "ready",
      whyItMatters: "The number can be inspected rather than accepted as AI output.",
      evidence: {
        source: "Continuity scorecard",
        sourceUpdatedAt: "2026-07-17T17:00:00-07:00",
        calculation: "Weighted completed signals ÷ total eligible signal weight × 100.",
        limitations: ["One input has not been refreshed for 23 days."]
      }
    },
    {
      id: "answer-next-test",
      label: "Recommendation",
      title: "Best next test",
      body: "Refresh the stale input before treating the decline as settled, then test whether assigning the missing owner changes completion timing.",
      kind: "recommendation",
      status: "ready",
      whyItMatters: "It tests the uncertainty before escalating the conclusion.",
      evidence: {
        source: "Current answer and working brief",
        assumptions: ["The source registers can be refreshed without changing scope."],
        limitations: ["No action has been created."]
      }
    }
  ];
}

export function buildConversationSchedule(
  question: string,
  runOrdinal = 1
): ScheduledIxReferencePlaybackEvent[] {
  const normalizedId = question.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 28) || "question";
  const playbackId = `conversation-${normalizedId}-${runOrdinal}`;
  const regions = answerRegions(question);
  let sequence = 0;
  const nextSequence = () => ++sequence;
  const scheduled: ScheduledIxReferencePlaybackEvent[] = [
    { afterMs: 0, event: event(playbackId, nextSequence(), { type: "playback_started", intentLabel: question }) },
    { afterMs: 220, event: event(playbackId, nextSequence(), {
      type: "context_displayed",
      context: RECOVERY_CONTEXT
    }) },
    { afterMs: 330, event: event(playbackId, nextSequence(), {
      type: "display_phase_changed",
      phase: "interpreting",
      label: "Connecting this question to the metric",
      detail: "Using the same quarter, sources, and working brief."
    }) },
    { afterMs: 440, event: event(playbackId, nextSequence(), {
      type: "fixture_revision_displayed",
      artifact: {
        artifactId: "recovery-contextual-answer",
        artifactType: "pds.ix.contextual_answer@1",
        revision: 1,
        status: "partial",
        title: "Contextual answer",
        changedRegionIds: [regions[0].id],
        regions: regions.slice(0, 1)
      }
    }) },
    { afterMs: 440, event: event(playbackId, nextSequence(), {
      type: "display_phase_changed",
      phase: "checking",
      label: "Checking the number and the weak assumption",
      detail: "Keeping calculation and uncertainty separate from the conclusion."
    }) },
    { afterMs: 200, event: event(playbackId, nextSequence(), {
      type: "fixture_revision_displayed",
      artifact: {
        artifactId: "recovery-contextual-answer",
        artifactType: "pds.ix.contextual_answer@1",
        revision: 2,
        status: "partial",
        title: "Contextual answer",
        changedRegionIds: [regions[1].id],
        regions: regions.slice(0, 2)
      }
    }) },
    { afterMs: 420, event: event(playbackId, nextSequence(), {
      type: "fixture_revision_displayed",
      artifact: {
        artifactId: "recovery-contextual-answer",
        artifactType: "pds.ix.contextual_answer@1",
        revision: 3,
        status: "ready",
        title: "Contextual answer",
        changedRegionIds: [regions[2].id],
        regions
      }
    }) },
    { afterMs: 240, event: event(playbackId, nextSequence(), {
      type: "playback_finished",
      outcome: "completed",
      message: "The answer is ready to inspect, challenge, or add to the working brief."
    }) }
  ];
  return scheduled;
}

function recipeArtifact(
  artifactId: string,
  revision: number,
  title: string,
  referenceRecipe: IxReferenceRecipeArtifact,
  changedRegionIds: readonly string[],
  status: IxArtifact["status"]
): IxArtifact {
  return {
    artifactId,
    artifactType: `pds.ix.${referenceRecipe.kind}@1`,
    revision,
    status,
    title,
    changedRegionIds,
    regions: [],
    referenceRecipe
  };
}

function buildRecipeSchedule(
  playbackId: string,
  intentLabel: string,
  phaseOne: { label: string; detail: string },
  phaseTwo: { label: string; detail: string },
  partial: IxArtifact,
  complete: IxArtifact,
  message: string,
  resolvedContext: IxResolvedContext = RECOVERY_CONTEXT
): ScheduledIxReferencePlaybackEvent[] {
  let sequence = 0;
  const nextSequence = () => ++sequence;
  return [
    { afterMs: 0, event: event(playbackId, nextSequence(), { type: "playback_started", intentLabel }) },
    { afterMs: 180, event: event(playbackId, nextSequence(), { type: "context_displayed", context: resolvedContext }) },
    { afterMs: 300, event: event(playbackId, nextSequence(), {
      type: "display_phase_changed",
      phase: "interpreting",
      ...phaseOne
    }) },
    { afterMs: 420, event: event(playbackId, nextSequence(), { type: "fixture_revision_displayed", artifact: partial }) },
    { afterMs: 360, event: event(playbackId, nextSequence(), {
      type: "display_phase_changed",
      phase: "checking",
      ...phaseTwo
    }) },
    { afterMs: 420, event: event(playbackId, nextSequence(), { type: "fixture_revision_displayed", artifact: complete }) },
    { afterMs: 220, event: event(playbackId, nextSequence(), {
      type: "playback_finished",
      outcome: "completed",
      message
    }) }
  ];
}

export function buildAdaptiveCompositionSchedule(
  focus: "operating-picture" | "decisions" = "operating-picture",
  pinnedIds: readonly string[] = [],
  startRevision = 1
): ScheduledIxReferencePlaybackEvent[] {
  const baseBlocks = [
    {
      id: "readiness-signal",
      kind: "signal" as const,
      title: "Recovery readiness is below target",
      body: "67% readiness is 18 points below the current-quarter target.",
      whyIncluded: "It is the material signal on the dashboard you opened.",
      trace: claimTrace("Continuity scorecard", "Current readiness 67% minus target 85% equals an 18-point gap.", "Jul 17, 2026 at 5:00 PM")
    },
    {
      id: "quarter-comparison",
      kind: "comparison" as const,
      title: "The quarter weakened",
      body: "Two exercises became overdue; one supporting input is 23 days old.",
      whyIncluded: "The comparison explains the movement instead of repeating the score.",
      trace: claimTrace("Recovery exercise register", "Compare planned dates with Aug 9 where status is not complete.")
    },
    {
      id: "decision-move",
      kind: "move" as const,
      title: "Resolve ownership before changing the forecast",
      body: "Name the missing owner and refresh the stale input before the next operating review.",
      whyIncluded: "This is the smallest move that can change the outcome now.",
      trace: claimTrace("Ownership roster + scorecard", "Select the unresolved condition that blocks accountable follow-through.", "Aug 8, 2026 at 4:30 PM")
    }
  ];
  const ordered = focus === "decisions"
    ? [baseBlocks[2], baseBlocks[0], baseBlocks[1]]
    : baseBlocks;
  const pinned = ordered.filter(({ id }) => pinnedIds.includes(id));
  const blocks = [...pinned, ...ordered.filter(({ id }) => !pinnedIds.includes(id))];
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "adaptive-composition",
    focus,
    blocks: blocks.slice(0, 2),
    changeSummary: focus === "decisions"
      ? ["Moved the decision-ready action first", "Kept your selected items in view"]
      : ["Started with the signal already in view"]
  };
  const completeRecipe: IxReferenceRecipeArtifact = {
    ...partialRecipe,
    blocks,
    changeSummary: focus === "decisions"
      ? ["Moved the decision-ready action first", "Kept evidence context", "Kept your selected items in view"]
      : ["Kept the dashboard signal first", "Added comparison and one useful move"]
  };
  const playbackId = `composition-${focus}-${pinnedIds.join("-") || "none"}-${startRevision}`;
  return buildRecipeSchedule(
    playbackId,
    `Compose the recovery view for ${focus}`,
    { label: "Choosing the useful shape", detail: "Using the current focus, material change, and kept items." },
    { label: "Checking what moved and what stayed", detail: "Explaining the composition without hiding source context." },
    recipeArtifact("recovery-composition", startRevision, "Purpose-shaped recovery view", partialRecipe, partialRecipe.blocks.map(({ id }) => id), "partial"),
    recipeArtifact("recovery-composition", startRevision + 1, "Purpose-shaped recovery view", completeRecipe, completeRecipe.blocks.slice(partialRecipe.blocks.length).map(({ id }) => id), "ready"),
    "The view is ready; the change summary and source context remain visible."
  );
}

export function buildWorkingGoalPlanSchedule(revised = false, startRevision = 1): ScheduledIxReferencePlaybackEvent[] {
  const firstSteps = [
    {
      id: "confirm-owner",
      outcome: "Confirm the accountable recovery owner",
      state: "working" as const,
      trace: claimTrace("Recovery ownership roster", "The exercise remains ownerless in the current authorized roster.", "Aug 8, 2026 at 4:30 PM")
    },
    {
      id: "refresh-input",
      outcome: "Refresh the 23-day-old readiness input",
      state: "next" as const,
      checkpoint: "Use the source-system refresh, not a copied number",
      trace: claimTrace("Continuity scorecard", "Aug 9 evaluation date minus Jul 17 source refresh equals 23 days.", "Jul 17, 2026 at 5:00 PM")
    }
  ];
  const finalSteps = revised
    ? [
        firstSteps[0],
        {
          id: "secure-handoff",
          outcome: "Secure an external handoff date",
          state: "next" as const,
          checkpoint: "Named external owner and committed date",
          trace: claimTrace("External handoff status", "The handoff has no inspectable committed date.", "Refresh time unknown")
        },
        firstSteps[1],
        {
          id: "review",
          outcome: "Review the recovery outlook",
          state: "required" as const,
          checkpoint: "Human operating review",
          trace: claimTrace("Recovery operating cadence", "This checkpoint is required by the deterministic fixture plan.")
        }
      ]
    : [
        ...firstSteps,
        {
          id: "coordinate-tests",
          outcome: "Bring both overdue tests into one recovery action",
          state: "next" as const,
          trace: claimTrace("Recovery exercise register", "Group the two overdue tests because they share the current-quarter outcome.")
        },
        {
          id: "review",
          outcome: "Review the recovery outlook",
          state: "required" as const,
          checkpoint: "Human operating review",
          trace: claimTrace("Recovery operating cadence", "This checkpoint is required by the deterministic fixture plan.")
        }
      ];
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "working-goal-plan",
    goal: "Restore credible recovery readiness to at least 85%",
    revisionNote: revised ? "Future work changed: external handoff is now explicit; owner confirmation is still in progress." : "First useful path from the current operating picture.",
    steps: finalSteps.slice(0, 2),
    preservedWork: revised ? ["Owner confirmation remains in progress", "Existing source notes", "Required human review"] : []
  };
  const completeRecipe: IxReferenceRecipeArtifact = { ...partialRecipe, steps: finalSteps };
  return buildRecipeSchedule(
    `goal-plan-${revised ? "revised" : "initial"}-${startRevision}`,
    revised ? "Revise the path without changing the goal" : "Build a path to recovery readiness",
    { label: revised ? "Finding the blocked path" : "Laying out outcome-based steps", detail: "Separating adaptive work from the required human checkpoint." },
    { label: "Checking the plan revision", detail: "Preserving existing work and naming what changed." },
    recipeArtifact("recovery-goal-plan", startRevision, "Recovery readiness goal plan", partialRecipe, revised ? ["secure-handoff"] : partialRecipe.steps.map(({ id }) => id), "partial"),
    recipeArtifact("recovery-goal-plan", startRevision + 1, "Recovery readiness goal plan", completeRecipe, completeRecipe.steps.slice(partialRecipe.steps.length).map(({ id }) => id), "ready"),
    revised ? "Future work changed; the goal, in-progress ownership step, and notes remain." : "A revisable path is ready; the operating review remains explicit."
  );
}

export function buildAdaptiveInformationLensSchedule(
  lens: "executive-read" | "operator-read" = "executive-read",
  startRevision = 1,
  keptIds: readonly string[] = []
): ScheduledIxReferencePlaybackEvent[] {
  const fullRecordFields = [
    { id: "score", label: "Readiness", value: "67%", reason: "Current reported result", trace: claimTrace("Continuity scorecard", "Weighted completed signal value divided by eligible signal weight.", "Jul 17, 2026 at 5:00 PM") },
    { id: "target", label: "Target", value: "85%", reason: "Current-quarter commitment", trace: claimTrace("Quarter objective register", "Approved current-quarter target.") },
    { id: "overdue", label: "Overdue exercises", value: "2", reason: "Main driver of the decline", trace: claimTrace("Recovery exercise register", "Count planned dates before Aug 9 with status not complete.") },
    { id: "owner", label: "Accountable owner", value: "One missing", reason: "Required to recover the date", trace: claimTrace("Recovery ownership roster", "One overdue exercise has no matched accountable owner.", "Aug 8, 2026 at 4:30 PM") },
    { id: "source-age", label: "Oldest source", value: "23 days", reason: "Makes the result provisional", trace: claimTrace("Continuity scorecard", "Aug 9 evaluation minus Jul 17 refresh.", "Jul 17, 2026 at 5:00 PM") },
    { id: "handoff", label: "External handoff", value: "Date unresolved", reason: "Operator sequencing dependency", trace: claimTrace("External handoff status", "No committed date is available in this fixture.", "Refresh time unknown") },
    { id: "quarter", label: "Reporting quarter", value: "Q3 2026", reason: "Record scope", trace: claimTrace("Quarter objective register", "Current reporting period.") },
    { id: "service", label: "Service", value: "Member access", reason: "Affected service", trace: claimTrace("Service catalog", "Service linked to the recovery record.") },
    { id: "tier", label: "Criticality tier", value: "Tier 1", reason: "Business impact context", trace: claimTrace("Service criticality register", "Approved service tier.") },
    { id: "rto", label: "Recovery objective", value: "4 hours", reason: "Required operating threshold", trace: claimTrace("Continuity plan", "Current approved recovery-time objective.") },
    { id: "last-test", label: "Last completed test", value: "Jun 18, 2026", reason: "Most recent proof", trace: claimTrace("Recovery exercise register", "Most recent exercise with complete status.") },
    { id: "next-test", label: "Next planned test", value: "Aug 14, 2026", reason: "Upcoming checkpoint", trace: claimTrace("Recovery exercise register", "Earliest future planned exercise date.") },
    { id: "open-actions", label: "Open actions", value: "4", reason: "Remaining work", trace: claimTrace("Recovery action register", "Count actions not complete or cancelled.") },
    { id: "risk", label: "Residual risk", value: "Moderate", reason: "Current risk view", trace: claimTrace("Continuity risk register", "Current approved residual-risk label.") },
    { id: "dependency", label: "Critical dependency", value: "External identity service", reason: "Binding dependency", trace: claimTrace("Recovery dependency register", "Highest-impact unresolved dependency.") },
    { id: "review-date", label: "Next operating review", value: "Aug 12, 2026", reason: "Decision timing", trace: claimTrace("Operations calendar", "Next scheduled recovery operating review.") },
    { id: "evidence-count", label: "Evidence items", value: "4", reason: "Basis of this read", trace: claimTrace("Resolved context", "Count browser-safe source summaries used by the fixture.") },
    { id: "record-status", label: "Record status", value: "Provisional", reason: "One source is stale", trace: claimTrace("Resolved context", "Mixed freshness plus one unknown source yields provisional status.") }
  ];
  const visibleIds = lens === "executive-read"
    ? ["score", "target", "overdue", "owner"]
    : ["overdue", "owner", "source-age", "handoff", "score"];
  const purposeIds = visibleIds.filter((id) => !keptIds.includes(id));
  const selectedIds = [...keptIds, ...purposeIds];
  const visibleFields = selectedIds.map((id) => fullRecordFields.find((field) => field.id === id)).filter((field) => field !== undefined);
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "adaptive-information-lens",
    lens,
    objective: lens === "executive-read" ? "Understand whether intervention is needed" : "Prepare the next recovery action",
    visibleFields: visibleFields.slice(0, 3),
    fullRecordFields,
    omissionExplanation: "Nothing was removed from the record. Lower-priority fields remain in Full record."
  };
  const completeRecipe: IxReferenceRecipeArtifact = { ...partialRecipe, visibleFields };
  return buildRecipeSchedule(
    `information-lens-${lens}-${startRevision}-${keptIds.join("-") || "none"}`,
    `Shape the recovery record for the ${lens}`,
    { label: "Finding what this task needs", detail: "Ranking canonical fields by role, objective, and record condition." },
    { label: "Checking what was omitted", detail: "Keeping a complete record fallback and explaining the lens." },
    recipeArtifact("recovery-information-lens", startRevision, "Recovery record lens", partialRecipe, partialRecipe.visibleFields.map(({ id }) => id), "partial"),
    recipeArtifact("recovery-information-lens", startRevision + 1, "Recovery record lens", completeRecipe, completeRecipe.visibleFields.slice(partialRecipe.visibleFields.length).map(({ id }) => id), "ready"),
    "The task-shaped view is ready; the full canonical record is still available."
  );
}

export function buildSituationToStrategySchedule(
  challenge?: IxStrategyChallenge,
  startRevision = 1,
  priorThesis?: { thesis: string; reasoning: string }
): ScheduledIxReferencePlaybackEvent[] {
  const baseThesis = "Recovery readiness is an ownership-and-coordination problem before it is a capacity problem.";
  const thesisByCategory = {
    "source-freshness": "The score may be lagging reality; refreshed evidence should decide whether ownership is still the main constraint.",
    capacity: "Capacity remains an unproven alternative explanation; test it before treating ownership as the main constraint.",
    "causal-link": "The observed ownership gap and readiness decline move together, but the current fixture does not yet prove causation."
  } as const;
  const counterviewByCategory = {
    "source-freshness": "A refreshed source could show the operating picture has already improved.",
    capacity: "Insufficient recovery capacity—not ownership—could be the binding constraint.",
    "causal-link": "A third condition could explain both the missing ownership and the readiness decline."
  } as const;
  const revisionByCategory = {
    "source-freshness": "Reduced the ownership conclusion, elevated source freshness, and retained the proposed test.",
    capacity: "Reduced the ownership conclusion and elevated capacity as the next falsification test.",
    "causal-link": "Changed the causal claim to correlation and added a discriminating test."
  } as const;
  const challengeAssumption = challenge?.category === "capacity"
    ? {
        id: "capacity-evidence",
        statement: "Available recovery capacity is sufficient for the proposed dates.",
        test: "Compare assigned recovery demand with available skilled capacity.",
        trace: claimTrace("Recovery work plan", "This is a deterministic challenge input; capacity data is not present.", "Refresh time unknown")
      }
    : challenge?.category === "causal-link"
      ? {
          id: "causal-test",
          statement: "Ownership is the condition that caused the readiness decline.",
          test: "Compare otherwise similar exercises with and without named ownership.",
          trace: claimTrace("Recovery exercise register + ownership roster", "A comparison is required; co-occurrence alone is not causation.")
        }
      : {
          id: "freshness",
          statement: "The stale readiness input has not materially changed.",
          test: "Refresh it before accepting the thesis.",
          trace: claimTrace("Continuity scorecard", "This source is 23 days old and may overturn the thesis.", "Jul 17, 2026 at 5:00 PM")
        };
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "situation-to-strategy",
    thesis: challenge ? thesisByCategory[challenge.category] : baseThesis,
    confidence: "moderate",
    thesisTrace: claimTrace("Recovery exercise register + ownership roster", "Compare overdue work, named ownership, and the unresolved external handoff."),
    signals: ["Two exercises overdue", "One missing owner", "One 23-day-old input"],
    assumptions: [
      {
        id: "capacity",
        statement: "Current teams have enough capacity once ownership is clear.",
        test: "Compare scheduled work with available recovery capacity.",
        trace: claimTrace("Recovery work plan", "This is a hypothesis; capacity data is not present in the fixture.", "Refresh time unknown")
      }
    ],
    intervention: "Name the missing owner and secure the handoff date as one bounded recovery action.",
    interventionTrace: claimTrace("Ownership roster + external handoff status", "Address the two unresolved conditions most directly connected to follow-through."),
    measure: "Both overdue tests regain committed dates and readiness reaches at least 85%.",
    counterview: challenge ? counterviewByCategory[challenge.category] : undefined,
    priorThesis: challenge ? priorThesis ?? {
      thesis: baseThesis,
      reasoning: "The first read emphasized two overdue exercises, one missing owner, and the unresolved handoff."
    } : undefined,
    revisionChange: challenge ? revisionByCategory[challenge.category] : undefined,
    challenge
  };
  const completeRecipe: IxReferenceRecipeArtifact = {
    ...partialRecipe,
    assumptions: [
      ...partialRecipe.assumptions,
      challengeAssumption
    ]
  };
  return buildRecipeSchedule(
    `situation-strategy-${challenge?.category ?? "initial"}-${startRevision}`,
    challenge ? `Challenge the recovery thesis: ${challenge.text}` : "Turn the recovery picture into a testable strategy",
    { label: "Forming a point of view", detail: "Separating observed signals from a causal hypothesis." },
    { label: "Trying to disprove the thesis", detail: "Naming assumptions, counterview, intervention, and measure." },
    recipeArtifact("recovery-strategy", startRevision, "Recovery strategy thesis", partialRecipe, challenge ? ["thesis", `challenge-${challenge.category}`] : ["thesis", "capacity"], "partial"),
    recipeArtifact("recovery-strategy", startRevision + 1, "Recovery strategy thesis", completeRecipe, [challengeAssumption.id], "ready"),
    "The thesis is ready to challenge; its assumptions and success measure are explicit."
  );
}

export function buildAttentionStewardshipSchedule(empty = false, startRevision = 1): ScheduledIxReferencePlaybackEvent[] {
  const items = empty ? [] : [
    {
      id: "missing-owner",
      title: "One recovery exercise needs an owner",
      reason: "It is overdue and no one can currently change its date.",
      usefulMove: "Assign or identify the accountable owner.",
      whyMe: "You are viewing the continuity portfolio and can route ownership clarification.",
      priorityDerivation: "Material service + overdue date + missing owner + useful move available.",
      state: "open" as const,
      trace: claimTrace("Recovery exercise register + ownership roster", "Match overdue exercises to the current accountable-owner roster.", "Aug 8, 2026 at 4:30 PM")
    },
    {
      id: "stale-input",
      title: "Refresh the readiness input before the review",
      reason: "The 23-day-old input can change the conclusion.",
      usefulMove: "Request a source-system refresh.",
      whyMe: "The next operating read is in your current fixture context.",
      priorityDerivation: "Decision-relevant source + stale for 23 days + review approaching.",
      state: "open" as const,
      trace: claimTrace("Continuity scorecard + operations calendar", "Compare source refresh date with evaluation date and next review.", "Jul 17, 2026 at 5:00 PM")
    }
  ];
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "attention-stewardship",
    evaluatedCount: 12,
    items: items.slice(0, 1),
    emptyMessage: empty
      ? "No item is assigned to this advisory view. External handoff detail remains unavailable and outside this role’s current ownership."
      : undefined
  };
  const completeRecipe: IxReferenceRecipeArtifact = { ...partialRecipe, items };
  return buildRecipeSchedule(
    `attention-${empty ? "calm" : "material"}-${startRevision}`,
    empty ? "Check whether anything needs attention" : "Find where this user can change the outcome",
    { label: "Looking across the operating picture", detail: "Testing materiality, ownership, timing, and ability to act." },
    { label: "Checking whether this is truly yours", detail: "Avoiding a noisy list of every warning." },
    recipeArtifact("recovery-attention", startRevision, "What needs my attention", partialRecipe, items.slice(0, 1).map(({ id }) => id), "partial"),
    recipeArtifact("recovery-attention", startRevision + 1, "What needs my attention", completeRecipe, items.slice(partialRecipe.items.length).map(({ id }) => id), "ready"),
    empty ? "No owned action is ready; the distinct advisory context and its unresolved handoff gap remain visible." : "Two bounded items are ready to review, correct, or defer.",
    empty ? CALM_ATTENTION_CONTEXT : RECOVERY_CONTEXT
  );
}

export const AMBIENT_TUNING_CORPUS: IxAmbientTuningCorpus = {
  windowStart: "2026-07-11T00:00:00-07:00",
  windowEnd: "2026-08-09T23:59:59-07:00",
  timezone: "America/Los_Angeles",
  formula: "Filter by relevance; defer qualifying quiet-hour events; group remaining events by local day for daily cadence or seven-day window for weekly cadence.",
  events: [
    { id: "tune-01", occurredAt: "2026-07-12T08:20:00-07:00", material: true, inQuietHours: false, label: "Readiness moved 4 points" },
    { id: "tune-02", occurredAt: "2026-07-14T19:10:00-07:00", material: false, inQuietHours: true, label: "Exercise note added" },
    { id: "tune-03", occurredAt: "2026-07-18T15:40:00-07:00", material: true, inQuietHours: false, label: "Exercise became overdue" },
    { id: "tune-04", occurredAt: "2026-07-21T06:35:00-07:00", material: true, inQuietHours: true, label: "Owner reference removed" },
    { id: "tune-05", occurredAt: "2026-07-22T11:05:00-07:00", material: false, inQuietHours: false, label: "Evidence description changed" },
    { id: "tune-06", occurredAt: "2026-07-28T13:30:00-07:00", material: false, inQuietHours: false, label: "Review attendee updated" },
    { id: "tune-07", occurredAt: "2026-07-31T20:15:00-07:00", material: true, inQuietHours: true, label: "External handoff lost its date" },
    { id: "tune-08", occurredAt: "2026-08-02T09:00:00-07:00", material: false, inQuietHours: false, label: "Draft note prepared" },
    { id: "tune-09", occurredAt: "2026-08-05T16:45:00-07:00", material: true, inQuietHours: false, label: "Readiness moved below 70%" },
    { id: "tune-10", occurredAt: "2026-08-08T17:50:00-07:00", material: false, inQuietHours: false, label: "Roster timestamp advanced" },
    { id: "tune-11", occurredAt: "2026-08-09T08:55:00-07:00", material: true, inQuietHours: false, label: "Readiness reached 67%" }
  ]
};

export function previewAmbientTuning(
  tuning: IxAmbientTuning,
  corpus: IxAmbientTuningCorpus = AMBIENT_TUNING_CORPUS
) {
  const qualified = corpus.events.filter((candidate) => tuning.relevance === "operational" || candidate.material);
  const deferred = tuning.quiet ? qualified.filter(({ inQuietHours }) => inQuietHours) : [];
  const deliverable = qualified.filter((candidate) => !deferred.some(({ id }) => id === candidate.id));
  const windowStart = Date.parse(corpus.windowStart);
  const deliveryBuckets = new Set(deliverable.map(({ occurredAt }) => tuning.cadence === "daily"
    ? occurredAt.slice(0, 10)
    : String(Math.floor((Date.parse(occurredAt) - windowStart) / (7 * 24 * 60 * 60 * 1000)))));
  return {
    qualified: qualified.length,
    deferred: deferred.length,
    deliveries: deliveryBuckets.size,
    eventCount: corpus.events.length,
    windowStart: corpus.windowStart,
    windowEnd: corpus.windowEnd,
    timezone: corpus.timezone,
    formula: corpus.formula
  };
}

export function buildAmbientAgentContinuitySchedule(): ScheduledIxReferencePlaybackEvent[] {
  const partialRecipe: IxReferenceRecipeArtifact = {
    kind: "ambient-agent-continuity",
    capabilityName: "Recovery assurance",
    accountableOwner: "Service continuity",
    sinceAway: [
      { id: "readiness-moved", summary: "Recovery readiness moved from 70% to 67%", trace: claimTrace("Continuity scorecard", "Compare the last observed score with the current fixture score.", "Aug 9, 2026 at 8:55 AM") },
      { id: "register-newer", summary: "The exercise register now reports data as of 9:12 AM", trace: claimTrace("Recovery exercise register", "Observe the source timestamp; this capability did not write the source.") }
    ],
    currentReveal: [],
    workstreams: [
      {
        id: "recovery-watch",
        name: "Readiness movement",
        purpose: "Watch material readiness movement",
        state: "working",
        resumeState: "working",
        update: "Comparing two overdue exercises with current ownership.",
        trace: claimTrace("Exercise register + ownership roster", "Join overdue exercise references to authorized owner references.", "Aug 8, 2026 at 4:30 PM")
      },
      {
        id: "source-watch",
        name: "Source freshness",
        purpose: "Notice stale decision inputs",
        state: "completed",
        update: "Source comparison finished in this reveal; no source was changed.",
        trace: claimTrace("Resolved context ledger", "Compare each source timestamp with the Aug 9 evaluation time.")
      }
    ],
    tuningCorpus: AMBIENT_TUNING_CORPUS,
    tuningSummary: "Material changes only · daily digest · quiet 6 PM–7 AM",
    fixedAuthority: "This capability can observe and prepare suggestions. It cannot approve, fund, assign, or change source systems."
  };
  const completeRecipe: IxReferenceRecipeArtifact = {
    ...partialRecipe,
    currentReveal: [
      { id: "draft-prepared", summary: "Prepared a two-item draft recovery note in this fixture; no source record was changed", trace: claimTrace("Current deterministic reveal", "Synthesize the two displayed attention items from the fixture context.") }
    ],
    workstreams: [
      {
        id: "recovery-watch",
        name: "Readiness movement",
        purpose: "Watch material readiness movement",
        state: "completed",
        update: "Prepared a two-item draft note in this fixture; no source was changed.",
        trace: claimTrace("Exercise register + ownership roster", "Compare overdue work with current ownership.")
      },
      {
        id: "source-watch",
        name: "Source freshness",
        purpose: "Notice stale decision inputs",
        state: "watching",
        resumeState: "watching",
        update: "Watching the remaining 23-day-old input.",
        trace: claimTrace("Continuity scorecard", "Observe source age; do not request or perform a refresh.", "Jul 17, 2026 at 5:00 PM")
      },
      {
        id: "handoff-watch",
        name: "External handoff",
        purpose: "Track the unresolved handoff",
        state: "needs-input",
        update: "Needs an authorized owner reference before it can continue.",
        trace: claimTrace("External handoff status", "No inspectable owner reference is available.", "Refresh time unknown")
      },
      {
        id: "capacity-watch",
        name: "Capacity hypothesis",
        purpose: "Check whether capacity changes the thesis",
        state: "paused",
        resumeState: "working",
        update: "Paused by you yesterday; it would resume the prior working state.",
        trace: claimTrace("Fixture control history", "The deterministic fixture records the prior working state.")
      },
      {
        id: "old-watch",
        name: "Previous quarter",
        purpose: "Watch a completed quarter",
        state: "stopped",
        update: "Stopped after the quarter closed.",
        trace: claimTrace("Quarter objective register", "The prior quarter is terminal in this fixture.")
      }
    ]
  };
  return buildRecipeSchedule(
    "ambient-agent-continuity",
    "Review what helpful agents noticed while you were away",
    { label: "Reconnecting to useful work", detail: "Separating completed updates from work still in progress." },
    { label: "Checking what needs you", detail: "Showing source, limits, and any agent waiting for input." },
    recipeArtifact("ambient-agent-continuity", 1, "Agents helping you", partialRecipe, ["recovery-watch", "source-watch"], "partial"),
    recipeArtifact("ambient-agent-continuity", 2, "Agents helping you", completeRecipe, completeRecipe.workstreams.map(({ id }) => id), "ready"),
    "Your since-away observations are separate from work completed during this fixture reveal."
  );
}

export function transitionAmbientWorkstream(
  workstream: IxAmbientWorkstream,
  command: "pause" | "resume" | "stop"
): IxAmbientWorkstream {
  if (workstream.state === "completed" || workstream.state === "needs-input" || workstream.state === "stopped") {
    return workstream;
  }
  if (command === "pause" && (workstream.state === "watching" || workstream.state === "working")) {
    return { ...workstream, state: "paused", resumeState: workstream.state };
  }
  if (command === "resume" && workstream.state === "paused" && workstream.resumeState) {
    return { ...workstream, state: workstream.resumeState };
  }
  if (command === "stop") {
    return { ...workstream, state: "stopped" };
  }
  return workstream;
}
