import { Routes, Route, useParams, Link, useSearchParams } from "react-router";
import { AppShell } from "./AppShell";
import { crmUiContract, type AppfwUiEntityContract } from "../generated/appfw-ui-contract";
import { EntityListView } from "../scaffold/EntityListView";
import { DashboardScreen } from "../features/dashboard/DashboardScreen";
import { AccountsScreen } from "../features/accounts/AccountsScreen";
import { AccountHealthDashboard } from "../features/accounts/AccountHealthDashboard";
import { PipelineScreen } from "../features/pipeline/PipelineScreen";
import { ActivitiesScreen } from "../features/activities/ActivitiesScreen";
import { AuditScreen } from "../features/audit/AuditScreen";
import { PageHeader, StateView } from "../components/ui";
import type { AppfwRecord } from "../lib/appfwClient";
import { resolveNewRecordDraftRef } from "../scaffold/routeIdentity";

export function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<DashboardScreen />} />
        {/* Human-owned, opinionated workflow screens (ADR 0009). */}
        <Route path="accounts" element={<AccountsScreen />} />
        <Route path="pipeline" element={<PipelineScreen />} />
        <Route path="activities" element={<ActivitiesScreen />} />
        <Route path="audit" element={<AuditScreen />} />
        {/* Generic model-driven fallback for any entity (ADR 0007). */}
        <Route path="data/:routeSegment/:recordId?" element={<EntityDataRoute />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  );
}

function EntityDataRoute() {
  const { routeSegment, recordId } = useParams();
  const [searchParams] = useSearchParams();
  const entity = crmUiContract.entities.find((candidate) => candidate.routeSegment === routeSegment);
  if (!entity) return <NotFound />;
  const isNew = recordId === "new";
  const initialDraft = isNew ? searchParamsToRecord(entity, searchParams) : undefined;
  return (
    <>
      <PageHeader
        title={entity.caption.plural}
        subtitle={`Generic model-driven view · ${entity.schemaName}.${entity.typeName}`}
      />
      <EntityListView
        entity={entity}
        initialMode={isNew ? "new" : recordId ? "record" : "list"}
        initialRecordId={!isNew && recordId ? recordId : null}
        initialDraft={initialDraft}
        recordInsight={accountRecordInsight(entity)}
      />
    </>
  );
}

function accountRecordInsight(entity: AppfwUiEntityContract) {
  if (entity.schemaName !== "crm" || entity.typeName !== "Account") return undefined;
  return {
    label: "Account dashboard",
    formLabel: "Account form",
    render: (recordKey: string) => <AccountHealthDashboard accountId={recordKey} />
  };
}

function searchParamsToRecord(entity: AppfwUiEntityContract, searchParams: URLSearchParams): AppfwRecord {
  const draft = resolveNewRecordDraftRef(entity, searchParams.get("draft"));
  if (draft) return draft;

  const record: AppfwRecord = {};
  searchParams.forEach((value, key) => {
    if (key === "draft") return;
    record[key] = value;
  });
  return record;
}

function NotFound() {
  return (
    <>
      <PageHeader title="Not found" />
      <StateView kind="error" title="No such screen" detail="Check the URL or return to the dashboard." />
      <p className="mt-4 text-sm">
        <Link to="/" style={{ color: "var(--pds-color-brand-blue-deep)" }}>
          ← Back to dashboard
        </Link>
      </p>
    </>
  );
}
