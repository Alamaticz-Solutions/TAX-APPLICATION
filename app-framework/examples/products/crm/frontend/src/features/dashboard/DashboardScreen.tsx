import { type ReactNode, useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import {
  Gauge,
  RefreshCw,
  Target,
  TrendingUp,
  Users
} from "lucide-react";
import { Badge, ChartShell, InlineAlert, PageHeader, StateView } from "../../components/ui";
import { useAppfwClient } from "../../app/providers";
import {
  loadCrmDashboardAnalytics,
  type CrmDashboardAnalytics
} from "./dashboardAnalytics";
import {
  ActivityQueue,
  DashboardSkeleton,
  KpiCard,
  LeadConversionDonut,
  LeadQueue,
  OpportunityTable,
  PipelineChart,
  RevenueBars,
  TelemetryPanel,
  formatCurrency,
  formatNumber,
  formatPercent,
  formatTime,
  traceSummary
} from "./DashboardWidgets";

export function DashboardScreen() {
  const client = useAppfwClient();
  const [analytics, setAnalytics] = useState<CrmDashboardAnalytics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    loadCrmDashboardAnalytics(client)
      .then((result) => {
        if (!active) return;
        setAnalytics(result);
      })
      .catch((caught: unknown) => {
        if (!active) return;
        setError(caught instanceof Error ? caught.message : String(caught));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [client, refreshKey]);

  const telemetry = useMemo(() => traceSummary(analytics?.traces ?? []), [analytics]);

  return (
    <>
      <PageHeader
        title="CRM Command Center"
        subtitle="Live pipeline, account revenue, lead conversion, work queue, and request telemetry."
        actions={
          <button
            type="button"
            className="crm-button secondary"
            onClick={() => setRefreshKey((value) => value + 1)}
            disabled={loading}
          >
            <RefreshCw size={15} />
            Refresh
          </button>
        }
      />

      {error && !analytics ? (
        <StateView kind="error" title="Could not load CRM dashboard" detail={error} />
      ) : null}

      {!analytics && loading ? <DashboardSkeleton /> : null}

      {analytics ? (
        <div className="crm-dashboard">
          <div className="crm-dashboard-status">
            <Badge tone="success">Live CRM data</Badge>
            <span>{analytics.traces.length} requests</span>
            <span>{Math.round(telemetry.avgMs)} ms avg</span>
            <span>Updated {formatTime(analytics.generatedAt)}</span>
          </div>

          {analytics.warnings.length ? (
            <InlineAlert
              tone="warning"
              title="Dashboard warnings"
              detail={analytics.warnings.slice(0, 2).join(" | ")}
            />
          ) : null}

          <section className="crm-kpi-grid" aria-label="CRM key performance indicators">
            <KpiCard
              title="Open pipeline"
              value={formatCurrency(analytics.kpis.openPipeline)}
              detail={`${formatNumber(analytics.kpis.opportunityCount)} open opportunities`}
              icon={<Target size={18} />}
              tone="blue"
            />
            <KpiCard
              title="Weighted pipeline"
              value={formatCurrency(analytics.kpis.weightedPipeline)}
              detail={`${formatCurrency(analytics.kpis.avgDealSize)} average deal size`}
              icon={<TrendingUp size={18} />}
              tone="teal"
            />
            <KpiCard
              title="Account revenue"
              value={formatCurrency(analytics.kpis.accountRevenue)}
              detail={`${formatNumber(analytics.kpis.accountCount)} accounts`}
              icon={<Gauge size={18} />}
              tone="green"
            />
            <KpiCard
              title="Lead conversion"
              value={formatPercent(analytics.kpis.leadConversionRate)}
              detail={`${formatNumber(analytics.kpis.openLeads)} open leads`}
              icon={<Users size={18} />}
              tone="gold"
            />
          </section>

          <section className="crm-dashboard-grid">
            <ChartShell
              className="crm-dashboard-panel crm-panel-wide"
              subtitle="Pipeline"
              title="Stage value and probability"
              actions={<DashboardEntityCaption to="/data/pipeline">Opportunities</DashboardEntityCaption>}
            >
              <PipelineChart stages={analytics.pipelineStages} />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel"
              subtitle="Conversion"
              title="Lead movement"
              actions={<DashboardEntityCaption to="/data/leads">Leads</DashboardEntityCaption>}
            >
              <LeadConversionDonut
                converted={analytics.kpis.convertedLeads}
                open={analytics.kpis.openLeads}
                rate={analytics.kpis.leadConversionRate}
              />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel"
              subtitle="Territory"
              title="Revenue by state"
              actions={<DashboardEntityCaption to="/data/accounts">Accounts</DashboardEntityCaption>}
            >
              <RevenueBars buckets={analytics.accountStates} />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel crm-panel-wide"
              subtitle="Active deals"
              title="Largest active deals"
              actions={<DashboardEntityCaption to="/data/pipeline">Opportunities</DashboardEntityCaption>}
            >
              <OpportunityTable opportunities={analytics.topOpportunities} />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel"
              subtitle="Lead queue"
              title="High-value prospects"
              actions={<DashboardEntityCaption to="/data/leads">Leads</DashboardEntityCaption>}
            >
              <LeadQueue leads={analytics.leadQueue} />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel"
              subtitle="Follow-up work"
              title="Open follow-up work"
              actions={<DashboardEntityCaption to="/data/activities">Activities</DashboardEntityCaption>}
            >
              <ActivityQueue activities={analytics.activityQueue} openCount={analytics.kpis.openActivities} />
            </ChartShell>

            <ChartShell
              className="crm-dashboard-panel crm-panel-wide"
              subtitle="Observability"
              title="Dashboard request profile"
            >
              <TelemetryPanel traces={analytics.traces} />
            </ChartShell>
          </section>
        </div>
      ) : null}
    </>
  );
}

function DashboardEntityCaption({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Link className="crm-dashboard-entity-caption" to={to} title={`Open ${String(children).toLowerCase()} list`}>
      {children}
    </Link>
  );
}
