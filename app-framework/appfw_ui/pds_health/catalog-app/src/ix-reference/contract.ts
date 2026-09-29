export const IX_REFERENCE_PLAYBACK_SCHEMA = "pds.ix.reference_playback@1" as const;
export const IX_MAX_PUBLIC_ARTIFACT_REVISION = 9_007_199_254_740_991 as const;

/**
 * These state types drive deterministic catalog fixture playback only.
 * They are not canonical runtime authority and must not be used to approve,
 * complete, cancel, or fail real work.
 */

export type IxReferenceDisplayPhase =
  | "understanding"
  | "gathering"
  | "resolving"
  | "interpreting"
  | "composing"
  | "checking"
  | "waiting";

export type IxReferencePlaybackStatus =
  | "idle"
  | "running"
  | "cancelled"
  | "completed"
  | "failed";

export type IxReferencePlaybackPhase =
  | "fresh"
  | "started"
  | "context"
  | "drafts"
  | "terminal";

export type IxArtifactStatus =
  | "partial"
  | "ready"
  | "revising"
  | "stale"
  | "failed";

export type IxClaimKind =
  | "observation"
  | "interpretation"
  | "hypothesis"
  | "recommendation";

export type IxEvidence = {
  source: string;
  owner?: string;
  sourceUpdatedAt?: string;
  retrievedAt?: string;
  calculation?: string;
  assumptions?: readonly string[];
  limitations?: readonly string[];
};

export type IxArtifactRegion = {
  id: string;
  label: string;
  title: string;
  body: string;
  kind: IxClaimKind;
  status: "partial" | "ready" | "stale";
  whyItMatters: string;
  evidence: IxEvidence;
  challengePrompt?: string;
};

export type IxClaimSourceRef =
  | {
      sourceRef: string;
      label: string;
      kind: "authorized-context";
      freshness: "current" | "stale" | "unknown";
      refreshedAt?: string;
    }
  | {
      sourceRef: string;
      label: string;
      kind: "deterministic-fixture";
      freshness: "unknown";
      observedAt: string;
    };

export type IxClaimTrace = {
  sources: readonly IxClaimSourceRef[];
  derivation: string;
};

export type IxCompositionBlock = {
  id: string;
  kind: "signal" | "comparison" | "move";
  title: string;
  body: string;
  whyIncluded: string;
  trace: IxClaimTrace;
};

export type IxGoalStep = {
  id: string;
  outcome: string;
  state: "complete" | "working" | "next" | "required";
  checkpoint?: string;
  trace: IxClaimTrace;
};

export type IxLensField = {
  id: string;
  label: string;
  value: string;
  reason: string;
  trace: IxClaimTrace;
};

export type IxStrategyAssumption = {
  id: string;
  statement: string;
  test: string;
  trace: IxClaimTrace;
};

export type IxAttentionItem = {
  id: string;
  title: string;
  reason: string;
  usefulMove: string;
  whyMe: string;
  priorityDerivation: string;
  state: "open" | "covered" | "not-mine" | "snoozed" | "challenged";
  trace: IxClaimTrace;
};

export type IxAmbientWorkstream = {
  id: string;
  name: string;
  purpose: string;
  state: "watching" | "working" | "completed" | "needs-input" | "paused" | "stopped";
  update: string;
  resumeState?: "watching" | "working";
  trace: IxClaimTrace;
};

export type IxSinceAwayUpdate = {
  id: string;
  summary: string;
  trace: IxClaimTrace;
};

export type IxStrategyChallengeCategory = "source-freshness" | "capacity" | "causal-link";

export type IxStrategyChallenge = {
  category: IxStrategyChallengeCategory;
  text: string;
};

export type IxAmbientTuning = {
  relevance: "material" | "operational";
  cadence: "daily" | "weekly";
  quiet: boolean;
};

export type IxAmbientTuningEvent = {
  id: string;
  occurredAt: string;
  material: boolean;
  inQuietHours: boolean;
  label: string;
};

export type IxAmbientTuningCorpus = {
  windowStart: string;
  windowEnd: string;
  timezone: string;
  formula: string;
  events: readonly IxAmbientTuningEvent[];
};

/**
 * Closed, catalog-only presentation payloads carried by the private
 * pds.ix.reference_playback@1 fixture player. Products and shared runtimes
 * still own their real schemas, authority, lifecycle, and render registries.
 */
