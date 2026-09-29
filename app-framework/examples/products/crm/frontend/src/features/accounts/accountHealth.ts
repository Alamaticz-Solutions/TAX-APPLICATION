import type { AppfwOperationRequest, AppfwOperationResult } from "../../lib/appfwClient";

type AccountHealthClient = {
  graphql<TData, TVariables extends Record<string, unknown>>(
    request: AppfwOperationRequest<TVariables>
  ): Promise<AppfwOperationResult<TData>>;
};

type AccountHealthData = {
  accountHealth?: AccountHealthSummary;
};

type AccountHealthRefreshData = {
  refreshAccountHealthStoredProcedure?: AccountHealthStoredProcedureResult | null;
};

export type AccountHealthSummary = {
  account?: {
    id?: string;
    name?: string;
    website?: string | null;
    annual_revenue?: number | null;
    number_of_employees?: number | null;
    location?: {
      city?: string | null;
      state?: string | null;
      country?: string | null;
    };
  };
  health?: {
    score?: number;
    grade?: string;
    risk_level?: string;
    summary?: string;
    generated_at?: string;
  };
  stored_snapshot?: AccountHealthStoredProcedureResult | null;
  financials?: {
    annual_revenue?: number;
    pipeline_value?: number;
    weighted_pipeline_value?: number;
    quote_value?: number;
    open_quote_value?: number;
  };
  opportunities?: {
    total_count?: number;
    open_count?: number;
    won_count?: number;
    closed_lost_count?: number;
    stage_distribution?: AccountHealthStage[];
    top_open?: AccountHealthOpportunity[];
  };
  quotes?: {
    total_count?: number;
    open_count?: number;
    expired_count?: number;
    largest_open?: AccountHealthQuote | null;
  };
  activities?: {
    total_count?: number;
    open_count?: number;
    overdue_count?: number;
    last_activity_date?: string | null;
    next_due_date?: string | null;
    stale_days?: number | null;
    engagement_status?: string;
  };
  contacts?: {
    total_count?: number;
    complete_contact_count?: number;
    missing_email_count?: number;
  };
  data_quality?: {
    profile_completeness?: number;
    fields?: AccountHealthQualityField[];
  };
  signals?: AccountHealthSignal[];
  recommended_actions?: AccountHealthAction[];
  analysis?: {
    sample_limit?: number;
    note?: string;
  };
};

export type AccountHealthStoredProcedureResult = {
  account_id?: string;
  health_score?: number | null;
  refreshed_at?: string | null;
  source?: string | null;
};

export type AccountHealthStage = {
  stage?: string;
  count?: number;
  amount?: number;
};

export type AccountHealthOpportunity = {
  id?: string;
  name?: string;
  amount?: number | null;
  close_date?: string | null;
  stage?: string | null;
  probability?: number | null;
};

export type AccountHealthQuote = {
  id?: string;
  name?: string;
  quote_number?: string;
  total_price?: number | null;
  expiration_date?: string | null;
  status?: string | null;
};

export type AccountHealthQualityField = {
  name?: string;
  complete?: boolean;
};

export type AccountHealthSignal = {
  category?: string;
  severity?: string;
  label?: string;
  detail?: string;
};

export type AccountHealthAction = {
  priority?: string;
  label?: string;
  detail?: string;
};

export type AccountHealthResult = {
  accountId: string;
  health: AccountHealthSummary | null;
  request: {
    accountHealthMs: number;
    requestId: string;
  };
};

export type AccountHealthRefreshResult = {
  accountId: string;
  storedSnapshot: AccountHealthStoredProcedureResult | null;
  request: {
    procedureMs: number;
    requestId: string;
  };
};

const ACCOUNT_HEALTH = `query AccountHealthDashboard($accountId: String!) {
  accountHealth(accountId: $accountId)
}`;

const REFRESH_ACCOUNT_HEALTH_STORED_PROCEDURE = `mutation RefreshAccountHealthStoredProcedure($accountId: String!, $healthScore: Float!) {
  refreshAccountHealthStoredProcedure(accountId: $accountId, healthScore: $healthScore)
}`;

export async function loadAccountHealth(client: AccountHealthClient, accountId: string): Promise<AccountHealthResult> {
  // Keep custom-method GraphQL isolated in this feature loader until the UI
  // contract exposes workflow method metadata alongside generated entities.
  const health = await client.graphql<AccountHealthData, { accountId: string }>({
    schemaName: "crm",
    operationName: "account_health_dashboard",
    query: ACCOUNT_HEALTH,
    variables: { accountId }
  });

  return {
    accountId,
    health: health.data.accountHealth ?? null,
    request: {
      accountHealthMs: health.responseMs,
      requestId: health.requestId
    }
  };
}

export async function refreshAccountHealthSnapshot(
  client: AccountHealthClient,
  accountId: string,
  healthScore: number
): Promise<AccountHealthRefreshResult> {
  const refresh = await client.graphql<AccountHealthRefreshData, { accountId: string; healthScore: number }>({
    schemaName: "crm",
    operationName: "refresh_account_health_stored_procedure",
    query: REFRESH_ACCOUNT_HEALTH_STORED_PROCEDURE,
    variables: { accountId, healthScore }
  });

  return {
    accountId,
    storedSnapshot: refresh.data.refreshAccountHealthStoredProcedure ?? null,
    request: {
      procedureMs: refresh.responseMs,
      requestId: refresh.requestId
    }
  };
}
