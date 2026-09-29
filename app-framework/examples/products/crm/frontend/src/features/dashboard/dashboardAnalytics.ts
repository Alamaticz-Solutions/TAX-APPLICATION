import type { AppfwOperationRequest, AppfwOperationResult } from "../../lib/appfwClient";

type DashboardClient = {
  graphql<TData, TVariables extends Record<string, unknown>>(
    request: AppfwOperationRequest<TVariables>
  ): Promise<AppfwOperationResult<TData>>;
};

type AggregateConnection = {
  query_count?: number;
  items?: AggregateRow[];
};

type QueryConnection<TItem> = {
  query_count?: number;
  items?: TItem[];
};

type AggregateRow = Record<string, unknown>;

type AccountSummaryData = {
  aggregateAccounts?: AggregateConnection;
};

type PipelineData = {
  aggregateOpportunities?: AggregateConnection;
};

type LeadSummaryData = {
  aggregateLeads?: AggregateConnection;
};

type ActivitySummaryData = {
  aggregateActivities?: AggregateConnection;
};

type StageListData = {
  queryOpportunityStages?: QueryConnection<OpportunityStageRecord>;
};

type OpportunityListData = {
  queryOpportunities?: QueryConnection<OpportunityRecord>;
};

type LeadListData = {
  queryLeads?: QueryConnection<LeadRecord>;
};

type ActivityListData = {
  queryActivities?: QueryConnection<ActivityRecord>;
};

export type CrmDashboardAnalytics = {
  generatedAt: string;
  kpis: {
    accountCount: number;
    accountRevenue: number;
    avgAccountEmployees: number;
    openPipeline: number;
    weightedPipeline: number;
    opportunityCount: number;
    avgDealSize: number;
    openLeads: number;
    convertedLeads: number;
    leadConversionRate: number;
    openActivities: number;
  };
  accountStates: AccountStateBucket[];
  pipelineStages: PipelineStageBucket[];
  topOpportunities: OpportunityRecord[];
  leadQueue: LeadRecord[];
  activityQueue: ActivityRecord[];
  traces: DashboardTrace[];
  warnings: string[];
};

export type AccountStateBucket = {
  state: string;
  accountCount: number;
  totalRevenue: number;
  avgEmployees: number;
};

export type PipelineStageBucket = {
  stageId: string;
  stageName: string;
  opportunityCount: number;
  totalAmount: number;
  avgAmount: number;
  probability: number;
  weightedAmount: number;
  isClosed: boolean;
  isWon: boolean;
  sortOrder: number;
};

export type OpportunityStageRecord = {
  id?: string;
  name?: string;
  probability?: number;
  is_closed?: boolean;
  is_won?: boolean;
  sort_order?: number;
};

export type OpportunityRecord = {
  id?: string;
  name?: string;
  amount?: number;
  close_date?: string;
  next_step?: string | null;
  lead_source?: string | null;
  account?: {
    id?: string;
    name?: string;
  } | null;
  stage?: OpportunityStageRecord | null;
};

export type LeadRecord = {
  id?: string;
  first_name?: string;
  last_name?: string;
  company?: string;
  email?: string | null;
  annual_revenue?: number | null;
  is_converted?: boolean;
  status?: {
    id?: string;
    name?: string;
  } | null;
  source?: {
    id?: string;
    name?: string;
  } | null;
  industry?: {
    id?: string;
    name?: string;
  } | null;
};

export type ActivityRecord = {
  id?: string;
  subject?: string;
  activity_date?: string | null;
  due_date?: string | null;
  is_closed?: boolean;
  priority?: string | null;
  status?: string | null;
};

export type DashboardTrace = {
  label: string;
  requestId: string;
  correlationId: string;
  responseMs: number;
  httpStatus: number;
  queryCount: number;
  rowCount: number;
};

type SettledDashboardRequest<TData> =
  | {
      ok: true;
      label: string;
      result: AppfwOperationResult<TData>;
      trace: DashboardTrace;
    }
  | {
      ok: false;
      label: string;
      message: string;
    };

