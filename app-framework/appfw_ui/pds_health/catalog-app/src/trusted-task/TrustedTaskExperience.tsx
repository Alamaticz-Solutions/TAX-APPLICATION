import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  AppShell,
  Badge,
  Button,
  EmptyState,
  PageHeader
} from "@appfw/pds-health-components";
import {
  receiptResolvesWork,
  type TrustedTaskState
} from "./contract";
import { fixtureForTrustedTaskState } from "./fixtures";

export type TrustedTaskExperienceProps = {
  initialState: TrustedTaskState;
  deepLinkWorkToken: string | null;
  resumeToken: string | null;
  invalidInputs?: readonly string[];
};

function toneForState(state: TrustedTaskState): "neutral" | "accent" | "success" | "warning" | "danger" {
  if (state === "success-receipt" || state === "recovery") return "success";
  if (state === "denied" || state === "failed" || state === "error" || state === "unauthorized") return "danger";
  if (state === "stale" || state === "partial" || state === "offline") return "warning";
  if (state === "pending" || state === "permitted-preview") return "accent";
  return "neutral";
}

export function TrustedTaskExperience({
  initialState,
  deepLinkWorkToken,
  resumeToken,
  invalidInputs = []
}: TrustedTaskExperienceProps) {
  const [activeState, setActiveState] = useState(initialState);
  const [receiptVisible, setReceiptVisible] = useState(initialState === "success-receipt" || initialState === "failed" || initialState === "recovery");
  const overviewRef = useRef<HTMLHeadingElement>(null);
  const receiptRef = useRef<HTMLHeadingElement>(null);
  const fixture = useMemo(() => fixtureForTrustedTaskState(activeState), [activeState]);
  const restricted = activeState === "unauthorized" || activeState === "empty" || activeState === "loading" || activeState === "error";
  const exactDeepLink = Boolean(fixture.work && deepLinkWorkToken === fixture.work.workToken);
  const exactResume = Boolean(fixture.work && resumeToken === fixture.work.resumeToken);
  const receiptResolved = Boolean(fixture.receipt && fixture.work && receiptResolvesWork(fixture.receipt, fixture.work));
  const canEnterPermittedPreview = fixture.preview?.permission.result === "permitted";
  const canRestoreReceipt = fixture.preview?.permission.result === "permitted";

  const focusSoon = useCallback((target: "overview" | "receipt") => {
    window.requestAnimationFrame(() => {
      (target === "receipt" ? receiptRef.current : overviewRef.current)?.focus();
    });
  }, []);

  const enterPermittedPreview = useCallback(() => {
    const permittedFixture = fixtureForTrustedTaskState("permitted-preview");
    if (!canEnterPermittedPreview || permittedFixture.preview?.permission.result !== "permitted") return;
    const next = new URL(window.location.href);
    if (permittedFixture.work) next.searchParams.set("work", permittedFixture.work.workToken);
    next.searchParams.set("state", "permitted-preview");
    window.history.pushState({ state: "permitted-preview", focus: "overview" }, "", next);
    setActiveState("permitted-preview");
    setReceiptVisible(false);
    focusSoon("overview");
  }, [canEnterPermittedPreview, focusSoon]);

  const restoreReceipt = useCallback(() => {
    if (!canRestoreReceipt) return;
    const recoveryState = activeState === "failed" ? "failed" : "recovery";
    const recovery = fixtureForTrustedTaskState(recoveryState);
    const next = new URL(window.location.href);
    next.searchParams.set("state", recoveryState);
    if (recovery.work) {
      next.searchParams.set("work", recovery.work.workToken);
      next.searchParams.set("resume", recovery.work.resumeToken);
    }
    window.history.pushState({ state: recoveryState, focus: "receipt" }, "", next);
    setActiveState(recoveryState);
    setReceiptVisible(true);
    focusSoon("receipt");
  }, [activeState, canRestoreReceipt, focusSoon]);

  useEffect(() => {
    const onPopState = (event: PopStateEvent) => {
      const historyState = event.state?.state;
      const urlState = new URL(window.location.href).searchParams.get("state");
      const state = historyState === "permitted-preview" || historyState === "recovery" || historyState === "failed"
        ? historyState
        : urlState;
      if (state === "permitted-preview" || state === "recovery" || state === "failed") {
        setActiveState(state);
        setReceiptVisible(state === "recovery" || state === "failed");
        focusSoon(state === "recovery" || state === "failed" ? "receipt" : "overview");
      }
    };
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, [focusSoon]);

  useEffect(() => {
    if (exactResume && fixture.receipt) {
      setReceiptVisible(true);
      focusSoon("receipt");
    } else if (exactDeepLink) {
      focusSoon("overview");
    }
  }, [exactDeepLink, exactResume, fixture.receipt, focusSoon]);

  useEffect(() => {
    if (receiptVisible && fixture.receipt) focusSoon("receipt");
  }, [fixture.receipt, focusSoon, receiptVisible]);

  const notice = activeState === "loading" ? (
    <div className="trusted-task__loading" role="status" aria-busy="true">
      <strong>Loading trusted-task context</strong>
      <span>{fixture.message}</span>
    </div>
  ) : activeState === "permitted-preview" ? null : (
    <Alert
      className="trusted-task__notice"
      data-state-notice={activeState}
      tone={toneForState(activeState)}
      title={activeState === "recovery" ? "Exact context restored" : activeState.replace(/-/g, " ")}
      detail={fixture.message}
    >
      {activeState === "failed" && canRestoreReceipt ? (
        <Button size="sm" variant="quiet" onClick={restoreReceipt}>Restore exact context</Button>
      ) : canEnterPermittedPreview && activeState !== "success-receipt" ? (
        <Button size="sm" variant="quiet" onClick={enterPermittedPreview}>Return to permitted preview</Button>
      ) : null}
    </Alert>
  );

  return (
    <AppShell
      className="trusted-task"
      data-n1-state={activeState}
      brand={<strong>PDS</strong>}
      navigation={<><span>My Work</span><span aria-current="page">Trusted task</span></>}
      topBar={<><Badge tone="accent">Fixture-backed</Badge><span>{fixture.sourceLabel}</span></>}
      footer={<span>Local snapshot preparation · no action execution</span>}
    >
      <PageHeader
        eyebrow="N1 Trusted Task"
        title="Review before the next step"
        subtitle={`${fixture.fixtureVersion} · ${fixture.snapshotIdentity}`}
      />

      {invalidInputs.length ? (
        <Alert tone="warning" title="Unknown local inputs ignored" detail={`Deterministic defaults were used for: ${invalidInputs.join(", ")}.`} />
      ) : null}
      {exactDeepLink || exactResume ? (
        <Alert
          tone="accent"
          title="Opened from trusted local context"
          detail={exactResume
            ? "The exact fixture receipt and resume point are in view."
            : "A local notification or deep link resolved to this exact neutral work token."}
        />
      ) : null}
      {notice}

      {restricted ? (
        <EmptyState
          tone={activeState === "unauthorized" || activeState === "error" ? "danger" : "neutral"}
          title={activeState === "unauthorized" ? "Trusted-task content hidden" : "No trusted-task detail available"}
          detail={fixture.recoveryGuidance}
        />
      ) : fixture.work ? (
        <>
          <section className="trusted-task__overview" aria-labelledby="trusted-task-overview">
            <header className="trusted-task__section-head">
              <div>
                <p>Trusted work item</p>
                <h2 id="trusted-task-overview" ref={overviewRef} tabIndex={-1}>{fixture.work.title}</h2>
              </div>
              <Badge tone={toneForState(activeState)}>{fixture.work.statusLabel}</Badge>
            </header>
            <p className="trusted-task__summary">{fixture.work.summary}</p>
            <dl className="trusted-task__facts">
              <div><dt>Owner</dt><dd>{fixture.work.ownerLabel}</dd></div>
              <div><dt>Freshness</dt><dd>{fixture.work.freshnessLabel}</dd></div>
              <div><dt>Reason for attention</dt><dd>{fixture.work.reasonForAttention}</dd></div>
              <div><dt>Consequence</dt><dd>{fixture.work.consequence}</dd></div>
            </dl>
          </section>

          <div className="trusted-task__workspace">
            <section className="trusted-task__evidence" aria-labelledby="trusted-task-evidence">
              <header className="trusted-task__section-head">
                <div><p>Evidence and timeline</p><h2 id="trusted-task-evidence">What supports this task</h2></div>
              </header>
              <p><strong>{fixture.work.evidenceSource}</strong></p>
              <p>{fixture.work.evidenceSummary}</p>
              <time dateTime={fixture.work.evidenceObservedAt}>{fixture.work.evidenceObservedAt}</time>
              <ol>
                {fixture.work.timeline.map((entry) => <li key={entry.at}><time dateTime={entry.at}>{entry.at}</time><span>{entry.label}</span></li>)}
              </ol>
            </section>

            <section className="trusted-task__preview" aria-labelledby="trusted-task-preview">
              <header className="trusted-task__section-head">
                <div><p>Safest permitted next step</p><h2 id="trusted-task-preview">Governed preview</h2></div>
                <Badge tone={fixture.preview?.permission.result === "permitted" ? "success" : "danger"}>
                  {fixture.preview?.permission.result ?? "unavailable"}
                </Badge>
              </header>
              {fixture.preview ? (
                <>
                  <h3>{fixture.preview.nextStepLabel}</h3>
                  <p>{fixture.preview.consequence}</p>
                  <Alert
                    tone={fixture.preview.permission.result === "permitted" ? "accent" : "danger"}
                    title="Preview only"
                    detail={fixture.preview.disclosure}
                  />
                  <dl className="trusted-task__preview-meta">
                    <div><dt>Permission result</dt><dd>{fixture.preview.permission.result}</dd></div>
                    <div><dt>Reason</dt><dd>{fixture.preview.permission.reason}</dd></div>
                    <div><dt>Policy reference</dt><dd>{fixture.preview.permission.policyReference}</dd></div>
                    <div><dt>Operation state</dt><dd>{activeState === "pending" ? "simulated pending" : "not started"}</dd></div>
                  </dl>
                </>
              ) : <p>Preview remains closed because its fixture permission result is unavailable.</p>}
            </section>
          </div>

          <section className="trusted-task__receipt" aria-labelledby="trusted-task-receipt">
            <header className="trusted-task__section-head">
              <div><p>Receipt and recovery</p><h2 id="trusted-task-receipt" ref={receiptRef} tabIndex={-1}>Exact resume point</h2></div>
              <Badge tone={receiptResolved ? "success" : "neutral"}>{receiptResolved ? "resolved" : "not recorded"}</Badge>
            </header>
            {receiptVisible && fixture.receipt ? (
              <dl className="trusted-task__receipt-grid">
                <div><dt>Receipt</dt><dd>{fixture.receipt.receiptId}</dd></div>
                <div><dt>Correlation</dt><dd>{fixture.receipt.correlationId}</dd></div>
                <div><dt>Operation identity</dt><dd>{fixture.receipt.operationId}</dd></div>
                <div><dt>Operation</dt><dd>{fixture.receipt.operationState}</dd></div>
                <div><dt>Audit</dt><dd>{fixture.receipt.auditPosture}</dd></div>
                <div><dt>Undo / compensation</dt><dd>{fixture.receipt.undoPosture}</dd></div>
                <div><dt>Resume token</dt><dd>{fixture.receipt.resumeToken}</dd></div>
              </dl>
            ) : (
              <div className="trusted-task__receipt-empty">
                <p>No simulated receipt is visible in this fixture posture.</p>
                {canRestoreReceipt ? (
                  <Button className="trusted-task__recovery-action" size="sm" variant="quiet" onClick={restoreReceipt}>Restore exact context</Button>
                ) : (
                  <p>Receipt recovery remains unavailable until the fixture reports a permitted result.</p>
                )}
              </div>
            )}
          </section>
        </>
      ) : null}
    </AppShell>
  );
}
