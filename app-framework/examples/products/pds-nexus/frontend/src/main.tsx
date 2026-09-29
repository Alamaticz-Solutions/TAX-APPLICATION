import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  AgentTimeline,
  AiAttributionAffordance,
  AttentionMarker,
  AppShell,
  Badge,
  Button,
  ChartLegend,
  ChartShell,
  CommandPalette,
  ConfidenceSignal,
  CitationList,
  DataGridDensityControl,
  DataGridPagination,
  DataGridShell,
  DataGridToolbar,
  DateField,
  Dialog,
  EntityRefCard,
  EvidenceSummary,
  FlowGraphShell,
  FormLayout,
  FreshnessIndicator,
  GeneratedViewShell,
  IntentPreview,
  KpiTile,
  MessageComposer,
  MessageThread,
  MetricTrend,
  PageHeader,
  RecommendationCard,
  SelectField,
  Surface,
  SwitchField,
  TextField,
  ToolCallStatus,
  ValidationSummary,
  StreamingText,
  type AgentTimelineItem,
  type CitationItem,
  type CommandPaletteItem,
  type EvidenceSummaryClaim,
  type FlowGraphEdge,
  type FlowGraphNode,
  type MessageItem,
  type PdsDataGridColumn,
  type PdsDensity
} from '@appfw/pds-health-components';
import { denovoWorkflowUiContract } from './generated/appfw-ui-contract';
import { MyWorkIntelligentExperience } from './features/intelligent-experience';
import { AboutPage } from './features/nexus-a0/AboutPage';
import './styles.css';

const appUiContract = {
  displayName: 'PDS Nexus',
  modelStatus: 'Model-backed synthetic projection',
  provider: denovoWorkflowUiContract.provider.dataSourceType,
  schema: denovoWorkflowUiContract.schemaName,
  schemaLabel: 'De Novo workflow'
};

type DeNovoTask = Record<string, unknown> & {
  id: string;
  recordLocator: string;
  task: string;
  ownerTeam: string;
  phase: string;
  risk: string;
  daysStalled: number;
  freshness: string;
  tenant: 'tenant_a' | 'tenant_b';
};

const denovoRows: DeNovoTask[] = [
  {
    id: 'DN-001',
    recordLocator: 'rl_11111111111111111111111111111111',
    task: 'Credentialing packet',
    ownerTeam: 'Provider onboarding',
    phase: 'Licensing',
    risk: 'High',
    daysStalled: 8,
    freshness: '2026-07-02T00:00:00Z',
    tenant: 'tenant_a'
  },
  {
    id: 'DN-002',
    recordLocator: 'rl_22222222222222222222222222222222',
    task: 'Equipment approval',
    ownerTeam: 'Clinical operations',
    phase: 'Buildout',
    risk: 'Medium',
    daysStalled: 6,
    freshness: '2026-07-02T00:00:00Z',
    tenant: 'tenant_a'
  },
  {
    id: 'DN-003',
    recordLocator: 'rl_33333333333333333333333333333333',
    task: 'Lease amendment review',
    ownerTeam: 'Real estate',
    phase: 'Site readiness',
    risk: 'Medium',
    daysStalled: 5,
    freshness: '2026-07-02T00:00:00Z',
    tenant: 'tenant_a'
  },
  {
    id: 'DN-004',
    recordLocator: 'rl_44444444444444444444444444444444',
    task: 'Training roster confirmation',
    ownerTeam: 'Learning team',
    phase: 'Launch readiness',
    risk: 'Low',
    daysStalled: 2,
    freshness: '2026-07-02T00:00:00Z',
    tenant: 'tenant_a'
  }
];

const stalledRows = denovoRows.filter((row) => row.daysStalled >= 5);

