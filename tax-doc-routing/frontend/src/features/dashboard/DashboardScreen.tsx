import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { AlertTriangle, CheckCircle2, FolderOpen, Loader2, Plus } from 'lucide-react';
import { BarChart } from '@appfw/pds-health-components/charts';
import { List } from '@appfw/pds-health-components/experience';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { Button } from '@appfw/pds-health-components/primitives';
import { ChartLegend, ChartShell, EmptyState, InlineAlert, KpiTile, Surface } from '@appfw/pds-health-components/surfaces';
import { WorkQueueItem } from '@appfw/pds-health-components/work-surfaces';
import { CasesGrid } from '../../components/CasesGrid';
import { StatusBadge } from '../../components/StatusBadge';
import { ALL_STATUSES, STATUS, STATUS_META, TERMINAL_STATUSES } from '../shared/config/statuses';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import type { RecordStatus, RoutingCase } from '../shared/types';
import { formatDate } from '../shared/utils/format';

type Tile = {
  key: string;
  label: string;
  icon: typeof FolderOpen;
  tone: 'accent' | 'warning' | 'danger' | 'success';
  match: RecordStatus[];
  detail: (count: number, total: number, cases: RoutingCase[]) => string;
};

const pct = (n: number, total: number) => (total ? `${Math.round((n / total) * 100)}% of all cases` : 'No cases yet');

const TILES: Tile[] = [
  {
    key: 'open',
    label: 'Open',
    icon: FolderOpen,
    tone: 'accent',
    match: [STATUS.OPEN, STATUS.IN_PROGRESS],
    detail: (_n, _t, cases) => `${cases.filter((c) => c.status === STATUS.IN_PROGRESS).length} in progress · ${cases.filter((c) => c.status === STATUS.OPEN).length} not started`
  },
  { key: 'processing', label: 'Processing', icon: Loader2, tone: 'warning', match: [STATUS.PROCESSING], detail: (n) => (n ? 'Being filed right now' : 'Nothing running') },
  { key: 'exception', label: 'Exceptions', icon: AlertTriangle, tone: 'danger', match: [STATUS.EXCEPTION], detail: (n) => (n ? 'Need attention' : 'All clear') },
  { key: 'completed', label: 'Completed', icon: CheckCircle2, tone: 'success', match: [STATUS.COMPLETED], detail: (n, total) => pct(n, total) }
];

const byUpdated = (a: RoutingCase, b: RoutingCase) => b.updated.localeCompare(a.updated);

export function DashboardScreen() {
  const { user, role, visibleCases, exceptionQueue, draftApi, hasPermission } = useTaxRouting();
  // The Assigned To column is a real permission (business spec 12.3: "See who each record is
  // assigned to"), not a role literal — see docs/architecture/rbac-design.md.
  const canSeeAssignee = hasPermission('routing_record.view_assignee');
  const navigate = useNavigate();

  const sorted = useMemo(() => [...visibleCases].sort(byUpdated), [visibleCases]);
  const work = useMemo(() => sorted.filter((c) => !TERMINAL_STATUSES.includes(c.status)).slice(0, 8), [sorted]);
  const recent = sorted.slice(0, 5);
  const exceptionCount = exceptionQueue.length;

  const byStatus = ALL_STATUSES.map((st) => ({ label: st, value: visibleCases.filter((c) => c.status === st).length, tone: STATUS_META[st].tone }));
  const maxCount = Math.max(...byStatus.map((r) => r.value), 1);

  return (
    <div className="tax-stack">
      <PageHeader
        title="Dashboard"
        subtitle={`Welcome back, ${user.name.split(' ')[0]}! ${role === 'admin' ? "Here's the team's activity summary." : "Here's your activity summary."}`}
        actions={
          <Button
            variant="primary"
            onClick={() => {
              draftApi.reset();
              navigate('/new/client');
            }}
          >
            <Plus size={16} aria-hidden="true" /> Create New Request
          </Button>
        }
      />

      {exceptionCount > 0 ? (
        <InlineAlert
          tone="danger"
          title={`${exceptionCount} record${exceptionCount === 1 ? '' : 's'} require attention`}
          detail="Open the exception queue to see what failed and retry the step."
          action={
            <Button size="sm" variant="secondary" onClick={() => navigate('/exceptions')}>
              View exception queue
            </Button>
          }
        />
      ) : null}

      <section className="tax-kpi-grid" aria-label="Key figures">
        {TILES.map(({ key, label, icon: Icon, tone, match, detail }) => {
          const matching = visibleCases.filter((c) => match.includes(c.status));
          return (
            <KpiTile
              key={key}
              label={label}
              value={matching.length}
              detail={detail(matching.length, visibleCases.length, matching)}
              icon={<Icon size={18} aria-hidden="true" />}
              tone={key === 'exception' && matching.length === 0 ? 'success' : tone}
            />
          );
        })}
      </section>

      <Surface title="Work I Need To Do" subtitle={role === 'admin' ? 'All staff' : 'Assigned to you'} density="compact">
        <CasesGrid
          cases={work}
          ariaLabel="Work I need to do"
          options={{ task: true, emails: true, assignedTo: canSeeAssignee }}
          emptyTitle="You're all caught up"
          emptyDetail="No records need your attention right now."
        />
      </Surface>

      <section className="tax-two-col tax-equal" aria-label="Status and recent cases">
        <ChartShell
          title="Cases by status"
          subtitle={role === 'admin' ? 'Across all staff' : 'Assigned to you'}
          footer={<ChartLegend items={byStatus.map((r) => ({ id: r.label, label: r.label, value: String(r.value), tone: r.tone }))} />}
        >
          <BarChart
            ariaLabel={`Cases by status: ${byStatus.map((r) => `${r.label} ${r.value}`).join(', ')}`}
            maxValue={maxCount}
            data={byStatus}
          />
        </ChartShell>

        <Surface
          title="Recent Cases"
          subtitle="Last five updated"
          density="compact"
          actions={
            <Button size="sm" variant="quiet" onClick={() => navigate(role === 'admin' ? '/admin/cases' : '/cases')}>
              View all
            </Button>
          }
        >
          {recent.length === 0 ? (
            <EmptyState title="No recent cases" detail="Cases you work on will appear here." />
          ) : (
            <List aria-label="Recent cases">
              {recent.map((c) => (
                <WorkQueueItem
                  key={c.id}
                  as="li"
                  density="compact"
                  activation={{ kind: 'button', onActivate: () => navigate(`/records/${c.id}`) }}
                  title={c.id}
                  description={c.client.fullName}
                  metadata={formatDate(c.updated)}
                  status={<StatusBadge status={c.status} />}
                />
              ))}
            </List>
          )}
        </Surface>
      </section>
    </div>
  );
}
