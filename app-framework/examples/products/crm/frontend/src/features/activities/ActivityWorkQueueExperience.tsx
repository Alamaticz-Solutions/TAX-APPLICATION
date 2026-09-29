import { useEffect, useId, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import {
  ChevronRight,
  Clock3,
  Info,
  RefreshCw,
  ShieldCheck
} from "lucide-react";
import { Button, FeedbackState } from "../../components/ui";
import { ActivityDecisionSpine } from "./ActivityDecisionSpine";
import {
  activityLocalReviewPostureKey,
  activityExperienceStateLabels,
  deriveActivityDecisionSpine,
  type ActivityDecisionSpineViewModel,
  type ActivityExperienceState,
  type ActivityExperienceSurface,
  type ActivityLocalReviewState
} from "./activityDecisionSpineModel";
import {
  activityWorkQueueFixture,
  type ActivityQueueFixtureItem
} from "./activityWorkQueueFixture";
import "./activityWorkQueueExperience.css";

export function ActivityWorkQueueExperience() {
  const [selectedId, setSelectedId] = useState(activityWorkQueueFixture[0].id);
  const [experienceState, setExperienceState] = useState<ActivityExperienceState>("ready");
  const [localReviewStates, setLocalReviewStates] = useState<
    Partial<Record<string, ActivityLocalReviewState>>
  >({});
  const stateControlRef = useRef<HTMLSelectElement>(null);
  const restoreStateControlFocusRef = useRef(false);
  const selected = useMemo(
    () => activityWorkQueueFixture.find((item) => item.id === selectedId) ?? activityWorkQueueFixture[0],
    [selectedId]
  );
  const localReviewKey = useMemo(
    () => activityLocalReviewPostureKey(selected, experienceState),
    [experienceState, selected]
  );
  const localReviewState = localReviewStates[localReviewKey] ?? "idle";
  const decisionModel = useMemo(
    () => deriveActivityDecisionSpine(selected, experienceState, localReviewState),
    [experienceState, localReviewState, selected]
  );

  function recover() {
    restoreStateControlFocusRef.current = true;
    setExperienceState("ready");
  }

  function updateLocalReviewState(state: ActivityLocalReviewState) {
    setLocalReviewStates((current) => ({ ...current, [localReviewKey]: state }));
  }

  useEffect(() => {
    if (experienceState === "ready" && restoreStateControlFocusRef.current) {
      stateControlRef.current?.focus();
      restoreStateControlFocusRef.current = false;
    }
  }, [experienceState]);

  return (
    <section className="activity-experience" aria-labelledby="activities-heading">
      <ExperienceHeader
        state={experienceState}
        onStateChange={setExperienceState}
        controlRef={stateControlRef}
      />
      {!decisionModel.surface.showWorkItem ? (
        <ExperienceStatePanel surface={decisionModel.surface} onRecover={recover} />
      ) : (
        <>
          {decisionModel.surface.retainedNotice ? (
            <RetainedStateNotice
              notice={decisionModel.surface.retainedNotice}
              onRecover={recover}
            />
          ) : null}
          <div className="activity-experience__layout">
            <Queue
              selectedId={selected.id}
              onSelect={setSelectedId}
              items={activityWorkQueueFixture}
              stale={experienceState === "stale"}
            />
            <ActivityDetail
              item={selected}
              model={decisionModel}
              onLocalReviewStateChange={updateLocalReviewState}
            />
          </div>
        </>
      )}
    </section>
  );
}

function ExperienceHeader({
  state,
  onStateChange,
  controlRef
}: {
  state: ActivityExperienceState;
  onStateChange: (state: ActivityExperienceState) => void;
  controlRef: RefObject<HTMLSelectElement | null>;
}) {
  return (
    <header className="activity-experience__header">
      <div className="activity-experience__intro">
        <p className="activity-experience__eyebrow">Work queue</p>
        <h1 id="activities-heading">Activities that need attention</h1>
        <p className="activity-experience__lede">
          Keep relationship context, evidence, dependency, consequence, and local acknowledgment in one inspectable path.
        </p>
        <p className="activity-experience__posture" aria-label="Local preview posture">
          <ShieldCheck size={17} aria-hidden="true" />
          <strong>Local preview only</strong>
          <span>No provider action was sent</span>
        </p>
      </div>
      <label className="activity-experience__state-control">
        <span>Fixture state</span>
        <select
          ref={controlRef}
          value={state}
          aria-label="Activity experience fixture state"
          onChange={(event) => onStateChange(event.target.value as ActivityExperienceState)}
        >
          {Object.entries(activityExperienceStateLabels).map(([value, label]) => (
            <option value={value} key={value}>{label}</option>
          ))}
        </select>
      </label>
    </header>
  );
}

function Queue({
  items,
  selectedId,
  onSelect,
  stale
}: {
  items: ActivityQueueFixtureItem[];
  selectedId: string;
  onSelect: (id: string) => void;
  stale: boolean;
}) {
  return (
    <aside className="activity-queue" aria-label="Activity work queue">
      <div className="activity-queue__header">
        <div>
          <p className="activity-queue__label">Today</p>
          <strong>Priority work</strong>
        </div>
        <span>Ordered by due time</span>
      </div>
      <ul className="activity-queue__list" aria-label="Select an activity">
        {items.map((item) => {
          const selected = item.id === selectedId;
          return (
            <li className="activity-queue__entry" key={item.id}>
              <button
                type="button"
                className={`activity-queue-item ${selected ? "is-selected" : ""}`}
                aria-current={selected ? "true" : undefined}
                onClick={() => onSelect(item.id)}
              >
                <span
                  className={`activity-priority activity-priority--${item.priority.toLowerCase()}`}
                  aria-hidden="true"
                />
                <span className="activity-queue-item__body">
                  <span className="activity-queue-item__topline">
                    <strong>{item.subject}</strong>
                    <span>{stale ? "Stale" : item.freshness}</span>
                  </span>
                  <span>{item.account}</span>
                  <span className="activity-queue-item__meta">
                    {item.priority} priority · {item.owner} · {item.dueLabel}
                  </span>
                </span>
                <ChevronRight aria-hidden="true" size={18} />
              </button>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}

function ActivityDetail({
  item,
  model,
  onLocalReviewStateChange
}: {
  item: ActivityQueueFixtureItem;
  model: ActivityDecisionSpineViewModel;
  onLocalReviewStateChange: (state: ActivityLocalReviewState) => void;
}) {
  const headingId = useId();

  return (
    <article className="activity-detail" aria-labelledby={headingId}>
      <div className="activity-detail__topline">
        <div>
          <p className="activity-experience__eyebrow">{item.account}</p>
          <h2 id={headingId}>{item.subject}</h2>
        </div>
        <span className={`activity-status activity-status--${item.priority.toLowerCase()}`}>
          {item.priority} priority
        </span>
      </div>
      <div className="activity-detail__summary">
        <p>{item.summary}</p>
        <div className="activity-detail__facts">
          <Fact icon={<Clock3 size={16} />} label="Due" value={item.dueLabel} />
          <Fact icon={<ShieldCheck size={16} />} label="Owner" value={item.owner} />
          <Fact icon={<RefreshCw size={16} />} label="Freshness" value={model.effectiveFreshness} />
        </div>
      </div>
      <ActivityDecisionSpine
        model={model}
        onLocalReviewStateChange={onLocalReviewStateChange}
      />
      <p className="activity-detail__correlation">
        Fixture-backed local prototype · Correlation: <code>{item.correlationId}</code> · No provider action was sent
      </p>
    </article>
  );
}

function ExperienceStatePanel({
  surface,
  onRecover
}: {
  surface: ActivityExperienceSurface;
  onRecover: () => void;
}) {
  if (!surface.feedbackKind || !surface.title || !surface.detail) {
    throw new Error("Adverse activity surface is missing feedback content");
  }

  return (
    <FeedbackState
      className="activity-state-panel"
      kind={surface.feedbackKind}
      title={<h2>{surface.title}</h2>}
      detail={surface.detail}
      metadata={surface.correlationId ? { correlationId: surface.correlationId } : undefined}
      action={surface.recoverable ? (
        <Button variant="primary" onClick={onRecover}>Return to ready state</Button>
      ) : undefined}
    >
      {surface.feedbackKind === "loading" ? (
        <span className="activity-state-panel__loading-cue" aria-hidden="true">
          <RefreshCw className="is-spinning" size={20} />
        </span>
      ) : null}
    </FeedbackState>
  );
}

function RetainedStateNotice({
  notice,
  onRecover
}: {
  notice: NonNullable<ActivityExperienceSurface["retainedNotice"]>;
  onRecover: () => void;
}) {
  return (
    <aside
      className="activity-retained-notice"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      <Info size={19} aria-hidden="true" />
      <div>
        <strong>{notice.title}</strong>
        <p>{notice.detail}</p>
      </div>
      <Button variant="secondary" size="sm" onClick={onRecover}>Return to ready state</Button>
    </aside>
  );
}

function Fact({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="activity-fact">
      <span aria-hidden="true">{icon}</span>
      <div><span>{label}</span><strong>{value}</strong></div>
    </div>
  );
}