const denovoColumns: PdsDataGridColumn<DeNovoTask>[] = [
  { key: 'task', header: 'Task', width: '30%' },
  { key: 'ownerTeam', header: 'Owner team', width: '24%' },
  { key: 'phase', header: 'Phase', width: '22%' },
  {
    key: 'risk',
    header: 'Risk',
    width: '14%',
    render: (row) => (
      <Badge tone={row.risk === 'High' ? 'danger' : row.risk === 'Medium' ? 'warning' : 'neutral'}>
        {row.risk}
      </Badge>
    )
  },
  { key: 'daysStalled', header: 'Days stalled', width: '10%', align: 'end' }
];

const stalledFlowNodes: FlowGraphNode[] = stalledRows.map((row) => ({
  id: row.id,
  label: row.task,
  detail: `${row.ownerTeam} - ${row.daysStalled} days stalled`,
  status: row.risk === 'High' ? 'blocked' : 'waiting'
}));

const stalledFlowEdges: FlowGraphEdge[] = [
  { id: 'edge-dn001-dn002', from: 'DN-001', to: 'DN-002', label: 'Launch dependency' },
  { id: 'edge-dn002-dn003', from: 'DN-002', to: 'DN-003', label: 'Readiness sequence' }
];

const stalledCitations: CitationItem[] = stalledRows.map((row) => ({
  id: row.recordLocator,
  label: row.task,
  source: 'Synthetic De Novo projection',
  detail: `${row.ownerTeam}; fixture watermark ${row.freshness}`
}));

const evidenceClaims: EvidenceSummaryClaim[] = [
  {
    id: 'claim-visible-tenant',
    text: 'Three stalled records resolve for tenant_a; the tenant_b sentinel is intentionally absent.',
    citation: 'nexus-denovo-synthetic.json'
  },
  {
    id: 'claim-preview-only',
    text: 'The recommended follow-up opens IntentPreview and cannot execute a ServiceNow write.',
    citation: 'nexus-pilot-contract.yaml'
  },
  {
    id: 'claim-flowgraph',
    text: 'The generated view is a FlowGraphShell composition over resolved record locators.',
    citation: 'answer_envelope@1 local fixture'
  },
  {
    id: 'claim-lineage',
    text: 'Freshness and lineage stop at the retained local fixture until Unit A reconciles ServiceNow source truth.',
    citation: 'nexus-pilot-contract.yaml'
  }
];

const sourceLineage = [
  {
    stage: 'Synthetic fixture',
    evidence: 'nexus-denovo-synthetic.json',
    boundary: 'workspace-local',
    detail: 'Retained 176-task, 20-team De Novo tracker shape.'
  },
  {
    stage: 'Reviewed projection',
    evidence: 'model-proposal.yaml',
    boundary: 'accepted local config',
    detail: 'DeNovoTask, Workstream, and Team accepted as provisional read models.'
  },
  {
    stage: 'Generated contract',
    evidence: 'artifacts.json',
    boundary: 'generated from local model',
    detail: 'Backend, database, and frontend contracts are generated evidence.'
  }
];

const timelineItems: AgentTimelineItem[] = [
  {
    id: 'turn-started',
    title: 'User asked for stalled tasks',
    status: 'completed',
    timestamp: 'T+0s',
    detail: 'Read-only pilot question captured in the native message panel.'
  },
  {
    id: 'refs-resolved',
    title: 'Resolved tenant-scoped references',
    status: 'completed',
    timestamp: 'T+1s',
    detail: '3 record locators resolved; 0 tenant_b records surfaced.',
    evidence: 'red_team.leaks_found = 0'
  },
  {
    id: 'graph-opened',
    title: 'Opened generated FlowGraph view',
    status: 'completed',
    timestamp: 'T+2s',
    detail: 'FlowGraphShell rendered from viewRegistry-compatible shape.'
  },
  {
    id: 'write-blocked',
    title: 'Write-shaped follow-up stays preview-only',
    status: 'blocked',
    timestamp: 'T+3s',
    detail: 'G1 delegated auth and ServiceNow Unit A evidence are not present.'
  }
];

const workstreamOptions = [
  { value: 'all', label: 'All workstreams' },
  { value: 'licensing', label: 'Licensing' },
  { value: 'buildout', label: 'Buildout' },
  { value: 'launch-readiness', label: 'Launch readiness' }
];

