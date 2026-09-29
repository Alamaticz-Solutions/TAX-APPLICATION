import type {
  AppfwUiEntityContract,
  AppfwUiOperationContract
} from "../generated/appfw-ui-contract";
import type { CrmAuthContext } from "./authContext";
import type { CrmTenantContext } from "./tenantContext";

const RECORD_LOCATOR_PREFIX = "rl_";

export type AppfwErrorCategory =
  | "validation"
  | "policy_denied"
  | "auth"
  | "provider"
  | "network"
  | "unknown";

export type AppfwClientContext = {
  baseUrl?: string;
  authorization?: string;
  tenantId?: string;
  auth?: CrmAuthContext;
  tenant?: CrmTenantContext;
  timezone?: string;
  fetchImpl?: typeof fetch;
};

export type AppfwOperationRequest<TVariables extends Record<string, unknown>> = {
  schemaName: string;
  operationName: string;
  query: string;
  variables: TVariables;
};

export type AppfwOperationResult<TData> = {
  data: TData;
  requestId: string;
  correlationId: string;
  responseMs: number;
  httpStatus: number;
};

export type AppfwRecord = Record<string, unknown>;

export type AppfwEntityListVariables = {
  skip?: number;
  limit?: number;
  after?: string | null;
  filter?: unknown;
  sort?: unknown;
  selection?: readonly string[];
};

export type AppfwEntityPage = {
  skip: number;
  limit: number;
  pageCount: number;
  pageIndex: number;
  queryCount: number;
  nextCursor?: string | null;
  previousCursor?: string | null;
};

export type AppfwEntityListData = {
  entity: AppfwUiEntityContract;
  operation: AppfwUiOperationContract;
  selection: readonly string[];
  rows: AppfwRecord[];
  page: AppfwEntityPage;
};

export type AppfwEntityRecordData = {
  entity: AppfwUiEntityContract;
  operation: AppfwUiOperationContract;
  selection: readonly string[];
  record: AppfwRecord | null;
};

export type AppfwEntitySaveMode = "create" | "update";

export type AppfwEntityDeleteData = {
  entity: AppfwUiEntityContract;
  operation: AppfwUiOperationContract;
  affectedRows: number;
};

export type AppfwEntityDeleteOptions = {
  confirmed?: boolean;
};

export type AppfwOperationError = {
  message: string;
  category: AppfwErrorCategory;
  requestId?: string;
  correlationId?: string;
  httpStatus?: number;
  responseMs?: number;
  validation?: Record<string, string[]>;
};

export class AppfwClientError extends Error {
  readonly details: AppfwOperationError;

  constructor(details: AppfwOperationError) {
    super(details.message);
    this.name = "AppfwClientError";
    this.details = details;
  }
}

