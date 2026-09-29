import { Fragment, type ReactNode } from "react";
import {
  Badge,
  Button,
  FreshnessIndicator
} from "@appfw/pds-health-components";
import {
  ProgressiveResponse,
  ResolvedContextDisclosure,
  WorkStatus
} from "@appfw/pds-health-components/intelligence-presentation";
import {
  pdsIxPresentationSchema,
  type PdsIxPresentationEnvelope
} from "@appfw/pds-health-components/intelligence-presentation-model";
import type {
  IxArtifact,
  IxClaimTrace,
  IxResolvedContext,
  IxReferencePlaybackState
} from "./contract";

export function IntelligenceProgress({
  state,
  onStop,
  onResume
}: {
  state: IxReferencePlaybackState;
  onStop: () => void;
  onResume: () => void;
}) {
  const running = state.status === "running" && !state.fixturePlaybackPaused;
  const resumable = state.status === "running" && state.fixturePlaybackPaused;
  return (
    <WorkStatus
      className="ix-progress"
      data-state={state.status}
      ariaLabel="Intelligent work status"
      model={{
        label: state.phaseLabel,
        detail: state.phaseDetail,
        active: running,
        actionLabel: running ? "Pause demo" : resumable ? "Resume demo" : undefined
      }}
      action={running ? (
        <Button size="sm" variant="secondary" onClick={onStop}>Pause demo</Button>
      ) : resumable ? (
        <Button size="sm" variant="secondary" onClick={onResume}>Resume demo</Button>
      ) : undefined}
    />
  );
}

export function ContextSet({
  context,
  fallback,
  onCorrect
}: {
  context?: IxResolvedContext;
  fallback: IxResolvedContext;
  onCorrect?: () => void;
}) {
  const resolved = context ?? fallback;
  return (
    <ResolvedContextDisclosure
      className="ix-context-set"
      ariaLabel="Context used for this intelligent work"
      model={{
        eyebrow: "Working from",
        title: resolved.focusLabel,
        detail: resolved.detail,
        announcement: `Context resolved for ${resolved.focusLabel}. ${resolved.sourceCount} sources and ${resolved.gaps.length} gaps.`,
        gaps: resolved.gaps,
        evidence: {
          summary: `View ${resolved.sourceCount} sources`,
          items: resolved.sources.map((source) => ({
            id: `ix-source-${source.sourceRef}`,
            sourceRef: source.sourceRef,
            label: source.label,
            value: (source.refreshedAt
              ? `${contextFreshnessDetail(source.freshness)} · refreshed ${formatTimestamp(source.refreshedAt)}`
              : "Refresh time unknown")
              + (source.inspectable ? " · available to inspect" : " · detail unavailable")
          }))
        },
        metaLabel: `${resolved.sourceCount} resolved sources · ${contextFreshnessDetail(resolved.freshness)}`,
        actionLabel: onCorrect ? "Check context" : undefined
      }}
      meta={<FreshnessIndicator
        value={contextFreshnessValue(resolved.freshness)}
        label={`${resolved.sourceCount} resolved sources`}
        detail={contextFreshnessDetail(resolved.freshness)}
        timestamp={`Evaluated ${formatTimestamp(resolved.evaluatedAt)}`}
      />}
      action={onCorrect ? (
        <Button size="sm" variant="quiet" onClick={onCorrect}>Check context</Button>
      ) : undefined}
    />
  );
}