export type IxReferenceRecipeArtifact =
  | {
      kind: "adaptive-composition";
      focus: "operating-picture" | "decisions";
      blocks: readonly IxCompositionBlock[];
      changeSummary: readonly string[];
    }
  | {
      kind: "working-goal-plan";
      goal: string;
      revisionNote: string;
      steps: readonly IxGoalStep[];
      preservedWork: readonly string[];
    }
  | {
      kind: "adaptive-information-lens";
      lens: "executive-read" | "operator-read";
      objective: string;
      visibleFields: readonly IxLensField[];
      fullRecordFields: readonly IxLensField[];
      omissionExplanation: string;
    }
  | {
      kind: "situation-to-strategy";
      thesis: string;
      confidence: "low" | "moderate";
      thesisTrace: IxClaimTrace;
      signals: readonly string[];
      assumptions: readonly IxStrategyAssumption[];
      intervention: string;
      interventionTrace: IxClaimTrace;
      measure: string;
      counterview?: string;
      priorThesis?: { thesis: string; reasoning: string };
      revisionChange?: string;
      challenge?: IxStrategyChallenge;
    }
  | {
      kind: "attention-stewardship";
      evaluatedCount: number;
      items: readonly IxAttentionItem[];
      emptyMessage?: string;
    }
  | {
      kind: "ambient-agent-continuity";
      capabilityName: string;
      accountableOwner: string;
      sinceAway: readonly IxSinceAwayUpdate[];
      currentReveal: readonly IxSinceAwayUpdate[];
      workstreams: readonly IxAmbientWorkstream[];
      tuningCorpus: IxAmbientTuningCorpus;
      tuningSummary: string;
      fixedAuthority: string;
    };

export type IxArtifact = {
  artifactId: string;
  artifactType: string;
  revision: number;
  status: IxArtifactStatus;
  title: string;
  changedRegionIds: readonly string[];
  regions: readonly IxArtifactRegion[];
  referenceRecipe?: IxReferenceRecipeArtifact;
};

export type IxContextFreshness = "current" | "stale" | "mixed" | "unknown";

export type IxResolvedSourceSummary = {
  sourceRef: string;
  label: string;
  refreshedAt?: string;
  freshness: IxContextFreshness;
  inspectable: boolean;
};

export type IxResolvedContext = {
  focusLabel: string;
  detail: string;
  sourceCount: number;
  evaluatedAt: string;
  freshness: IxContextFreshness;
  sources: readonly IxResolvedSourceSummary[];
  gaps: readonly string[];
};

type IxReferencePlaybackEventBase = {
  schemaVersion: typeof IX_REFERENCE_PLAYBACK_SCHEMA;
  eventId: string;
  playbackId: string;
  sequence: number;
  occurredAt: string;
};

export type IxReferencePlaybackEvent =
  | (IxReferencePlaybackEventBase & {
      type: "playback_started";
      intentLabel: string;
    })
  | (IxReferencePlaybackEventBase & {
      type: "context_displayed";
      context: IxResolvedContext;
    })
  | (IxReferencePlaybackEventBase & {
      type: "display_phase_changed";
      phase: IxReferenceDisplayPhase;
      label: string;
      detail: string;
    })
  | (IxReferencePlaybackEventBase & {
      type: "fixture_revision_displayed";
      artifact: IxArtifact;
    })
  | (IxReferencePlaybackEventBase & {
      type: "playback_finished";
      outcome: "completed" | "partial" | "cancelled";
      message: string;
    })
  | (IxReferencePlaybackEventBase & {
      type: "playback_error_displayed";
      code: string;
      message: string;
    });

export type ScheduledIxReferencePlaybackEvent = {
  afterMs: number;
  event: IxReferencePlaybackEvent;
};

export type IxReferencePlaybackState = {
  playbackId?: string;
  status: IxReferencePlaybackStatus;
  playbackPhase: IxReferencePlaybackPhase;
  fixturePlaybackPaused: boolean;
  sequence: number;
  phase?: IxReferenceDisplayPhase;
  phaseLabel: string;
  phaseDetail: string;
  context?: IxResolvedContext;
  artifact?: IxArtifact;
  artifactDisplayedInPlayback: boolean;
  terminalMessage?: string;
  error?: string;
};

export const initialIxReferencePlaybackState: IxReferencePlaybackState = {
  status: "idle",
  playbackPhase: "fresh",
  fixturePlaybackPaused: false,
  sequence: 0,
  artifactDisplayedInPlayback: false,
  phaseLabel: "Ready",
  phaseDetail: "No intelligent work is running."
};

