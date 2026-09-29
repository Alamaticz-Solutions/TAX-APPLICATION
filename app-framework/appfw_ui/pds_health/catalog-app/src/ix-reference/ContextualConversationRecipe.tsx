import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import {
  Badge,
  Button,
  KpiTile,
  MetricTrend
} from "@appfw/pds-health-components";
import type { IxArtifact, IxArtifactRegion } from "./contract";
import {
  ReferenceBoundaryNote,
  ContextSet,
  IntelligenceProgress,
  ProgressiveArtifact
} from "./components";
import { buildConversationSchedule, RECOVERY_CONTEXT } from "./fixtures";
import { useFixtureRun } from "./useFixtureRun";

const QUICK_QUESTIONS = [
  "Why did recovery readiness fall?",
  "What would change the outlook?",
  "Challenge the weakest assumption"
] as const;

export function ContextualConversationRecipe({
  workingBrief,
  initialQuestion,
  onWorkingBriefChange,
  onBackToAnalysis
}: {
  workingBrief?: IxArtifact;
  initialQuestion?: string;
  onWorkingBriefChange: (artifact: IxArtifact) => void;
  onBackToAnalysis: () => void;
}) {
  const run = useFixtureRun();
  const [input, setInput] = useState(initialQuestion ?? "");
  const [currentQuestion, setCurrentQuestion] = useState("");
  const [addedRegionId, setAddedRegionId] = useState<string>();
  const runOrdinal = useRef(0);

  function ask(question: string) {
    const trimmed = question.trim();
    if (!trimmed || run.state.status === "running") return;
    setCurrentQuestion(trimmed);
    setInput("");
    setAddedRegionId(undefined);
    runOrdinal.current += 1;
    run.start(buildConversationSchedule(trimmed, runOrdinal.current));
  }

  useEffect(() => {
    if (!initialQuestion) return;
    setInput(initialQuestion);
    ask(initialQuestion);
  // `ask` intentionally uses the current deterministic controller; initialQuestion is the trigger.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialQuestion]);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    ask(input);
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      ask(input);
    }
  }

  function addFinding(region: IxArtifactRegion) {
    if (!workingBrief) return;
    const id = `follow-up-${region.id}-${workingBrief.revision + 1}`;
    const finding: IxArtifactRegion = {
      ...region,
      id,
      label: "Follow-up finding",
      title: `${region.title} · added from the contextual question`,
      body: region.body,
      whyItMatters: "The follow-up now belongs to the working artifact, not only to the conversation."
    };
    onWorkingBriefChange({
      ...workingBrief,
      revision: workingBrief.revision + 1,
      status: "ready",
      changedRegionIds: [id],
      regions: [...workingBrief.regions, finding]
    });
    setAddedRegionId(region.id);
  }

  const answer = run.state.artifact;
  const directAnswer = answer?.regions[0];

  return (
    <section className="ix-recipe" aria-labelledby="contextual-conversation-title">
      <header className="ix-recipe__header">
        <span>
          <span className="ix-eyebrow">Reference 2 · Open-ended interaction</span>
          <h2 id="contextual-conversation-title">Ask about this, without losing the work</h2>
          <p>The question remains attached to the metric, evidence, and editable brief. The answer arrives as inspectable objects—not only prose.</p>
        </span>
        <Badge tone="accent">Same context · same work loop</Badge>
      </header>

      <div className="ix-conversation-layout">
        <aside className="ix-conversation-context" aria-label="Current application context">
          <KpiTile
            label="Recovery readiness"
            value="67%"
            detail="Current quarter · target 85%"
            tone="warning"
            trend={<MetricTrend value="−15 pts" label="this quarter" tone="negative" direction="down" />}
          />
          <section className="ix-brief-connection">
            <span className="ix-eyebrow">Working artifact connected</span>
            <strong>{workingBrief?.title ?? "No working brief yet"}</strong>
            <span>
              {workingBrief
                ? `Revision ${workingBrief.revision} · ${workingBrief.regions.length} usable regions`
                : "Return to Analyze Why to create the first artifact."}
            </span>
            <Button size="sm" variant="quiet" onClick={onBackToAnalysis}>Open analysis</Button>
          </section>
        </aside>

        <div className="ix-conversation-work">
          <ContextSet context={run.state.context} fallback={RECOVERY_CONTEXT} />

          {currentQuestion ? (
            <section className="ix-current-question" aria-label="Current contextual question">
              <span className="ix-eyebrow">You asked about this metric</span>
              <strong>{currentQuestion}</strong>
            </section>
          ) : (
            <section className="ix-current-question">
              <span className="ix-eyebrow">Ready in this context</span>
              <strong>Probe the decline, test an assumption, or explore what could change.</strong>
            </section>
          )}

          {run.state.status !== "idle" ? (
            <IntelligenceProgress state={run.state} onStop={run.stop} onResume={run.resume} />
          ) : null}

          <ProgressiveArtifact
            artifact={answer}
            active={run.state.status === "running"}
            emptyMessage="Typed answer objects will appear here as soon as each is ready."
            onChallenge={(prompt) => setInput(prompt)}
            footer={directAnswer ? (
              <>
                <span>The answer remains inspectable before it is added to the brief.</span>
                <Button
                  variant="primary"
                  disabled={!workingBrief || addedRegionId === directAnswer.id}
                  onClick={() => addFinding(directAnswer)}
                >
                  {addedRegionId === directAnswer.id ? "Added to working brief" : "Add finding to working brief"}
                </Button>
              </>
            ) : undefined}
          />

          <div className="ix-quick-questions" aria-label="Suggested contextual questions">
            {QUICK_QUESTIONS.map((question) => (
              <button
                className="ix-choice"
                type="button"
                key={question}
                disabled={run.state.status === "running"}
                onClick={() => ask(question)}
              >
                {question}
              </button>
            ))}
          </div>

          <form className="ix-composer" onSubmit={submit}>
            <label htmlFor="ix-contextual-question">Ask a follow-up about recovery readiness</label>
            <span className="ix-composer__row">
              <textarea
                id="ix-contextual-question"
                rows={2}
                value={input}
                placeholder="Ask a follow-up about this metric…"
                disabled={run.state.status === "running"}
                onChange={(event) => setInput(event.currentTarget.value)}
                onKeyDown={handleComposerKeyDown}
              />
              <Button
                type="submit"
                variant="primary"
                disabled={!input.trim() || run.state.status === "running"}
              >
                Ask
              </Button>
            </span>
          </form>
        </div>
      </div>

      <ReferenceBoundaryNote>
        This is intentionally not a general chat destination. Conversation is one way to redirect or challenge a continuing application task and its artifact.
      </ReferenceBoundaryNote>
    </section>
  );
}
