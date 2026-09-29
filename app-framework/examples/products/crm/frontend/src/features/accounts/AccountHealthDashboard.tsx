import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import {
  Activity,
  CheckCircle2,
  CircleAlert,
  Gauge,
  RefreshCw,
  ShieldCheck,
  TrendingUp
} from "lucide-react";
import {
  Badge,
  ChartShell,
  EmptyState,
  InlineAlert,
  KpiTile,
  Skeleton,
  StateView,
  Surface
} from "../../components/ui";
import { useAppfwClient } from "../../app/providers";
import {
  formatCurrency,
  formatNumber,
  formatPercent,
  formatTime
} from "../../components/analytics";
import {
  loadAccountHealth,
  refreshAccountHealthSnapshot,
  type AccountHealthAction,
  type AccountHealthRefreshResult,
  type AccountHealthResult,
  type AccountHealthSignal,
  type AccountHealthStage
} from "./accountHealth";

export function AccountHealthDashboard({ accountId }: { accountId: string }) {
  const client = useAppfwClient();
  const [data, setData] = useState<AccountHealthResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [refreshResult, setRefreshResult] = useState<AccountHealthRefreshResult | null>(null);
  const [refreshingSnapshot, setRefreshingSnapshot] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    loadAccountHealth(client, accountId)
      .then((result) => {
        if (!active) return;
        setData(result);
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
  }, [accountId, client, refreshKey]);

  const health = data?.health ?? null;
  const score = numberOr(health?.health?.score, 0);
  const profileCompleteness = numberOr(health?.data_quality?.profile_completeness, 0);

  async function handleRefreshStoredSnapshot() {
    if (!health || refreshingSnapshot) return;
    setRefreshError(null);
    setRefreshingSnapshot(true);
    try {
      const result = await refreshAccountHealthSnapshot(client, accountId, score);
      setRefreshResult(result);
      setRefreshKey((value) => value + 1);
    } catch (caught: unknown) {
      setRefreshError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setRefreshingSnapshot(false);
    }
  }

  if (error && !data) {
    return <StateView kind="error" title="Could not load account health" detail={error} />;
  }

  if (!data && loading) {
    return <AccountHealthSkeleton />;
  }

  return (
    <section className="crm-account-health" aria-label="Account health dashboard">
      <Surface
        className="crm-account-health-hero"
        title={(
          <>
            <span className="crm-kicker">Account 360</span>
            <span>Health dashboard</span>
          </>
        )}
        subtitle={(
          <>
            Custom method view powered by <code>accountHealth(accountId)</code> and a stored procedure refresh.
          </>
        )}
        actions={(
          <div className="crm-account-health-actions">
            <button
              type="button"
              className="crm-button primary"
              onClick={handleRefreshStoredSnapshot}
              disabled={loading || refreshingSnapshot || !health}
            >
              <RefreshCw size={15} />
              {refreshingSnapshot ? "Refreshing snapshot" : "Refresh stored snapshot"}
            </button>
            <button
              type="button"
              className="crm-button secondary"
              onClick={() => setRefreshKey((value) => value + 1)}
              disabled={loading}
            >
              <RefreshCw size={15} />
              Refresh
            </button>
          </div>
        )}
      >
        {error ? (
          <InlineAlert tone="warning" title="Account health warning" detail={error} />
        ) : null}
        {refreshError ? (
          <InlineAlert tone="warning" title="Snapshot refresh warning" detail={refreshError} />
        ) : null}

        {!health ? (
          <StateView
            kind="empty"
            title="No account health payload"
            detail="The account health custom method did not return a payload for this record."
          />
        ) : (
          <>
            <div className="crm-account-health-summary">
              <div className={`crm-health-score risk-${normalizeToken(health.health?.risk_level)}`}>
                <span>Health score</span>
                <strong>{score}</strong>
                <small>
                  {health.health?.grade ?? "Unscored"} · {humanize(health.health?.risk_level ?? "unknown")} risk
                </small>
              </div>
              <div className="crm-account-health-copy">
                <Badge tone={score >= 80 ? "success" : score >= 60 ? "accent" : "danger"}>
                  {health.account?.name ?? "Selected account"}
                </Badge>
                <h3>{health.health?.summary ?? "No health summary returned."}</h3>
                <p>
                  {accountLocation(health.account?.location)} ·{" "}
                  {formatNumber(numberOr(health.account?.number_of_employees, 0))} employees ·{" "}
                  {formatCurrency(numberOr(health.account?.annual_revenue, 0))} revenue
                </p>
              </div>
            </div>

            <div className="crm-kpi-grid">
              <HealthKpi
                title="Weighted pipeline"
                value={formatCurrency(numberOr(health.financials?.weighted_pipeline_value, 0))}
                detail={`${formatCurrency(numberOr(health.financials?.pipeline_value, 0))} gross pipeline`}
                icon={<TrendingUp size={18} />}
                tone="blue"
              />
              <HealthKpi
                title="Open opportunities"
                value={formatNumber(numberOr(health.opportunities?.open_count, 0))}
                detail={`${formatNumber(numberOr(health.opportunities?.won_count, 0))} won sampled deals`}
                icon={<Gauge size={18} />}
                tone="teal"
              />
              <HealthKpi
                title="Engagement"
                value={humanize(health.activities?.engagement_status ?? "unknown")}
                detail={`${formatNumber(numberOr(health.activities?.overdue_count, 0))} overdue activities`}
                icon={<Activity size={18} />}
                tone="gold"
              />
              <HealthKpi
                title="Profile quality"
                value={formatPercent(profileCompleteness / 100)}
                detail={`${formatNumber(numberOr(health.contacts?.total_count, 0))} related contacts`}
                icon={<ShieldCheck size={18} />}
                tone="green"
              />
            </div>
          </>
        )}
      </Surface>

      {health ? (
        <div className="crm-dashboard-grid">
          <ChartShell
            className="crm-dashboard-panel crm-panel-wide"
            subtitle="Pipeline"
            title="Stage distribution"
          >
            <StageDistribution stages={health.opportunities?.stage_distribution ?? []} />
          </ChartShell>

          <ChartShell
            className="crm-dashboard-panel"
            subtitle="Recommended"
            title="Next best actions"
          >
            <ActionList actions={health.recommended_actions ?? []} />
          </ChartShell>

          <ChartShell
            className="crm-dashboard-panel"
            subtitle="Signals"
            title="Risk and quality signals"
          >
            <SignalList signals={health.signals ?? []} />
          </ChartShell>

          <ChartShell
            className="crm-dashboard-panel"
            subtitle="Quotes"
            title="Commercial coverage"
          >
            <MetricList
              rows={[
                ["Open quotes", formatNumber(numberOr(health.quotes?.open_count, 0))],
                ["Expired quotes", formatNumber(numberOr(health.quotes?.expired_count, 0))],
                ["Open quote value", formatCurrency(numberOr(health.financials?.open_quote_value, 0))],
                ["Largest quote", formatCurrency(numberOr(health.quotes?.largest_open?.total_price, 0))]
              ]}
            />
          </ChartShell>

          <ChartShell
            className="crm-dashboard-panel"
            subtitle="Data quality"
            title="Profile completeness"
          >
            <QualityList fields={health.data_quality?.fields ?? []} />
          </ChartShell>

          <ChartShell
            className="crm-dashboard-panel crm-panel-wide"
            subtitle="Observability"
            title="Account health request"
          >
            <MetricList
              rows={[
                ["Health method", `${Math.round(data?.request?.accountHealthMs ?? 0)} ms`],
                [
                  "Stored procedure",
                  refreshResult
                    ? `${Math.round(refreshResult.request.procedureMs)} ms`
                    : health.stored_snapshot?.refreshed_at
                      ? "Snapshot stored"
                      : "Not refreshed"
                ],
                [
                  "Stored score",
                  formatNumber(numberOr(refreshResult?.storedSnapshot?.health_score ?? health.stored_snapshot?.health_score, 0))
                ],
                ["Request", data?.request?.requestId ? "Captured" : "Pending"],
                ["Generated", health.health?.generated_at ? formatTime(health.health.generated_at) : "Just now"]
              ]}
            />
          </ChartShell>
        </div>
      ) : null}
    </section>
  );
}