const AGGREGATE_ACCOUNTS = `query DashboardAggregateAccounts(
  $filter: JSON
  $groupBy: JSON
  $metrics: JSON
  $having: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  aggregateAccounts(
    filter: $filter
    groupBy: $groupBy
    metrics: $metrics
    having: $having
    sort: $sort
    skip: $skip
    limit: $limit
  ) {
    query_count
    items
  }
}`;

const AGGREGATE_OPPORTUNITIES = `query DashboardAggregateOpportunities(
  $filter: JSON
  $groupBy: JSON
  $metrics: JSON
  $having: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  aggregateOpportunities(
    filter: $filter
    groupBy: $groupBy
    metrics: $metrics
    having: $having
    sort: $sort
    skip: $skip
    limit: $limit
  ) {
    query_count
    items
  }
}`;

const AGGREGATE_LEADS = `query DashboardAggregateLeads(
  $filter: JSON
  $groupBy: JSON
  $metrics: JSON
  $having: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  aggregateLeads(
    filter: $filter
    groupBy: $groupBy
    metrics: $metrics
    having: $having
    sort: $sort
    skip: $skip
    limit: $limit
  ) {
    query_count
    items
  }
}`;

const AGGREGATE_ACTIVITIES = `query DashboardAggregateActivities(
  $filter: JSON
  $groupBy: JSON
  $metrics: JSON
  $having: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  aggregateActivities(
    filter: $filter
    groupBy: $groupBy
    metrics: $metrics
    having: $having
    sort: $sort
    skip: $skip
    limit: $limit
  ) {
    query_count
    items
  }
}`;

const QUERY_OPPORTUNITY_STAGES = `query DashboardOpportunityStages($skip: Int, $limit: Int, $sort: JSON) {
  queryOpportunityStages(skip: $skip, limit: $limit, sort: $sort) {
    query_count
    items {
      id
      name
      probability
      is_closed
      is_won
      sort_order
    }
  }
}`;

const QUERY_TOP_OPPORTUNITIES = `query DashboardTopOpportunities($skip: Int, $limit: Int, $sort: JSON) {
  queryOpportunities(skip: $skip, limit: $limit, sort: $sort) {
    query_count
    items {
      id
      name
      amount
      close_date
      next_step
      lead_source
      account {
        id
        name
      }
      stage {
        id
        name
        probability
        is_closed
        is_won
        sort_order
      }
    }
  }
}`;

const QUERY_LEAD_QUEUE = `query DashboardLeadQueue($filter: JSON, $skip: Int, $limit: Int, $sort: JSON) {
  queryLeads(filter: $filter, skip: $skip, limit: $limit, sort: $sort) {
    query_count
    items {
      id
      first_name
      last_name
      company
      email
      annual_revenue
      is_converted
      status {
        id
        name
      }
      source {
        id
        name
      }
      industry {
        id
        name
      }
    }
  }
}`;

const QUERY_ACTIVITY_QUEUE = `query DashboardActivityQueue($filter: JSON, $skip: Int, $limit: Int, $sort: JSON) {
  queryActivities(filter: $filter, skip: $skip, limit: $limit, sort: $sort) {
    query_count
    items {
      id
      subject
      activity_date
      due_date
      is_closed
      priority
      status
    }
  }
}`;

const SUMMARY_VARIABLES = {
  filter: null,
  groupBy: null,
  metrics: [
    { fn: "count", alias: "account_count" },
    { fn: "sum", field: "annualRevenue", alias: "total_revenue" },
    { fn: "avg", field: "numberOfEmployees", alias: "avg_employees" },
    { fn: "count_distinct", field: "billingCity", alias: "city_count" },
    { fn: "max", field: "annualRevenue", alias: "max_revenue" }
  ],
  having: null,
  sort: null,
  skip: 0,
  limit: 1
};