const trackerValidation = {
  sourceEvidence: ['Synthetic fixture only; ServiceNow export is not connected.']
};

const readinessLegend = [
  { id: 'on-track', label: 'On track', tone: 'success' as const, value: '116' },
  { id: 'watch', label: 'Watch', tone: 'warning' as const, value: '46' },
  { id: 'stalled', label: 'Stalled', tone: 'danger' as const, value: '14' }
];

const workspaceCommands: CommandPaletteItem[] = [
  {
    id: 'workspace:tracker',
    group: 'Workspace',
    label: 'Open De Novo tracker',
    detail: 'Synthetic 176-task pilot fixture',
    href: '#tracker'
  },
  {
    id: 'workspace:evidence',
    group: 'Evidence',
    label: 'Review source evidence',
    detail: '.appfw/source-evidence/nexus-denovo-synthetic.json',
    href: '#evidence'
  },
  {
    id: 'workspace:model',
    group: 'Model',
    label: 'Review provisional projection model',
    detail: appUiContract.schema,
    href: '#model'
  },
  {
    id: 'workspace:chat',
    group: 'AI',
    label: 'Show stalled-task answer',
    detail: 'Local CH8 consumer-wiring proof',
    href: '#chat'
  },
  {
    id: 'workspace:graph',
    group: 'AI',
    label: 'Open generated graph',
    detail: 'FlowGraphShell over resolved record locators',
    href: '#generated-view'
  },
  {
    id: 'workspace:guardrails',
    group: 'Governance',
    label: 'Preview write guardrails',
    detail: 'IntentPreview only; no ServiceNow write token',
    href: '#guardrails'
  },
  {
    id: 'workspace:about',
    group: 'Workspace',
    label: 'Open About',
    detail: 'Visible B_IX consume identities',
    href: '#about'
  }
];

