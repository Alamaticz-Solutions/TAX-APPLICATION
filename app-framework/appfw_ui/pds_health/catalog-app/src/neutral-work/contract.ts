export type NeutralWorkOperationalState =
  | "loading"
  | "populated"
  | "empty"
  | "error"
  | "partial"
  | "stale"
  | "offline"
  | "unauthorized"
  | "forbidden"
  | "conflict"
  | "timeout"
  | "success"
  | "recovery";

export const NEUTRAL_WORK_OPERATIONAL_STATES = [
  "loading",
  "populated",
  "empty",
  "error",
  "partial",
  "stale",
  "offline",
  "unauthorized",
  "forbidden",
  "conflict",
  "timeout",
  "success",
  "recovery"
] as const satisfies readonly NeutralWorkOperationalState[];

export type NeutralWorkAttention = "none" | "watch" | "needed";
export type NeutralWorkApproval = "not-needed" | "pending" | "observed";
export type NeutralWorkPermissionResult = "permitted" | "restricted";

export type NeutralWorkItem = Readonly<{
  id: string;
  token: string;
  title: string;
  summary: string;
  searchText: string;
  statusLabel: string;
  statusValue: string;
  attention: NeutralWorkAttention;
  approval: NeutralWorkApproval;
  freshnessLabel: string;
  timestamp: string;
  evidenceSummary: string;
}>;

export type NeutralWorkNotification = Readonly<{
  id: string;
  itemId: string;
  label: string;
  read: boolean;
}>;

export type NeutralWorkFixture = Readonly<{
  state: NeutralWorkOperationalState;
  sourceLabel: string;
  permissionResult: NeutralWorkPermissionResult;
  message: string;
  retryPosture: string;
  recoveryContext: string;
  query: string;
  statusFilter: string;
  selectedItemId: string | null;
  focusTarget: string | null;
  items: readonly NeutralWorkItem[];
  notifications: readonly NeutralWorkNotification[];
}>;

export function isNeutralWorkOperationalState(value: string | null): value is NeutralWorkOperationalState {
  return value !== null && NEUTRAL_WORK_OPERATIONAL_STATES.some((state) => state === value);
}
export function isKnownNeutralWorkToken(
  token: string | null,
  items: readonly NeutralWorkItem[]
): boolean {
  return token !== null && items.some((item) => item.token === token);
}
