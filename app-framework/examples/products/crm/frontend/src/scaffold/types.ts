import type { AppfwUiEntityContract } from "../generated/appfw-ui-contract";
import type { AppfwRecord } from "../lib/appfwClient";

export type EntityScaffoldMode = "list" | "record" | "new";

export type RequestTrace = {
  requestId: string;
  correlationId: string;
  responseMs: number;
};

export type FilterJoin = "and" | "or";

export type FilterOperator =
  | "_contains"
  | "_not_contains"
  | "_starts"
  | "_ends"
  | "_eq"
  | "_ne"
  | "_gt"
  | "_gte"
  | "_lt"
  | "_lte"
  | "_in"
  | "_not_in";

export type FilterRuleDraft = {
  id: string;
  fieldName: string;
  operator: FilterOperator;
  value: string;
};

export type SortDirection = "" | "ASC" | "DESC";

export type QueryFormState = "empty" | "clean" | "dirty";

export type FilterOperatorOption = {
  value: FilterOperator;
  label: string;
  list?: boolean;
};

export type LookupOption = {
  value: string;
  label: string;
};

export type RelationshipSelectorMode = "lookup" | "entity-list";

export type LookupState = {
  status: "idle" | "loading" | "loaded" | "error";
  options: LookupOption[];
  rows: AppfwRecord[];
  error?: string;
};

export type RelationshipLookupState = LookupState & {
  targetEntity?: AppfwUiEntityContract;
};