export function ProgressiveArtifact({
  artifact,
  active = false,
  editedBodies,
  emptyMessage,
  editable = false,
  onEdit,
  onChallenge,
  footer
}: {
  artifact?: IxArtifact;
  active?: boolean;
  editedBodies?: Readonly<Record<string, string>>;
  emptyMessage: string;
  editable?: boolean;
  onEdit?: (regionId: string, value: string) => void;
  onChallenge?: (prompt: string) => void;
  footer?: ReactNode;
}) {
  if (!artifact) {
    return <div className="ix-artifact-empty">{emptyMessage}</div>;
  }

  const registered = artifact.artifactType === "pds.ix.working_brief@1"
    || artifact.artifactType === "pds.ix.contextual_answer@1";
  if (!registered) {
    return (
      <section className="ix-artifact-fallback" role="status">
        <strong>This result is preserved but has no registered presentation.</strong>
        <span>Use the ordinary record view while the renderer is unavailable.</span>
      </section>
    );
  }

  const changedRegionSummary = artifact.regions
    .filter((region) => artifact.changedRegionIds.includes(region.id))
    .map((region) => region.title)
    .join(", ") || "artifact status";

  const announcement = `${artifact.title} revision ${artifact.revision}. Updated: ${singleTerminalPunctuation(changedRegionSummary)}`;
  const presentation: PdsIxPresentationEnvelope = {
    identity: {
      schemaVersion: pdsIxPresentationSchema,
      presentationId: artifact.artifactId,
      revision: artifact.revision
    },
    announcement,
    response: {
      eyebrow: "Progressive artifact",
      title: artifact.title,
      metaLabel: `Revision ${artifact.revision} · ${artifact.status}`,
      announcement,
      active,
      emptyState: emptyMessage,
      regions: artifact.regions.map((region) => ({
        id: region.id,
        status: region.status,
        label: region.label,
        title: region.title,
        body: editedBodies?.[region.id] ?? region.body,
        whyItMatters: region.whyItMatters,
        changed: artifact.changedRegionIds.includes(region.id),
        editable,
        evidence: {
          summary: "Where did this come from?",
          items: [
            { label: "Source", value: region.evidence.source },
            ...(region.evidence.owner ? [{ label: "Owner", value: region.evidence.owner }] : []),
            ...(region.evidence.sourceUpdatedAt ? [{ label: "Last refreshed", value: formatTimestamp(region.evidence.sourceUpdatedAt) }] : []),
            ...(region.evidence.retrievedAt ? [{ label: "Used for this read", value: formatTimestamp(region.evidence.retrievedAt) }] : []),
            ...(region.evidence.calculation ? [{ label: "How calculated", value: region.evidence.calculation }] : []),
            ...(region.evidence.assumptions?.length ? [{ label: "Assumptions", value: region.evidence.assumptions.join(" ") }] : []),
            ...(region.evidence.limitations?.length ? [{ label: "Limits", value: region.evidence.limitations.join(" ") }] : [])
          ]
        },
        actionLabel: region.challengePrompt && onChallenge ? "Challenge this" : undefined
      }))
    }
  };

  return (
    <ProgressiveResponse
      className="ix-artifact"
      data-presentation-schema={presentation.identity.schemaVersion}
      data-presentation-id={presentation.identity.presentationId}
      data-presentation-revision={presentation.identity.revision}
      model={presentation.response}
      meta={<Badge tone={artifact.status === "stale" ? "warning" : "neutral"}>
        Revision {artifact.revision} · {artifact.status}
      </Badge>}
      renderBody={(region) => editable ? (
            <label className="ix-artifact-region__editor">
              <span className="ix-sr-only">Edit {region.title}</span>
              <textarea
                value={region.body}
                rows={5}
                onChange={(event) => onEdit?.(region.id, event.currentTarget.value)}
              />
            </label>
          ) : (
            <p>{region.body}</p>
          )}
      renderAction={(region) => (
        <button
          className="ix-text-action"
          type="button"
          onClick={() => {
            const sourceRegion = artifact.regions.find(({ id }) => id === region.id);
            onChallenge?.(sourceRegion?.challengePrompt ?? "Challenge this claim");
          }}
        >
          {region.actionLabel}
        </button>
      )}
      footer={footer}
    />
  );
}

export function ReferenceBoundaryNote({ children }: { children: ReactNode }) {
  return <p className="ix-reference-boundary-note">{children}</p>;
}