export function createAppfwClient(context: AppfwClientContext = {}) {
  const fetchImpl = context.fetchImpl ?? fetch;

  async function graphql<TData, TVariables extends Record<string, unknown>>(
    request: AppfwOperationRequest<TVariables>
  ): Promise<AppfwOperationResult<TData>> {
    const startedAt = performance.now();
    const requestId = newRequestId(request.schemaName, request.operationName);
    const correlationId = requestId;
    const response = await fetchImpl(schemaEndpoint(context.baseUrl, request.schemaName), {
      method: "POST",
      headers: requestHeaders(context, requestId, correlationId),
      body: JSON.stringify({ query: request.query, variables: request.variables })
    }).catch((error: unknown) => {
      throw new AppfwClientError({
        message: error instanceof Error ? error.message : "Network request failed",
        category: "network",
        requestId,
        correlationId,
        responseMs: performance.now() - startedAt
      });
    });
    const responseMs = performance.now() - startedAt;
    const payload = (await response.json().catch(() => ({}))) as GraphqlPayload<TData>;
    const responseRequestId = response.headers.get("x-request-id") ?? requestId;
    const responseCorrelationId = response.headers.get("x-correlation-id") ?? correlationId;

    if (!response.ok || payload.errors?.length) {
      throw new AppfwClientError({
        message: errorMessage(payload, response.statusText),
        category: errorCategory(response.status, payload),
        requestId: responseRequestId,
        correlationId: responseCorrelationId,
        httpStatus: response.status,
        responseMs,
        validation: validationErrors(payload)
      });
    }

    return {
      data: (payload.data ?? {}) as TData,
      requestId: responseRequestId,
      correlationId: responseCorrelationId,
      responseMs,
      httpStatus: response.status
    };
  }

  async function queryEntityList(
    entity: AppfwUiEntityContract,
    variables: AppfwEntityListVariables = {}
  ): Promise<AppfwOperationResult<AppfwEntityListData>> {
    const operation = requiredOperation(
      entity,
      (candidate) => candidate.kind === "query" && candidate.returnsShape === "connection",
      "query connection"
    );
    const selection = operationSelection(entity, operation, "list", variables.selection);
    const query = `query ${operation.graphqlName}($filter: JSON, $sort: JSON, $skip: Int, $limit: Int, $after: String) {
  ${operation.graphqlName}(filter: $filter, sort: $sort, skip: $skip, limit: $limit, after: $after) {
    date_time
    request_duration
    skip
    limit
    page_count
    page_index
    query_count
    next_cursor
    previous_cursor
    items {
      ${selection.join("\n      ")}
    }
  }
}`;
    const result = await graphql<EntityQueryPayload, AppfwEntityListVariables>({
      schemaName: entity.schemaName,
      operationName: operation.name,
      query,
      variables: {
        filter: variables.filter ?? undefined,
        sort: variables.sort ?? undefined,
        skip: variables.skip,
        limit: variables.limit ?? 25,
        after: variables.after ?? undefined
      }
    });
    const connection = queryConnection(result.data, operation.graphqlName);

    return {
      ...result,
      data: {
        entity,
        operation,
        selection,
        rows: extractRows(connection),
        page: {
          skip: numberOr(connection.skip, 0),
          limit: numberOr(connection.limit, variables.limit ?? 25),
          pageCount: numberOr(connection.page_count, 0),
          pageIndex: numberOr(connection.page_index, 0),
          queryCount: numberOr(connection.query_count, 0),
          nextCursor: stringOrNull(connection.next_cursor),
          previousCursor: stringOrNull(connection.previous_cursor)
        }
      }
    };
  }

  async function findEntityRecord(
    entity: AppfwUiEntityContract,
    id: string,
    selectionOverride?: readonly string[]
  ): Promise<AppfwOperationResult<AppfwEntityRecordData>> {
    const operation = requiredOperation(
      entity,
      (candidate) => candidate.kind === "query" && candidate.returnsShape === "record",
      "find record"
    );
    const selection = operationSelection(entity, operation, "detail", selectionOverride);
    const usesLocator = isRecordLocator(entity, id);
    const locatorOperation = entity.addressing?.locatorResolveOperation;
    const fieldName = usesLocator
      ? locatorOperation?.graphqlName ?? `${operation.graphqlName}ByLocator`
      : operation.graphqlName;
    const variableName = usesLocator ? "locator" : "id";
    const query = `query ${fieldName}($${variableName}: String!) {
  ${fieldName}(${variableName}: $${variableName}) {
    ${selection.join("\n    ")}
  }
}`;
    const result = await graphql<Record<string, AppfwRecord | null>, { id?: string; locator?: string }>({
      schemaName: entity.schemaName,
      operationName: usesLocator ? locatorOperation?.name ?? `${operation.name}_by_locator` : operation.name,
      query,
      variables: usesLocator ? { locator: id } : { id }
    });

    return {
      ...result,
      data: {
        entity,
        operation,
        selection,
        record: (result.data[fieldName] ?? null) as AppfwRecord | null
      }
    };
  }

  async function saveEntityRecord(
    entity: AppfwUiEntityContract,
    mode: AppfwEntitySaveMode,
    input: AppfwRecord,
    selectionOverride?: readonly string[]
  ): Promise<AppfwOperationResult<AppfwEntityRecordData>> {
    const operation = requiredOperation(
      entity,
      (candidate) =>
        candidate.kind === "mutation" &&
        candidate.returnsShape === "record" &&
        candidate.name.startsWith(`${mode}_`),
      `${mode} record`
    );
    const selection = operationSelection(entity, operation, "detail", selectionOverride);
    const query = `mutation ${operation.graphqlName}($input: Input${entity.typeName}!) {
  ${operation.graphqlName}(input: $input) {
    ${selection.join("\n    ")}
  }
}`;
    const result = await graphql<Record<string, AppfwRecord>, { input: AppfwRecord }>({
      schemaName: entity.schemaName,
      operationName: operation.name,
      query,
      variables: { input }
    });

    return {
      ...result,
      data: {
        entity,
        operation,
        selection,
        record: (result.data[operation.graphqlName] ?? null) as AppfwRecord | null
      }
    };
  }

  async function deleteEntityRecord(
    entity: AppfwUiEntityContract,
    input: AppfwRecord,
    options: AppfwEntityDeleteOptions = {}
  ): Promise<AppfwOperationResult<AppfwEntityDeleteData>> {
    const operation = entity.operations.find(
      (candidate) =>
        candidate.kind === "mutation" &&
        candidate.returnsShape === "scalar" &&
        candidate.name.startsWith("delete_")
    );
    if (!operation) {
      throw new AppfwClientError({
        message: `No generated delete record operation exists for ${entity.schemaName}.${entity.typeName}.`,
        category: "unknown"
      });
    }
    if (operation.disabledReason && !(options.confirmed && isDeleteConfirmationDisabledReason(operation.disabledReason))) {
      throw new AppfwClientError({
        message: operation.disabledReason,
        category: "policy_denied"
      });
    }
    const query = `mutation ${operation.graphqlName}($input: Input${entity.typeName}!) {
  ${operation.graphqlName}(input: $input)
}`;
    const result = await graphql<Record<string, unknown>, { input: AppfwRecord }>({
      schemaName: entity.schemaName,
      operationName: operation.name,
      query,
      variables: { input }
    });

    return {
      ...result,
      data: {
        entity,
        operation,
        affectedRows: numberOr(result.data[operation.graphqlName], 0)
      }
    };
  }

  return { graphql, queryEntityList, findEntityRecord, saveEntityRecord, deleteEntityRecord };
}