export function beginIxReferencePlaybackState(
  playbackId: string,
  intentLabel: string,
  preservedArtifact?: IxArtifact
): IxReferencePlaybackState {
  if (preservedArtifact) {
    validateIxArtifact(preservedArtifact);
  }
  return {
    playbackId,
    status: "running",
    playbackPhase: "fresh",
    fixturePlaybackPaused: false,
    sequence: 0,
    phase: "understanding",
    phaseLabel: "Starting with what you are viewing",
    phaseDetail: intentLabel,
    artifact: preservedArtifact
      ? { ...preservedArtifact, changedRegionIds: [] }
      : undefined,
    artifactDisplayedInPlayback: false
  };
}

const MAX_CONTEXT_SOURCES = 32;
const MAX_CONTEXT_GAPS = 32;
const MAX_ARTIFACT_REGIONS = 64;
const MAX_FOCUS_VALUE_BYTES = 256;
const MAX_LABEL_BYTES = 512;
const MAX_DETAIL_BYTES = 2048;
const RFC3339_PATTERN = /^(\d{4})-(\d{2})-(\d{2})[Tt ](\d{2}):(\d{2}):(\d{2})(\.\d+)?(?:([Zz])|([+\-−])(\d{2}):(\d{2}))$/;

function validateSafePositiveInteger(field: string, value: number): void {
  if (!Number.isSafeInteger(value) || value < 1 || value > IX_MAX_PUBLIC_ARTIFACT_REVISION) {
    throw new Error(`${field} must be a safe positive integer.`);
  }
}

function validateText(field: string, value: string, maxBytes: number): void {
  if (!value.trim() || new TextEncoder().encode(value.trim()).byteLength > maxBytes) {
    throw new Error(`${field} must contain bounded plain text.`);
  }
}

function validateKey(field: string, value: string, maxBytes: number): void {
  validateText(field, value, maxBytes);
  if (!/^[A-Za-z0-9._:/@-]+$/.test(value)) {
    throw new Error(`${field} contains unsupported characters.`);
  }
}

function parseRfc3339(field: string, value: string): bigint {
  const match = RFC3339_PATTERN.exec(value);
  if (!match) {
    throw new Error(`${field} must be an RFC 3339 timestamp.`);
  }
  const [
    ,
    yearText,
    monthText,
    dayText,
    hourText,
    minuteText,
    secondText,
    fraction = "",
    utcDesignator,
    offsetSign,
    offsetHourText,
    offsetMinuteText
  ] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);
  const offsetHour = Number(offsetHourText ?? 0);
  const offsetMinute = Number(offsetMinuteText ?? 0);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [0, 31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month] ?? 0;
  if (day < 1 || day > daysInMonth || hour > 23 || minute > 59 || second > 60
    || offsetHour > 23 || offsetMinute > 59) {
    throw new Error(`${field} must be an RFC 3339 timestamp.`);
  }

  const normalizedOffset = utcDesignator
    ? "Z"
    : `${offsetSign === "−" ? "-" : offsetSign}${offsetHourText}:${offsetMinuteText}`;
  const normalizedSecond = second === 60 ? "59" : secondText;
  const normalized = `${yearText}-${monthText}-${dayText}T${hourText}:${minuteText}:${normalizedSecond}${normalizedOffset}`;
  const parsed = Date.parse(normalized);
  if (!Number.isFinite(parsed)) {
    throw new Error(`${field} must be an RFC 3339 timestamp.`);
  }
  const fractionalNanoseconds = BigInt((fraction.slice(1) || "0").padEnd(9, "0").slice(0, 9));
  const leapSecondNanoseconds = second === 60 ? 1_000_000_000n : 0n;
  return BigInt(parsed) * 1_000_000n + fractionalNanoseconds + leapSecondNanoseconds;
}

export function validateIxResolvedContext(context: IxResolvedContext): void {
  validateText("IX context focus label", context.focusLabel, MAX_LABEL_BYTES);
  validateText("IX context detail", context.detail, MAX_DETAIL_BYTES);
  const evaluatedAt = parseRfc3339("IX context evaluation time", context.evaluatedAt);
  if (context.sources.length > MAX_CONTEXT_SOURCES) {
    throw new Error("IX context contains too many source summaries.");
  }
  if (context.gaps.length > MAX_CONTEXT_GAPS) {
    throw new Error("IX context contains too many known gaps.");
  }
  if (context.sourceCount !== context.sources.length) {
    throw new Error("IX context source count does not match its source summaries.");
  }

  const sourceRefs = new Set<string>();
  for (const source of context.sources) {
    validateKey("IX context source reference", source.sourceRef, MAX_FOCUS_VALUE_BYTES);
    validateText("IX context source label", source.label, MAX_LABEL_BYTES);
    if (sourceRefs.has(source.sourceRef)) {
      throw new Error("IX context source references must be unique.");
    }
    sourceRefs.add(source.sourceRef);

    if (source.refreshedAt) {
      const refreshedAt = parseRfc3339("IX context source refresh time", source.refreshedAt);
      if (refreshedAt > evaluatedAt) {
        throw new Error("IX context source refresh time cannot follow context evaluation.");
      }
    } else if (source.freshness !== "unknown") {
      throw new Error("A source without a refresh time must use unknown freshness.");
    }
  }

  for (const gap of context.gaps) {
    validateText("IX context gap", gap, MAX_DETAIL_BYTES);
  }

  if (context.sources.length === 0 && context.freshness !== "unknown") {
    throw new Error("Empty IX context must use unknown freshness.");
  }
  if (context.freshness === "current"
    && (context.sources.length === 0
      || context.sources.some((source) => source.freshness !== "current"))) {
    throw new Error("Current IX context requires current source summaries.");
  }
}