function AccountHealthSkeleton() {
  return (
    <section className="crm-account-health">
      <Skeleton variant="card" lines={3} label="Loading account health" />
    </section>
  );
}

function HealthKpi({
  title,
  value,
  detail,
  icon,
  tone
}: {
  title: string;
  value: string;
  detail: string;
  icon: ReactNode;
  tone: "blue" | "teal" | "green" | "gold";
}) {
  return (
    <KpiTile
      className={`crm-kpi-card tone-${tone}`}
      label={title}
      value={value}
      detail={detail}
      icon={icon}
      tone={pdsKpiTone(tone)}
    />
  );
}

function StageDistribution({ stages }: { stages: AccountHealthStage[] }) {
  if (!stages.length) return <AccountHealthEmptyState detail="No sampled opportunities returned." />;
  const maxAmount = Math.max(...stages.map((stage) => numberOr(stage.amount, 0)), 1);
  return (
    <div className="crm-bar-list">
      {stages.map((stage) => {
        const amount = numberOr(stage.amount, 0);
        return (
          <div className="crm-bar-row" key={stage.stage ?? "Unstaged"}>
            <div className="crm-bar-label">
              <strong>{stage.stage ?? "Unstaged"}</strong>
              <span>{formatNumber(numberOr(stage.count, 0))} deals</span>
            </div>
            <div className="crm-bar-track">
              <span style={{ "--crm-bar-size": `${Math.max((amount / maxAmount) * 100, 4)}%` } as CSSProperties} />
            </div>
            <div className="crm-bar-value">{formatCurrency(amount)}</div>
          </div>
        );
      })}
    </div>
  );
}

