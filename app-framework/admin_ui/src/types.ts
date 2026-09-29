export type DataType =
  | "Uuid"
  | "UuidArray"
  | "ObjectId"
  | "ObjectIdArray"
  | "Boolean"
  | "String"
  | "StringArray"
  | "Date"
  | "DateTime"
  | "Time"
  | "Int8"
  | "Int8Array"
  | "Int16"
  | "Int16Array"
  | "Int32"
  | "Int32Array"
  | "Int64"
  | "Int64Array"
  | "Float32"
  | "Float64"
  | "Enum"
  | "EnumArray"
  | "Object"
  | "ObjectArray"
  | "Json"
  | "JsonArray"
  | "NavToOne"
  | "NavToMany"
  | "ManyToMany";

export type StandardMethod = "FindById" | "GetAll" | "Query" | "Create" | "Update" | "Delete";

export type CustomMethodKind = "Query" | "Mutation" | "Command";

export type TypeMethodArg = {
  name: string;
  arg_type: string;
};

export type CustomMethod = {
  name: string;
  kind: CustomMethodKind;
  args: TypeMethodArg[];
  return_type: string;
};

export type DataSourceType = "PostgreSQL" | "MongoDB" | "MsSqlServer" | "Snowflake";

export type MigrationDetail = {
  id: string;
  name: string;
  schema?: string | null;
  data_source: string;
  dialect: string;
  phase: string;
  path: string;
  description?: string | null;
};

export type SchemaHealthTone = "ok" | "warning" | "error" | "unknown";

export type SchemaHealthItem = {
  status: SchemaHealthTone;
  label: string;
  message: string;
};

export type ProviderCapabilityDetail = {
  area_key: string;
  area_label: string;
  status: string;
  reason?: string | null;
  evidence: Array<{
    kind: string;
    contract: string;
  }>;
};

export type SchemaHealth = {
  migration_status: SchemaHealthItem;
  pending_drift: SchemaHealthItem;
  connectivity: SchemaHealthItem;
  entity_count: number;
  table_entity_count: number;
  migration_count: number;
  provider_capabilities: ProviderCapabilityDetail[];
};

export type SchemaModel = {
  id: string;
  name: string;
  description: string;
  data_source_name: string;
  data_source_type?: DataSourceType | null;
  filter_capabilities?: FilterCapabilities | null;
  latest_migration?: MigrationDetail | null;
  health?: SchemaHealth | null;
};

export type ForeignKey = {
  schema_name: string;
  type_name: string;
};

export type NavByFkProperty = {
  schema_name: string;
  type_name: string;
  prop_name: string;
  resolved: ForeignKey;
};

export type ManyToManyProperty = {
  target_schema: string;
  target_type: string;
  junction_table: string;
  junction_schema?: string | null;
  local_key: string;
  foreign_key: string;
};

export type PropertyType = {
  id: string;
  name: string;
  caption: string;
  is_key: boolean;
  is_caption: boolean;
  is_required: boolean;
  is_read_only: boolean;
  is_concurrency_control: boolean;
  data_type: DataType;
  computed: string;
  default_value: unknown;
  foreign_key: ForeignKey | null;
  nav_by_fk_property: NavByFkProperty | null;
  many_to_many_property: ManyToManyProperty | null;
  nested_entity_type: unknown;
  enum_type_name: string | null;
  meta: Record<string, unknown> | null;
};

export type EntityType = {
  id: string;
  schema_name: string;
  pascal_1: string;
  pascal_n: string;
  snake_1: string;
  snake_n: string;
  caption_1: string;
  caption_n: string;
  is_union: boolean;
  is_table: boolean;
  base_type: string | null;
  facets: string[] | null;
  meta?: Record<string, unknown> | null;
  standard_methods: StandardMethod[] | null;
  custom_methods?: CustomMethod[] | null;
  props: PropertyType[];
};

export type GridColumnKind = "native" | "related";

export type GridColumn = {
  id: string;
  kind: GridColumnKind;
  label: string;
  pathLabel: string;
  data_type: DataType;
  prop: PropertyType;
  relation_prop?: PropertyType;
  target_prop?: PropertyType;
  path: string[];
  sortable: boolean;
};

export type AdminModel = {
  backend_version: string;
  troubleshooting_enabled?: boolean;
  schemas: SchemaModel[];
  entity_types: EntityType[];
};

export type RecordValue = Record<string, unknown>;

export type QueryResult = {
  query_count?: number;
  queryCount?: number;
  page_count?: number;
  pageCount?: number;
  items?: RecordValue[];
};

export type FormActionStatus = {
  message: string;
  tone: StatusTone;
  action?: "load" | "save" | "delete";
  responseMs?: number | null;
  isBusy?: boolean;
};

