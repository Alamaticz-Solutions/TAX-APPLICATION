export const activityWorkQueueContract = "activity-work-queue.v2" as const;

export type ActivityDecisionSignalState = "ready" | "needs-attention" | "blocked" | "unavailable";

export type ActivityQueueItem = {
  id: string;
  subject: string;
  account: string;
  owner: string;
  dueLabel: string;
  priority: "High" | "Normal" | "Low";
  status: "Open" | "In progress" | "Completed";
  freshness: "Current" | "Refreshing" | "Stale";
  summary: string;
  correlationId: string;
  decisionPath: {
    relationship: {
      detail: string;
      state: ActivityDecisionSignalState;
    };
    evidence: {
      detail: string;
      state: ActivityDecisionSignalState;
    };
    dependency: {
      detail: string;
      state: ActivityDecisionSignalState;
    };
    consequence: {
      title: string;
      detail: string;
    };
  };
};

export type ActivityQueueFixtureItem = ActivityQueueItem;

export const activityWorkQueueFixture: ActivityQueueFixtureItem[] = [
  {
    id: "activity-acme-discovery",
    subject: "Discovery call follow-up",
    account: "Acme Analytics",
    owner: "Maya Chen",
    dueLabel: "Due today, 2:30 PM",
    priority: "High",
    status: "Open",
    freshness: "Current",
    summary: "Confirm stakeholder priorities before the proposal outline is shared.",
    decisionPath: {
      relationship: {
        detail: "3 active contacts and 1 account plan",
        state: "ready"
      },
      evidence: {
        detail: "Discovery notes captured 18 minutes ago",
        state: "ready"
      },
      dependency: {
        detail: "Proposal outline waiting for stakeholder confirmation",
        state: "needs-attention"
      },
      consequence: {
        title: "Proposal context stays in review",
        detail: "Recording this review preserves the checkpoint locally; it does not send a proposal or change account data."
      }
    },
    correlationId: "crm-demo-activity-001",
  },
  {
    id: "activity-northstar-demo",
    subject: "Prepare operating review",
    account: "Northstar Manufacturing",
    owner: "Riley Patel",
    dueLabel: "Tomorrow, 9:00 AM",
    priority: "Normal",
    status: "In progress",
    freshness: "Refreshing",
    summary: "Align the operating review narrative with the latest account health snapshot.",
    decisionPath: {
      relationship: {
        detail: "2 account owners and 1 open opportunity",
        state: "ready"
      },
      evidence: {
        detail: "Last-known health score is 72 while the deterministic refresh runs",
        state: "needs-attention"
      },
      dependency: {
        detail: "Operating review is scheduled for tomorrow",
        state: "ready"
      },
      consequence: {
        title: "Operating-review context stays local",
        detail: "Recording this review preserves a local checkpoint; it does not update the account health snapshot."
      }
    },
    correlationId: "crm-demo-activity-002",
  },
  {
    id: "activity-medical-renewal",
    subject: "Review renewal dependencies",
    account: "MediSystems",
    owner: "Alex Morgan",
    dueLabel: "Friday, 11:00 AM",
    priority: "High",
    status: "Open",
    freshness: "Stale",
    summary: "Locate the missing renewal prerequisite before the customer review.",
    decisionPath: {
      relationship: {
        detail: "4 stakeholders are mapped in retained context",
        state: "ready"
      },
      evidence: {
        detail: "The approval artifact is older than its expected refresh window",
        state: "needs-attention"
      },
      dependency: {
        detail: "One renewal prerequisite is blocked pending a fresh artifact",
        state: "blocked"
      },
      consequence: {
        title: "Renewal review stays local",
        detail: "Recording this review preserves the observed block locally; it does not change or clear a renewal dependency."
      }
    },
    correlationId: "crm-demo-activity-003",
  }
];

export const activityQueueCorrelationPrefix = "crm-demo-activity";