export async function loadCrmDashboardAnalytics(client: DashboardClient): Promise<CrmDashboardAnalytics> {
  const [
    accountSummary,
    accountsByState,
    pipelineByStage,
    stages,
    leadConversion,
    activitySummary,
    topOpportunities,
    leadQueue,
    activityQueue
  ] = await Promise.all([
    runDashboardRequest<AccountSummaryData>(
      client,
      "Account summary",
      "dashboard_account_summary",
      AGGREGATE_ACCOUNTS,
      SUMMARY_VARIABLES,
      (data) => aggregateTraceCount(data.aggregateAccounts)
    ),
    runDashboardRequest<AccountSummaryData>(
      client,
      "Revenue by state",
      "dashboard_account_states",
      AGGREGATE_ACCOUNTS,
      {
        filter: null,
        groupBy: [{ field: "billingState", alias: "state" }],
        metrics: [
          { fn: "count", alias: "account_count" },
          { fn: "sum", field: "annualRevenue", alias: "total_revenue" },
          { fn: "avg", field: "numberOfEmployees", alias: "avg_employees" }
        ],
        having: { account_count: { _gte: 1 } },
        sort: { total_revenue: "DESC" },
        skip: 0,
        limit: 8
      },
      (data) => aggregateTraceCount(data.aggregateAccounts)
    ),
    runDashboardRequest<PipelineData>(
      client,
      "Pipeline by stage",
      "dashboard_pipeline_by_stage",
      AGGREGATE_OPPORTUNITIES,
      {
        filter: null,
        groupBy: [{ field: "stageId", alias: "stage_id" }],
        metrics: [
          { fn: "count", alias: "opportunity_count" },
          { fn: "sum", field: "amount", alias: "total_amount" },
          { fn: "avg", field: "amount", alias: "avg_amount" }
        ],
        having: { opportunity_count: { _gte: 1 } },
        sort: { total_amount: "DESC" },
        skip: 0,
        limit: 12
      },
      (data) => aggregateTraceCount(data.aggregateOpportunities)
    ),
    runDashboardRequest<StageListData>(
      client,
      "Stage lookup",
      "dashboard_stage_lookup",
      QUERY_OPPORTUNITY_STAGES,
      { skip: 0, limit: 50, sort: { sort_order: "ASC" } },
      (data) => connectionTraceCount(data.queryOpportunityStages)
    ),
    runDashboardRequest<LeadSummaryData>(
      client,
      "Lead conversion",
      "dashboard_lead_conversion",
      AGGREGATE_LEADS,
      {
        filter: null,
        groupBy: [{ field: "isConverted", alias: "is_converted" }],
        metrics: [{ fn: "count", alias: "lead_count" }],
        having: { lead_count: { _gte: 1 } },
        sort: { lead_count: "DESC" },
        skip: 0,
        limit: 2
      },
      (data) => aggregateTraceCount(data.aggregateLeads)
    ),
    runDashboardRequest<ActivitySummaryData>(
      client,
      "Activity status",
      "dashboard_activity_status",
      AGGREGATE_ACTIVITIES,
      {
        filter: null,
        groupBy: [{ field: "isClosed", alias: "is_closed" }],
        metrics: [{ fn: "count", alias: "activity_count" }],
        having: { activity_count: { _gte: 1 } },
        sort: { activity_count: "DESC" },
        skip: 0,
        limit: 2
      },
      (data) => aggregateTraceCount(data.aggregateActivities)
    ),
    runDashboardRequest<OpportunityListData>(
      client,
      "Top opportunities",
      "dashboard_top_opportunities",
      QUERY_TOP_OPPORTUNITIES,
      { skip: 0, limit: 25, sort: { amount: "DESC" } },
      (data) => connectionTraceCount(data.queryOpportunities)
    ),
    runDashboardRequest<LeadListData>(
      client,
      "Lead queue",
      "dashboard_lead_queue",
      QUERY_LEAD_QUEUE,
      {
        filter: { is_converted: { _eq: false } },
        skip: 0,
        limit: 7,
        sort: { annual_revenue: "DESC" }
      },
      (data) => connectionTraceCount(data.queryLeads)
    ),
    runDashboardRequest<ActivityListData>(
      client,
      "Open activities",
      "dashboard_activity_queue",
      QUERY_ACTIVITY_QUEUE,
      {
        filter: { is_closed: { _eq: false } },
        skip: 0,
        limit: 7,
        sort: { due_date: "ASC" }
      },
      (data) => connectionTraceCount(data.queryActivities)
    )
  ]);

  const settled = [
    accountSummary,
    accountsByState,
    pipelineByStage,
    stages,
    leadConversion,
    activitySummary,
    topOpportunities,
    leadQueue,
    activityQueue
  ];
  const warnings = settled.filter((request) => !request.ok).map((request) => `${request.label}: ${request.message}`);
  const traces = settled.flatMap((request) => (request.ok ? [request.trace] : []));

  if (!traces.length) {
    throw new Error(warnings[0] ?? "CRM analytics could not be loaded.");
  }

  const accountSummaryRow = firstAggregateRow(accountSummary, "aggregateAccounts");
  const stateRows = aggregateRows(accountsByState, "aggregateAccounts");
  const pipelineRows = aggregateRows(pipelineByStage, "aggregateOpportunities");
  const stageRecords = connectionItems<StageListData, OpportunityStageRecord>(stages, "queryOpportunityStages");
  const leadRows = aggregateRows(leadConversion, "aggregateLeads");
  const activityRows = aggregateRows(activitySummary, "aggregateActivities");
  const topOpportunityItems = connectionItems<OpportunityListData, OpportunityRecord>(
    topOpportunities,
    "queryOpportunities"
  )
    .filter((opportunity) => !opportunity.stage?.is_closed)
    .slice(0, 7);

  const stageById = new Map(stageRecords.map((stage) => [String(stage.id ?? ""), stage]));
  const pipelineStages = pipelineRows
    .map((row) => pipelineStageBucket(row, stageById.get(String(row.stage_id ?? ""))))
    .sort((left, right) => {
      if (left.sortOrder !== right.sortOrder) return left.sortOrder - right.sortOrder;
      return right.totalAmount - left.totalAmount;
    });

  const openPipelineStages = pipelineStages.filter((stage) => !stage.isClosed);
  const totalPipeline = sumBy(openPipelineStages, (stage) => stage.totalAmount);
  const opportunityCount = sumBy(openPipelineStages, (stage) => stage.opportunityCount);
  const leadCounts = leadConversionCounts(leadRows);
  const openActivityCount = activityStatusCount(activityRows, false);

  return {
    generatedAt: new Date().toISOString(),
    kpis: {
      accountCount: numberValue(accountSummaryRow.account_count),
      accountRevenue: numberValue(accountSummaryRow.total_revenue),
      avgAccountEmployees: numberValue(accountSummaryRow.avg_employees),
      openPipeline: totalPipeline,
      weightedPipeline: sumBy(openPipelineStages, (stage) => stage.weightedAmount),
      opportunityCount,
      avgDealSize: opportunityCount > 0 ? totalPipeline / opportunityCount : 0,
      openLeads: leadCounts.open,
      convertedLeads: leadCounts.converted,
      leadConversionRate: leadCounts.total > 0 ? leadCounts.converted / leadCounts.total : 0,
      openActivities: openActivityCount
    },
    accountStates: stateRows.map(accountStateBucket),
    pipelineStages,
    topOpportunities: topOpportunityItems,
    leadQueue: connectionItems<LeadListData, LeadRecord>(leadQueue, "queryLeads"),
    activityQueue: connectionItems<ActivityListData, ActivityRecord>(activityQueue, "queryActivities"),
    traces,
    warnings
  };
}

