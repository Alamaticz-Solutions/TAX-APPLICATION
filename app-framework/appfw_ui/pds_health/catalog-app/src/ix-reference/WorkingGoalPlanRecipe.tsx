import { useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildWorkingGoalPlanSchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

export function WorkingGoalPlanRecipe() {
  const run = useFixtureRun();
  const [humanNotes, setHumanNotes] = useState<Record<string, { label: string; note: string }>>({});
  const [revised, setRevised] = useState(false);
  const recipe = run.state.artifact?.referenceRecipe?.kind === "working-goal-plan"
    ? run.state.artifact.referenceRecipe
    : undefined;
  const visibleStepIds = new Set(recipe?.steps.map(({ id }) => id) ?? []);
  const retainedNotes = Object.entries(humanNotes).filter(([id, value]) => value.note.trim() && !visibleStepIds.has(id));

  function start(nextRevised = false) {
    setRevised(nextRevised);
    const preservedArtifact = run.state.artifact;
    run.start(
      buildWorkingGoalPlanSchedule(nextRevised, (preservedArtifact?.revision ?? 0) + 1),
      { preservedArtifact }
    );
  }

  return (
    <section className="ix-recipe" aria-labelledby="working-goal-plan-title">
      <header className="ix-recipe__header"><span><span className="ix-eyebrow">Reference 4 · Agentic process</span><h2 id="working-goal-plan-title">Keep the goal stable while the path learns</h2><p>Agentic reasoning proposes outcome-based steps, while required checkpoints, existing work, and human notes stay explicit.</p></span><Badge tone="accent">Revisable, not improvised</Badge></header>
      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />
      {recipe ? (
        <section className="ix-special-artifact" aria-labelledby="goal-plan-artifact-title">
          <header className="ix-special-artifact__header"><span><span className="ix-eyebrow">Stable goal</span><h3 id="goal-plan-artifact-title">{recipe.goal}</h3><p>{recipe.revisionNote}</p></span><Badge tone="neutral">Plan revision {run.state.artifact?.revision}</Badge></header>
          {recipe.preservedWork.length ? <aside className="ix-preserved-work"><strong>Preserved through this replan:</strong> {recipe.preservedWork.join(" · ")}</aside> : null}
          <ol className="ix-goal-plan">
            {recipe.steps.map((step, index) => (
              <li key={step.id} data-state={step.state}>
                <span className="ix-goal-plan__number">{index + 1}</span>
                <span><strong>{step.outcome}</strong><small>{step.state === "required" ? "Required human checkpoint" : step.state}</small>{step.checkpoint ? <p>{step.checkpoint}</p> : null}
                  <ClaimTrace trace={step.trace} />
                  <label className="ix-inline-note">Your note<input value={humanNotes[step.id]?.note ?? ""} onChange={(event) => {
                    const note = event.currentTarget.value;
                    setHumanNotes((current) => ({ ...current, [step.id]: { label: step.outcome, note } }));
                  }} /></label>
                </span>
              </li>
            ))}
          </ol>
          {retainedNotes.length ? <aside className="ix-retained-notes"><strong>Notes retained from the earlier path</strong>{retainedNotes.map(([, value]) => <p key={value.label}><b>{value.label}:</b> {value.note}</p>)}</aside> : null}
          <footer className="ix-special-artifact__footer"><span>{revised ? "The blocked path changed; your notes remain." : "A newly discovered constraint can revise this path."}</span><Button variant="primary" disabled={run.state.status === "running"} onClick={() => start(true)}>Replan around external handoff</Button></footer>
        </section>
      ) : <div className="ix-artifact-empty"><Button variant="primary" onClick={() => start(false)}>Build a working plan</Button><span> The plan will expose its first usable steps while it is still checking the path.</span></div>}
      <ReferenceBoundaryNote>The plan is a deterministic, editable reference. It does not create assignments or replace mandatory product controls.</ReferenceBoundaryNote>
    </section>
  );
}