export function RecipeRevisionAnnouncement({ artifact }: { artifact?: IxArtifact }) {
  if (!artifact?.referenceRecipe) return null;
  const recipe = artifact.referenceRecipe;
  const changed = new Set(artifact.changedRegionIds);
  let labels = recipe.kind === "adaptive-composition"
    ? recipe.blocks.filter(({ id }) => changed.has(id)).map(({ title }) => title)
    : recipe.kind === "working-goal-plan"
      ? recipe.steps.filter(({ id }) => changed.has(id)).map(({ outcome }) => outcome)
      : recipe.kind === "adaptive-information-lens"
        ? recipe.visibleFields.filter(({ id }) => changed.has(id)).map(({ label }) => label)
        : recipe.kind === "situation-to-strategy"
          ? [recipe.revisionChange ?? "Thesis, assumptions, and intervention"]
          : recipe.kind === "attention-stewardship"
            ? recipe.items.filter(({ id }) => changed.has(id)).map(({ title }) => title)
            : recipe.workstreams.filter(({ id }) => changed.has(id)).map(({ name }) => name);
  if (recipe.kind === "working-goal-plan" && labels.length === 0) {
    labels = [recipe.revisionNote];
  }
  if (recipe.kind === "adaptive-composition" && labels.length === 0) {
    labels = [...recipe.changeSummary];
  }
  const updateSummary = labels.join(", ") || "the result summary";
  return (
    <p className="ix-sr-only" role="status" aria-live="polite" aria-atomic="true">
      {artifact.title} revision {artifact.revision} is {artifact.status}. Updated: {singleTerminalPunctuation(updateSummary)}
    </p>
  );
}

function singleTerminalPunctuation(value: string): string {
  const trimmed = value.trim();
  const punctuation = trimmed.match(/[.!?]+$/u)?.[0]?.[0] ?? ".";
  return `${trimmed.replace(/[.!?]+$/u, "")}${punctuation}`;
}

export function ClaimTrace({ trace }: { trace: IxClaimTrace }) {
  return (
    <details className="ix-evidence ix-claim-trace">
      <summary>Inspect source and derivation</summary>
      <dl>
        {trace.sources.map((source) => (
          <Fragment key={source.sourceRef}>
            <dt>{source.kind === "authorized-context" ? "Authorized source" : "Fixture input"}</dt>
            <dd>
              {source.kind === "authorized-context" ? (
                <a href={`#ix-source-${source.sourceRef}`} onClick={() => revealClaimSource(source.sourceRef)}>{source.label}</a>
              ) : (
                <span>{source.label} · deterministic fixture input</span>
              )}
              <span> · {claimSourceFreshness(source)}</span>
            </dd>
          </Fragment>
        ))}
        <dt>How derived</dt><dd>{trace.derivation}</dd>
      </dl>
    </details>
  );
}

function revealClaimSource(sourceRef: string): void {
  const target = document.getElementById(`ix-source-${sourceRef}`);
  const disclosure = target?.closest("details");
  if (disclosure instanceof HTMLDetailsElement) disclosure.open = true;
}

function claimSourceFreshness(source: IxClaimTrace["sources"][number]): string {
  if (source.kind === "deterministic-fixture") {
    return `freshness ${source.freshness} · observed ${source.observedAt}`;
  }
  return source.refreshedAt
    ? `freshness ${source.freshness} · refreshed ${formatTimestamp(source.refreshedAt)}`
    : `freshness ${source.freshness} · refresh time unknown`;
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return value;
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit"
  }).format(date);
}

function contextFreshnessValue(
  freshness: IxResolvedContext["freshness"]
): "current" | "stale" | "unknown" {
  if (freshness === "current") return "current";
  if (freshness === "unknown") return "unknown";
  return "stale";
}

function contextFreshnessDetail(freshness: IxResolvedContext["freshness"]): string {
  if (freshness === "mixed") return "Mixed freshness";
  if (freshness === "unknown") return "Refresh time unknown";
  return freshness === "current" ? "Current" : "Stale";
}
