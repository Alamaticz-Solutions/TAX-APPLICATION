# GraphiQL Examples

This guide gives application developers paste-ready GraphQL examples for the
generated CRM model. It is meant for quick local exploration, demos, and agent
handoff.

## Start The Backend

For normal local exploration with the synthetic local admin user:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

For local data exploration that also bypasses Rego policy filters:

```bash
ENV_NAME=local APP_BYPASS_POLICIES_IN_LOCAL=true API_PORT=8080 scripts/appfw serve
```

Then open:

```text
http://127.0.0.1:8080/crm
```

## Headers

In GraphiQL, open the headers panel and paste JSON headers.

For local development with `ENV_NAME=local`, an `Authorization` header is not
required. The backend creates a synthetic `local-dev` admin user. You can still
send a timezone header:

```json
{
  "x-timezone": "America/Denver"
}
```

For authenticated environments, add a provider-issued or local test JWT:

```json
{
  "Authorization": "Bearer <jwt>",
  "x-timezone": "America/Denver"
}
```

Do not paste real tokens into committed docs, screenshots, tickets, or prompts.
Keep local token files ignored.

## Naming Rules

Generated operation names are GraphQL-style camelCase:

```text
getAccounts
queryAccounts
findAccount
createAccount
updateAccount
deleteAccount
```

Generated model fields are snake_case:

```text
industry_id
billing_city
annual_revenue
number_of_employees
```

The variable panel must contain valid JSON. Do not use YAML, comments, or
trailing commas.

When an operation declares variables such as `$skip` and `$limit`, paste the
matching JSON into GraphiQL's **Variables** panel before running it. If the
variables panel is empty, stale, or the operation signature was only partially
pasted, GraphiQL may report messages such as `Variable "skip" is not defined`
or `Variable "$skip" of required type "Int!" was not provided`.

Generated query and aggregate APIs also allow `skip` and `limit` to be omitted.
The backend then applies `APP_QUERY_DEFAULT_PAGE_SIZE` and caps explicit limits
with `APP_QUERY_MAX_PAGE_SIZE`.

Common filter operators include:

```text
_eq, _ne, _lt, _lte, _gt, _gte, _contains, _starts, _ends, _in, _not_in
```

Use `_and` and `_or` for conjunctions.

## Query: First Page Of Accounts

Paste this in the operation editor:

```graphql
query AccountList($skip: Int!, $limit: Int!) {
  queryAccounts(skip: $skip, limit: $limit) {
    query_count
    page_count
    next_cursor
    previous_cursor
    items {
      id
      name
      website
      billing_city
      billing_state
      annual_revenue
      industry {
        id
        name
      }
      version
    }
  }
}
```

Paste this in the variables panel:

```json
{
  "skip": 0,
  "limit": 5
}
```

Use `next_cursor` as the opaque `after` value for keyset-style pagination:

```graphql
query AccountListAfter($limit: Int!, $after: String) {
  queryAccounts(limit: $limit, after: $after) {
    next_cursor
    items {
      id
      name
    }
  }
}
```

## Query: Filter And Sort Accounts

This returns California accounts sorted by annual revenue.

```graphql
query FilteredAccounts(
  $filter: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  queryAccounts(filter: $filter, sort: $sort, skip: $skip, limit: $limit) {
    query_count
    items {
      id
      name
      billing_city
      billing_state
      annual_revenue
      industry {
        name
      }
    }
  }
}
```

```json
{
  "filter": {
    "billing_state": {
      "_eq": "CA"
    }
  },
  "sort": {
    "annualRevenue": "DESC"
  },
  "skip": 0,
  "limit": 10
}
```

## Query: Find One Account

This uses a seeded account ID.

```graphql
query FindAccount($id: String!) {
  findAccount(id: $id) {
    id
    name
    website
    billing_city
    billing_state
    industry_id
    industry {
      id
      name
      sort_order
    }
    version
  }
}
```

```json
{
  "id": "a0c00000-0000-4000-8000-100000000001"
}
```

## Query: Leads With Related Lookup Data

This shows nested `NavToOne` relationships for lead status, source, and
industry.

