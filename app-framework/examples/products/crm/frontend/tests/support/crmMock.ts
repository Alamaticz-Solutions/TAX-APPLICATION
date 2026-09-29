import type { Page, Route } from "@playwright/test";

type GraphqlRequest = {
  query: string;
  variables?: Record<string, unknown>;
};

export type CapturedGraphqlRequest = {
  operation: string;
  variables: Record<string, unknown>;
  headers: Record<string, string>;
};

type MockRecord = Record<string, unknown>;

const requestHeaders = {
  "content-type": "application/json",
  "x-request-id": "playwright-request",
  "x-correlation-id": "playwright-correlation"
};

const industries = [
  {
    id: "ind-health",
    record_locator: "rl_industry_health",
    name: "Healthcare"
  },
  {
    id: "ind-technology",
    record_locator: "rl_industry_technology",
    name: "Technology"
  }
];

const activityTypes = [
  {
    id: "atype-call",
    record_locator: "rl_activity_type_call",
    name: "Call"
  },
  {
    id: "atype-demo",
    record_locator: "rl_activity_type_demo",
    name: "Demo"
  }
];

const accounts = [
  {
    id: "acct-001",
    record_locator: "rl_account_001",
    name: "Acme Analytics",
    website: "https://acme.example",
    phone: "555-3021",
    email: "info@acme.example",
    tags: "strategic,health",
    billing_street: "100 Market Street",
    billing_city: "San Diego",
    billing_state: "CA",
    billing_postal_code: "92101",
    billing_country: "US",
    annual_revenue: 12_500_000,
    health_score: 86,
    health_last_refreshed_at: "2026-06-04T12:00:00.000Z",
    number_of_employees: 420,
    description: "Enterprise analytics account.",
    industry_id: "ind-health",
    industry: industries[0],
    version: 3
  },
  {
    id: "acct-002",
    record_locator: "rl_account_002",
    name: "Northstar Manufacturing",
    website: "https://northstar.example",
    phone: "555-4477",
    email: "ops@northstar.example",
    tags: "manufacturing",
    billing_street: "42 Industrial Way",
    billing_city: "Phoenix",
    billing_state: "AZ",
    billing_postal_code: "85001",
    billing_country: "US",
    annual_revenue: 8_250_000,
    health_score: 72,
    health_last_refreshed_at: "2026-06-04T12:00:00.000Z",
    number_of_employees: 260,
    description: "Regional manufacturing account.",
    industry_id: "ind-technology",
    industry: industries[1],
    version: 2
  }
];

const opportunityStages = [
  {
    id: "stage-qualification",
    record_locator: "rl_stage_qualification",
    name: "Qualification",
    probability: 0.25,
    is_closed: false,
    is_won: false,
    sort_order: 1
  },
  {
    id: "stage-proposal",
    record_locator: "rl_stage_proposal",
    name: "Proposal",
    probability: 0.65,
    is_closed: false,
    is_won: false,
    sort_order: 2
  },
  {
    id: "stage-won",
    record_locator: "rl_stage_won",
    name: "Closed Won",
    probability: 1,
    is_closed: true,
    is_won: true,
    sort_order: 3
  }
];

const opportunities = [
  {
    id: "opp-001",
    record_locator: "rl_opportunity_001",
    name: "Acme rollout",
    amount: 1_200_000,
    close_date: "2026-07-15",
    next_step: "Finalize implementation plan",
    lead_source: "Partner",
    account_id: "acct-001",
    account: { id: "acct-001", record_locator: "rl_account_001", name: "Acme Analytics" },
    stage_id: "stage-proposal",
    stage: opportunityStages[1],
    version: 1
  },
  {
    id: "opp-002",
    record_locator: "rl_opportunity_002",
    name: "Northstar expansion",
    amount: 650_000,
    close_date: "2026-08-03",
    next_step: "Security review",
    lead_source: "Web",
    account_id: "acct-002",
    account: { id: "acct-002", record_locator: "rl_account_002", name: "Northstar Manufacturing" },
    stage_id: "stage-qualification",
    stage: opportunityStages[0],
    version: 1
  }
];

