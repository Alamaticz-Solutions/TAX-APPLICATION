import { useEffect, useRef, type ReactElement, type RefObject } from "react";
import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  Eye,
  FileCheck2,
  Info,
  Link2,
  ReceiptText,
  Users
} from "lucide-react";
import { Button } from "../../components/ui";
import type {
  ActivityDecisionSpineViewModel,
  ActivityDecisionStageId,
  ActivityDecisionStageTone,
  ActivityLocalReviewState
} from "./activityDecisionSpineModel";

export function ActivityDecisionSpine({
  model,
  onLocalReviewStateChange
}: {
  model: ActivityDecisionSpineViewModel;
  onLocalReviewStateChange: (state: ActivityLocalReviewState) => void;
}) {
  const consequenceToolRef = useRef<HTMLDivElement>(null);
  const receiptRef = useRef<HTMLDivElement>(null);
  const pendingFocusRef = useRef<"record" | "receipt" | null>(null);

  useEffect(() => {
    if (pendingFocusRef.current === "record" && model.localReviewState === "inspected") {
      consequenceToolRef.current
        ?.querySelector<HTMLButtonElement>('button[data-activity-action="record-receipt"]')
        ?.focus();
      pendingFocusRef.current = null;
    }
    if (pendingFocusRef.current === "receipt" && model.localReviewState === "receipted") {
      receiptRef.current?.focus();
      pendingFocusRef.current = null;
    }
  }, [model.localReviewState]);

  function inspectConsequence() {
    pendingFocusRef.current = "record";
    onLocalReviewStateChange("inspected");
  }

  function recordReceipt() {
    pendingFocusRef.current = "receipt";
    onLocalReviewStateChange("receipted");
  }

  return (
    <section
      className="activity-decision-spine"
      aria-label="Decision path from relationship context to local receipt"
    >
      <header className="activity-decision-spine__header">
        <div>
          <p className="activity-experience__eyebrow">Local review</p>
          <h3>Decision path</h3>
        </div>
        <p>Follow the causal context in one place before acknowledging the review.</p>
      </header>
      <ol className="activity-decision-spine__rail" role="list">
        {model.stages.map((stage) => (
          <li
            className={`activity-decision-spine__stage is-${stage.tone}`}
            data-stage={stage.id}
            key={stage.id}
          >
            <div className="activity-decision-spine__stage-heading">
              <span className="activity-decision-spine__stage-icon" aria-hidden="true">
                <StageIcon id={stage.id} />
              </span>
              <strong>{stage.label}</strong>
            </div>
            <span className={`activity-decision-spine__state is-${stage.tone}`}>
              <StateIcon tone={stage.tone} />
              <span>{stage.stateLabel}</span>
            </span>
            <p>{stage.detail}</p>
            {stage.id === "consequence" ? (
              <ConsequenceTool
                model={model}
                toolRef={consequenceToolRef}
                onInspect={inspectConsequence}
                onRecordReceipt={recordReceipt}
              />
            ) : null}
            {stage.id === "receipt" && model.localReviewState === "receipted" ? (
              <div
                className="activity-decision-spine__receipt"
                ref={receiptRef}
                role="status"
                aria-live="polite"
                aria-atomic="true"
                tabIndex={-1}
              >
                <CheckCircle2 size={17} aria-hidden="true" />
                <span>
                  <strong>Local review receipt recorded</strong>
                  <span>No provider action was sent.</span>
                </span>
              </div>
            ) : null}
          </li>
        ))}
      </ol>
    </section>
  );
}

function ConsequenceTool({
  model,
  toolRef,
  onInspect,
  onRecordReceipt
}: {
  model: ActivityDecisionSpineViewModel;
  toolRef: RefObject<HTMLDivElement | null>;
  onInspect: () => void;
  onRecordReceipt: () => void;
}) {
  return (
    <div
      className="activity-decision-spine__tool"
      ref={toolRef}
      role="group"
      aria-label="Consequence review"
    >
      <span className="activity-decision-spine__local-label">Local preview only</span>
      {model.localReviewState !== "idle" ? (
        <div className="activity-decision-spine__inspection">
          <strong>{model.consequence.title}</strong>
          <p>{model.consequence.detail}</p>
          {model.constraint ? <p className="activity-decision-spine__constraint">{model.constraint}</p> : null}
        </div>
      ) : null}
      {model.canInspect ? (
        <Button variant="primary" size="sm" onClick={onInspect}>
          Inspect consequence
        </Button>
      ) : null}
      {model.canRecordReceipt ? (
        <Button
          data-activity-action="record-receipt"
          variant="primary"
          size="sm"
          onClick={onRecordReceipt}
        >
          Record local review receipt
        </Button>
      ) : null}
    </div>
  );
}

function StageIcon({ id }: { id: ActivityDecisionStageId }) {
  const icons: Record<ActivityDecisionStageId, ReactElement> = {
    relationship: <Users size={18} />,
    evidence: <FileCheck2 size={18} />,
    dependency: <Link2 size={18} />,
    consequence: <Eye size={18} />,
    receipt: <ReceiptText size={18} />
  };
  return icons[id];
}

function StateIcon({ tone }: { tone: ActivityDecisionStageTone }) {
  if (tone === "clear" || tone === "recorded") {
    return <CheckCircle2 size={14} aria-hidden="true" />;
  }
  if (tone === "blocked") {
    return <AlertCircle size={14} aria-hidden="true" />;
  }
  if (tone === "attention" || tone === "limited") {
    return <Clock3 size={14} aria-hidden="true" />;
  }
  return <Info size={14} aria-hidden="true" />;
}
