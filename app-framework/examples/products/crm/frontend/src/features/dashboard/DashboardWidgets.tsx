import type { ReactNode } from "react";
import {
  ArcElement,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  type ChartOptions,
  type TooltipItem
} from "chart.js";
import { Bar, Doughnut, Line } from "react-chartjs-2";
import { Activity, Clock } from "lucide-react";
import { formatCurrency, formatNumber, formatPercent } from "../../components/analytics";
import { EmptyState, KpiTile, Skeleton, StateView } from "../../components/ui";
import type {
  AccountStateBucket,
  ActivityRecord,
  DashboardTrace,
  LeadRecord,
  OpportunityRecord,
  PipelineStageBucket
} from "./dashboardAnalytics";

export { PanelHeader, formatCurrency, formatNumber, formatPercent, formatTime } from "../../components/analytics";

ChartJS.register(
  ArcElement,
  BarElement,
  CategoryScale,
  Filler,
  Legend,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip
);
ChartJS.defaults.font.family =
  '"InterVariable", "Inter Fallback: Segoe UI", "Inter Fallback: Arial", ui-sans-serif, system-ui, sans-serif';
ChartJS.defaults.color = "#545860";

const CHART_BLUE = "#00A9EB";
const CHART_BLUE_DARK = "#0090CC";
const CHART_BLUE_LIGHT = "rgba(0, 169, 235, 0.14)";
const CHART_TEAL = "#2DC6C6";
const CHART_GREEN = "#27AE60";
const CHART_GOLD = "#F5A623";
const CHART_GRAY = "#545860";
const CHART_BORDER = "rgba(84, 88, 96, 0.12)";

export function DashboardSkeleton() {
  return (
    <div className="crm-dashboard">
      <div className="crm-kpi-grid">
        {Array.from({ length: 4 }).map((_, index) => (
          <Skeleton
            key={index}
            variant="card"
            lines={3}
            label={`Loading dashboard metric ${index + 1}`}
          />
        ))}
      </div>
      <StateView kind="loading" title="Loading dashboard analytics" />
    </div>
  );
}

