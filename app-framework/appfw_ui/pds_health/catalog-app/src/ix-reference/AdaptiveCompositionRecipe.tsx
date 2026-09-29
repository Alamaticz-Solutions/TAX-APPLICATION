import { useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildAdaptiveCompositionSchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

export function AdaptiveCompositionRecipe() {
  const run = useFixtureRun();
  const [focus, setFocus] = useState<"operating-picture" | "decisions">("operating-picture");
  const [pinnedIds, setPinnedIds] = useState<readonly string[]>([]);
  const recipe = run.state.artifact?.referenceRecipe?.kind === "adaptive-composition"
    ? run.state.artifact.referenceRecipe
    : undefined;

  function compose(nextFocus = focus) {
    setFocus(nextFocus);
    const preservedArtifact = run.state.artifact;
    run.start(
      buildAdaptiveCompositionSchedule(nextFocus, pinnedIds, (preservedArtifact?.revision ?? 0) + 1),
      { preservedArtifact }
    );
  }

  function togglePin(id: string) {
    setPinnedIds((current) => current.includes(id)
      ? current.filter((candidate) => candidate !== id)
      : [...current, id]);
  }

  return (
    <section className="ix-recipe" aria-labelledby="adaptive-composition-title">
      <header className="ix-recipe__header">
        <span>
          <span className="ix-eyebrow">Reference 3 · Intelligent layout</span>
          <h2 id="adaptive-composition-title">Show the right shape for the work</h2>
          <p>The same governed facts become a useful operating view or decision view. Your focus and pins survive recomposition.</p>
        </span>
        <Badge tone="accent">Registered components only</Badge>
      </header>

      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      <div className="ix-recipe-actions" aria-label="Choose composition focus">
            <span>Compose for:</span>
        <button className="ix-choice" type="button" aria-pressed={focus === "operating-picture"} disabled={run.state.status === "running"} onClick={() => compose("operating-picture")}>Operating picture</button>
        <button className="ix-choice" type="button" aria-pressed={focus === "decisions"} disabled={run.state.status === "running"} onClick={() => compose("decisions")}>Decisions needed</button>
        {recipe ? <Button size="sm" variant="secondary" disabled={run.state.status === "running"} onClick={() => compose()}>Recompose with kept items</Button> : null}
      </div>

      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />

      {recipe ? (
        <section className="ix-special-artifact" aria-labelledby="composition-artifact-title">
          <header className="ix-special-artifact__header">
            <span><span className="ix-eyebrow">Adaptive composition</span><h3 id="composition-artifact-title">Recovery readiness · {recipe.focus === "decisions" ? "decision view" : "operating view"}</h3></span>
            <Badge tone="neutral">Revision {run.state.artifact?.revision}</Badge>
          </header>
          <div className="ix-composition-grid" data-focus={recipe.focus}>
            {recipe.blocks.map((block) => (
              <article className="ix-purpose-card" data-kind={block.kind} key={block.id}>
                <span className="ix-eyebrow" data-active={run.state.status === "running" && run.state.artifact?.changedRegionIds.includes(block.id) || undefined}>{block.kind}</span>
                <h4>{block.title}</h4><p>{block.body}</p>
                <details><summary>Why is this here?</summary><p>{block.whyIncluded}</p></details>
                <ClaimTrace trace={block.trace} />
                <button className="ix-text-action" type="button" aria-pressed={pinnedIds.includes(block.id)} onClick={() => togglePin(block.id)}>{pinnedIds.includes(block.id) ? "Remove keep" : "Keep in view"}</button>
              </article>
            ))}
          </div>
          <aside className="ix-change-summary"><strong>What changed in this view</strong><ul>{recipe.changeSummary.map((change) => <li key={change}>{change}</li>)}</ul></aside>
        </section>
      ) : <div className="ix-artifact-empty"><Button variant="primary" onClick={() => compose()}>Compose this view</Button><span> The first useful blocks appear before the composition finishes.</span></div>}

      <ReferenceBoundaryNote>Deterministic reference only. The recipe selects from registered PDS presentations; it does not execute model-authored UI.</ReferenceBoundaryNote>
    </section>
  );
}