async function runDashboardRequest<TData>(
  client: DashboardClient,
  label: string,
  operationName: string,
  query: string,
  variables: Record<string, unknown>,
  traceCounts: (data: TData) => { queryCount: number; rowCount: number }
): Promise<SettledDashboardRequest<TData>> {
  try {
    const result = await client.graphql<TData, Record<string, unknown>>({
      schemaName: "crm",
      operationName,
      query,
      variables
    });
    const counts = traceCounts(result.data);
    return {
      ok: true,
      label,
      result,
      trace: {
        label,
        requestId: result.requestId,
        correlationId: result.correlationId,
        responseMs: result.responseMs,
        httpStatus: result.httpStatus,
        queryCount: counts.queryCount,
        rowCount: counts.rowCount
      }
    };
  } catch (error) {
    return {
      ok: false,
      label,
      message: error instanceof Error ? error.message : String(error)
    };
  }
}

function firstAggregateRow<TData>(
  request: SettledDashboardRequest<TData>,
  key: keyof TData
): AggregateRow {
  return aggregateRows(request, key)[0] ?? {};
}

function aggregateRows<TData>(request: SettledDashboardRequest<TData>, key: keyof TData): AggregateRow[] {
  if (!request.ok) return [];
  const connection = request.result.data[key] as AggregateConnection | undefined;
  return Array.isArray(connection?.items) ? connection.items : [];
}