```graphql
query HealthcareLeads($filter: JSON, $skip: Int!, $limit: Int!) {
  queryLeads(filter: $filter, skip: $skip, limit: $limit) {
    query_count
    items {
      id
      first_name
      last_name
      company
      email
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
      converted_account {
        id
        name
      }
    }
  }
}
```

```json
{
  "filter": {
    "_and": [
      {
        "industry_id": {
          "_eq": "10100070-0000-4000-8000-100000000001"
        }
      },
      {
        "is_converted": {
          "_eq": false
        }
      }
    ]
  },
  "skip": 0,
  "limit": 5
}
```

## Query: Opportunities With Account And Stage

```graphql
query LargeOpportunities(
  $filter: JSON
  $sort: JSON
  $skip: Int!
  $limit: Int!
) {
  queryOpportunities(filter: $filter, sort: $sort, skip: $skip, limit: $limit) {
    query_count
    items {
      id
      name
      amount
      close_date
      account {
        id
        name
      }
      stage {
        id
        name
        sort_order
      }
    }
  }
}
```

```json
{
  "filter": {
    "amount": {
      "_gte": 100000
    }
  },
  "sort": {
    "amount": "DESC"
  },
  "skip": 0,
  "limit": 5
}
```

## Aggregation Notes

Generated aggregate operations follow the same model-driven naming pattern:

```text
aggregateAccounts
aggregateOpportunities
aggregateLeads
```

Aggregate rows are dynamic JSON because the shape depends on the selected
groups and metrics. In GraphiQL, select `items` directly. Do not try to select
typed subfields under `items`.

Aggregate inputs:

| Input | Purpose |
| --- | --- |
| `groupBy` | Array of property names or `{ "field": "...", "alias": "..." }` objects. |
| `metrics` | Array of metric objects with `fn`, optional `field`, and `alias`. |
| `having` | Filter-like object keyed by metric aliases. |
| `sort` | Object keyed by group or metric aliases. |

Supported metric functions are:

```text
count, count_distinct, sum, avg, min, max
```

Field names in `groupBy` and `metrics` are resolved against configured model
properties. These examples use camelCase field names such as `annualRevenue`
and `billingState`; snake_case property names also resolve to the same model
properties. `having` and `sort` reference output aliases, not source fields.

## Aggregate: Accounts By Billing State

This groups accounts by billing state, then calculates count, total revenue,
and average employee count.

```graphql
query AccountsByState(
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
    page_count
    page_index
    items
  }
}
```

```json
{
  "filter": null,
  "groupBy": [
    {
      "field": "billingState",
      "alias": "state"
    }
  ],
  "metrics": [
    {
      "fn": "count",
      "alias": "account_count"
    },
    {
      "fn": "sum",
      "field": "annualRevenue",
      "alias": "total_revenue"
    },
    {
      "fn": "avg",
      "field": "numberOfEmployees",
      "alias": "avg_employees"
    }
  ],
  "having": {
    "account_count": {
      "_gte": 1
    }
  },
  "sort": {
    "total_revenue": "DESC"
  },
  "skip": 0,
  "limit": 10
}
```

Expected `items` rows look like this:

```json
[
  {
    "state": "CA",
    "account_count": 21,
    "total_revenue": 699750000,
    "avg_employees": 113.61904761904762
  }
]
```

## Aggregate: Account Revenue Summary

When `groupBy` is `null`, the result is a single summary row for the filtered
record set.

```graphql
query AccountRevenueSummary(
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
}
```

```json
{
  "filter": {
    "annual_revenue": {
      "_gte": 1000000
    }
  },
  "groupBy": null,
  "metrics": [
    {
      "fn": "count",
      "alias": "account_count"
    },
    {
      "fn": "sum",
      "field": "annualRevenue",
      "alias": "total_revenue"
    },
    {
      "fn": "min",
      "field": "annualRevenue",
      "alias": "min_revenue"
    },
    {
      "fn": "max",
      "field": "annualRevenue",
      "alias": "max_revenue"
    },
    {
      "fn": "count_distinct",
      "field": "billingCity",
      "alias": "city_count"
    }
  ],
  "having": null,
  "sort": null,
  "skip": 0,
  "limit": 1
}
```

## Aggregate: Opportunity Pipeline By Stage

This groups opportunities by stage ID and summarizes pipeline value. Use
`queryOpportunities` if you want to resolve the stage IDs to names in the same
exploration session.

