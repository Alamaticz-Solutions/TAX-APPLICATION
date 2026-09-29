import { useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildAdaptiveInformationLensSchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

export function AdaptiveInformationLensRecipe() {
  const run = useFixtureRun();
  const [lens, setLens] = useState<"executive-read" | "operator-read">("executive-read");
  const [keptIds, setKeptIds] = useState<readonly string[]>([]);
  const recipe = run.state.artifact?.referenceRecipe?.kind === "adaptive-information-lens" ? run.state.artifact.referenceRecipe : undefined;
  function applyLens(nextLens: typeof lens) {
    setLens(nextLens);
    const preservedArtifact = run.state.artifact;
    run.start(
      buildAdaptiveInformationLensSchedule(nextLens, (preservedArtifact?.revision ?? 0) + 1, keptIds),
      { preservedArtifact }
    );
  }
  function toggleKeep(id: string) {
    setKeptIds((current) => current.includes(id) ? current.filter((candidate) => candidate !== id) : [...current, id]);
  }

  return (
    <section className="ix-recipe" aria-labelledby="information-lens-title">
      <header className="ix-recipe__header"><span><span className="ix-eyebrow">Reference 5 · Role and task fit</span><h2 id="information-lens-title">Replace the giant form with the useful read</h2><p>The canonical record stays complete. Intelligence selects what this task needs, explains why, and keeps Full record one gesture away.</p></span><Badge tone="accent">Purpose-shaped, never lossy</Badge></header>
      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      <div className="ix-recipe-actions"><span>Current task:</span><button className="ix-choice" type="button" aria-pressed={lens === "executive-read"} disabled={run.state.status === "running"} onClick={() => applyLens("executive-read")}>Decide whether to intervene</button><button className="ix-choice" type="button" aria-pressed={lens === "operator-read"} disabled={run.state.status === "running"} onClick={() => applyLens("operator-read")}>Prepare recovery work</button></div>
      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />
      {recipe ? (
        <section className="ix-special-artifact" aria-labelledby="lens-artifact-title">
          <header className="ix-special-artifact__header"><span><span className="ix-eyebrow">{recipe.lens === "executive-read" ? "Executive read" : "Operator read"}</span><h3 id="lens-artifact-title">{recipe.objective}</h3></span><Badge tone="neutral">{recipe.visibleFields.length} fields in focus</Badge></header>
          <dl className="ix-information-lens">{recipe.visibleFields.map((field) => <div key={field.id}><dt>{field.label}</dt><dd>{field.value}</dd><small>{field.reason}</small><ClaimTrace trace={field.trace} /><button className="ix-text-action" type="button" aria-pressed={keptIds.includes(field.id)} onClick={() => toggleKeep(field.id)}>{keptIds.includes(field.id) ? "Remove keep" : "Keep in view"}</button></div>)}</dl>
          <details className="ix-full-record"><summary>Open full record · {recipe.fullRecordFields.length} fields</summary><dl>{recipe.fullRecordFields.map((field) => <div key={field.id}><dt>{field.label}</dt><dd>{field.value}</dd><ClaimTrace trace={field.trace} /></div>)}</dl></details>
          <p className="ix-lens-explanation"><strong>Why this shape:</strong> {recipe.omissionExplanation}</p>
        </section>
      ) : <div className="ix-artifact-empty"><Button variant="primary" onClick={() => applyLens(lens)}>Shape this record for my task</Button></div>}
      <ReferenceBoundaryNote>This fixture changes presentation, not authorization or canonical data. Role alone never decides what a user may access.</ReferenceBoundaryNote>
    </section>
  );
}
