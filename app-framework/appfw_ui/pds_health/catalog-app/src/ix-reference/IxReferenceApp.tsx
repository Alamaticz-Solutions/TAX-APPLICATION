import { useState } from "react";
import { Badge, PdsHealthLogo } from "@appfw/pds-health-components";
import type { IxArtifact } from "./contract";
import { AnalyzeWhyRecipe } from "./AnalyzeWhyRecipe";
import { ContextualConversationRecipe } from "./ContextualConversationRecipe";
import { AdaptiveCompositionRecipe } from "./AdaptiveCompositionRecipe";
import { WorkingGoalPlanRecipe } from "./WorkingGoalPlanRecipe";
import { AdaptiveInformationLensRecipe } from "./AdaptiveInformationLensRecipe";
import { SituationToStrategyRecipe } from "./SituationToStrategyRecipe";
import { AttentionStewardshipRecipe } from "./AttentionStewardshipRecipe";
import { AmbientAgentContinuityRecipe } from "./AmbientAgentContinuityRecipe";
import {
  IX_REFERENCES,
  type IxReferenceId
} from "./fixtures";

export default function IxReferenceApp() {
  const [activeReference, setActiveReference] = useState<IxReferenceId>("analyze-why");
  const [workingBrief, setWorkingBrief] = useState<IxArtifact>();
  const [conversationRequest, setConversationRequest] = useState({ id: 0, question: "" });

  function openConversation(question = "") {
    setConversationRequest((current) => ({ id: current.id + 1, question }));
    setActiveReference("contextual-conversation");
  }

  return (
    <main className="ix-gallery">
      <header className="ix-gallery__masthead">
        <a className="ix-gallery__brand" href="./" target="_top" aria-label="Return to the full PDS Design System">
          <PdsHealthLogo variant="wordmark" />
          <span>PDS Design System</span>
        </a>
        <span className="ix-gallery__mode">Deterministic IX reference streams</span>
      </header>

      <section className="ix-gallery__intro" aria-labelledby="ix-gallery-title">
        <span>
          <span className="ix-eyebrow">Intelligent Experience reference gallery</span>
          <h1 id="ix-gallery-title">Intelligence that lives in the work</h1>
          <p>
            Eight interactive experience references test one theory: intelligence should notice,
            explain, create useful work, accept challenge, and keep the user in control.
          </p>
        </span>
        <Badge tone="accent">8 interactive references</Badge>
      </section>

      <nav className="ix-reference-nav" aria-label="Eight Intelligent Experience references">
        {IX_REFERENCES.map((reference) => (
          <button
            type="button"
            key={reference.id}
            className="ix-reference-nav__item"
            data-active={activeReference === reference.id || undefined}
            aria-current={activeReference === reference.id ? "page" : undefined}
            onClick={() => setActiveReference(reference.id)}
          >
            <span>{reference.number}</span>
            <strong>{reference.name}</strong>
            <small>Interactive</small>
          </button>
        ))}
      </nav>

      <section className="ix-gallery__workspace">
        {activeReference === "analyze-why" ? (
          <AnalyzeWhyRecipe
            workingBrief={workingBrief}
            onArtifactChange={setWorkingBrief}
            onAskAboutThis={openConversation}
          />
        ) : null}
        {activeReference === "contextual-conversation" ? (
          <ContextualConversationRecipe
            key={conversationRequest.id}
            workingBrief={workingBrief}
            initialQuestion={conversationRequest.question}
            onWorkingBriefChange={setWorkingBrief}
            onBackToAnalysis={() => setActiveReference("analyze-why")}
          />
        ) : null}
        {activeReference === "adaptive-composition" ? <AdaptiveCompositionRecipe /> : null}
        {activeReference === "working-goal-plan" ? <WorkingGoalPlanRecipe /> : null}
        {activeReference === "adaptive-information-lens" ? <AdaptiveInformationLensRecipe /> : null}
        {activeReference === "situation-to-strategy" ? <SituationToStrategyRecipe /> : null}
        {activeReference === "attention-stewardship" ? <AttentionStewardshipRecipe /> : null}
        {activeReference === "ambient-agent-continuity" ? <AmbientAgentContinuityRecipe /> : null}
      </section>

      <footer className="ix-gallery__footer">
        <span>Reference behavior only. Live provider and durable continuity arrive through the App Framework IX core.</span>
        <a href="./" target="_top" aria-label="Open the full PDS Design System">Open the full Design System</a>
      </footer>
    </main>
  );
}