function referenceRecipeRegionIds(referenceRecipe: IxReferenceRecipeArtifact): readonly string[] {
  switch (referenceRecipe.kind) {
    case "adaptive-composition":
      return referenceRecipe.blocks.map(({ id }) => id);
    case "working-goal-plan":
      return referenceRecipe.steps.map(({ id }) => id);
    case "adaptive-information-lens":
      return referenceRecipe.visibleFields.map(({ id }) => id);
    case "situation-to-strategy":
      return [
        "thesis",
        "intervention",
        ...referenceRecipe.assumptions.map(({ id }) => id),
        ...(referenceRecipe.challenge ? [`challenge-${referenceRecipe.challenge.category}`] : [])
      ];
    case "attention-stewardship":
      return referenceRecipe.items.map(({ id }) => id);
    case "ambient-agent-continuity":
      return referenceRecipe.workstreams.map(({ id }) => id);
  }
}

export function validateIxArtifact(artifact: IxArtifact): void {
  validateKey("IX artifact identifier", artifact.artifactId, MAX_FOCUS_VALUE_BYTES);
  validateKey("IX artifact type", artifact.artifactType, MAX_FOCUS_VALUE_BYTES);
  validateSafePositiveInteger("IX artifact revision", artifact.revision);
  validateText("IX artifact title", artifact.title, MAX_LABEL_BYTES);
  if (!(["partial", "ready", "revising", "stale", "failed"] as const).includes(artifact.status)) {
    throw new Error("IX artifact status is not supported.");
  }
  if (artifact.regions.length > MAX_ARTIFACT_REGIONS) {
    throw new Error("IX artifact contains too many regions.");
  }

  const regionIds = new Set<string>();
  for (const region of artifact.regions) {
    validateKey("IX artifact region identifier", region.id, MAX_FOCUS_VALUE_BYTES);
    if (regionIds.has(region.id)) {
      throw new Error("IX artifact region identifiers must be unique.");
    }
    regionIds.add(region.id);
  }
  for (const regionId of artifact.referenceRecipe ? referenceRecipeRegionIds(artifact.referenceRecipe) : []) {
    validateKey("IX artifact region identifier", regionId, MAX_FOCUS_VALUE_BYTES);
    if (regionIds.has(regionId)) {
      throw new Error("IX artifact region identifiers must be unique.");
    }
    regionIds.add(regionId);
  }

  const changedRegionIds = new Set<string>();
  for (const changedRegionId of artifact.changedRegionIds) {
    validateKey("IX changed-region identifier", changedRegionId, MAX_FOCUS_VALUE_BYTES);
    if (changedRegionIds.has(changedRegionId)) {
      throw new Error("IX changed-region identifiers must be unique.");
    }
    if (!regionIds.has(changedRegionId)) {
      throw new Error("Every IX changed-region identifier must belong to that artifact revision.");
    }
    changedRegionIds.add(changedRegionId);
  }
}