const leads = [
  {
    id: "lead-001",
    record_locator: "rl_lead_001",
    first_name: "Maya",
    last_name: "Chen",
    company: "GreenFields",
    email: "maya@example.test",
    annual_revenue: 2_500_000,
    is_converted: false,
    status: { id: "lead-status-open", name: "Open" },
    source: { id: "lead-source-web", name: "Web" },
    industry: industries[0],
    version: 1
  },
  {
    id: "lead-002",
    record_locator: "rl_lead_002",
    first_name: "Riley",
    last_name: "Patel",
    company: "MediSystems",
    email: "riley@example.test",
    annual_revenue: 900_000,
    is_converted: true,
    status: { id: "lead-status-converted", name: "Converted" },
    source: { id: "lead-source-partner", name: "Partner" },
    industry: industries[1],
    version: 1
  }
];

const activities = [
  {
    id: "act-001",
    record_locator: "rl_activity_001",
    subject: "Discovery Call - Acme Analytics",
    description: "Initial discovery call to confirm stakeholder goals.",
    activity_date: "2026-05-24",
    due_date: "2026-05-24",
    is_closed: false,
    priority: "High",
    status: "Open",
    type_id: "atype-call",
    activity_type: activityTypes[0],
    account_id: "acct-001",
    account: { id: "acct-001", record_locator: "rl_account_001", name: "Acme Analytics" },
    version: 5
  },
  {
    id: "act-002",
    record_locator: "rl_activity_002",
    subject: "Demo - Northstar Manufacturing",
    description: "Show account dashboard and operating telemetry.",
    activity_date: "2026-05-29",
    due_date: "2026-05-30",
    is_closed: true,
    priority: "Normal",
    status: "Completed",
    type_id: "atype-demo",
    activity_type: activityTypes[1],
    account_id: "acct-002",
    account: { id: "acct-002", record_locator: "rl_account_002", name: "Northstar Manufacturing" },
    version: 2
  }
];

export async function installCrmApiMock(page: Page, options: { delayMs?: number } = {}) {
  const captured: CapturedGraphqlRequest[] = [];
  const state = {
    accounts: accounts.map((row) => ({ ...row })),
    activities: activities.map((row) => ({ ...row }))
  };

  await page.route("**/crm", async (route) => {
    const request = route.request();
    const body = (request.postDataJSON() ?? {}) as GraphqlRequest;
    const variables = body.variables ?? {};
    const operation = operationName(body.query);
    captured.push({
      operation,
      variables,
      headers: request.headers()
    });

    if (options.delayMs && options.delayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, options.delayMs));
    }

    await route.fulfill({
      status: 200,
      headers: requestHeaders,
      body: JSON.stringify({ data: dataForQuery(body.query, variables, state) })
    });
  });

  return {
    requests: captured,
    requestsFor: (operation: string) => captured.filter((request) => request.operation === operation),
    latestFor: (operation: string) => captured.filter((request) => request.operation === operation).at(-1)
  };
}