export type AuditTimelineEvent = {
  audit_id?: string;
  occurred_at?: string;
  tenant_id?: string | null;
  actor_user_name?: string;
  actor_roles?: string[];
  action?: string;
  outcome?: string;
  record_id?: string | null;
  diff_json?: Record<string, { before?: unknown; after?: unknown }> | null;
  policy_json?: Record<string, unknown> | null;
  redactions_json?: Record<string, unknown> | null;
  event_hash?: string;
  prev_hash?: string | null;
};

export type AuditTimelineStatus = {
  isLoading: boolean;
  tone: StatusTone;
  message: string;
  responseMs?: number | null;
  enabled?: boolean;
  audited?: boolean;
  events: AuditTimelineEvent[];
  currentPolicy?: {
    allow: boolean;
    filter?: unknown;
  } | null;
};

export type GraphqlTrace = {
  id: string;
  operation: string;
  schemaName: string;
  query: string;
  variables: Record<string, unknown>;
  data?: Record<string, unknown>;
  payload?: Record<string, unknown>;
  error?: string;
  responseMs: number;
  httpStatus: number;
  requestId?: string;
  correlationId?: string;
  responseRequestId?: string | null;
  responseCorrelationId?: string | null;
  createdAt: string;
  ok: boolean;
};

export type RecordDiff = {
  prop: PropertyType;
  before: unknown;
  after: unknown;
};

export type FormQueryPreview = {
  operation: "FindById";
  method: string;
  query: string;
  variables: Record<string, unknown>;
};

export type SaveMutationPreview = {
  operation: "Create" | "Update";
  method: string;
  query: string;
  variables: Record<string, unknown>;
};

export type CustomMethodPreview = {
  method: CustomMethod;
  fieldName: string;
  operationKind: "query" | "mutation";
  operationName: string;
  query: string;
  variables: Record<string, unknown>;
};

export type CustomMethodStatus = {
  isBusy?: boolean;
  tone: StatusTone;
  message: string;
  responseMs?: number | null;
  data?: unknown;
};

export type AdvancedFilterJoin = "and" | "or";

export type AdvancedFilterOperator =
  | "_eq"
  | "_ne"
  | "_contains"
  | "_not_contains"
  | "_starts"
  | "_ends"
  | "_regex"
  | "_overlaps"
  | "_not_overlaps"
  | "_contained_by"
  | "_not_contained_by"
  | "_lt"
  | "_lte"
  | "_gt"
  | "_gte"
  | "_in"
  | "_not_in"
  | "_before"
  | "_during"
  | "_after";

export type FilterValueShape = "scalar" | "list" | "scalar_or_list" | "period";

export type FilterOperatorCapability = {
  op: AdvancedFilterOperator;
  label: string;
  value_shape: FilterValueShape;
  supported: boolean;
  unsupported_reason?: string | null;
};

export type FilterDataTypeCapability = {
  data_type: DataType;
  operators: FilterOperatorCapability[];
};

export type FilterCapabilities = {
  provider: DataSourceType;
  data_types: FilterDataTypeCapability[];
};

export type AdvancedFilterRule = {
  kind: "rule";
  id: string;
  propId: string;
  operator: AdvancedFilterOperator;
  value: string;
};

export type AdvancedFilterGroup = {
  kind: "group";
  id: string;
  join: AdvancedFilterJoin;
  children: AdvancedFilterNode[];
};

export type AdvancedFilterNode = AdvancedFilterRule | AdvancedFilterGroup;

export type AdvancedFilterState = AdvancedFilterGroup;

export type SavedGridView = {
  visibleColumnIds: string[];
  recordSearch: string;
  sort: GridSort | null;
  advancedFilter?: AdvancedFilterState | null;
};

export type LookupOption = {
  value: string;
  label: string;
};

export type ActiveScope = {
  entityId: string;
  label: string;
  filter?: Record<string, unknown> | null;
  snapshot?: RecordValue[] | null;
};

export type RelationshipAction = {
  prop: PropertyType;
  target: EntityType | null;
  title: string;
  subtitle: string;
  disabled: boolean;
  reason?: string;
  filter?: Record<string, unknown> | null;
  snapshot?: RecordValue[] | null;
};

export type MetaChip = {
  label: string;
  description?: string;
};

export type MetaGroup = {
  label: string;
  description: string;
  items: MetaChip[];
};

export type DrawerMode = "create" | "edit";

export type StatusTone = "idle" | "ok" | "error";

export type SortDirection = "asc" | "desc";

export type GridSort = {
  propId: string;
  propName: string;
  direction: SortDirection;
};