type GraphqlPayload<TData> = {
  data?: TData;
  errors?: GraphqlError[];
};

type GraphqlError = {
  message?: string;
  extensions?: {
    code?: string;
    category?: string;
    validation?: Record<string, string[]>;
  };
};

type EntityQueryPayload = Record<string, EntityQueryConnection | undefined>;

type EntityQueryConnection = {
  date_time?: unknown;
  request_duration?: unknown;
  skip?: unknown;
  limit?: unknown;
  page_count?: unknown;
  page_index?: unknown;
  query_count?: unknown;
  next_cursor?: unknown;
  previous_cursor?: unknown;
  items?: unknown;
};

function requestHeaders(context: AppfwClientContext, requestId: string, correlationId: string) {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    "x-request-id": requestId,
    "x-correlation-id": correlationId,
    "x-timezone": context.timezone ?? browserTimezone()
  };
  const authorization = context.authorization ?? context.auth?.authorization;
  const tenantId = context.tenantId ?? context.tenant?.tenantId;
  if (authorization) headers.authorization = authorization;
  if (tenantId) headers["x-tenant-id"] = tenantId;
  return headers;
}

function schemaEndpoint(baseUrl: string | undefined, schemaName: string) {
  const path = `/${schemaName.replace(/_/g, "-")}`;
  return baseUrl ? `${baseUrl.replace(/\/$/, "")}${path}` : path;
}

