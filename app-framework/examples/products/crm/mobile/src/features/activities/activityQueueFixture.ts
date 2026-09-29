export const activityWorkQueueContract = "activity-work-queue.v1" as const;

export type ActivityQueueItem = {
  id: string;
  subject: string;
  account: string;
  owner: string;
  dueLabel: string;
  status: "Open" | "In progress" | "Completed";
  freshness: "Current" | "Refreshing" | "Stale";
  summary: string;
  dependency: string;
};

export const activityQueueFixture: ActivityQueueItem[] = [
  { id: "activity-acme-discovery", subject: "Discovery call follow-up", account: "Acme Analytics", owner: "Maya Chen", dueLabel: "Due today, 2:30 PM", status: "Open", freshness: "Current", summary: "Confirm stakeholder priorities before the proposal outline is shared.", dependency: "Stakeholder notes are complete; proposal context needs review." },
  { id: "activity-northstar-demo", subject: "Prepare operating review", account: "Northstar Manufacturing", owner: "Riley Patel", dueLabel: "Tomorrow, 9:00 AM", status: "In progress", freshness: "Refreshing", summary: "Align the operating review narrative with the latest account health snapshot.", dependency: "Account health refresh is underway." },
  { id: "activity-medical-renewal", subject: "Review renewal dependencies", account: "MediSystems", owner: "Alex Morgan", dueLabel: "Friday, 11:00 AM", status: "Open", freshness: "Stale", summary: "Locate the missing renewal prerequisite before the customer review.", dependency: "One approval artifact has not refreshed from the local fixture." }
];