function StalledFlowGraph({ nodes, edges }: { nodes: readonly FlowGraphNode[]; edges: readonly FlowGraphEdge[] }) {
  return (
    <div className="app-flow-proof" aria-label="Stalled task flow proof">
      <div className="app-flow-proof__nodes">
        {nodes.map((node) => (
          <article key={node.id} className="app-flow-proof__node" data-status={node.status}>
            <strong>{node.label}</strong>
            {node.detail ? <span>{node.detail}</span> : null}
          </article>
        ))}
      </div>
      <ol className="app-flow-proof__edges" aria-label="Flow relationships">
        {edges.map((edge) => (
          <li key={edge.id}>
            <span>{edge.from}</span>
            <strong>{edge.label}</strong>
            <span>{edge.to}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}

function App() {
  const [reviewOpen, setReviewOpen] = useState(false);
  const [density, setDensity] = useState<PdsDensity>('compact');
  const [showOnlyStalled, setShowOnlyStalled] = useState(true);
  const [chatPrompt, setChatPrompt] = useState('What tasks are stalled?');
  const [locationHash, setLocationHash] = useState(
    typeof window === 'undefined' ? '' : window.location.hash
  );

  useEffect(() => {
    const onHashChange = () => setLocationHash(window.location.hash);
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  const filteredRows = showOnlyStalled
    ? denovoRows.filter((row) => row.daysStalled >= 5)
    : denovoRows;

  const conversationMessages: MessageItem[] = [
    {
      id: 'msg-user-stalled',
      role: 'user',
      author: 'Nexus pilot user',
      timestamp: 'Local fixture',
      content: 'What tasks are stalled?'
    },
    {
      id: 'msg-assistant-stalled',
      role: 'assistant',
      author: 'PDS Nexus assistant',
      timestamp: 'answer_envelope@1',
      status: <Badge tone="success">Grounded</Badge>,
      metadata: <ConfidenceSignal value="high" detail="All shown refs resolve inside tenant_a." />,
      content: (
        <div className="app-chat-answer">
          <StreamingText>
            Three tenant-scoped De Novo tasks are stalled. The highest-risk item is the credentialing packet,
            stalled for eight days with Provider onboarding.
          </StreamingText>
          <ToolCallStatus
            label="Resolved answer envelope"
            status="completed"
            detail="record_locator refs resolved through the product contract; writes remain disabled."
            requestId="chat-eval-stalled-tasks-basic"
          />
          <div className="app-ref-grid">
            {stalledRows.map((row) => (
              <EntityRefCard
                key={row.recordLocator}
                title={row.task}
                entityType="DeNovoTask"
                caption={`${row.ownerTeam} - ${row.phase}`}
                recordLocator={row.recordLocator}
                href={`/data/de_novo_tasks/${row.recordLocator}`}
                status={<Badge tone={row.risk === 'High' ? 'danger' : 'warning'}>{row.risk}</Badge>}
              />
            ))}
          </div>
        </div>
      )
    }
  ];

  if (locationHash === '#about') {
    return (
      <AppShell
        brand={(
          <div className="app-brand">
            <strong>{appUiContract.displayName}</strong>
            <span>{appUiContract.schemaLabel} pilot</span>
          </div>
        )}
        navigation={(
          <div className="app-nav" aria-label="Workspace sections">
            <a href="#my-work-ix">My Work IX</a>
            <a href="#tracker">Tracker</a>
            <a href="#chat">AI answer</a>
            <a href="#generated-view">Graph</a>
            <a href="#model">Model</a>
            <a href="#evidence">Evidence</a>
            <a href="#guardrails">Guardrails</a>
            <a href="#about">About</a>
          </div>
        )}
        topBar={(
          <div className="app-topbar">
            <CommandPalette
              items={workspaceCommands}
              triggerLabel="Search Nexus pilot"
              searchPlaceholder="Search pilot commands"
            />
          </div>
        )}
        footer={(
          <div className="app-shell-footer">
            <span>Synthetic pilot</span>
            <strong>{appUiContract.provider}</strong>
          </div>
        )}
      >
        <AboutPage />
      </AppShell>
    );
  }

  return (
    <AppShell
      brand={(
        <div className="app-brand">
          <strong>{appUiContract.displayName}</strong>
          <span>{appUiContract.schemaLabel} pilot</span>
        </div>
      )}
      navigation={(
        <div className="app-nav" aria-label="Workspace sections">
          <a href="#my-work-ix">My Work IX</a>
          <a href="#tracker">Tracker</a>
          <a href="#chat">AI answer</a>
          <a href="#generated-view">Graph</a>
          <a href="#model">Model</a>
          <a href="#evidence">Evidence</a>
          <a href="#guardrails">Guardrails</a>
          <a href="#about">About</a>
        </div>
      )}
      topBar={(
        <div className="app-topbar">
          <CommandPalette
            items={workspaceCommands}
            triggerLabel="Search Nexus pilot"
            searchPlaceholder="Search pilot commands"
          />
        </div>
      )}
      footer={(
        <div className="app-shell-footer">
          <span>Synthetic pilot</span>
          <strong>{appUiContract.provider}</strong>
        </div>
      )}
    >
      <PageHeader
        eyebrow={appUiContract.schemaLabel}
        title="De Novo stalled-task pilot"
        subtitle="Local CH8 consumer-wiring proof; synthetic fixture only; not connected to ServiceNow"
        actions={<Badge tone="accent">Preview only</Badge>}
      />

      <MyWorkIntelligentExperience />

      <div className="app-status-grid" aria-label="Workspace status">
        <Surface id="model" title="Projection model" subtitle="Provisional until Unit A export" density="compact">
          <p>Read-only De Novo task, workstream, team, and milestone projection.</p>
          <Badge>{appUiContract.modelStatus}</Badge>
        </Surface>
        <Surface id="evidence" title="Source evidence" subtitle="Synthetic seed shape" density="compact">
          <p>176 tasks, 20 teams, tenant-scoped fixture refs; freshness watermark 2026-07-02.</p>
          <Badge tone="success">Synthetic</Badge>
        </Surface>
        <Surface id="guardrails" title="Governed writes" subtitle="No live write execution" density="compact">
          <p>Recommendations open Intent Preview only. G1 evidence is still required.</p>
          <Badge tone="warning">Blocked by Unit A</Badge>
        </Surface>
      </div>

      <div className="app-kpi-grid" aria-label="Pilot metrics">
        <KpiTile
          label="Synthetic tasks"
          value="176"
          detail="Modeled from De Novo tracker shape"
          tone="accent"
          trend={<MetricTrend value="20" label="teams" tone="accent" direction="flat" />}
        />
        <KpiTile
          label="Stalled"
          value="14"
          detail="Six or more days without movement"
          tone="warning"
          trend={<MetricTrend value="+3" label="this week" tone="warning" direction="up" />}
        />
        <KpiTile
          label="Write posture"
          value="0"
          detail="No executable SaaS writes in pilot"
          tone="success"
          trend={<MetricTrend value="preview" label="only" tone="positive" direction="flat" />}
        />
      </div>

      <div className="app-chat-grid" id="chat">
        <Surface title="Stalled-task answer" subtitle="Native chat surface over local deterministic fixture" density="compact">
          <MessageThread
            title="Ask Nexus"
            description="Static CH8 proof: chat -> answer envelope -> resolved refs -> generated view."
            messages={conversationMessages}
            actions={<Badge tone="success">red_team.leaks_found = 0</Badge>}
          />
          <MessageComposer
            value={chatPrompt}
            disabled
            helperText="Live chat execution is disabled until CH1-CH7 and release evidence graduate."
            submitLabel="Disabled"
            onValueChange={setChatPrompt}
          />
        </Surface>

        <div className="app-chat-sidecar">
          <EvidenceSummary
            title="Why this answer is safe to show"
            description="Every claim is grounded in local synthetic source evidence and typed refs."
            claims={evidenceClaims}
            freshness={<FreshnessIndicator value="current" label="Fixture freshness" timestamp="2026-07-02" />}
            citations={<CitationList citations={stalledCitations} />}
          />
          <Surface title="Freshness and lineage" subtitle="Where the proof stops" density="compact">
            <ol className="app-lineage-list" aria-label="Nexus source lineage">
              {sourceLineage.map((item) => (
                <li key={item.stage}>
                  <span>{item.stage}</span>
                  <strong>{item.evidence}</strong>
                  <p>{item.detail}</p>
                  <Badge tone="accent">{item.boundary}</Badge>
                </li>
              ))}
            </ol>
          </Surface>
          <AgentTimeline
            title="Agent timeline"
            description="Persistent workflow trace for the local fixture turn."
            items={timelineItems}
          />
        </div>
      </div>

      <GeneratedViewShell
        id="generated-view"
        title="Generated stalled-task flow"
        description="FlowGraphShell composition over resolved tenant_a record locators."
        viewId="nexus.stalledTasks.flowGraph"
        grounding="grounded"
        attribution={(
          <AiAttributionAffordance
            detail="Composed from synthetic De Novo projection refs."
            model="local deterministic fixture"
            generatedAt="2026-07-02"
            sourceCount={stalledRows.length}
          />
        )}
        freshness={<FreshnessIndicator value="current" label="Source freshness" timestamp="2026-07-02" />}
        confidence={<ConfidenceSignal value="high" detail="Refs resolved with tenant_a policy context." />}
        toolbar={<Badge tone="accent">viewRegistry compatible</Badge>}
      >
        <FlowGraphShell
          title="Stalled task dependencies"
          description="Renderer-neutral proof; product can swap in @xyflow/react behind the PDS shell."
          nodes={stalledFlowNodes}
          edges={stalledFlowEdges}
          renderGraph={StalledFlowGraph}
          legend={<span>Blocked = high risk; waiting = stalled but recoverable.</span>}
        />
      </GeneratedViewShell>

      <div className="app-recommendation-grid">
        <RecommendationCard
          rank="1"
          title="Create a follow-up ticket for credentialing"
          reason="Credentialing packet is the highest-risk stalled task and has not moved for eight days."
          risk="medium"
          grounding="grounded"
          evidence={<span>Evidence: rl_11111111111111111111111111111111</span>}
          actions={<Button variant="primary" onClick={() => setReviewOpen(true)}>Open Intent Preview</Button>}
        />
        <AttentionMarker
          tone="warning"
          risk="medium"
          label="Preview-only execution"
          detail="The recommendation cannot write to ServiceNow without G1 delegated auth and Unit A evidence."
        />
      </div>

      <div className="app-work-grid" id="tracker">
        <Surface title="Tracker controls" subtitle="Read-only filters over synthetic source evidence" density="compact">
          <ValidationSummary validation={trackerValidation} />
          <FormLayout
            columns="two"
            footer={(
              <>
                <Button variant="quiet" onClick={() => setShowOnlyStalled(false)}>Show all</Button>
                <Button variant="primary" onClick={() => setReviewOpen(true)}>Preview recommendation</Button>
              </>
            )}
          >
            <SelectField
              label="Workstream"
              defaultValue="all"
              options={workstreamOptions}
            />
            <TextField label="Tenant" defaultValue="tenant_a" readOnly hint="Tenant-scoped fixture context." />
            <DateField label="As of" defaultValue="2026-07-02" hint="Fixture freshness watermark." />
            <SwitchField
              id="stalled-only"
              label="Show stalled only"
              checked={showOnlyStalled}
              onCheckedChange={setShowOnlyStalled}
              detail="Filters rows with five or more stalled days."
            />
          </FormLayout>
        </Surface>

        <Surface title="De Novo task queue" subtitle="Compact grid over generated product shape" density="compact">
          <DataGridToolbar
            ariaLabel="De Novo task controls"
            search={<input className="app-search" type="search" aria-label="Search De Novo tasks" placeholder="Search tasks" />}
            summary={(
              <>
                <strong>{filteredRows.length} shown</strong>
                <span>{density} density</span>
              </>
            )}
            actions={<DataGridDensityControl value={density} onChange={setDensity} label="Density" />}
            density={density}
          />
          <DataGridShell
            columns={denovoColumns}
            rows={filteredRows}
            rowKey="id"
            ariaLabel="De Novo task queue"
            density={density}
          />
          <DataGridPagination pageSize={25} pageIndex={1} startRow={1} endRow={filteredRows.length} totalRows={filteredRows.length} responseMs={28} />
        </Surface>
      </div>

      <ChartShell
        title="Readiness distribution"
        subtitle="Synthetic 176-task shape across 20 teams"
        footer={<ChartLegend items={readinessLegend} />}
      >
        <div className="app-chart-bars" aria-label="Readiness distribution">
          <span className="app-chart-bar is-ready"><b>116 on track</b></span>
          <span className="app-chart-bar is-review"><b>46 watch</b></span>
          <span className="app-chart-bar is-draft"><b>14 stalled</b></span>
        </div>
      </ChartShell>

      <Dialog
        open={reviewOpen}
        title="Preview recommendation"
        description="The pilot can propose the shape of a follow-up, but it cannot execute a ServiceNow write."
        onClose={() => setReviewOpen(false)}
        closeLabel="Close recommendation preview"
        footer={<Button variant="primary" onClick={() => setReviewOpen(false)}>Close preview</Button>}
      >
        <IntentPreview
          title="Create follow-up ticket"
          description="Preview-only recommendation for the credentialing packet stalled eight days."
          actor="service:pds-nexus-preview"
          policy="Blocked until G1 delegated auth and ServiceNow Unit A evidence exist."
          confidence="Synthetic"
          evidence="rl_11111111111111111111111111111111"
          changes={[
            {
              id: 'ticket',
              label: 'Proposed write',
              before: 'No ticket',
              after: 'Draft ServiceNow follow-up',
              tone: 'warning'
            },
            {
              id: 'execution',
              label: 'Execution mode',
              before: 'None',
              after: 'Intent preview only',
              tone: 'accent'
            }
          ]}
          actions={<Button variant="quiet" disabled>Confirm disabled</Button>}
        />
      </Dialog>
    </AppShell>
  );
}

createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