export function KpiCard({
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

function pdsKpiTone(tone: "blue" | "teal" | "green" | "gold"): "accent" | "success" | "warning" {
  if (tone === "green") return "success";
  if (tone === "gold") return "warning";
  return "accent";
}

export function PipelineChart({ stages }: { stages: PipelineStageBucket[] }) {
  if (!stages.length) return <EmptyPanel detail="No pipeline stages returned by the backend." />;
  return (
    <div className="crm-chart-wrap tall" role="img" aria-label="Pipeline value by opportunity stage">
      <Bar
        aria-label="Pipeline value by opportunity stage chart"
        role="img"
        data={{
          labels: stages.map((stage) => stage.stageName),
          datasets: [
            {
              label: "Pipeline value",
              data: stages.map((stage) => stage.totalAmount),
              backgroundColor: stages.map((stage) => (stage.isClosed ? CHART_GRAY : CHART_BLUE)),
              borderColor: stages.map((stage) => (stage.isClosed ? CHART_GRAY : CHART_BLUE_DARK)),
              borderRadius: 8,
              borderSkipped: false
            },
            {
              label: "Weighted value",
              data: stages.map((stage) => stage.weightedAmount),
              backgroundColor: CHART_TEAL,
              borderColor: CHART_TEAL,
              borderRadius: 8,
              borderSkipped: false
            }
          ]
        }}
        options={pipelineOptions}
      />
    </div>
  );
}

export function LeadConversionDonut({
  converted,
  open,
  rate
}: {
  converted: number;
  open: number;
  rate: number;
}) {
  return (
    <div className="crm-donut-layout">
      <div className="crm-donut-chart">
        <Doughnut
          aria-label="Lead conversion donut chart"
          role="img"
          data={{
            labels: ["Converted", "Open"],
            datasets: [
              {
                data: [converted, open],
                backgroundColor: [CHART_BLUE, CHART_GRAY],
                borderColor: "#ffffff",
                borderWidth: 4,
                hoverOffset: 8
              }
            ]
          }}
          options={doughnutOptions}
        />
        <strong>{formatPercent(rate)}</strong>
      </div>
      <dl className="crm-donut-metrics">
        <div>
          <dt>Converted</dt>
          <dd>{formatNumber(converted)}</dd>
        </div>
        <div>
          <dt>Open</dt>
          <dd>{formatNumber(open)}</dd>
        </div>
      </dl>
    </div>
  );
}

export function RevenueBars({ buckets }: { buckets: AccountStateBucket[] }) {
  if (!buckets.length) return <EmptyPanel detail="No account territory aggregates returned by the backend." />;
  return (
    <div className="crm-chart-wrap" role="img" aria-label="Account revenue by billing state">
      <Bar
        aria-label="Account revenue by billing state chart"
        role="img"
        data={{
          labels: buckets.map((bucket) => bucket.state),
          datasets: [
            {
              label: "Revenue",
              data: buckets.map((bucket) => bucket.totalRevenue),
              backgroundColor: [CHART_BLUE, CHART_TEAL, CHART_GREEN, CHART_GOLD, CHART_BLUE_DARK],
              borderColor: "#ffffff",
              borderRadius: 7,
              borderSkipped: false
            }
          ]
        }}
        options={revenueOptions}
      />
    </div>
  );
}

export function OpportunityTable({ opportunities }: { opportunities: OpportunityRecord[] }) {
  if (!opportunities.length) return <EmptyPanel detail="No opportunities returned by the backend." />;
  return (
    <div className="crm-opportunity-table" tabIndex={0} aria-label="Largest active deals table">
      <table>
        <thead>
          <tr>
            <th>Deal</th>
            <th>Account</th>
            <th>Stage</th>
            <th>Close</th>
            <th>Amount</th>
          </tr>
        </thead>
        <tbody>
          {opportunities.map((opportunity) => (
            <tr key={opportunity.id ?? opportunity.name}>
              <td>
                <strong>{opportunity.name ?? "Untitled opportunity"}</strong>
                <span>{opportunity.next_step ?? opportunity.lead_source ?? "No next step captured"}</span>
              </td>
              <td>{opportunity.account?.name ?? "No account"}</td>
              <td>{opportunity.stage?.name ?? "Unknown"}</td>
              <td>{formatDate(opportunity.close_date)}</td>
              <td>{formatCurrency(opportunity.amount ?? 0)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function LeadQueue({ leads }: { leads: LeadRecord[] }) {
  if (!leads.length) return <EmptyPanel detail="No open leads returned by the backend." />;
  return (
    <div className="crm-record-list">
      {leads.map((lead) => (
        <div key={lead.id ?? `${lead.company}-${lead.email}`}>
          <strong>{lead.company ?? personName(lead)}</strong>
          <span>{personName(lead)} · {lead.source?.name ?? "Unknown source"}</span>
          <small>{formatCurrency(lead.annual_revenue ?? 0)} potential revenue</small>
        </div>
      ))}
    </div>
  );
}

export function ActivityQueue({ activities, openCount }: { activities: ActivityRecord[]; openCount: number }) {
  if (!activities.length) return <EmptyPanel detail="No open activities returned by the backend." />;
  return (
    <div className="crm-record-list">
      <div className="crm-activity-summary">
        <Activity size={15} />
        <span>{formatNumber(openCount)} open activities in scope</span>
      </div>
      {activities.map((activity) => (
        <div key={activity.id ?? activity.subject}>
          <strong>{activity.subject ?? "Untitled activity"}</strong>
          <span>{activity.status ?? "Open"} · {activity.priority ?? "Normal priority"}</span>
          <small>
            <Clock size={12} />
            Due {formatDate(activity.due_date)}
          </small>
        </div>
      ))}
    </div>
  );
}

export function TelemetryPanel({ traces }: { traces: DashboardTrace[] }) {
  const summary = traceSummary(traces);
  return (
    <div className="crm-telemetry">
      <div className="crm-telemetry-summary">
        <div>
          <span>Requests</span>
          <strong>{traces.length}</strong>
        </div>
        <div>
          <span>Average</span>
          <strong>{Math.round(summary.avgMs)} ms</strong>
        </div>
        <div>
          <span>Slowest</span>
          <strong>{Math.round(summary.slowestMs)} ms</strong>
        </div>
      </div>
      <div className="crm-telemetry-detail">
        <div className="crm-chart-wrap mini" role="img" aria-label="Dashboard request latency profile">
          <Line
            aria-label="Dashboard request latency profile chart"
            role="img"
            data={{
              labels: traces.map((trace) => trace.label),
              datasets: [
                {
                  label: "Response time",
                  data: traces.map((trace) => trace.responseMs),
                  borderColor: CHART_BLUE,
                  backgroundColor: CHART_BLUE_LIGHT,
                  borderWidth: 2,
                  pointBackgroundColor: CHART_BLUE,
                  pointRadius: 3,
                  fill: true,
                  tension: 0.36
                }
              ]
            }}
            options={latencyOptions}
          />
        </div>
        <div className="crm-trace-list" tabIndex={0} aria-label="Dashboard request telemetry list">
          {traces.map((trace) => (
            <div key={trace.requestId}>
              <span>{trace.label}</span>
              <strong>{Math.round(trace.responseMs)} ms</strong>
              <small>
                {trace.rowCount} rows · {trace.queryCount} total · {trace.requestId}
              </small>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function traceSummary(traces: DashboardTrace[]) {
  if (!traces.length) return { avgMs: 0, slowestMs: 0 };
  const totalMs = traces.reduce((total, trace) => total + trace.responseMs, 0);
  return {
    avgMs: totalMs / traces.length,
    slowestMs: Math.max(...traces.map((trace) => trace.responseMs))
  };
}

function EmptyPanel({ detail }: { detail: string }) {
  return <EmptyState title="No dashboard data" detail={detail} />;
}

const pipelineOptions: ChartOptions<"bar"> = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: "bottom",
      labels: {
        boxWidth: 10,
        boxHeight: 10,
        usePointStyle: true
      }
    },
    tooltip: {
      callbacks: {
        label: (item: TooltipItem<"bar">) => `${item.dataset.label}: ${formatCurrency(Number(item.parsed.y ?? 0))}`
      }
    }
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { maxRotation: 0, font: { size: 10 } }
    },
    y: {
      beginAtZero: true,
      grid: { color: CHART_BORDER },
      ticks: {
        font: { size: 10 },
        callback: (value) => formatCurrency(Number(value))
      }
    }
  }
};

const revenueOptions: ChartOptions<"bar"> = {
  indexAxis: "y",
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (item: TooltipItem<"bar">) => formatCurrency(Number(item.parsed.x ?? 0))
      }
    }
  },
  scales: {
    x: {
      beginAtZero: true,
      grid: { color: CHART_BORDER },
      ticks: {
        font: { size: 10 },
        callback: (value) => formatCurrency(Number(value))
      }
    },
    y: {
      grid: { display: false },
      ticks: { font: { size: 11 } }
    }
  }
};

const doughnutOptions: ChartOptions<"doughnut"> = {
  responsive: true,
  maintainAspectRatio: false,
  cutout: "70%",
  plugins: {
    legend: {
      position: "bottom",
      labels: {
        boxWidth: 10,
        boxHeight: 10,
        usePointStyle: true
      }
    },
    tooltip: {
      callbacks: {
        label: (item: TooltipItem<"doughnut">) => `${item.label}: ${formatNumber(Number(item.raw ?? 0))}`
      }
    }
  }
};

const latencyOptions: ChartOptions<"line"> = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (item: TooltipItem<"line">) => `${Math.round(Number(item.parsed.y ?? 0))} ms`
      }
    }
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { display: false }
    },
    y: {
      beginAtZero: true,
      grid: { color: CHART_BORDER },
      ticks: {
        font: { size: 10 },
        callback: (value) => `${Math.round(Number(value))} ms`
      }
    }
  }
};

function personName(lead: LeadRecord) {
  const name = [lead.first_name, lead.last_name].filter(Boolean).join(" ");
  return name || "Unknown lead";
}

function formatDate(value: string | null | undefined) {
  if (!value) return "No date";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric"
  }).format(date);
}