function newRequestId(schemaName: string, operationName: string) {
  const safeSchema = schemaName.replace(/[^a-zA-Z0-9]/g, "-");
  const safeOperation = operationName.replace(/[^a-zA-Z0-9]/g, "-");
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `crm-ui-${safeSchema}-${safeOperation}-${random}`;
}

function browserTimezone() {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

function errorMessage<TData>(payload: GraphqlPayload<TData>, fallback: string) {
  const messages = payload.errors?.map((error) => error.message).filter(Boolean) ?? [];
  return messages.length ? messages.join("; ") : fallback || "App Framework request failed";
}

function errorCategory<TData>(httpStatus: number, payload: GraphqlPayload<TData>): AppfwErrorCategory {
  const rawCategory = payload.errors?.find((error) => error.extensions?.category)?.extensions?.category;
  const rawCode = payload.errors?.find((error) => error.extensions?.code)?.extensions?.code;
  const normalized = String(rawCategory ?? rawCode ?? "").toLowerCase();

  if (httpStatus === 401 || httpStatus === 403 || normalized.includes("auth")) return "auth";
  if (normalized.includes("validation")) return "validation";
  if (normalized.includes("policy") || normalized.includes("denied")) return "policy_denied";
  if (normalized.includes("provider")) return "provider";
  return "unknown";
}

function validationErrors<TData>(payload: GraphqlPayload<TData>) {
  return payload.errors?.find((error) => error.extensions?.validation)?.extensions?.validation;
}

function requiredOperation(
  entity: AppfwUiEntityContract,
  predicate: (operation: AppfwUiOperationContract) => boolean,
  label: string
) {
  const operation = entity.operations.find(predicate);
  if (!operation) {
    throw new AppfwClientError({
      message: `No generated ${label} operation exists for ${entity.schemaName}.${entity.typeName}.`,
      category: "unknown"
    });
  }
  if (operation.disabledReason) {
    throw new AppfwClientError({
      message: operation.disabledReason,
      category: "policy_denied"
    });
  }
  return operation;
}

function isDeleteConfirmationDisabledReason(reason: string) {
  return reason === "Generated delete actions require product confirmation UX before enabling.";
}

function operationSelection(
  entity: AppfwUiEntityContract,
  operation: AppfwUiOperationContract,
  surface: "list" | "detail",
  selectionOverride?: readonly string[]
) {
  const contractFields =
    selectionOverride?.length
      ? selectionOverride
      : operation.selectionPreset.length > 0
      ? operation.selectionPreset
      : surface === "list"
        ? entity.scaffold.list.fields
        : entity.scaffold.detail.fields;
  const validFields = new Set([...entity.fields.map((field) => field.name), routeIdField(entity)]);
  const selection = contractFields.filter((fieldName) => validFields.has(fieldName) && isGraphqlName(fieldName));
  if (selection.length > 0) return Array.from(new Set(selection));
  return [entity.primaryKey, entity.captionField].filter((fieldName) => fieldName && isGraphqlName(fieldName));
}

function routeIdField(entity: AppfwUiEntityContract) {
  const routeKind = entity.addressing?.idKinds.find((kind) => kind.kind === entity.addressing.routeIdKind);
  return routeKind?.field ?? "record_locator";
}

function isRecordLocator(entity: AppfwUiEntityContract, value: string) {
  const routeKind = entity.addressing?.idKinds.find((kind) => kind.kind === entity.addressing.routeIdKind);
  return (routeKind?.kind ?? "record_locator") === "record_locator" && value.startsWith(RECORD_LOCATOR_PREFIX);
}

function isGraphqlName(value: string) {
  return /^[_A-Za-z][_0-9A-Za-z]*$/.test(value);
}

function queryConnection(payload: EntityQueryPayload, fieldName: string): EntityQueryConnection {
  return payload[fieldName] ?? {};
}

function extractRows(connection: EntityQueryConnection): AppfwRecord[] {
  return Array.isArray(connection.items) ? (connection.items as AppfwRecord[]) : [];
}

function numberOr(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function stringOrNull(value: unknown) {
  return typeof value === "string" && value ? value : null;
}
