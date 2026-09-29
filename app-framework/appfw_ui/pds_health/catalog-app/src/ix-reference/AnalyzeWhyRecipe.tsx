import { useEffect, useMemo, useState } from "react";
import {
  Badge,
  Button,
  KpiTile,
  MetricTrend
} from "@appfw/pds-health-components";
import type { IxArtifact } from "./contract";
import {
  ReferenceBoundaryNote,
  ContextSet,
  IntelligenceProgress,
  ProgressiveArtifact
} from "./components";
import { buildAnalyzeWhySchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

type AnalyzeFocus = "continuity" | "roadblocks" | "decisions";

function applyHumanEdits(
  artifact: IxArtifact,
  editedBodies: Readonly<Record<string, string>>
): IxArtifact {
  return {
    ...artifact,
    regions: artifact.regions.map((region) => ({
      ...region,
      body: editedBodies[region.id] ?? region.body
    }))
  };
}

export function AnalyzeWhyRecipe({
  workingBrief,
  onArtifactChange,
  onAskAboutThis
}: {
  workingBrief?: IxArtifact;
  onArtifactChange: (artifact: IxArtifact) => void;
  onAskAboutThis: (question?: string) => void;
}) {
  const run = useFixtureRun();
  const [focus, setFocus] = useState<AnalyzeFocus>("continuity");
  const [editedBodies, setEditedBodies] = useState<Record<string, string>>({});
  const [contextOpen, setContextOpen] = useState(false);

  const materializedArtifact = useMemo(() => {
    const sourceArtifact = run.state.artifact ?? workingBrief;
    if (!sourceArtifact) return undefined;
    return applyHumanEdits(sourceArtifact, editedBodies);
  }, [editedBodies, run.state.artifact, workingBrief]);

  useEffect(() => {
    if (!run.state.artifact) return;
    onArtifactChange(applyHumanEdits(run.state.artifact, editedBodies));
  }, [editedBodies, onArtifactChange, run.state.artifact]);

  function startFresh() {
    setFocus("continuity");
    if (!materializedArtifact) setEditedBodies({});
    run.start(
      buildAnalyzeWhySchedule("continuity", materializedArtifact),
      { preservedArtifact: materializedArtifact }
    );
  }

  function redirect(nextFocus: AnalyzeFocus) {
    if (nextFocus === focus || !materializedArtifact) return;
    setFocus(nextFocus);
    run.start(
      buildAnalyzeWhySchedule(nextFocus, materializedArtifact),
      { preservedArtifact: materializedArtifact }
    );
  }

  return (
    <section className="ix-recipe" aria-labelledby="analyze-why-title">
      <header className="ix-recipe__header">
        <span>
          <span className="ix-eyebrow">Reference 1 · Inline intelligence</span>
          <h2 id="analyze-why-title">Analyze why, without leaving the dashboard</h2>
          <p>Turn the metric already in view into progressively useful, editable work.</p>
        </span>
        <Badge tone="accent">Interactive deterministic stream</Badge>
      </header>

      <div className="ix-metric-and-action">
        <KpiTile
          className="ix-metric"
          label="Recovery readiness"
          value="67%"
          detail="Current quarter · target 85%"
          tone="warning"
          trend={<MetricTrend value="−15 pts" label="this quarter" tone="negative" direction="down" />}
        />
        <div className="ix-metric-action">
          <strong>Something material changed.</strong>
          <span>Two exercises are overdue; one supporting input is stale.</span>
          <Button
            variant="primary"
            onClick={startFresh}
            disabled={run.state.status === "running"}
          >
            {run.state.status === "idle" && !materializedArtifact ? "Analyze why" : "Refresh analysis"}
          </Button>
        </div>
      </div>

      <ContextSet
        context={run.state.context}
        fallback={RECOVERY_CONTEXT}
        onCorrect={() => setContextOpen((open) => !open)}
      />
      {contextOpen ? (
        <section className="ix-context-inspector" aria-label="Resolved context details">
          <strong>What this reference is using</strong>
          <p>Recovery readiness, current-quarter filter, four synthetic authorized source references, and the current working brief.</p>
          <ul>
            {RECOVERY_CONTEXT.gaps.map((gap) => <li key={gap}>{gap}</li>)}
          </ul>
          <p>The browser supplies only the focus reference. A real product backend reloads and authorizes the data.</p>
        </section>
      ) : null}

      {run.state.status !== "idle" ? (
        <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} />
      ) : null}

      {materializedArtifact ? (
        <div className="ix-redirect" aria-label="Redirect analysis without losing the brief">
          <span>Reconsider without starting over:</span>
          {([
            ["continuity", "Service continuity"],
            ["roadblocks", "Roadblocks"],
            ["decisions", "Decisions needed"]
          ] as const).map(([value, label]) => (
            <button
              key={value}
              className="ix-choice"
              type="button"
              aria-pressed={focus === value}
              disabled={run.state.status === "running" && focus === value}
              onClick={() => redirect(value)}
            >
              {label}
            </button>
          ))}
        </div>
      ) : null}

      <ProgressiveArtifact
        artifact={materializedArtifact}
        active={run.state.status === "running"}
        editedBodies={editedBodies}
        emptyMessage="Useful sections will appear here as soon as they are ready."
        editable
        onEdit={(regionId, value) => {
          const nextEdits = { ...editedBodies, [regionId]: value };
          setEditedBodies(nextEdits);
          if (materializedArtifact) {
            onArtifactChange(applyHumanEdits(materializedArtifact, nextEdits));
          }
        }}
        onChallenge={(prompt) => onAskAboutThis(prompt)}
        footer={materializedArtifact ? (
          <>
            <span>Your edits stay with the brief when the focus changes.</span>
            <Button variant="primary" onClick={() => onAskAboutThis("Why did recovery readiness fall?")}>Ask a follow-up</Button>
          </>
        ) : undefined}
      />

      <ReferenceBoundaryNote>
        This catalog reference replays deterministic display fixtures. It demonstrates presentation behavior only; it does not claim a live model, provider, runtime lifecycle, or durable task store.
      </ReferenceBoundaryNote>
    </section>
  );
}
