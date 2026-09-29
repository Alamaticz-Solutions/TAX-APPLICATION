import { useEffect, useMemo, useRef, useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import type { IxAmbientTuning, IxAmbientWorkstream } from "./contract";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildAmbientAgentContinuitySchedule, previewAmbientTuning, RECOVERY_CONTEXT, transitionAmbientWorkstream } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

const INITIAL_TUNING: IxAmbientTuning = { relevance: "material", cadence: "daily", quiet: true };

function tuningSummary(tuning: IxAmbientTuning): string {
  return `${tuning.relevance === "material" ? "Material changes only" : "Operational details included"} · ${tuning.cadence} digest · ${tuning.quiet ? "quiet 6 PM–7 AM" : "no quiet period"}`;
}

export function AmbientAgentContinuityRecipe() {
  const run = useFixtureRun();
  const [workstreamOverrides, setWorkstreamOverrides] = useState<Record<string, IxAmbientWorkstream>>({});
  const [appliedTuning, setAppliedTuning] = useState<IxAmbientTuning>(INITIAL_TUNING);
  const [previousTuning, setPreviousTuning] = useState<IxAmbientTuning>();
  const [draftTuning, setDraftTuning] = useState<IxAmbientTuning>(INITIAL_TUNING);
  const [tuningRevision, setTuningRevision] = useState(1);
  const [previewing, setPreviewing] = useState(false);
  const [stateAnnouncement, setStateAnnouncement] = useState("");
  const pendingFocusRef = useRef<string>();
  const recipe = run.state.artifact?.referenceRecipe?.kind === "ambient-agent-continuity" ? run.state.artifact.referenceRecipe : undefined;
  const workstreams = useMemo(() => recipe?.workstreams.map((workstream) => workstreamOverrides[workstream.id] ?? workstream) ?? [], [recipe, workstreamOverrides]);
  const preview = recipe ? previewAmbientTuning(draftTuning, recipe.tuningCorpus) : undefined;

  useEffect(() => {
    const targetId = pendingFocusRef.current;
    if (!targetId) return;
    document.getElementById(targetId)?.focus();
    pendingFocusRef.current = undefined;
  }, [previewing, previousTuning, tuningRevision, workstreamOverrides]);

  function command(workstream: IxAmbientWorkstream, nextCommand: "pause" | "resume" | "stop") {
    const transitioned = transitionAmbientWorkstream(workstream, nextCommand);
    pendingFocusRef.current = transitioned.state === "stopped"
      ? `ambient-workstream-${workstream.id}`
      : `ambient-workstream-control-${workstream.id}`;
    setWorkstreamOverrides((current) => ({ ...current, [workstream.id]: transitioned }));
    setStateAnnouncement(`${workstream.name} is now ${transitioned.state.replace("-", " ")}.`);
  }
  function openPreview() {
    pendingFocusRef.current = "ambient-tuning-relevance";
    setPreviewing(true);
    setStateAnnouncement("Tuning preview opened. No settings have changed.");
  }
  function cancelPreview() {
    pendingFocusRef.current = "ambient-tuning-control";
    setDraftTuning(appliedTuning);
    setPreviewing(false);
    setStateAnnouncement("Tuning preview cancelled. No settings changed.");
  }
  function applyPreview() {
    pendingFocusRef.current = "ambient-tuning-control";
    setPreviousTuning(appliedTuning);
    setAppliedTuning(draftTuning);
    setTuningRevision((revision) => revision + 1);
    setPreviewing(false);
    setStateAnnouncement(`Tuning revision ${tuningRevision + 1} applied. Authority did not change.`);
  }
  function rollback() {
    if (!previousTuning) return;
    pendingFocusRef.current = "ambient-tuning-control";
    setAppliedTuning(previousTuning);
    setDraftTuning(previousTuning);
    setPreviousTuning(undefined);
    setTuningRevision((revision) => revision + 1);
    setPreviewing(false);
    setStateAnnouncement(`Tuning rollback recorded as revision ${tuningRevision + 1}. Authority did not change.`);
  }

  return (
    <section className="ix-recipe" aria-labelledby="ambient-agents-title">
      <header className="ix-recipe__header"><span><span className="ix-eyebrow">Reference 8 · Ambient Agent Continuity</span><h2 id="ambient-agents-title">Agents helping you</h2><p>One accountable capability carries several visible workstreams through time: what changed while you were away, what completed during this reveal, what needs you, and what you can tune or stop.</p></span><Badge tone="accent">Calm, visible continuity</Badge></header>
      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      <p className="ix-sr-only" role="status" aria-live="polite" aria-atomic="true">{stateAnnouncement}</p>
      {!recipe ? <div className="ix-since-away"><span><span className="ix-eyebrow">Since you were away</span><strong>2 observations are ready to reconnect</strong><p>No source record was changed by this fixture.</p></span><Button variant="primary" onClick={() => run.start(buildAmbientAgentContinuitySchedule())}>Review capability updates</Button></div> : null}
      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />
      {recipe ? (
        <section className="ix-special-artifact" aria-labelledby="ambient-artifact-title">
          <header className="ix-special-artifact__header"><span><span className="ix-eyebrow">Accountable capability · {recipe.accountableOwner}</span><h3 id="ambient-artifact-title">{recipe.capabilityName}</h3></span><Badge tone="neutral">Deterministic fixture</Badge></header>

          <section aria-labelledby="since-away-title"><h4 id="since-away-title">Observed since you were away</h4><div className="ix-update-grid">{recipe.sinceAway.map((update) => <article key={update.id}><strong>{update.summary}</strong><ClaimTrace trace={update.trace} /></article>)}</div></section>
          {recipe.currentReveal.length ? <section aria-labelledby="current-reveal-title"><h4 id="current-reveal-title">Completed during this reveal</h4><div className="ix-update-grid">{recipe.currentReveal.map((update) => <article key={update.id}><strong>{update.summary}</strong><ClaimTrace trace={update.trace} /></article>)}</div></section> : null}

          <section aria-labelledby="workstreams-title"><h4 id="workstreams-title">Capability workstreams</h4><div className="ix-agent-grid">{workstreams.map((workstream) => {
            const canPause = workstream.state === "watching" || workstream.state === "working";
            const canResume = workstream.state === "paused";
            const canStop = canPause || canResume;
            return <article id={`ambient-workstream-${workstream.id}`} tabIndex={-1} key={workstream.id} data-state={workstream.state}><header><span className="ix-agent-state" data-active={run.state.status === "running" && workstream.state === "working" || undefined}>{workstream.state.replace("-", " ")}</span><h4>{workstream.name}</h4></header><p>{workstream.purpose}</p><strong>{workstream.update}</strong><ClaimTrace trace={workstream.trace} />{canPause || canResume || canStop ? <div className="ix-inline-actions">{canPause ? <button id={`ambient-workstream-control-${workstream.id}`} type="button" onClick={() => command(workstream, "pause")}>Pause</button> : null}{canResume ? <button id={`ambient-workstream-control-${workstream.id}`} type="button" onClick={() => command(workstream, "resume")}>Resume {workstream.resumeState ?? "prior work"}</button> : null}{canStop ? <button type="button" onClick={() => command(workstream, "stop")}>Stop</button> : null}</div> : <small>No playback control is available for this {workstream.state.replace("-", " ")} workstream.</small>}</article>;
          })}</div></section>

          <section className="ix-agent-tuning" aria-labelledby="agent-tuning-title"><header><span><span className="ix-eyebrow">Tune what reaches you · revision {tuningRevision}</span><h4 id="agent-tuning-title">{tuningSummary(appliedTuning)}</h4></span>{!previewing ? <Button id="ambient-tuning-control" size="sm" variant="secondary" onClick={openPreview}>Preview tuning</Button> : null}</header>
            {previewing && preview ? <div className="ix-tuning-form"><label htmlFor="ambient-tuning-relevance">Relevance<select id="ambient-tuning-relevance" value={draftTuning.relevance} onChange={(event) => setDraftTuning((current) => ({ ...current, relevance: event.currentTarget.value as IxAmbientTuning["relevance"] }))}><option value="material">Material changes only</option><option value="operational">Include operational detail</option></select></label><label>Digest cadence<select value={draftTuning.cadence} onChange={(event) => setDraftTuning((current) => ({ ...current, cadence: event.currentTarget.value as IxAmbientTuning["cadence"] }))}><option value="daily">Daily</option><option value="weekly">Weekly</option></select></label><label className="ix-check"><input type="checkbox" checked={draftTuning.quiet} onChange={(event) => setDraftTuning((current) => ({ ...current, quiet: event.currentTarget.checked }))} /> Quiet from 6 PM to 7 AM</label><aside><strong>30-day deterministic preview:</strong> From {preview.eventCount} fixture events, {preview.qualified} qualify, {preview.deferred} are deferred by quiet hours, and {preview.deliveries} digest {preview.deliveries === 1 ? "delivery" : "deliveries"} would appear.</aside><details className="ix-tuning-corpus"><summary>Inspect the 11-event fixture corpus and formula</summary><p><strong>Window:</strong> {preview.windowStart} through {preview.windowEnd}</p><p><strong>Timezone:</strong> {preview.timezone}</p><p><strong>Formula:</strong> {preview.formula}</p><ol>{recipe.tuningCorpus.events.map((event) => <li key={event.id}><strong>{event.occurredAt}</strong> · {event.label} · {event.material ? "material" : "operational"}{event.inQuietHours ? " · quiet hours" : ""}</li>)}</ol></details><div className="ix-inline-actions"><button type="button" onClick={applyPreview}>Apply as revision {tuningRevision + 1}</button><button type="button" onClick={cancelPreview}>Cancel preview</button></div></div> : null}
            <footer><span><strong>Authority is not tunable.</strong> {recipe.fixedAuthority}</span><Button size="sm" variant="quiet" disabled={!previousTuning} onClick={rollback}>Rollback tuning</Button></footer>
          </section>
        </section>
      ) : null}
      <ReferenceBoundaryNote>This page simulates since-away continuity. It has no live provider, scheduler, durable worker, background execution, or permission to act.</ReferenceBoundaryNote>
    </section>
  );
}