function connectionItems<TData, TItem>(request: SettledDashboardRequest<TData>, key: keyof TData): TItem[] {
  if (!request.ok) return [];
  const connection = request.result.data[key] as QueryConnection<TItem> | undefined;
  return Array.isArray(connection?.items) ? connection.items : [];
}

function aggregateTraceCount(connection: AggregateConnection | undefined) {
  return {
    queryCount: numberValue(connection?.query_count),
    rowCount: Array.isArray(connection?.items) ? connection.items.length : 0
  };
}

function connectionTraceCount<TItem>(connection: QueryConnection<TItem> | undefined) {
  return {
    queryCount: numberValue(connection?.query_count),
    rowCount: Array.isArray(connection?.items) ? connection.items.length : 0
  };
}

function accountStateBucket(row: AggregateRow): AccountStateBucket {
  return {
    state: stringValue(row.state, "Unknown"),
    accountCount: numberValue(row.account_count),
    totalRevenue: numberValue(row.total_revenue),
    avgEmployees: numberValue(row.avg_employees)
  };
}

function pipelineStageBucket(row: AggregateRow, stage: OpportunityStageRecord | undefined): PipelineStageBucket {
  const totalAmount = numberValue(row.total_amount);
  const probability = normalizedProbability(stage?.probability);
  return {
    stageId: stringValue(row.stage_id, "unknown"),
    stageName: stage?.name ?? compactId(row.stage_id),
    opportunityCount: numberValue(row.opportunity_count),
    totalAmount,
    avgAmount: numberValue(row.avg_amount),
    probability,
    weightedAmount: totalAmount * probability,
    isClosed: Boolean(stage?.is_closed),
    isWon: Boolean(stage?.is_won),
    sortOrder: Number.isFinite(stage?.sort_order) ? Number(stage?.sort_order) : 999
  };
}

function normalizedProbability(value: unknown) {
  const probability = numberValue(value);
  if (probability > 1) return probability / 100;
  if (probability < 0) return 0;
  return probability;
}

function leadConversionCounts(rows: AggregateRow[]) {
  let converted = 0;
  let open = 0;
  for (const row of rows) {
    const count = numberValue(row.lead_count);
    if (Boolean(row.is_converted)) converted += count;
    else open += count;
  }
  return {
    converted,
    open,
    total: converted + open
  };
}

function activityStatusCount(rows: AggregateRow[], isClosed: boolean) {
  return rows
    .filter((row) => Boolean(row.is_closed) === isClosed)
    .reduce((total, row) => total + numberValue(row.activity_count), 0);
}

function sumBy<TItem>(items: TItem[], value: (item: TItem) => number) {
  return items.reduce((total, item) => total + value(item), 0);
}

function numberValue(value: unknown) {
  const parsed = typeof value === "number" ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function stringValue(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value : fallback;
}

function compactId(value: unknown) {
  const raw = stringValue(value, "Unknown");
  return raw.length > 12 ? raw.slice(0, 8) : raw;
}