```graphql
query PipelineByStage(
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
}
```

```json
{
  "filter": {
    "amount": {
      "_gte": 10000
    }
  },
  "groupBy": [
    {
      "field": "stageId",
      "alias": "stage_id"
    }
  ],
  "metrics": [
    {
      "fn": "count",
      "alias": "opportunity_count"
    },
    {
      "fn": "sum",
      "field": "amount",
      "alias": "total_amount"
    },
    {
      "fn": "avg",
      "field": "amount",
      "alias": "avg_amount"
    }
  ],
  "having": {
    "total_amount": {
      "_gte": 100000
    }
  },
  "sort": {
    "total_amount": "DESC"
  },
  "skip": 0,
  "limit": 10
}
```

## Mutation: Create Industry

This creates a lookup row. Change the name if you run it multiple times and
want easy-to-read results.

```graphql
mutation CreateIndustry($input: InputIndustry!) {
  createIndustry(input: $input) {
    id
    name
    description
    sort_order
    version
  }
}
```

```json
{
  "input": {
    "name": "GraphiQL Example Industry",
    "description": "Created from GraphiQL",
    "sort_order": 999,
    "version": 0
  }
}
```

## Mutation: Create Account

This creates an account linked to the seeded Healthcare industry.

```graphql
mutation CreateAccount($input: InputAccount!) {
  createAccount(input: $input) {
    id
    name
    website
    phone
    email
    billing_city
    billing_state
    annual_revenue
    number_of_employees
    industry_id
    industry {
      id
      name
    }
    version
  }
}
```

```json
{
  "input": {
    "name": "GraphiQL Example Account",
    "website": "https://graphiql-example.example.com",
    "phone": "555-0199",
    "email": "hello@graphiql-example.example.com",
    "billing_street": "100 Demo Way",
    "billing_city": "Irvine",
    "billing_state": "CA",
    "billing_postal_code": "92618",
    "billing_country": "US",
    "annual_revenue": 1500000.0,
    "number_of_employees": 25,
    "description": "Created from GraphiQL",
    "industry_id": "10100070-0000-4000-8000-100000000001",
    "version": 0
  }
}
```

## Mutation: Create Lead

This creates a lead linked to seeded LeadStatus, LeadSource, and Industry rows.

```graphql
mutation CreateLead($input: InputLead!) {
  createLead(input: $input) {
    id
    first_name
    last_name
    company
    email
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
    version
  }
}
```

```json
{
  "input": {
    "first_name": "Avery",
    "last_name": "Lopez",
    "company": "GraphiQL Lead Co.",
    "email": "avery.lopez@example.com",
    "phone": "555-0210",
    "title": "Director of IT",
    "website": "https://graphiql-lead.example.com",
    "description": "Created from GraphiQL",
    "annual_revenue": 2500000.0,
    "number_of_employees": 28,
    "is_converted": false,
    "status_id": "1eadcc00-0000-4000-8000-100000000001",
    "source_id": "1eadcc01-0000-4000-8000-100000000001",
    "industry_id": "10100070-0000-4000-8000-100000000002",
    "version": 0
  }
}
```

## Notes On Updates And Deletes

Generated update and delete mutations use optimistic concurrency through the
`version` field. For now, prefer create/query examples in GraphiQL and use the
generated API tests for update/delete coverage. Some providers return large
version values that are awkward to round-trip through GraphiQL's JavaScript
editor without losing integer precision.

## Seeded Lookup IDs

Useful seeded IDs for examples:

| Entity | Name | ID |
| --- | --- | --- |
| Industry | Healthcare | `10100070-0000-4000-8000-100000000001` |
| Industry | Technology | `10100070-0000-4000-8000-100000000002` |
| LeadStatus | New | `1eadcc00-0000-4000-8000-100000000001` |
| LeadStatus | Qualified | `1eadcc00-0000-4000-8000-100000000003` |
| LeadSource | Web | `1eadcc01-0000-4000-8000-100000000001` |
| LeadSource | Partner Referral | `1eadcc01-0000-4000-8000-100000000003` |
| OpportunityStage | Proposal | `0998a900-0000-4000-8000-100000000003` |
| Account | Acme Dental Group | `a0c00000-0000-4000-8000-100000000001` |
