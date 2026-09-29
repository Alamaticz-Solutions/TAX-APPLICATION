import { useEffect, useMemo, useRef, useState } from "react";
import { Badge, Button } from "@appfw/pds-health-components";
import type { IxAttentionItem } from "./contract";
import { ClaimTrace, ReferenceBoundaryNote, ContextSet, IntelligenceProgress, RecipeRevisionAnnouncement } from "./components";
import { buildAttentionStewardshipSchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

type Correction = { itemId: string; prior: IxAttentionItem["state"]; next: IxAttentionItem["state"] };

export function AttentionStewardshipRecipe() {
  const run = useFixtureRun();
  const [states, setStates] = useState<Record<string, IxAttentionItem["state"]>>({});
  const [lastCorrection, setLastCorrection] = useState<Correction>();
  const [stateAnnouncement, setStateAnnouncement] = useState("");
  const pendingFocusRef = useRef<string>();
  const recipe = run.state.artifact?.referenceRecipe?.kind === "attention-stewardship" ? run.state.artifact.referenceRecipe : undefined;
  const items = useMemo(() => recipe?.items.map((item) => ({ ...item, state: states[item.id] ?? item.state })) ?? [], [recipe, states]);
  const activeItems = items.filter(({ state }) => state === "open");
  const correctedItems = items.filter(({ state }) => state !== "open");

  useEffect(() => {
    const targetId = pendingFocusRef.current;
    if (!targetId) return;
    document.getElementById(targetId)?.focus();
    pendingFocusRef.current = undefined;
  }, [lastCorrection, states]);

  function correct(item: IxAttentionItem, next: IxAttentionItem["state"]) {
    const prior = states[item.id] ?? item.state;
    pendingFocusRef.current = next === "open"
      ? `attention-action-${item.id}`
      : `attention-restore-${item.id}`;
    setStates((current) => ({ ...current, [item.id]: next }));
    setLastCorrection({ itemId: item.id, prior, next });
    setStateAnnouncement(next === "open"
      ? `${item.title} restored to active attention.`
      : `${item.title} moved to corrected or deferred as ${next.replace("-", " ")}.`);
  }
  function undo() {
    if (!lastCorrection) return;
    const item = items.find(({ id }) => id === lastCorrection.itemId);
    pendingFocusRef.current = lastCorrection.prior === "open"
      ? `attention-action-${lastCorrection.itemId}`
      : `attention-restore-${lastCorrection.itemId}`;
    setStates((current) => ({ ...current, [lastCorrection.itemId]: lastCorrection.prior }));
    setLastCorrection(undefined);
    setStateAnnouncement(item
      ? `${item.title} restored to ${lastCorrection.prior.replace("-", " ")}.`
      : `The last attention correction was restored to ${lastCorrection.prior.replace("-", " ")}.`);
  }
  function scan(empty: boolean) {
    const preservedArtifact = run.state.artifact;
    run.start(
      buildAttentionStewardshipSchedule(empty, (preservedArtifact?.revision ?? 0) + 1),
      { preservedArtifact }
    );
  }

  return (
    <section className="ix-recipe" aria-labelledby="attention-title">
      <header className="ix-recipe__header"><span><span className="ix-eyebrow">Reference 7 · Calm attention</span><h2 id="attention-title">Show what I can change—not every warning</h2><p>Reason across materiality, ownership, timing, and ability to act. Let the user correct the ranking and show an honest empty state.</p></span><Badge tone="accent">Attention is scarce</Badge></header>
      <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />
      <p className="ix-sr-only" role="status" aria-live="polite" aria-atomic="true">{stateAnnouncement}</p>
      <div className="ix-recipe-actions"><Button variant="primary" disabled={run.state.status === "running"} onClick={() => scan(false)}>What needs my attention?</Button><Button variant="secondary" disabled={run.state.status === "running"} onClick={() => scan(true)}>Show the calm state</Button>{lastCorrection ? <Button variant="quiet" onClick={undo}>Undo last correction</Button> : null}</div>
      {run.state.status !== "idle" ? <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} /> : null}
      <RecipeRevisionAnnouncement artifact={run.state.artifact} />
      {recipe ? (
        <section className="ix-special-artifact" aria-labelledby="attention-artifact-title">
          <header className="ix-special-artifact__header"><span><span className="ix-eyebrow">Evaluated {recipe.evaluatedCount} signals</span><h3 id="attention-artifact-title">{items.length ? `${activeItems.length} ${activeItems.length === 1 ? "thing" : "things"} may need you` : "No owned action is ready"}</h3></span><Badge tone={items.length ? "warning" : "neutral"}>{items.length ? "Bounded list" : "Unresolved signals remain"}</Badge></header>
          {items.length ? <><section aria-labelledby="active-attention-title"><h4 id="active-attention-title">Active attention</h4><div className="ix-attention-list">{activeItems.length ? activeItems.map((item) => <AttentionCard key={item.id} item={item} mode="active" onCorrect={correct} />) : <p className="ix-muted-state">You corrected every proposed item. They remain inspectable below.</p>}</div></section>{correctedItems.length ? <section className="ix-corrected-attention" aria-labelledby="corrected-attention-title"><h4 id="corrected-attention-title">Corrected or deferred</h4><div className="ix-attention-list">{correctedItems.map((item) => <AttentionCard key={item.id} item={item} mode="corrected" onCorrect={correct} />)}</div></section> : null}</> : <div className="ix-calm-state"><strong>No action is assigned to this advisory view.</strong><p>{recipe.emptyMessage}</p><span>The fixture used a distinct authorized context; its unavailable handoff detail remains visible rather than being called all clear.</span></div>}
          {lastCorrection ? <p className="ix-correction-note">Correction recorded for this fixture: {lastCorrection.next.replace("-", " ")}. This feedback changes the local attention view and can be undone.</p> : null}
        </section>
      ) : null}
      <ReferenceBoundaryNote>This reference neither assigns work nor claims a durable attention ledger. Corrections are local fixture state and reset with the page.</ReferenceBoundaryNote>
    </section>
  );
}

function AttentionCard({ item, mode, onCorrect }: { item: IxAttentionItem; mode: "active" | "corrected"; onCorrect: (item: IxAttentionItem, state: IxAttentionItem["state"]) => void }) {
  return (
    <article id={`attention-item-${item.id}`} tabIndex={-1} data-state={item.state}>
      <span className="ix-eyebrow">{item.state}</span><h4>{item.title}</h4>
      <strong>Why this</strong><p>{item.reason}</p>
      <strong>Why me</strong><p>{item.whyMe}</p>
      <strong>How priority was derived</strong><p>{item.priorityDerivation}</p>
      <strong>Useful move</strong><p>{item.usefulMove}</p>
      <ClaimTrace trace={item.trace} />
      {mode === "active" ? <div className="ix-inline-actions"><button id={`attention-action-${item.id}`} type="button" onClick={() => onCorrect(item, "covered")}>Already covered</button><button type="button" onClick={() => onCorrect(item, "not-mine")}>Not mine</button><button type="button" onClick={() => onCorrect(item, "snoozed")}>Snooze</button><button type="button" onClick={() => onCorrect(item, "challenged")}>Challenge priority</button></div> : <div className="ix-inline-actions"><button id={`attention-restore-${item.id}`} type="button" onClick={() => onCorrect(item, "open")}>Restore to active attention</button></div>}
    </article>
  );
}