function ActionList({ actions }: { actions: AccountHealthAction[] }) {
  if (!actions.length) return <AccountHealthEmptyState detail="No recommended actions returned." />;
  return (
    <div className="crm-record-list">
      {actions.map((action) => (
        <div key={`${action.priority}-${action.label}`}>
          <strong>{action.label ?? "Action"}</strong>
          <span>{humanize(action.priority ?? "normal")} priority</span>
          <small>{action.detail ?? "No detail provided."}</small>
        </div>
      ))}
    </div>
  );
}

function SignalList({ signals }: { signals: AccountHealthSignal[] }) {
  if (!signals.length) {
    return (
      <div className="crm-account-health-ok">
        <CheckCircle2 size={18} />
        <span>No risk signals returned.</span>
      </div>
    );
  }
  return (
    <div className="crm-record-list">
      {signals.slice(0, 5).map((signal) => (
        <div key={`${signal.category}-${signal.label}`}>
          <strong>{signal.label ?? "Signal"}</strong>
          <span>
            <CircleAlert size={13} />
            {humanize(signal.severity ?? "info")} · {humanize(signal.category ?? "general")}
          </span>
          <small>{signal.detail ?? "No detail provided."}</small>
        </div>
      ))}
    </div>
  );
}

function MetricList({ rows }: { rows: Array<[string, string]> }) {
  return (
    <dl className="crm-health-metric-list">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function QualityList({ fields }: { fields: Array<{ name?: string; complete?: boolean }> }) {
  if (!fields.length) return <AccountHealthEmptyState detail="No quality fields returned." />;
  return (
    <div className="crm-health-quality-list">
      {fields.map((field) => (
        <span key={field.name} className={field.complete ? "is-complete" : ""}>
          {field.complete ? <CheckCircle2 size={14} /> : <CircleAlert size={14} />}
          {humanize(field.name ?? "field")}
        </span>
      ))}
    </div>
  );
}

function AccountHealthEmptyState({ detail }: { detail: string }) {
  return <EmptyState title="No account health data" detail={detail} />;
}

function pdsKpiTone(tone: "blue" | "teal" | "green" | "gold"): "accent" | "success" | "warning" {
  if (tone === "green") return "success";
  if (tone === "gold") return "warning";
  return "accent";
}

function numberOr(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function accountLocation(location: { city?: string | null; state?: string | null; country?: string | null } | undefined) {
  const parts = [location?.city, location?.state, location?.country].filter(Boolean);
  return parts.length ? parts.join(", ") : "No location";
}

function humanize(value: string) {
  return value
    .replace(/_/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function normalizeToken(value: string | undefined) {
  return (value ?? "unknown").toLowerCase().replace(/[^a-z0-9]+/g, "-");
}