function dataForQuery(query: string, variables: Record<string, unknown>, state: { accounts: MockRecord[]; activities: MockRecord[] }) {
  if (query.includes("accountHealth(")) {
    return { accountHealth: accountHealthPayload() };
  }
  if (query.includes("refreshAccountHealthStoredProcedure(")) {
    return { refreshAccountHealthStoredProcedure: refreshAccountHealthStoredProcedure(state.accounts, variables) };
  }
  if (query.includes("aggregateAccounts(")) {
    return { aggregateAccounts: aggregateAccounts(variables) };
  }
  if (query.includes("aggregateOpportunities(")) {
    return { aggregateOpportunities: aggregateOpportunities() };
  }
  if (query.includes("aggregateLeads(")) {
    return { aggregateLeads: aggregateLeads() };
  }
  if (query.includes("aggregateActivities(")) {
    return { aggregateActivities: aggregateActivities() };
  }
  if (query.includes("queryAccounts(")) {
    return { queryAccounts: connection(filterRows(state.accounts, variables), variables) };
  }
  if (query.includes("findAccountByLocator(") || query.includes("findAccount(")) {
    return { [query.includes("findAccountByLocator(") ? "findAccountByLocator" : "findAccount"]: findRecord(state.accounts, variables) };
  }
  if (query.includes("updateAccount(")) {
    const updated = updateAccount(state.accounts, variables);
    return { updateAccount: updated };
  }
  if (query.includes("deleteAccount(")) {
    return { deleteAccount: 1 };
  }
  if (query.includes("queryActivities(")) {
    return { queryActivities: connection(filterRows(state.activities, variables), variables) };
  }
  if (query.includes("findActivityByLocator(") || query.includes("findActivity(")) {
    return { [query.includes("findActivityByLocator(") ? "findActivityByLocator" : "findActivity"]: findRecord(state.activities, variables) };
  }
  if (query.includes("queryActivityTypes(")) {
    return { queryActivityTypes: connection(activityTypes, variables) };
  }
  if (query.includes("queryIndustries(")) {
    return { queryIndustries: connection(industries, variables) };
  }
  if (query.includes("queryOpportunityStages(")) {
    return { queryOpportunityStages: connection(opportunityStages, variables) };
  }
  if (query.includes("queryOpportunities(")) {
    return { queryOpportunities: connection(opportunities, variables) };
  }
  if (query.includes("queryLeads(")) {
    return { queryLeads: connection(leads, variables) };
  }

  const fallback = query.match(/\b(query[A-Z][A-Za-z0-9_]*)\s*\(/)?.[1];
  return fallback ? { [fallback]: connection([], variables) } : {};
}

function operationName(query: string) {
  return query.match(/\b(?:query|mutation)\s+([A-Za-z0-9_]+)/)?.[1] ?? "anonymous";
}

function connection(rows: readonly MockRecord[], variables: Record<string, unknown>) {
  const skip = numberValue(variables.skip, 0);
  const limit = numberValue(variables.limit, 25);
  const pageRows = rows.slice(skip, skip + limit);
  return {
    date_time: new Date("2026-06-04T12:00:00.000Z").toISOString(),
    request_duration: 9,
    skip,
    limit,
    page_count: rows.length ? Math.ceil(rows.length / limit) : 0,
    page_index: limit ? Math.floor(skip / limit) : 0,
    query_count: rows.length,
    next_cursor: skip + limit < rows.length ? `cursor-${skip + limit}` : null,
    previous_cursor: skip > 0 ? `cursor-${Math.max(0, skip - limit)}` : null,
    items: pageRows
  };
}

function filterRows(rows: readonly MockRecord[], variables: Record<string, unknown>) {
  const filter = JSON.stringify(variables.filter ?? {}).toLowerCase();
  if (!filter || filter === "{}" || filter === "null") return rows;
  if (filter.includes("acct-001")) return rows.filter((row) => String(row.account_id ?? row.id ?? "") === "acct-001");
  if (filter.includes("acct-002")) return rows.filter((row) => String(row.account_id ?? row.id ?? "") === "acct-002");
  if (filter.includes("atype-call")) return rows.filter((row) => String(row.type_id ?? row.id ?? "") === "atype-call");
  if (filter.includes("ca")) return rows.filter((row) => String(row.billing_state ?? "").toLowerCase() === "ca");
  if (filter.includes("north")) return rows.filter((row) => JSON.stringify(row).toLowerCase().includes("north"));
  if (filter.includes("acme")) return rows.filter((row) => JSON.stringify(row).toLowerCase().includes("acme"));
  return rows;
}

function findRecord(rows: readonly MockRecord[], variables: Record<string, unknown>) {
  const lookup = String(variables.locator ?? variables.id ?? "");
  return rows.find((row) => row.record_locator === lookup || row.id === lookup) ?? null;
}

function updateAccount(rows: MockRecord[], variables: Record<string, unknown>) {
  const input = (variables.input ?? {}) as MockRecord;
  const target = rows.find((row) => row.id === input.id || row.record_locator === input.record_locator) ?? rows[0];
  Object.assign(target, input, {
    id: String(input.id ?? target.id ?? "acct-001"),
    record_locator: String(input.record_locator ?? target.record_locator ?? "rl_account_001"),
    version: numberValue(target.version, 1) + 1
  });
  return target;
}

function refreshAccountHealthStoredProcedure(rows: MockRecord[], variables: Record<string, unknown>) {
  const accountId = String(variables.accountId ?? "");
  const healthScore = numberValue(variables.healthScore, 0);
  const target = rows.find((row) => row.id === accountId || row.record_locator === accountId) ?? rows[0];
  const refreshedAt = "2026-06-04T12:05:00.000Z";
  Object.assign(target, {
    health_score: healthScore,
    health_last_refreshed_at: refreshedAt,
    version: numberValue(target.version, 1) + 1
  });
  return {
    account_id: String(target.id ?? accountId),
    health_score: healthScore,
    refreshed_at: refreshedAt,
    source: "playwright-stored-procedure"
  };
}

function aggregateAccounts(variables: Record<string, unknown>) {
  const grouped = JSON.stringify(variables.groupBy ?? "").includes("billingState");
  return {
    query_count: grouped ? 2 : 1,
    items: grouped
      ? [
          { state: "CA", account_count: 1, total_revenue: 12_500_000, avg_employees: 420 },
          { state: "AZ", account_count: 1, total_revenue: 8_250_000, avg_employees: 260 }
        ]
      : [{ account_count: 2, total_revenue: 20_750_000, avg_employees: 340, city_count: 2, max_revenue: 12_500_000 }]
  };
}

function aggregateOpportunities() {
  return {
    query_count: 2,
    items: [
      { stage_id: "stage-proposal", opportunity_count: 1, total_amount: 1_200_000, avg_amount: 1_200_000 },
      { stage_id: "stage-qualification", opportunity_count: 1, total_amount: 650_000, avg_amount: 650_000 }
    ]
  };
}

function aggregateLeads() {
  return {
    query_count: 2,
    items: [
      { is_converted: false, lead_count: 1 },
      { is_converted: true, lead_count: 1 }
    ]
  };
}

function aggregateActivities() {
  return {
    query_count: 2,
    items: [
      { is_closed: false, activity_count: 1 },
      { is_closed: true, activity_count: 1 }
    ]
  };
}

function accountHealthPayload() {
  return {
    account: {
      id: "acct-001",
      name: "Acme Analytics",
      website: "https://acme.example",
      annual_revenue: 12_500_000,
      number_of_employees: 420,
      location: { city: "San Diego", state: "CA", country: "US" }
    },
    health: {
      score: 86,
      grade: "A",
      risk_level: "low",
      summary: "Strong engagement, active pipeline, and complete profile data.",
      generated_at: "2026-06-04T12:00:00.000Z"
    },
    stored_snapshot: {
      account_id: "acct-001",
      health_score: 86,
      refreshed_at: "2026-06-04T12:00:00.000Z",
      source: "crm.refresh_account_health_stored_procedure"
    },
    financials: {
      annual_revenue: 12_500_000,
      pipeline_value: 1_200_000,
      weighted_pipeline_value: 780_000,
      quote_value: 300_000,
      open_quote_value: 250_000
    },
    opportunities: {
      total_count: 2,
      open_count: 1,
      won_count: 1,
      closed_lost_count: 0,
      stage_distribution: [{ stage: "Proposal", count: 1, amount: 1_200_000 }],
      top_open: opportunities.slice(0, 1)
    },
    quotes: {
      total_count: 2,
      open_count: 1,
      expired_count: 0,
      largest_open: {
        id: "quote-001",
        name: "Acme enterprise quote",
        quote_number: "Q-1001",
        total_price: 250_000,
        expiration_date: "2026-07-01",
        status: "Open"
      }
    },
    activities: {
      total_count: 2,
      open_count: 1,
      overdue_count: 0,
      last_activity_date: "2026-05-24",
      next_due_date: "2026-06-10",
      stale_days: 3,
      engagement_status: "active"
    },
    contacts: {
      total_count: 4,
      complete_contact_count: 3,
      missing_email_count: 1
    },
    data_quality: {
      profile_completeness: 92,
      fields: [
        { name: "Website", complete: true },
        { name: "Billing address", complete: true },
        { name: "Primary contact", complete: true }
      ]
    },
    signals: [{ category: "Engagement", severity: "low", label: "Recent activity", detail: "Discovery call completed recently." }],
    recommended_actions: [{ priority: "high", label: "Schedule proposal review", detail: "Move the open opportunity toward negotiation." }],
    analysis: { sample_limit: 50, note: "Playwright deterministic account health fixture." }
  };
}

function numberValue(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}
