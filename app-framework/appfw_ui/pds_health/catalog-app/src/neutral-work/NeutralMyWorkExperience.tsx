import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode
} from "react";
import {
  Alert,
  AppShell,
  Badge,
  Button,
  CommandBar,
  EmptyState,
  PageHeader,
  SelectField,
  TextField
} from "@appfw/pds-health-components";
import {
  isKnownNeutralWorkToken,
  type NeutralWorkItem,
  type NeutralWorkOperationalState
} from "./contract";
import { allKnownNeutralWorkItems, fixtureForState } from "./fixtures";

export type NeutralMyWorkExperienceProps = {
  initialState: NeutralWorkOperationalState;
  deepLinkToken: string | null;
  invalidInputs?: readonly string[];
};

const statusOptions = [
  { value: "all", label: "All statuses" },
  { value: "ready", label: "Ready" },
  { value: "active", label: "In progress" },
  { value: "done", label: "Completed" }
] as const;

function badgeTone(value: string): "neutral" | "accent" | "success" | "warning" | "danger" {
  if (value === "ready" || value === "observed") return "success";
  if (value === "active" || value === "pending") return "accent";
  if (value === "needed") return "warning";
  return "neutral";
}

function StateNotice({
  state,
  message,
  retryPosture,
  recoveryAllowed,
  onRecover
}: {
  state: NeutralWorkOperationalState;
  message: string;
  retryPosture: string;
  recoveryAllowed: boolean;
  onRecover: () => void;
}) {
  if (state === "populated") return null;
  if (state === "loading") {
    return (
      <div className="neutral-work__loading" role="status" aria-busy="true">
        <strong>Loading work context</strong>
        <span>{message}</span>
      </div>
    );
  }

  const tone = state === "error" || state === "forbidden" || state === "unauthorized"
    ? "danger"
    : state === "partial" || state === "stale" || state === "offline" || state === "conflict" || state === "timeout"
      ? "warning"
      : state === "success" || state === "recovery"
        ? "success"
        : "neutral";

  return (
    <Alert
      className="neutral-work__state-notice"
      data-state-notice={state}
      tone={tone}
      title={state === "recovery" ? "Context restored" : `${state[0].toUpperCase()}${state.slice(1)} state`}
      detail={message}
    >
      {state !== "empty" && state !== "success" && state !== "recovery" ? (
        <div className="neutral-work__recovery-posture">
          <span>{retryPosture}</span>
          {recoveryAllowed ? (
            <Button size="sm" variant="quiet" onClick={onRecover}>Open local recovery</Button>
          ) : null}
        </div>
      ) : null}
    </Alert>
  );
}

function WorkPosture({ item }: { item: NeutralWorkItem }) {
  return (
    <dl className="neutral-work__posture" aria-label="Work posture">
      <div><dt>Status</dt><dd><Badge tone={badgeTone(item.statusValue)}>{item.statusLabel}</Badge></dd></div>
      <div><dt>Approval</dt><dd><Badge tone={badgeTone(item.approval)}>{item.approval}</Badge></dd></div>
      <div><dt>Attention</dt><dd><Badge tone={badgeTone(item.attention)}>{item.attention}</Badge></dd></div>
      <div><dt>Freshness</dt><dd>{item.freshnessLabel}</dd></div>
    </dl>
  );
}

