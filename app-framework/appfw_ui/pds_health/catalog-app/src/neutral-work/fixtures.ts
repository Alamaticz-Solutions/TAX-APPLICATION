import type {
  NeutralWorkFixture,
  NeutralWorkItem,
  NeutralWorkOperationalState
} from "./contract";

export const FIXTURE_SOURCE_LABEL = "Deterministic local simulation";

const ITEMS = [
  {
    id: "work-alpha",
    token: "alpha-review",
    title: "Review intake summary",
    summary: "Confirm the bounded context and evidence posture.",
    searchText: "review intake summary bounded context evidence",
    statusLabel: "Ready",
    statusValue: "ready",
    attention: "needed",
    approval: "pending",
    freshnessLabel: "Current",
    timestamp: "2026-07-17T16:30:00.000Z",
    evidenceSummary: "Three fixture references are available for inspection."
  },
  {
    id: "work-bravo",
    token: "bravo-follow-up",
    title: "Prepare follow-up notes",
    summary: "Collect the neutral observations for the next checkpoint.",
    searchText: "prepare follow up notes neutral observations checkpoint",
    statusLabel: "In progress",
    statusValue: "active",
    attention: "watch",
    approval: "not-needed",
    freshnessLabel: "Updated 12 minutes ago",
    timestamp: "2026-07-17T16:18:00.000Z",
    evidenceSummary: "Two fixture observations are retained locally."
  },
  {
    id: "work-charlie",
    token: "charlie-closed",
    title: "Archive completed context",
    summary: "Keep the closed item discoverable without exposing actions.",
    searchText: "archive completed context closed discoverable",
    statusLabel: "Completed",
    statusValue: "done",
    attention: "none",
    approval: "observed",
    freshnessLabel: "Completed today",
    timestamp: "2026-07-17T15:45:00.000Z",
    evidenceSummary: "The fixture completion summary is retained."
  }
] as const satisfies readonly NeutralWorkItem[];

const NOTIFICATIONS = [
  { id: "notice-alpha", itemId: "work-alpha", label: "Review context needs attention", read: false },
  { id: "notice-bravo", itemId: "work-bravo", label: "Follow-up context changed", read: true }
] as const;

function fixture(
  state: NeutralWorkOperationalState,
  values: Partial<Omit<NeutralWorkFixture, "state" | "sourceLabel">> = {}
): NeutralWorkFixture {
  return {
    state,
    sourceLabel: FIXTURE_SOURCE_LABEL,
    permissionResult: "permitted",
    message: "Local fixture state is ready.",
    retryPosture: "Switch to a local recovery fixture.",
    recoveryContext: "No prior local context is required.",
    query: "",
    statusFilter: "all",
    selectedItemId: ITEMS[0].id,
    focusTarget: null,
    items: ITEMS,
    notifications: NOTIFICATIONS,
    ...values
  };
}

export const NEUTRAL_WORK_FIXTURES = {
  loading: fixture("loading", {
    message: "Loading deterministic work context.",
    selectedItemId: null,
    items: [],
    notifications: []
  }),
  populated: fixture("populated"),
  empty: fixture("empty", {
    message: "No work matches the current local view.",
    selectedItemId: null,
    items: [],
    notifications: []
  }),
  error: fixture("error", {
    message: "The simulated source could not provide this view.",
    retryPosture: "Open the local recovery scenario. Reference SIM-ERR-1042.",
    selectedItemId: null,
    items: [],
    notifications: []
  }),
  partial: fixture("partial", {
    message: "Available work is shown; one simulated region is unavailable.",
    items: ITEMS.slice(0, 2)
  }),
  stale: fixture("stale", {
    message: "This is last-known fixture content from 2026-07-17 16:00 UTC.",
    retryPosture: "Freshness can be explored with the local recovery scenario."
  }),
  offline: fixture("offline", {
    message: "Offline simulation: retained local context remains visible.",
    retryPosture: "No network retry is available in this simulation.",
    items: ITEMS.slice(0, 2)
  }),
  unauthorized: fixture("unauthorized", {
    message: "Session context is unavailable. Work content is not displayed.",
    permissionResult: "restricted",
    retryPosture: "No local recovery is available without a new permission result.",
    selectedItemId: null,
    items: [],
    notifications: []
  }),
  forbidden: fixture("forbidden", {
    message: "This view is restricted. Work fields remain hidden.",
    permissionResult: "restricted",
    retryPosture: "No local recovery is available without a new permission result.",
    selectedItemId: null,
    items: [],
    notifications: []
  }),
  conflict: fixture("conflict", {
    message: "The selected fixture changed in another simulated view.",
    retryPosture: "Compare the retained summary, then open the local recovery scenario."
  }),
  timeout: fixture("timeout", {
    message: "The simulated view timed out; query and selection are retained.",
    retryPosture: "Open the local recovery scenario without contacting a source.",
    query: "review",
    statusFilter: "ready"
  }),
  success: fixture("success", {
    message: "Local navigation completed and the selected detail is in view."
  }),
  recovery: fixture("recovery", {
    message: "Prior local orientation has been restored.",
    recoveryContext: "Query review, Ready filter, selected detail, and row focus restored.",
    query: "review",
    statusFilter: "ready",
    selectedItemId: "work-alpha",
    focusTarget: "work-row-work-alpha"
  })
} as const satisfies Record<NeutralWorkOperationalState, NeutralWorkFixture>;

export function fixtureForState(state: NeutralWorkOperationalState): NeutralWorkFixture {
  return NEUTRAL_WORK_FIXTURES[state];
}

export function allKnownNeutralWorkItems(): readonly NeutralWorkItem[] {
  return ITEMS;
}
