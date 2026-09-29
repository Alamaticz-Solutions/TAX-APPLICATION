import { useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import type { IxStrategyChallengeCategory } from "./contract";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildSituationToStrategySchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

export function SituationToStrategyRecipe() {
  const run = useFixtureRun();
  const [challengeCategory, setChallengeCategory] = useState<IxStrategyChallengeCategory>("source-freshness");
  const [challenge, setChallenge] = useState("What if the stale source has already improved?");
  const recipe = run.state.artifact?.referenceRecipe?.kind === "situation-to-strategy" ? run.state.artifact.referenceRecipe : undefined;
  const start = (challenged = false) => {
    const preservedArtifact = run.state.artifact;
    const preservedRecipe = preservedArtifact?.referenceRecipe?.kind === "situation-to-strategy"
      ? preservedArtifact.referenceRecipe
      : undefined;
    run.start(
      buildSituationToStrategySchedule(
        challenged ? { category: challengeCategory, text: challenge.trim() } : undefined,
        (preservedArtifact?.revision ?? 0) + 1,
        preservedRecipe ? {
          thesis: preservedRecipe.thesis,
          reasoning: preservedRecipe.revisionChange ?? "Prior thesis and evidence retained from the previous revision."
        } : undefined
      ),
      { preservedArtifact }
    );
  };
  return (
    <section className="ix-recipe" aria-labelledby="situation-strategy-title">
      <header className="ix-recipe__header"><span><span className="ix-eyebrow">Reference 6 · Whole-picture reasoning</span><h2 id="situation-strategy-title">Turn signals into a strategy you can try to disprove</h2><p>Intelligence forms a useful point of view, then exposes its assumptions, counterview, intervention, and measure.</p></span><Badge tone="accent">Falsifiable, not fluent</Badge></header>
      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />
      {recipe ? (
        <section className="ix-special-artifact ix-strategy" aria-labelledby="strategy-artifact-title">
          <header className="ix-special-artifact__header"><span><span className="ix-eyebrow">Working thesis · {recipe.confidence} confidence</span><h3 id="strategy-artifact-title">{recipe.thesis}</h3><ClaimTrace trace={recipe.thesisTrace} /></span><Badge tone="warning">Provisional</Badge></header>
          {recipe.priorThesis ? <aside className="ix-prior-thesis"><span className="ix-eyebrow">Preserved from the prior revision</span><strong>{recipe.priorThesis.thesis}</strong><p>{recipe.priorThesis.reasoning}</p>{recipe.challenge ? <p><b>Challenge retained:</b> {recipe.challenge.text} · {recipe.challenge.category.replace("-", " ")}</p> : null}<b>What changed:</b> {recipe.revisionChange}</aside> : null}
          <div className="ix-strategy__grid"><section><h4>Signals this explains</h4><ul>{recipe.signals.map((signal) => <li key={signal}>{signal}</li>)}</ul></section><section><h4>Proposed intervention</h4><p>{recipe.intervention}</p><strong>Measure</strong><p>{recipe.measure}</p><ClaimTrace trace={recipe.interventionTrace} /></section></div>
          <section><h4>Assumptions to test</h4><div className="ix-assumption-grid">{recipe.assumptions.map((assumption) => <article key={assumption.id}><strong>{assumption.statement}</strong><p><b>Test:</b> {assumption.test}</p><ClaimTrace trace={assumption.trace} /><button className="ix-text-action" type="button" onClick={() => { setChallengeCategory(assumption.id.startsWith("capacity") ? "capacity" : assumption.id.startsWith("causal") ? "causal-link" : "source-freshness"); setChallenge(assumption.test); }}>Probe this assumption</button></article>)}</div></section>
          {recipe.counterview ? <aside className="ix-counterview"><strong>Counterview now in the thesis:</strong> {recipe.counterview}</aside> : null}
          <footer className="ix-strategy-challenge"><label htmlFor="ix-strategy-category">Challenge category<select id="ix-strategy-category" value={challengeCategory} onChange={(event) => setChallengeCategory(event.currentTarget.value as IxStrategyChallengeCategory)}><option value="source-freshness">Source freshness</option><option value="capacity">Available capacity</option><option value="causal-link">Causal link</option></select></label><label htmlFor="ix-strategy-challenge">Your challenge or counterevidence<input id="ix-strategy-challenge" value={challenge} onChange={(event) => setChallenge(event.currentTarget.value)} /></label><span>Challenge text and category are retained. Consecutive challenges continue the current thesis revision rather than restarting.</span><Button variant="primary" disabled={run.state.status === "running" || !challenge.trim()} onClick={() => start(true)}>Test this challenge</Button></footer>
        </section>
      ) : <div className="ix-artifact-empty"><Button variant="primary" onClick={() => start(false)}>Form a strategy from this picture</Button></div>}
      <ReferenceBoundaryNote>Deterministic reasoning fixture only. The thesis is explicitly provisional and no program or action is created.</ReferenceBoundaryNote>
    </section>
  );
}