export function NeutralMyWorkExperience({
  initialState,
  deepLinkToken,
  invalidInputs = []
}: NeutralMyWorkExperienceProps) {
  const initialFixture = fixtureForState(initialState);
  const initialDeepLink = deepLinkToken
    ? allKnownNeutralWorkItems().find((item) => item.token === deepLinkToken)?.id ?? null
    : null;
  const [activeState, setActiveState] = useState(initialState);
  const [query, setQuery] = useState(initialFixture.query);
  const [statusFilter, setStatusFilter] = useState(initialFixture.statusFilter);
  const [selectedItemId, setSelectedItemId] = useState(
    invalidInputs.includes("item")
      ? null
      : initialDeepLink ?? (initialState === "recovery" ? initialFixture.selectedItemId : null)
  );
  const [notificationRead, setNotificationRead] = useState<Record<string, boolean>>({});
  const [focusTarget, setFocusTarget] = useState<string | null>(initialFixture.focusTarget);
  const detailHeadingRef = useRef<HTMLHeadingElement>(null);

  const fixture = fixtureForState(activeState);
  const blocked = fixture.permissionResult === "restricted";
  const visibleItems = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return fixture.items.filter((item) => {
      const queryMatches = !needle || item.searchText.includes(needle);
      const statusMatches = statusFilter === "all" || item.statusValue === statusFilter;
      return queryMatches && statusMatches;
    });
  }, [fixture.items, query, statusFilter]);
  const selectedItem = blocked
    ? null
    : fixture.items.find((item) => item.id === selectedItemId) ?? null;

  const restoreFocus = useCallback((target: string | null) => {
    if (!target) return;
    window.requestAnimationFrame(() => {
      document.getElementById(target)?.focus();
    });
  }, []);

  const selectItem = useCallback((item: NeutralWorkItem, initiatorId: string, addHistory = true) => {
    setSelectedItemId(item.id);
    setFocusTarget(initiatorId);
    if (addHistory) {
      const queue = new URL(window.location.href);
      queue.searchParams.delete("item");
      window.history.replaceState(
        { itemToken: null, focusTarget: initiatorId, queueEntry: true },
        "",
        queue
      );
      const next = new URL(queue);
      next.searchParams.set("item", item.token);
      window.history.pushState(
        { itemToken: item.token, focusTarget: initiatorId, detailEntry: true },
        "",
        next
      );
    }
    window.requestAnimationFrame(() => detailHeadingRef.current?.focus());
  }, []);

  const leaveDetail = useCallback(() => {
    const target = focusTarget;
    if (window.history.state?.detailEntry === true) {
      window.history.back();
      return;
    }
    const queue = new URL(window.location.href);
    queue.searchParams.delete("item");
    window.history.replaceState(
      { itemToken: null, focusTarget: target, queueEntry: true },
      "",
      queue
    );
    setSelectedItemId(null);
    restoreFocus(target);
  }, [focusTarget, restoreFocus]);

  const applyFixtureState = useCallback((state: NeutralWorkOperationalState) => {
    const next = fixtureForState(state);
    setActiveState(state);
    setQuery(next.query);
    setStatusFilter(next.statusFilter);
    setSelectedItemId(next.selectedItemId);
    setFocusTarget(next.focusTarget);
    restoreFocus(next.focusTarget);
  }, [restoreFocus]);

  const openLocalRecovery = useCallback(() => {
    if (fixture.permissionResult === "restricted") return;
    applyFixtureState("recovery");
  }, [applyFixtureState, fixture.permissionResult]);

  useEffect(() => {
    const onPopState = (event: PopStateEvent) => {
      const token = typeof event.state?.itemToken === "string" ? event.state.itemToken : null;
      const target = typeof event.state?.focusTarget === "string" ? event.state.focusTarget : focusTarget;
      if (isKnownNeutralWorkToken(token, fixture.items)) {
        const item = fixture.items.find((candidate) => candidate.token === token);
        if (item) selectItem(item, target ?? `work-row-${item.id}`, false);
      } else {
        setSelectedItemId(null);
        restoreFocus(target);
      }
    };
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, [fixture.items, focusTarget, restoreFocus, selectItem]);

  useEffect(() => {
    if (selectedItemId) detailHeadingRef.current?.focus();
  }, [selectedItemId]);

  useEffect(() => {
    if (activeState === "recovery") restoreFocus(fixture.focusTarget);
  }, [activeState, fixture.focusTarget, restoreFocus]);

  const notifications = blocked ? [] : fixture.notifications;
  const unreadCount = notifications.filter((notice) => !(notificationRead[notice.id] ?? notice.read)).length;
  let queueContent: ReactNode;

  if (activeState === "loading") {
    queueContent = (
      <div className="neutral-work__queue-skeleton" aria-hidden="true">
        <span /><span /><span />
      </div>
    );
  } else if (blocked) {
    queueContent = (
      <EmptyState
        tone="danger"
        title="Work content hidden"
        detail="No queue rows or detail fields are available in this fixture state."
      />
    );
  } else if (visibleItems.length === 0) {
    queueContent = (
      <EmptyState
        title="No work in this view"
        detail={fixture.message}
        action={<Button size="sm" onClick={() => applyFixtureState("populated")}>Reset local view</Button>}
      />
    );
  } else {
    queueContent = (
      <ul className="neutral-work__queue-list">
        {visibleItems.map((item) => {
          const rowId = `work-row-${item.id}`;
          return (
            <li key={item.id}>
              <Button
                id={rowId}
                className="neutral-work__row"
                variant="quiet"
                aria-current={selectedItemId === item.id ? "true" : undefined}
                onClick={() => selectItem(item, rowId)}
              >
                <span className="neutral-work__row-copy">
                  <strong>{item.title}</strong>
                  <span>{item.summary}</span>
                  <span className="neutral-work__row-meta">{item.statusLabel} · {item.freshnessLabel}</span>
                </span>
                <Badge tone={badgeTone(item.attention)}>{item.attention === "none" ? "steady" : item.attention}</Badge>
              </Button>
            </li>
          );
        })}
      </ul>
    );
  }

  return (
    <AppShell
      className="neutral-work"
      data-n0-state={activeState}
      data-permission-result={fixture.permissionResult}
      brand={<strong>PDS</strong>}
      navigation={<><span aria-current="page">My Work</span><span>Notifications</span></>}
      topBar={<><Badge tone="accent">Fixture-backed</Badge><span>{fixture.sourceLabel}</span></>}
      footer={<span>Local evidence surface</span>}
    >
      <PageHeader
        eyebrow="Product-neutral composition"
        title="My Work"
        subtitle={`${fixture.sourceLabel} · ${activeState === "stale" ? "Last known at 16:00 UTC" : "Deterministic as of 16:30 UTC"}`}
      />

      {invalidInputs.length ? (
        <Alert
          tone="warning"
          title="Unknown local inputs ignored"
          detail={`Deterministic defaults were used for: ${invalidInputs.join(", ")}.`}
        />
      ) : null}

      <StateNotice
        state={activeState}
        message={fixture.message}
        retryPosture={fixture.retryPosture}
        recoveryAllowed={!blocked}
        onRecover={openLocalRecovery}
      />

      <CommandBar
        className="neutral-work__command-bar"
        filters={
          <>
            <TextField
              label="Search work"
              name="neutral-work-search"
              type="search"
              value={query}
              disabled={blocked || activeState === "loading"}
              onChange={(event) => setQuery(event.currentTarget.value.toLowerCase())}
            />
            <SelectField
              label="Status"
              name="neutral-work-status"
              value={statusFilter}
              options={statusOptions}
              disabled={blocked || activeState === "loading"}
              onChange={(event) => setStatusFilter(event.currentTarget.value)}
            />
          </>
        }
        resultSummary={`${visibleItems.length} work item${visibleItems.length === 1 ? "" : "s"}`}
      />

      <div className="neutral-work__workspace">
        <section className="neutral-work__queue" aria-labelledby="neutral-work-queue">
          <header className="neutral-work__section-head">
            <div><p>Queue</p><h2 id="neutral-work-queue">Current work</h2></div>
            <Badge tone={unreadCount ? "warning" : "neutral"}>{unreadCount} unread</Badge>
          </header>
          {queueContent}
        </section>

        <section className="neutral-work__detail" aria-labelledby="neutral-work-detail">
          {selectedItem ? (
            <>
              <header className="neutral-work__section-head">
                <div>
                  <p>Selected detail</p>
                  <h2 id="neutral-work-detail" ref={detailHeadingRef} tabIndex={-1}>{selectedItem.title}</h2>
                </div>
                <Button
                  size="sm"
                  variant="quiet"
                  onClick={leaveDetail}
                >
                  Back to queue
                </Button>
              </header>
              <p className="neutral-work__detail-summary">{selectedItem.summary}</p>
              <WorkPosture item={selectedItem} />
              <div className="neutral-work__evidence">
                <h3>Evidence summary</h3>
                <p>{selectedItem.evidenceSummary}</p>
                <time dateTime={selectedItem.timestamp}>{selectedItem.timestamp}</time>
              </div>
              {activeState === "conflict" ? (
                <Alert tone="warning" title="Compare retained context" detail={fixture.retryPosture} />
              ) : null}
            </>
          ) : (
            <>
              <header className="neutral-work__section-head">
                <div><p>Selected detail</p><h2 id="neutral-work-detail">Work detail</h2></div>
              </header>
              <EmptyState
                title="No detail selected"
                detail={blocked ? "Detail fields remain hidden." : "Choose a known work item to inspect its local fixture posture."}
              />
            </>
          )}
        </section>
      </div>

      <section className="neutral-work__notifications" aria-labelledby="neutral-work-notifications">
        <header className="neutral-work__section-head">
          <div><p>Notification center</p><h2 id="neutral-work-notifications">Local signals</h2></div>
          <Badge tone={unreadCount ? "warning" : "neutral"}>{unreadCount} unread</Badge>
        </header>
        {notifications.length ? (
          <ul>
            {notifications.map((notice) => {
              const item = fixture.items.find((candidate) => candidate.id === notice.itemId);
              const controlId = `notification-${notice.id}`;
              const read = notificationRead[notice.id] ?? notice.read;
              return item ? (
                <li key={notice.id}>
                  <Button
                    id={controlId}
                    variant="quiet"
                    onClick={() => {
                      setNotificationRead((current) => ({ ...current, [notice.id]: true }));
                      selectItem(item, controlId);
                    }}
                  >
                    {notice.label} <Badge tone={read ? "neutral" : "warning"}>{read ? "read" : "unread"}</Badge>
                  </Button>
                </li>
              ) : null;
            })}
          </ul>
        ) : <p>No notification content is available in this fixture state.</p>}
      </section>
    </AppShell>
  );
}