export function reduceIxReferencePlaybackEvent(state: IxReferencePlaybackState, event: IxReferencePlaybackEvent): IxReferencePlaybackState {
  if (event.schemaVersion !== IX_REFERENCE_PLAYBACK_SCHEMA) {
    throw new Error("Unsupported catalog playback schema.");
  }
  validateSafePositiveInteger("Catalog playback sequence", event.sequence);
  if (!Number.isSafeInteger(state.sequence) || state.sequence < 0) {
    throw new Error("IX prior event sequence must be a safe non-negative integer.");
  }
  if (state.playbackId && event.playbackId !== state.playbackId) {
    throw new Error("Catalog playback changed fixture identity.");
  }
  if (event.sequence !== state.sequence + 1) {
    throw new Error("Catalog playback sequence is not contiguous.");
  }
  if (state.playbackPhase === "terminal"
    || state.status === "completed"
    || state.status === "cancelled"
    || state.status === "failed") {
    throw new Error("Catalog playback step arrived after the fixture stopped.");
  }
  if (state.fixturePlaybackPaused) {
    throw new Error("IX fixture playback must resume before consuming another event.");
  }

  const nextBase = {
    ...state,
    playbackId: event.playbackId,
    sequence: event.sequence
  };

  switch (event.type) {
    case "playback_started": {
      if (state.playbackPhase !== "fresh") {
        throw new Error("Catalog playback start must be the first fixture step.");
      }
      return {
        ...nextBase,
        status: "running",
        playbackPhase: "started",
        fixturePlaybackPaused: false,
        phase: "understanding",
        phaseLabel: "Understanding the request",
        phaseDetail: event.intentLabel
      };
    }
    case "context_displayed": {
      if (state.playbackPhase !== "started") {
        throw new Error("Reference context must follow playback start and precede vignette display.");
      }
      validateIxResolvedContext(event.context);
      return {
        ...nextBase,
        playbackPhase: "context",
        context: event.context
      };
    }
    case "display_phase_changed": {
      if (state.playbackPhase !== "context" && state.playbackPhase !== "drafts") {
        throw new Error("Reference display phases require resolved fixture context.");
      }
      return {
        ...nextBase,
        status: "running",
        playbackPhase: "drafts",
        phase: event.phase,
        phaseLabel: event.label,
        phaseDetail: event.detail
      };
    }
    case "fixture_revision_displayed": {
      if (state.playbackPhase !== "context" && state.playbackPhase !== "drafts") {
        throw new Error("Fixture revisions require resolved reference context.");
      }
      validateIxArtifact(event.artifact);
      const previousRevision = state.artifact?.artifactId === event.artifact.artifactId
        ? state.artifact.revision
        : 0;
      if (previousRevision > 0) {
        validateSafePositiveInteger("IX prior artifact revision", previousRevision);
      }
      if (previousRevision === IX_MAX_PUBLIC_ARTIFACT_REVISION) {
        throw new Error("IX artifact at the public revision maximum cannot be continued.");
      }
      if (event.artifact.revision !== previousRevision + 1) {
        throw new Error("IX artifact revision must begin at 1 and advance by one.");
      }
      return {
        ...nextBase,
        playbackPhase: "drafts",
        artifact: event.artifact,
        artifactDisplayedInPlayback: true
      };
    }
    case "playback_finished": {
      const cancelled = event.outcome === "cancelled";
      if (state.playbackPhase === "fresh") {
        throw new Error("Playback finish requires a started fixture.");
      }
      if (!cancelled && (state.playbackPhase !== "drafts" || !state.artifactDisplayedInPlayback)) {
        throw new Error("Completed or partial gallery playback requires a useful fixture revision.");
      }
      return {
        ...nextBase,
        status: cancelled ? "cancelled" : "completed",
        playbackPhase: "terminal",
        fixturePlaybackPaused: false,
        phase: undefined,
        phaseLabel: cancelled
          ? "Stopped — your work is still here"
          : event.outcome === "completed"
            ? "Ready to use"
            : "Partial result ready",
        phaseDetail: event.message,
        terminalMessage: event.message
      };
    }
    case "playback_error_displayed": {
      if (state.playbackPhase === "fresh") {
        throw new Error("A displayed playback error requires a started fixture.");
      }
      return {
        ...nextBase,
        status: "failed",
        playbackPhase: "terminal",
        fixturePlaybackPaused: false,
        phase: undefined,
        phaseLabel: "Couldn’t finish this work",
        phaseDetail: event.message,
        terminalMessage: event.message,
        error: event.code
      };
    }
  }
}

export function pauseIxReferencePlaybackState(state: IxReferencePlaybackState): IxReferencePlaybackState {
  if (state.status !== "running" || state.fixturePlaybackPaused) return state;
  return {
    ...state,
    fixturePlaybackPaused: true,
    artifact: state.artifact
      ? { ...state.artifact, changedRegionIds: [] }
      : undefined,
    phase: undefined,
    phaseLabel: "Reference playback paused",
    phaseDetail: "Resume continues with the next unprocessed fixture event."
  };
}

export function resumeIxReferencePlaybackState(state: IxReferencePlaybackState): IxReferencePlaybackState {
  if (state.status !== "running" || !state.fixturePlaybackPaused) return state;
  return {
    ...state,
    fixturePlaybackPaused: false,
    phase: undefined,
    phaseLabel: "Resuming reference playback",
    phaseDetail: "Completed artifact regions and edits remain in place."
  };
}
