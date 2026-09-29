import { adminAuthHeaders } from "./authHeaders";
import { stringifyGraphqlRequest } from "./form";

export type GraphqlRequestResult = {
  data: Record<string, unknown>;
  payload: Record<string, unknown>;
  responseMs: number;
  httpStatus: number;
  requestId: string;
  correlationId: string;
  responseRequestId?: string | null;
  responseCorrelationId?: string | null;
};

export class GraphqlRequestError extends Error {
  payload: Record<string, unknown>;
  responseMs: number;
  httpStatus: number;
  requestId: string;
  correlationId: string;
  responseRequestId?: string | null;
  responseCorrelationId?: string | null;

  constructor(
    message: string,
    payload: Record<string, unknown>,
    responseMs: number,
    httpStatus: number,
    requestContext: GraphqlRequestContext
  ) {
    super(message);
    this.name = "GraphqlRequestError";
    this.payload = payload;
    this.responseMs = responseMs;
    this.httpStatus = httpStatus;
    this.requestId = requestContext.requestId;
    this.correlationId = requestContext.correlationId;
    this.responseRequestId = requestContext.responseRequestId;
    this.responseCorrelationId = requestContext.responseCorrelationId;
  }
}

export async function graphqlDetailed(
  schemaName: string,
  query: string,
  variables: Record<string, unknown>
): Promise<GraphqlRequestResult> {
  const startedAt = performance.now();
  const requestId = newRequestId(schemaName);
  const correlationId = requestId;
  const response = await fetch(schemaEndpoint(schemaName), {
    method: "POST",
    headers: {
      ...adminAuthHeaders(),
      "content-type": "application/json",
      "x-request-id": requestId,
      "x-correlation-id": correlationId,
      "x-timezone": Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC"
    },
    // Int64 values are digit-strings marked in normalizeInput; emit them as raw
    // JSON numbers so i64 fidelity is not lost to JS Number / JSON.stringify.
    body: stringifyGraphqlRequest(query, variables)
  });
  const responseMs = performance.now() - startedAt;
  const payload = await response.json().catch(() => ({} as Record<string, unknown>));
  const requestContext: GraphqlRequestContext = {
    requestId,
    correlationId,
    responseRequestId: response.headers.get("x-request-id"),
    responseCorrelationId: response.headers.get("x-correlation-id")
  };
  if (!response.ok || payload.errors) {
    const errors = Array.isArray(payload.errors) ? payload.errors : null;
    const message = errors
      ? errors.map((error: unknown) => String((error as { message?: unknown }).message ?? "GraphQL error")).join("; ")
      : response.statusText;
    throw new GraphqlRequestError(message || "GraphQL request failed", payload, responseMs, response.status, requestContext);
  }
  return {
    data: (payload.data ?? {}) as Record<string, unknown>,
    payload,
    responseMs,
    httpStatus: response.status,
    ...requestContext
  };
}

export async function graphql(schemaName: string, query: string, variables: Record<string, unknown>) {
  return (await graphqlDetailed(schemaName, query, variables)).data;
}

function schemaEndpoint(schemaName: string) {
  return `/${schemaName.replace(/_/g, "-")}`;
}

type GraphqlRequestContext = {
  requestId: string;
  correlationId: string;
  responseRequestId?: string | null;
  responseCorrelationId?: string | null;
};

function newRequestId(schemaName: string) {
  const safeSchema = schemaName.replace(/[^a-zA-Z0-9]/g, "-");
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `admin-${safeSchema}-${random}`;
}
