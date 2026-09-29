import { useState, type FC, type ReactElement } from "react";
import {
  // Actions
  Button,
  ButtonLink,
  ButtonGroup,
  ToggleButton,
  FloatingActionButton,
  IconButton,
  MenuButton,
  IntentPreview,
  ActionAudit,
  UndoCompensationState,
  MessageThread,
  Message,
  MessageComposer,
  StreamingText,
  ToolCallStatus,
  EntityRefCard,
  CitationList,
  ConfidenceSignal,
  AgentTimeline,
  FlowGraphShell,
  GeneratedViewShell,
  SuggestedAction,
  RecommendationCard,
  EvidenceSummary,
  InsightSummary,
  FreshnessIndicator,
  AttentionMarker,
  AiAttributionAffordance,
  AssistLevelControl,
  MemoryChip,
  CommandBar,
  CommandPalette,
  ConversationWorkspace,
  // Forms
  Field,
  FieldMetadata,
  FormLoadingPreview,
  FieldGroup,
  FormLayout,
  TextField,
  ComboboxField,
  TextArea,
  SelectField,
  DateField,
  TimeField,
  DateTimeField,
  DatePicker,
  TimePicker,
  InputGroup,
  MultiSelect,
  FileUpload,
  CheckboxField,
  SwitchField,
  LookupSelect,
  ValidationSummary,
  // Data grid
  DataGrid,
  DataGridShell,
  DataGridLoadingPreview,
  DataGridToolbar,
  DataGridColumnChooser,
  DataGridColumnChooserTrigger,
  DataGridColumnResizeHandle,
  DataGridControlPopover,
  DataGridDensityControl,
  DataGridFilterPanel,
  DataGridFilterGroup,
  DataGridFilterRule,
  DataGridFilterEmpty,
  DataGridFilterTrigger,
  DataGridSortButton,
  DataGridPagination,
  // Overlays
  Dialog,
  Drawer,
  Popover,
  PopoverTrigger,
  Tooltip,
  ConfirmDialog,
  // Feedback
  Alert,
  Banner,
  FeedbackState,
  OperationState,
  LoadingState,
  ErrorState,
  ForbiddenState,
  InlineAlert,
  EmptyState,
  Skeleton,
  Toast,
  ToastRegion,
  // Navigation
  AppShell,
  NarrativeWorkspace,
  AppearanceProvider,
  PdsHealthLogo,
  NavigationItem,
  IconSlot,
  Avatar,
  IdentitySummary,
  Breadcrumbs,
  PageHeader,
  Surface,
  Tabs,
  SegmentedControl,
  Badge,
  List,
  ListItem,
  SearchBar,
  ExplorationWorkspace,
  Toolbar,
  CardLink,
  InteractiveCard,
  WorkQueueItem,
  // Process
  ProcessProgress,
  ProcessStepper,
  // Analytics
  TimelineRangeSelector,
  RelationshipAtlas,
  RelationshipAtlasTable,
  RelationshipExplorer,
  KpiTile,
  MetricTrend,
  ChartShell,
  ChartLegend,
  BarChart,
  LineChart,
  AreaChart,
  DonutChart,
  type PdsAdvancedDataGridColumn,
  type PdsDataGridColumn,
  type PdsDensity,
  type PdsOption,
  type ToastItem
} from "@appfw/pds-health-components";
import {
  EvidenceDisclosure,
  ProgressiveResponse,
  ResolvedContextDisclosure,
  WorkStatus
} from "@appfw/pds-health-components/intelligence-presentation";
import { PdsIxRecipePresentation } from "@appfw/pds-health-components/ix-recipes";

// ---------------------------------------------------------------------------
// Shared demo data. Keep these examples product-neutral so the catalog does
// not become a source for copied fixture language.
// ---------------------------------------------------------------------------
type DemoRow = {
  id: string;
  record: string;
  stage: string;
  owner: string;
  value: number;
  updated: string;
};

const demoRows: DemoRow[] = [
  { id: "r-1", record: "Record Alpha", stage: "Draft", owner: "A. Rivera", value: 48200, updated: "2026-05-30" },
  { id: "r-2", record: "Record Beta", stage: "Review", owner: "J. Okafor", value: 119500, updated: "2026-06-02" },
  { id: "r-3", record: "Record Gamma", stage: "Ready", owner: "M. Chen", value: 7600, updated: "2026-06-08" },
  { id: "r-4", record: "Record Delta", stage: "Approved", owner: "P. Singh", value: 264000, updated: "2026-06-09" },
  { id: "r-5", record: "Record Epsilon", stage: "Review", owner: "T. Brooks", value: 31750, updated: "2026-06-11" },
  { id: "r-6", record: "Record Zeta", stage: "Ready", owner: "D. Alvarez", value: 88400, updated: "2026-06-12" },
  { id: "r-7", record: "Record Eta", stage: "Draft", owner: "S. Patel", value: 52600, updated: "2026-06-13" },
  { id: "r-8", record: "Record Theta", stage: "Approved", owner: "R. Wilson", value: 141200, updated: "2026-06-14" },
  { id: "r-9", record: "Record Iota", stage: "Ready", owner: "K. Morgan", value: 19750, updated: "2026-06-15" },
  { id: "r-10", record: "Record Kappa", stage: "Review", owner: "L. Nguyen", value: 97500, updated: "2026-06-16" },
  { id: "r-11", record: "Record Lambda", stage: "Draft", owner: "C. Davis", value: 68400, updated: "2026-06-17" },
  { id: "r-12", record: "Record Mu", stage: "Approved", owner: "E. Taylor", value: 215300, updated: "2026-06-18" }
];

const demoColumns: PdsDataGridColumn<DemoRow>[] = [
  { key: "record", header: "Record" },
  { key: "stage", header: "Stage" },
  { key: "owner", header: "Owner" },
  { key: "value", header: "Value", align: "end" },
  { key: "updated", header: "Updated", align: "end" }
];

const relationshipGroups = [
  { id: "inputs", label: "Inputs", detail: "Grounded sources" },
  { id: "decisions", label: "Decisions", detail: "Governed choices" },
  { id: "outcomes", label: "Outcomes", detail: "Measured effects" }
] as const;

const relationshipNodes = [
  { id: "evidence", label: "Evidence", group: "inputs", kind: "source", accent: "blue" as const },
  { id: "constraint", label: "Constraint", group: "inputs", kind: "source", accent: "neutral" as const },
  { id: "choice", label: "Portfolio choice", group: "decisions", kind: "decision", accent: "violet" as const },
  { id: "measure", label: "Success measure", group: "outcomes", kind: "outcome", accent: "teal" as const }
] as const;

const relationshipEdges = [
  { source: "evidence", target: "choice", kind: "supports" },
  { source: "constraint", target: "choice", kind: "bounds" },
  { source: "choice", target: "measure", kind: "evaluated by" }
] as const;

const RelationshipAtlasExample: FC = () => {
  const [selectedId, setSelectedId] = useState<string | null>("choice");
  return (
    <RelationshipAtlas
      ariaLabel="Decision relationship map"
      groups={relationshipGroups}
      nodes={relationshipNodes}
      edges={relationshipEdges}
      selectedId={selectedId}
      onSelectNode={setSelectedId}
    />
  );
};

const RelationshipAtlasTableExample: FC = () => {
  const [selectedId, setSelectedId] = useState<string | null>("choice");
  return (
    <RelationshipAtlasTable
      caption="Decision relationships"
      groups={relationshipGroups}
      nodes={relationshipNodes}
      edges={relationshipEdges}
      selectedId={selectedId}
      onSelectNode={setSelectedId}
    />
  );
};

const RelationshipExplorerExample: FC = () => {
  const [selectedId, setSelectedId] = useState<string | null>("choice");
  return (
    <RelationshipExplorer
      ariaLabel="Decision relationships"
      tableCaption="Decision relationships"
      groups={relationshipGroups}
      nodes={relationshipNodes}
      edges={relationshipEdges}
      selectedId={selectedId}
      onSelectNode={setSelectedId}
    />
  );
};

const ExplorationWorkspaceExample: FC = () => {
  const [selectedId, setSelectedId] = useState<string | null>("choice");
  const selectedNode = relationshipNodes.find((node) => node.id === selectedId);

  return (
    <ExplorationWorkspace
      title="System explorer"
      description="Move from object families to focused context and exact evidence without losing your place."
      headerActions={<Badge tone="accent">Interactive</Badge>}
      controls={<TextField label="Find an object" type="search" placeholder="Search this system" />}
      navigator={(
        <List variant="segmented">
          <ListItem headline="Inputs" supportingText="2 objects" />
          <ListItem headline="Decisions" supportingText="1 object" selected />
          <ListItem headline="Outcomes" supportingText="1 object" />
        </List>
      )}
      focusRegion={(
        <RelationshipExplorer
          ariaLabel="Decision relationships"
          tableCaption="Decision relationships"
          groups={relationshipGroups}
          nodes={relationshipNodes}
          edges={relationshipEdges}
          selectedId={selectedId}
          onSelectNode={setSelectedId}
        />
      )}
      inspector={(
        <div className="example-stack">
          <Badge tone="accent">Selected object</Badge>
          <strong>{selectedNode?.label ?? "No object selected"}</strong>
          <span className="example-note">
            Object context stays visible beside the focus view as the selection changes.
          </span>
        </div>
      )}
      evidenceRail={(
        <span className="example-note">
          Evidence context · authority · source · freshness · release identity
        </span>
      )}
    />
  );
};

const NarrativeWorkspaceExample: FC = () => {
  const [mode, setMode] = useState("story");

  return (
    <NarrativeWorkspace
      className="example-narrative-workspace"
      brand={(
        <span className="example-stack">
          <strong>Knowledge application</strong>
          <span className="example-note">Persistent orientation</span>
        </span>
      )}
      utilities={<Button size="sm">Ask</Button>}
      primaryNavigationLabel="Knowledge experience"
      primaryNavigation={(
        <SegmentedControl
          ariaLabel="Knowledge experience mode"
          value={mode}
          options={[
            { value: "story", label: "Story" },
            { value: "explore", label: "Explore" },
            { value: "apply", label: "Apply" }
          ]}
          onValueChange={setMode}
        />
      )}
      contextBand={<Badge tone="neutral">Role lens · All readers</Badge>}
      contextBandLabel="Reader context"
    >
      <PageHeader
        eyebrow="Document-scrolling floor plan"
        title="Keep the story and its controls in view"
        subtitle="The product supplies modes, role meaning, content, routes, evidence, and focus restoration."
      />
      <Surface title={mode === "story" ? "Orient" : mode === "explore" ? "Understand" : "Act"}>
        <p className="example-note">
          Long-form content keeps the browser document as its single scroll owner while the header and experience dock remain available.
        </p>
      </Surface>
    </NarrativeWorkspace>
  );
};

const advancedDemoColumns: PdsAdvancedDataGridColumn<DemoRow>[] = [
  { key: "record", header: "Record", description: "Stable record label", minWidth: 180, flex: 1.4 },
  { key: "stage", header: "Stage", minWidth: 132 },
  { key: "owner", header: "Owner", minWidth: 160 },
  { key: "value", header: "Value", type: "number", align: "end", minWidth: 132 },
  { key: "updated", header: "Updated", align: "end", minWidth: 132 },
  {
    key: "id",
    header: "Actions",
    sortable: false,
    filterable: false,
    minWidth: 280,
    render: (row) => (
      <span
        data-grid-embedded-actions
        style={{ alignItems: "center", display: "inline-flex", gap: "var(--pds-space-2)", whiteSpace: "nowrap" }}
      >
        <a href={`#data-grid-record-${row.id}`}>Open {row.record}</a>
        <label htmlFor={`data-grid-action-${row.id}`}>Action</label>
        <select id={`data-grid-action-${row.id}`} defaultValue="view">
          <option value="view">View</option>
          <option value="archive">Archive</option>
        </select>
        <span
          role="button"
          tabIndex={0}
          aria-label={`Flag ${row.record}`}
          data-activated="false"
          onClick={(event) => {
            event.currentTarget.dataset.activated = "true";
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.currentTarget.dataset.activated = "true";
            }
          }}
        >
          Flag
        </span>
      </span>
    )
  }
];

const demoOptions: PdsOption[] = [
  { value: "draft", label: "Draft" },
  { value: "review", label: "Review" },
  { value: "ready", label: "Ready" },
  { value: "approved", label: "Approved" }
];

function Glyph(): ReactElement {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M3 8h10M8 3v10" strokeLinecap="round" />
    </svg>
  );
}

function Row({ children }: { children: ReactElement | ReactElement[] }): ReactElement {
  return <div className="example-row">{children}</div>;
}

// ---------------------------------------------------------------------------
// Stateful example wrappers
// ---------------------------------------------------------------------------
const SegmentedControlExample: FC = () => {
  const [value, setValue] = useState("table");
  return (
    <SegmentedControl
      ariaLabel="View"
      value={value}
      onValueChange={setValue}
      options={[
        { value: "table", label: "Table" },
        { value: "board", label: "Board" },
        { value: "calendar", label: "Calendar" }
      ]}
    />
  );
};

const ToggleButtonExample: FC = () => {
  const [selected, setSelected] = useState(false);
  return (
    <ToggleButton selected={selected} onSelectedChange={setSelected} variant="outlined">
      Filter results
    </ToggleButton>
  );
};

const SearchBarExample: FC = () => (
  <SearchBar
    className="example-search-bar"
    label="Search workspace"
    placeholder="Search records, people, and actions"
    defaultValue="rec"
    defaultOpen
    items={[
      { id: "record-alpha", label: "Record Alpha", detail: "Recently updated", keywords: ["record", "review"] },
      { id: "record-beta", label: "Record Beta", detail: "Approval pending", keywords: ["record", "approval"] },
      { id: "create-record", label: "Create record", detail: "Suggested action", keywords: ["new", "action"] }
    ]}
  />
);

const MultiSelectExample: FC = () => {
  const [value, setValue] = useState<string[]>(["review"]);
  return <MultiSelect label="Stages" options={demoOptions} value={value} onValueChange={setValue} />;
};

const ComboboxFieldExample: FC = () => {
  const [value, setValue] = useState<string | null>("review");
  const [inputValue, setInputValue] = useState("Review");
  return (
    <ComboboxField
      id="catalog-combobox-stage"
      label="Stage"
      hint="Filter the neutral fixture stages."
      options={demoOptions}
      value={value}
      onValueChange={setValue}
      inputValue={inputValue}
      onInputValueChange={setInputValue}
      placeholder="Filter stages"
    />
  );
};

const LookupSelectExample: FC = () => {
  const [value, setValue] = useState<string | string[]>("");
  return (
    <LookupSelect
      aria-label="Owner lookup"
      options={[
        { value: "a-rivera", label: "A. Rivera" },
        { value: "j-okafor", label: "J. Okafor" },
        { value: "m-chen", label: "M. Chen" }
      ]}
      value={value}
      onValueChange={setValue}
      placeholder="Assign owner"
    />
  );
};

const SwitchFieldExample: FC = () => {
  const [checked, setChecked] = useState(true);
  return (
    <SwitchField
      id="example-switch"
      label="Active record"
      detail="Visible in default views"
      checked={checked}
      onCheckedChange={setChecked}
    />
  );
};

const AssistLevelControlExample: FC = () => {
  const [value, setValue] = useState<"suggest" | "copilot" | "autopilot">("suggest");
  return (
    <AssistLevelControl
      value={value}
      onValueChange={setValue}
      name="catalog-assist-level"
      detail="Choose per task type; Autopilot remains disabled until live governed-write evidence exists."
    />
  );
};

const TabsExample: FC = () => {
  const [tab, setTab] = useState("overview");
  return (
    <Tabs
      ariaLabel="Record sections"
      selectedId={tab}
      onChange={setTab}
      items={[
        { id: "overview", label: "Overview", content: <p className="example-note">Summary fields.</p> },
        { id: "events", label: "Events", content: <p className="example-note">Recent events.</p> },
        { id: "related", label: "Related", content: <p className="example-note">Linked records.</p> }
      ]}
    />
  );
};

const DialogExample: FC = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="primary" onClick={() => setOpen(true)}>Open dialog</Button>
      <Dialog
        open={open}
        title="Edit record"
        description="Changes apply after you save."
        onClose={() => setOpen(false)}
        footer={<Button variant="primary" onClick={() => setOpen(false)}>Save</Button>}
      >
        <p className="example-note">Focus is trapped here and returned to the trigger on close.</p>
      </Dialog>
    </>
  );
};

const DrawerExample: FC = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onClick={() => setOpen(true)}>Open drawer</Button>
      <Drawer open={open} title="Record details" side="right" onClose={() => setOpen(false)}>
        <p className="example-note">Side-panel work surface for contextual detail.</p>
      </Drawer>
    </>
  );
};

const ConfirmDialogExample: FC = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="danger" onClick={() => setOpen(true)}>Delete record</Button>
      <ConfirmDialog
        open={open}
        title="Delete record?"
        description="This action cannot be undone."
        tone="danger"
        confirmLabel="Delete"
        onConfirm={() => setOpen(false)}
        onCancel={() => setOpen(false)}
      />
    </>
  );
};

const PopoverExample: FC = () => {
  const [open, setOpen] = useState(false);
  return (
    <div className="example-popover">
      <PopoverTrigger controls="example-popover-panel" open={open} onClick={() => setOpen((value) => !value)}>
        Quick filters
      </PopoverTrigger>
      {open ? (
        <Popover id="example-popover-panel" ariaLabel="Quick filters" open={open} onClose={() => setOpen(false)}>
          <p className="example-note">Contextual controls anchored to a trigger.</p>
        </Popover>
      ) : null}
    </div>
  );
};

const ToastExample: FC = () => <Toast id="t-1" tone="success" title="Record saved" detail="All changes stored." />;

const ToastRegionExample: FC = () => {
  const [toasts, setToasts] = useState<ToastItem[]>([
    { id: "t-1", tone: "success", title: "Record saved" },
    { id: "t-2", tone: "warning", title: "Sync delayed", detail: "Retrying shortly." }
  ]);
  return (
    <div className="example-toast-region">
      <ToastRegion
        toasts={toasts}
        position="top-end"
        onDismiss={(id) => setToasts((items) => items.filter((item) => item.id !== id))}
      />
      {toasts.length === 0 ? (
        <Button onClick={() => setToasts([{ id: "t-1", tone: "success", title: "Record saved" }])}>
          Show toasts
        </Button>
      ) : null}
    </div>
  );
};

const messageItems = [
  {
    id: "m-1",
    role: "user" as const,
    author: "User",
    content: "Which records need review?",
    timestamp: "10:20"
  },
  {
    id: "m-2",
    role: "assistant" as const,
    author: "Assistant",
    content: (
      <StreamingText isStreaming>
        Three records need review. Two are blocked by missing evidence.
      </StreamingText>
    ),
    timestamp: "10:21",
    status: <ConfidenceSignal value="medium" detail="Grounded in generated refs" />
  }
];

const timelineItems = [
  { id: "t-1", title: "Parsed question", status: "completed" as const, timestamp: "10:20", detail: "Mapped to generated view refs" },
  { id: "t-2", title: "Retrieved records", status: "completed" as const, timestamp: "10:21", detail: "3 refs resolved" },
  { id: "t-3", title: "Write request blocked", status: "blocked" as const, timestamp: "10:22", detail: "IntentPreview only until G1 evidence exists" }
];

const flowNodes = [
  { id: "n-1", label: "Intake", detail: "2 records", status: "completed" as const },
  { id: "n-2", label: "Evidence review", detail: "1 blocked", status: "running" as const },
  { id: "n-3", label: "Approval", detail: "waiting", status: "waiting" as const }
];

const ambientClaims = [
  { id: "c-1", text: "Two records changed since the last review.", citation: "Resolved refs: 2" },
  { id: "c-2", text: "One proposed write remains preview-only.", citation: "Policy gate: G1" }
];

const ambientCitations = [
  { id: "a-1", label: "Generated ref 1", source: "Resolved record", detail: "Freshness watermark retained" },
  { id: "a-2", label: "Generated ref 2", source: "Resolved record", detail: "Policy context applied" }
];

const MessageComposerExample: FC = () => {
  const [value, setValue] = useState("Summarize the blocked records");
  return (
    <MessageComposer
      value={value}
      onValueChange={setValue}
      onSubmitMessage={() => setValue("")}
      helperText="Write actions surface as IntentPreview only."
    />
  );
};

const DensityControlExample: FC = () => {
  const [density, setDensity] = useState<PdsDensity>("compact");
  return <DataGridDensityControl value={density} onChange={setDensity} />;
};

const DataGridShellExample: FC = () => {
  const [density, setDensity] = useState<PdsDensity>("compact");
  const [selected, setSelected] = useState<string | null>("r-2");
  return (
    <div className="example-stack">
      <DataGridDensityControl value={density} onChange={setDensity} />
      <DataGridShell
        ariaLabel="Records"
        columns={demoColumns}
        rows={demoRows}
        rowKey="id"
        density={density}
        selectedRowKey={selected}
        onRowSelect={(row) => setSelected(row.id)}
      />
    </div>
  );
};

const DataGridExample: FC = () => {
  const [density, setDensity] = useState<PdsDensity>("compact");
  const [query, setQuery] = useState("");
  const [selectedCount, setSelectedCount] = useState(0);
  const [rows, setRows] = useState(demoRows);
  const [isLoading, setIsLoading] = useState(false);
  const [showEmpty, setShowEmpty] = useState(false);
  const [rowRevision, setRowRevision] = useState(0);
  const [activatedRow, setActivatedRow] = useState<string | null>(null);
  const [selectionMode, setSelectionMode] = useState<"single" | "multiple">("multiple");

  function replaceRows() {
    const nextRevision = rowRevision + 1;
    setRows((currentRows) => currentRows.map((row) => ({
      ...row,
      record: row.id === "r-1" ? `Record Alpha v${nextRevision}` : row.record
    })));
    setRowRevision(nextRevision);
  }

  return (
    <div className="example-stack">
      <div className="example-row" aria-label="Data grid proof controls">
        <Button variant="outlined" onClick={() => setIsLoading((value) => !value)}>
          {isLoading ? "Show data" : "Show loading"}
        </Button>
        <Button variant="outlined" onClick={() => setShowEmpty((value) => !value)}>
          {showEmpty ? "Restore rows" : "Show empty"}
        </Button>
        <Button variant="outlined" onClick={replaceRows}>Replace rows</Button>
        <Button
          variant="outlined"
          onClick={() => {
            setSelectedCount(0);
            setSelectionMode((value) => value === "multiple" ? "single" : "multiple");
          }}
        >
          {selectionMode === "multiple" ? "Use single selection" : "Use multiple selection"}
        </Button>
      </div>
      <DataGridToolbar
        ariaLabel="Advanced grid controls"
        density={density}
        search={(
          <label>
            <span className="visually-hidden">Filter records</span>
            <input
              type="search"
              placeholder="Filter records"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
        )}
        summary={`${showEmpty ? 0 : rows.length} records · ${selectedCount} selected`}
        actions={<DataGridDensityControl value={density} onChange={setDensity} />}
      />
      <DataGrid
        key={selectionMode}
        ariaLabel="Advanced records grid"
        columns={advancedDemoColumns}
        rows={showEmpty ? [] : rows}
        rowKey="id"
        density={density}
        isLoading={isLoading}
        quickFilterText={query}
        selectionMode={selectionMode}
        pagination
        pageSize={10}
        pageSizeOptions={[10, 25]}
        height={520}
        onRowActivate={(row) => setActivatedRow(row.id)}
        onSelectionChange={(selectedRows) => setSelectedCount(selectedRows.length)}
      />
      <output className="example-note" data-grid-activation-receipt>
        {activatedRow ? `Activated ${activatedRow}` : "No row activated"}
      </output>
      <p className="example-note">
        AG Grid Community runs behind the PDS adapter. Toggle visual theme and color mode above to inspect all four treatments.
      </p>
    </div>
  );
};

const ColumnChooserExample: FC = () => {
  const [selected, setSelected] = useState<string[]>(["record", "stage", "owner", "value"]);
  return (
    <DataGridColumnChooser
      groups={[
        {
          id: "core",
          title: "Core",
          options: [
            { id: "record", label: "Record" },
            { id: "stage", label: "Stage" },
            { id: "owner", label: "Owner" }
          ]
        },
        {
          id: "metrics",
          title: "Metrics",
          options: [
            { id: "value", label: "Value", detail: "Currency" },
            { id: "updated", label: "Updated", detail: "Date" }
          ]
        }
      ]}
      selectedIds={selected}
      onToggle={(id, checked) =>
        setSelected((current) => (checked ? [...current, id] : current.filter((value) => value !== id)))
      }
    />
  );
};

const ControlPopoverExample: FC = () => (
  <div className="example-control-popover">
    <DataGridColumnChooserTrigger
      selectedCount={4}
      totalCount={5}
      expanded
      controls="catalog-control-popover-preview"
    />
    <DataGridControlPopover
      id="catalog-control-popover-preview"
      ariaLabel="Column settings preview"
      size="sm"
      className="example-control-popover__panel"
    >
      <p className="example-note">Anchored control surface for grid settings.</p>
      <div className="example-row">
        <Button>Apply</Button>
        <Button variant="quiet">Reset</Button>
      </div>
    </DataGridControlPopover>
  </div>
);

const SortButtonExample: FC = () => {
  const [direction, setDirection] = useState<"asc" | "desc" | null>("desc");
  return (
    <DataGridSortButton
      label="Updated"
      ariaLabel="Sort by updated"
      direction={direction}
      onSort={() => setDirection((value) => (value === "desc" ? "asc" : value === "asc" ? null : "desc"))}
    />
  );
};

const FilterTriggerExample: FC = () => {
  const [count, setCount] = useState(2);
  return <DataGridFilterTrigger activeCount={count} onClick={() => setCount((value) => (value + 1) % 4)} />;
};

const ProcessStepperExample: FC = () => {
  const currentStepId = "verify";
  const [selectedStepId, setSelectedStepId] = useState<string | null>("approve");
  const steps = [
    {
      id: "intake",
      label: "Intake",
      description: "Collect required inputs",
      metadata: "2 checks"
    },
    {
      id: "verify",
      label: "Verify",
      description: "Run policy evidence",
      metadata: "Current",
      controls: "catalog-process-verify",
      content: <p className="example-note">Current-step content can remain mounted for review and keyboard access.</p>
    },
    {
      id: "approve",
      label: "Approve",
      description: "Human signoff",
      optional: true
    },
    {
      id: "certify",
      label: "Certify",
      description: "Release evidence",
      status: "blocked" as const,
      metadata: "Needs evidence"
    }
  ];

  return (
    <div className="example-stack">
      <ProcessStepper
        ariaLabel="Certification process"
        steps={steps}
        currentStepId={currentStepId}
        selectedStepId={selectedStepId}
        variant="milestone"
        onStepSelect={(step) =>
          setSelectedStepId((selected) => (selected === step.id ? null : step.id))
        }
      />
      <ProcessStepper
        ariaLabel="Responsive certification process"
        steps={steps.map((step) =>
          step.id === "verify"
            ? { ...step, controls: "catalog-process-verify-responsive" }
            : step
        )}
        currentStepId={currentStepId}
        compactPresentation="segments"
      />
      <ProcessStepper
        ariaLabel="Vertical certification process"
        orientation="vertical"
        density="compact"
        steps={steps.map((step) =>
          step.id === "verify"
            ? { ...step, controls: "catalog-process-verify-vertical" }
            : step
        )}
        currentStepId={currentStepId}
      />
    </div>
  );
};

const TimelineRangeSelectorExample: FC = () => {
  const [selection, setSelection] = useState({ startIndex: 6, width: 3 });
  const months = Array.from({ length: 12 }, (_, index) => ({
    id: `month-${index}`,
    label: new Date(2026, index, 1).toLocaleDateString("en-US", {
      month: "long",
      year: "numeric"
    }),
    scaleLabel: index % 3 === 0 ? `Q${Math.floor(index / 3) + 1}` : undefined,
    density: [1, 2, 0, 3, 1, 4, 2, 5, 3, 2, 4, 1][index]
  }));
  const endIndex = Math.min(
    months.length - 1,
    selection.startIndex + selection.width - 1
  );

  return (
    <TimelineRangeSelector
      ariaLabel="Delivery history window"
      items={months}
      startIndex={selection.startIndex}
      width={selection.width}
      selectionLabel={`${months[selection.startIndex]?.label} through ${months[endIndex]?.label}`}
      widthLabel={`${selection.width} months`}
      startBoundaryLabel="January 2026"
      endBoundaryLabel="December 2026"
      snapWidths={[1, 3, 6, 12]}
      onSelectionChange={setSelection}
    />
  );
};

const ConversationWorkspaceExample: FC = () => {
  const [draft, setDraft] = useState("");

  return (
    <ConversationWorkspace
      title="Ask workspace"
      description="Grounded guidance with product-owned prompts and actions."
      messages={[
        {
          id: "assistant-welcome",
          role: "assistant",
          author: "Assistant",
          content: "What would you like to review?"
        }
      ]}
      suggestedPrompts={[
        { id: "attention", value: "What needs attention?" },
        { id: "owner", value: "Who owns the next step?" }
      ]}
      onSuggestedPromptSelect={setDraft}
      composerProps={{
        value: draft,
        minRows: 1,
        placeholder: "Message the assistant",
        onValueChange: setDraft,
        onSubmitMessage: () => setDraft("")
      }}
    />
  );
};

const ProcessProgressExample: FC = () => (
  <div className="example-stack">
    <ProcessProgress
      label="Certification progress"
      detail="3 of 5"
      currentStep={3}
      totalSteps={5}
      ariaLabel="Certification progress"
    />
    <ProcessProgress
      label="Mobile flow"
      detail="2 of 4"
      currentStep={2}
      totalSteps={4}
      variant="dots"
      ariaLabel="Mobile flow progress"
    />
    <ProcessProgress
      label="Compact workflow"
      detail="Step 2 of 4"
      currentStep={2}
      totalSteps={4}
      variant="segments"
      ariaLabel="Compact workflow progress"
    />
  </div>
);

const chartLegendItems = [
  { id: "ready", label: "Ready", tone: "success" as const, value: "42%" },
  { id: "review", label: "Review", tone: "accent" as const, value: "38%" },
  { id: "blocked", label: "Blocked", tone: "danger" as const, value: "20%" }
];

const ChartVariationsExample: FC = () => (
  <div className="example-chart-variations">
    <ChartShell title="Bar trend" subtitle="Six periods" footer={<ChartLegend items={chartLegendItems.slice(0, 2)} />}>
      <div className="example-chart example-chart--bars" role="img" aria-label="Bar chart variation">
        {[40, 70, 55, 90, 65, 78].map((height, index) => (
          <span key={index} style={{ height: `${height}%` }} />
        ))}
      </div>
    </ChartShell>
    <ChartShell title="Stacked status" subtitle="By stage">
      <div className="example-chart-stacked" role="img" aria-label="Stacked bar chart variation">
        {[72, 54, 88].map((ready, index) => (
          <span key={index} className="example-chart-stacked__bar">
            <span style={{ width: `${ready}%` }} />
            <span style={{ width: `${100 - ready}%` }} />
          </span>
        ))}
      </div>
    </ChartShell>
    <ChartShell title="Line trend" subtitle="Throughput">
      <svg className="example-chart-line" viewBox="0 0 240 120" role="img" aria-label="Line chart variation">
        <path d="M16 96 L58 70 L100 80 L142 42 L184 54 L224 24" />
        <g>
          {[["16", "96"], ["58", "70"], ["100", "80"], ["142", "42"], ["184", "54"], ["224", "24"]].map(([cx, cy]) => (
            <circle key={`${cx}-${cy}`} cx={cx} cy={cy} r="4" />
          ))}
        </g>
      </svg>
    </ChartShell>
    <ChartShell title="Completion mix" subtitle="Current scope" footer={<ChartLegend items={chartLegendItems} />}>
      <div className="example-chart-donut" role="img" aria-label="Donut chart variation">
        <svg viewBox="0 0 120 120" aria-hidden="true">
          <circle className="example-chart-donut__base" cx="60" cy="60" r="42" />
          <circle className="example-chart-donut__slice example-chart-donut__slice--ready" cx="60" cy="60" r="42" />
          <circle className="example-chart-donut__slice example-chart-donut__slice--review" cx="60" cy="60" r="42" />
          <circle className="example-chart-donut__slice example-chart-donut__slice--blocked" cx="60" cy="60" r="42" />
        </svg>
        <strong>80%</strong>
      </div>
    </ChartShell>
    <ChartShell title="Sparkline" subtitle="Compact metric">
      <div className="example-chart-sparkline" role="img" aria-label="Sparkline chart variation">
        <svg viewBox="0 0 160 48" aria-hidden="true">
          <path d="M4 36 L30 28 L56 32 L82 18 L108 22 L134 12 L156 16" />
        </svg>
        <MetricTrend value="+8%" label="7d" direction="up" tone="positive" />
      </div>
    </ChartShell>
  </div>
);

// ---------------------------------------------------------------------------
// Example registry — one live, real-component preview per catalog entry.
// ---------------------------------------------------------------------------
export const componentExamples: Record<string, FC> = {
  // Actions
  Button: () => (
    <Row>
      <Button variant="filled">Filled</Button>
      <Button variant="tonal">Tonal</Button>
      <Button variant="outlined">Outlined</Button>
      <Button variant="text">Text</Button>
      <Button variant="elevated">Elevated</Button>
      <Button variant="danger">Danger</Button>
    </Row>
  ),
  ButtonLink: () => (
    <Row>
      <ButtonLink href="#records" variant="primary">Open records</ButtonLink>
      <ButtonLink href="#evidence" variant="secondary">Review evidence</ButtonLink>
    </Row>
  ),
  ToggleButton: ToggleButtonExample,
  FloatingActionButton: () => (
    <Row>
      <FloatingActionButton icon={<Glyph />} ariaLabel="Create" />
      <FloatingActionButton icon={<Glyph />} label="Create" tone="tertiary" />
    </Row>
  ),
  ButtonGroup: () => (
    <ButtonGroup variant="connected" ariaLabel="Record view">
      <Button variant="tonal">Table</Button>
      <Button variant="outlined">Board</Button>
      <Button variant="outlined">Timeline</Button>
    </ButtonGroup>
  ),
  IconButton: () => (
    <Row>
      <IconButton ariaLabel="Add" icon={<Glyph />} />
      <IconButton ariaLabel="Add" variant="primary" icon={<Glyph />} />
      <IconButton ariaLabel="Add" variant="danger" icon={<Glyph />} />
    </Row>
  ),
  MenuButton: () => (
    <MenuButton
      label="Actions"
      menuTone="vibrant"
      items={[
        { id: "export", label: "Export", description: "Download as CSV", badge: "CSV", selected: true },
        { id: "share", label: "Share" },
        { id: "delete", label: "Delete", tone: "danger" }
      ]}
    />
  ),
  Toolbar: () => (
    <div className="example-stack">
      <Toolbar ariaLabel="Editing tools" variant="floating">
        <IconButton ariaLabel="Add" variant="text" icon={<Glyph />} />
        <Button variant="text">Review</Button>
        <MenuButton label="More" variant="text" items={[{ id: "export", label: "Export" }]} />
      </Toolbar>
      <Toolbar ariaLabel="Primary tools" variant="vibrant">
        <Button variant="text">Back</Button>
        <Button variant="filled">Next</Button>
      </Toolbar>
    </div>
  ),
  IntentPreview: () => (
    <IntentPreview
      title="Preview generated write"
      description="Review the actor, policy result, and proposed field changes before confirming."
      actor="service:product-agent"
      policy="allowed by generated policy"
      confidence="review"
      evidence="req_8f2c"
      changes={[
        { id: "stage", label: "Stage", before: "Review", after: "Approved", tone: "success" },
        { id: "owner", label: "Owner", before: "Unassigned", after: "A. Rivera" }
      ]}
      actions={<Button variant="primary">Confirm</Button>}
    />
  ),
  ActionAudit: () => (
    <ActionAudit
      actor="service:product-agent"
      action="Approve generated write"
      status="approved"
      timestamp="2026-07-01 10:20 UTC"
      requestId="req_8f2c"
      correlationId="corr_19ab"
      items={[
        { id: "policy", label: "Policy", value: "generated.write.approve" },
        { id: "tenant", label: "Tenant", value: "enterprise" }
      ]}
    />
  ),
  UndoCompensationState: () => (
    <UndoCompensationState
      title="Undo window is open"
      detail="The write can be reversed until the downstream system finalizes the change."
      state="available"
      deadline="Expires in 9 minutes"
      action={<Button>Undo</Button>}
    />
  ),
  MessageThread: () => (
    <MessageThread
      title="Ask generated data"
      description="Point-in-time answer thread with grounded refs."
      messages={messageItems}
    />
  ),
  ConversationWorkspace: ConversationWorkspaceExample,
  Message: () => (
    <Message author="Assistant" role="assistant" timestamp="10:21" status={<ConfidenceSignal value="high" />}>
      Use generated record locators for any linked detail view.
    </Message>
  ),
  MessageComposer: MessageComposerExample,
  StreamingText: () => (
    <StreamingText isStreaming>
      Streaming answer text renders inside the PDS shell.
    </StreamingText>
  ),
  ToolCallStatus: () => (
    <div className="example-stack">
      <ToolCallStatus label="Retrieve generated refs" status="running" detail="Using viewRegistry addressing" />
      <ToolCallStatus label="Prepare write" status="blocked" detail="IntentPreview only" requestId="req_8f2c" />
    </div>
  ),
  EntityRefCard: () => (
    <EntityRefCard
      title="Record Alpha"
      entityType="Generated entity"
      caption="Needs evidence review"
      recordLocator="rl_record_alpha"
      href="#record-alpha"
      status={<Badge tone="warning">Review</Badge>}
    />
  ),
  CitationList: () => (
    <CitationList
      citations={[
        { id: "c-1", label: "Generated record", source: "query_records", detail: "fresh 2m ago" },
        { id: "c-2", label: "Policy result", source: "rego.allow", detail: "read allowed, write blocked" }
      ]}
    />
  ),
  ConfidenceSignal: () => (
    <Row>
      <ConfidenceSignal value="high" />
      <ConfidenceSignal value="medium" detail="Needs review" />
      <ConfidenceSignal value="low" detail="Sparse evidence" />
    </Row>
  ),
  AgentTimeline: () => (
    <AgentTimeline
      description="Persistent workflow panel for the answer lifecycle."
      items={timelineItems}
    />
  ),
  FlowGraphShell: () => (
    <FlowGraphShell
      title="Review flow"
      description="Renderer-neutral shell for generated graph context."
      nodes={flowNodes}
      edges={[
        { id: "e-1", from: "n-1", to: "n-2" },
        { id: "e-2", from: "n-2", to: "n-3" }
      ]}
      toolbar={<Button size="sm">Open graph</Button>}
      legend={<ChartLegend items={[{ id: "blocked", label: "Blocked", tone: "danger", value: "1" }]} />}
    />
  ),
  GeneratedViewShell: () => (
    <GeneratedViewShell
      title="Generated review view"
      description="A registered view composed from resolved refs."
      viewId="generated-review"
      grounding="partial"
      unresolvedCount={1}
      freshness={<FreshnessIndicator value="current" detail="Resolved 2 minutes ago" />}
      confidence={<ConfidenceSignal value="medium" detail="Partial source set" />}
    >
      <FlowGraphShell title="View body" nodes={flowNodes} />
    </GeneratedViewShell>
  ),
  SuggestedAction: () => (
    <SuggestedAction
      title="Prepare review message"
      reason="Two resolved records have stale evidence."
      risk="medium"
      evidence={<EvidenceSummary title="Why now" claims={ambientClaims} />}
      preview={
        <IntentPreview
          title="Preview generated write"
          policy="blocked until governed-write evidence exists"
          changes={[{ id: "message", label: "Message", after: "Draft reminder" }]}
        />
      }
      actions={<Button>Open preview</Button>}
    />
  ),
  RecommendationCard: () => (
    <RecommendationCard
      rank="1"
      title="Review stale evidence"
      reason="Freshness indicator moved from current to stale."
      risk="low"
      evidence={<AiAttributionAffordance detail="Grounded in two resolved refs." sourceCount={2} />}
    />
  ),
  EvidenceSummary: () => (
    <EvidenceSummary
      title="What changed"
      description="Grounded rollup with citations."
      claims={ambientClaims}
      freshness={<FreshnessIndicator value="current" detail="Fresh 2m ago" />}
      citations={<CitationList citations={ambientCitations} />}
    />
  ),
  InsightSummary: () => (
    <InsightSummary
      title="Risk insight"
      insightTone="warning"
      claims={ambientClaims}
      attribution={<AiAttributionAffordance detail="Model-derived; verify before acting." sourceCount={2} />}
    />
  ),
  FreshnessIndicator: () => (
    <Row>
      <FreshnessIndicator value="current" detail="Fresh 2m ago" />
      <FreshnessIndicator value="stale" detail="Source older than policy" />
      <FreshnessIndicator value="unknown" detail="No watermark" />
    </Row>
  ),
  AttentionMarker: () => (
    <Row>
      <AttentionMarker label="Needs review" detail="Medium risk cue" risk="medium" tone="warning" />
      <AttentionMarker label="Policy blocked" detail="High risk cue" risk="high" tone="danger" />
    </Row>
  ),
  AiAttributionAffordance: () => (
    <AiAttributionAffordance
      label="AI-assisted"
      detail="Composed from resolved product data."
      model="approved-gateway"
      generatedAt="10:21 UTC"
      sourceCount={2}
    />
  ),
  AssistLevelControl: () => <AssistLevelControlExample />,
  MemoryChip: () => (
    <MemoryChip
      detail="Using your last selected region and density preference."
      scope="tenant-scoped"
    />
  ),
  EvidenceDisclosure: () => (
    <EvidenceDisclosure model={{
      summary: "Inspect source and derivation",
      items: [{ sourceRef: "fixture-source", label: "Source", value: "Sanitized catalog fixture" }]
    }} />
  ),
  ResolvedContextDisclosure: () => (
    <ResolvedContextDisclosure model={{
      eyebrow: "Working from",
      title: "Current fixture context",
      announcement: "Context resolved from one fixture source with one gap.",
      gaps: ["Approval owner is unresolved."],
      evidence: {
        summary: "View source",
        items: [{ sourceRef: "fixture-source", label: "Source", value: "Sanitized catalog fixture" }]
      }
    }} />
  ),
  WorkStatus: () => (
    <WorkStatus model={{ label: "Checking fixture evidence", detail: "One source", active: true }} />
  ),
  ProgressiveResponse: () => (
    <ProgressiveResponse model={{
      eyebrow: "Progressive response",
      title: "Fixture working brief",
      announcement: "Fixture working brief revision 1. Updated: Current finding.",
      active: false,
      emptyState: "Useful sections will appear when ready.",
      regions: [{
        id: "finding",
        status: "ready",
        label: "Finding",
        title: "Current finding",
        body: "A bounded, sanitized presentation example."
      }]
    }} />
  ),
  PdsIxRecipePresentation: () => (
    <PdsIxRecipePresentation
      registration={{
        schemaVersion: "pds.ix.recipe_registration@1",
        recipeId: "analyze-why",
        intentKey: "pds.ix.intent.analyze-why@1",
        artifactType: "catalog.fixture.analysis@1",
        contentSchemaVersion: "pds.ix.presentation@1",
        rendererKey: "pds.ix.recipe.analyze-why@1",
        requiredCapabilities: [
          "pds.ix.capability.focus-context@1",
          "pds.ix.capability.progressive-presentation@1",
          "pds.ix.capability.evidence-disclosure@1",
          "pds.ix.capability.human-control@1",
          "pds.ix.capability.causal-explanation@1"
        ]
      }}
      presentation={{
        identity: {
          schemaVersion: "pds.ix.presentation@1",
          presentationId: "catalog-fixture-analysis",
          revision: 1
        },
        announcement: "Sanitized analysis fixture is ready.",
        response: {
          eyebrow: "Analyze why",
          title: "Sanitized analysis fixture",
          announcement: "Sanitized analysis fixture is ready.",
          active: false,
          regions: [],
          emptyState: "No analysis regions are present in this fixture."
        }
      }}
    />
  ),
  CommandBar: () => (
    <CommandBar
      filters={<TextField label="Search" name="catalog-command-search" placeholder="Search records" />}
      resultSummary="128 records"
      secondaryActions={<Button>Export</Button>}
      primaryAction={<Button variant="primary">New</Button>}
    />
  ),
  CommandPalette: () => (
    <CommandPalette
      enableGlobalShortcut={false}
      items={[
        { id: "go-records", group: "Navigate", label: "Records", detail: "Open the records workspace" },
        { id: "new-record", group: "Create", label: "New record" },
        { id: "settings", group: "Manage", label: "Settings" }
      ]}
    />
  ),

  // Forms
  Field: () => (
    <Field id="example-field" label="Record label" hint="Primary display value" required>
      <input className="pds-input" id="example-field" defaultValue="Record Alpha" />
    </Field>
  ),
  FieldMetadata: () => (
    <FieldMetadata tags={["required", "indexed"]}>Synced from source system</FieldMetadata>
  ),
  FormLoadingPreview: () => <FormLoadingPreview fieldCount={4} actionCount={2} />,
  FieldGroup: () => (
    <FieldGroup legend="Identity" description="Core fields for this record">
      <FormLayout columns="two">
        <TextField label="Primary label" name="primaryLabel" />
        <TextField label="Secondary label" name="secondaryLabel" />
      </FormLayout>
    </FieldGroup>
  ),
  FormLayout: () => (
    <FormLayout columns="two" footer={<Button variant="primary">Save</Button>}>
      <TextField label="Record label" name="catalog-form-record-name" />
      <SelectField label="Stage" name="catalog-form-stage" options={demoOptions} placeholder="Select" />
    </FormLayout>
  ),
  TextField: () => (
    <div className="example-stack">
      <TextField variant="filled" label="Filled field" name="catalog-text-filled" hint="Supporting text" />
      <TextField variant="outlined" label="Outlined field" name="catalog-text-outlined" defaultValue="Record Alpha" />
      <TextField variant="outlined" label="Email" name="catalog-text-email" error="Enter a valid email" defaultValue="not-an-email" />
    </div>
  ),
  ComboboxField: ComboboxFieldExample,
  TextArea: () => <TextArea label="Notes" name="notes" rows={3} defaultValue="Follow up next week." />,
  SelectField: () => (
    <div className="example-stack">
      <SelectField variant="filled" label="Stage" name="catalog-select-stage-filled" options={demoOptions} placeholder="Select a stage" />
      <SelectField variant="outlined" label="Owner stage" name="catalog-select-stage-outlined" options={demoOptions} defaultValue="review" />
    </div>
  ),
  DateField: () => <DateField variant="outlined" label="Close date" name="close" defaultValue="2026-06-30" />,
  TimeField: () => <TimeField label="Reminder" name="reminder" defaultValue="09:30" />,
  DateTimeField: () => <DateTimeField label="Follow up" name="followup" defaultValue="2026-06-30T09:30" />,
  DatePicker: () => <DatePicker label="Select date" defaultValue="2026-07-17" variant="docked" defaultOpen />,
  TimePicker: () => (
    <div className="example-row">
      <TimePicker label="Dial time picker" defaultValue="07:00" variant="dial" />
      <TimePicker label="Input time picker" defaultValue="09:30" variant="input" />
    </div>
  ),
  InputGroup: () => (
    <InputGroup prefix="$" suffix="USD">
      <input className="pds-input" inputMode="decimal" aria-label="Amount" defaultValue="48,200" />
    </InputGroup>
  ),
  MultiSelect: MultiSelectExample,
  FileUpload: () => <FileUpload label="Attachment" name="file" selectedLabel="contract.pdf" />,
  CheckboxField: () => <CheckboxField label="Subscribed to updates" name="subscribed" defaultChecked detail="Weekly summary" />,
  SwitchField: SwitchFieldExample,
  LookupSelect: LookupSelectExample,
  ValidationSummary: () => (
    <ValidationSummary validation={{ Email: ["is required"], Stage: ["is not a valid choice"] }} />
  ),

  // Data grid
  DataGrid: DataGridExample,
  DataGridShell: DataGridShellExample,
  DataGridLoadingPreview: () => <DataGridLoadingPreview columnCount={5} rowCount={5} density="compact" />,
  DataGridToolbar: () => (
    <DataGridToolbar
      ariaLabel="Record controls"
      search={<TextField label="Search" name="catalog-grid-search" placeholder="Search records" />}
      filters={<DataGridFilterTrigger activeCount={1} />}
      summary="128 records"
      actions={<Button variant="primary">New</Button>}
    />
  ),
  DataGridColumnChooser: ColumnChooserExample,
  DataGridColumnChooserTrigger: () => <DataGridColumnChooserTrigger selectedCount={4} totalCount={5} />,
  DataGridColumnResizeHandle: () => (
    <div className="example-resize">
      <span>Drag the handle</span>
      <DataGridColumnResizeHandle ariaLabel="Resize column" onResize={() => undefined} />
    </div>
  ),
  DataGridControlPopover: ControlPopoverExample,
  DataGridDensityControl: DensityControlExample,
  DataGridFilterPanel: () => (
    <DataGridFilterPanel title="Filters" summary="2 active">
      <DataGridFilterGroup isRoot>
        <DataGridFilterRule>
          <span className="example-note">Stage is one of: Review, Ready</span>
        </DataGridFilterRule>
        <DataGridFilterRule>
          <span className="example-note">Value is greater than 10,000</span>
        </DataGridFilterRule>
      </DataGridFilterGroup>
    </DataGridFilterPanel>
  ),
  DataGridFilterGroup: () => (
    <DataGridFilterGroup isRoot>
      <DataGridFilterRule>
        <span className="example-note">Owner is A. Rivera</span>
      </DataGridFilterRule>
    </DataGridFilterGroup>
  ),
  DataGridFilterRule: () => (
    <DataGridFilterRule>
      <span className="example-note">Stage equals Approved</span>
    </DataGridFilterRule>
  ),
  DataGridFilterEmpty: () => <DataGridFilterEmpty>No filters applied yet.</DataGridFilterEmpty>,
  DataGridFilterTrigger: FilterTriggerExample,
  DataGridSortButton: SortButtonExample,
  DataGridPagination: () => (
    <DataGridPagination
      pageIndex={1}
      pageCount={6}
      pageSize={25}
      startRow={1}
      endRow={25}
      totalRows={128}
      responseMs={42}
      onPageSizeChange={() => undefined}
      onFirstPage={() => undefined}
      onPreviousPage={() => undefined}
      onNextPage={() => undefined}
      onLastPage={() => undefined}
    />
  ),

  // Overlays
  Dialog: DialogExample,
  Drawer: DrawerExample,
  Popover: PopoverExample,
  PopoverTrigger: () => <PopoverTrigger controls="noop" badge="2">Filters</PopoverTrigger>,
  Tooltip: () => (
    <Tooltip content="Archived records stay searchable">
      <Button>Hover for help</Button>
    </Tooltip>
  ),
  ConfirmDialog: ConfirmDialogExample,

  // Feedback
  Alert: () => (
    <div className="example-stack">
      <Alert tone="accent" title="Heads up" detail="Filters now persist per view." />
      <Alert tone="danger" title="Sync failed" detail="We could not reach the server." />
    </div>
  ),
  Banner: () => <Banner tone="accent" title="New release" detail="Saved views are here." onDismiss={() => undefined} />,
  FeedbackState: () => (
    <FeedbackState kind="empty" title="No records found" detail="Adjust filters to see results." />
  ),
  OperationState: () => (
    <div className="example-stack" aria-label="Operation state vocabulary">
      <OperationState state="idle" title="Ready to start" detail="No request has been sent." />
      <OperationState state="pending" title="Submitting request" request={{ requestId: "req_8f2c", correlationId: "corr_19ab" }} />
      <OperationState state="success" title="Request completed" result={{ value: "Accepted", detail: "The consumer supplied this outcome." }} />
      <OperationState
        state="error"
        title="Request failed"
        detail="The consumer can offer a recovery action."
        request={{ requestId: "req_8f2c", correlationId: "corr_19ab" }}
        action={<Button>Try again</Button>}
      />
      <OperationState state="denied" title="Request denied" result={{ label: "Policy outcome", value: "Denied" }} />
      <OperationState state="cancelled" title="Request cancelled" result={{ value: "No change applied" }} />
    </div>
  ),
  LoadingState: () => <LoadingState title="Loading records" skeletonLines={3} metadata={{ requestId: "req_8f2c" }} />,
  ErrorState: () => (
    <ErrorState
      title="Request failed"
      detail="The records service did not respond."
      metadata={{ requestId: "req_8f2c", correlationId: "corr_19ab", responseMs: 4200 }}
      action={<Button>Try again</Button>}
    />
  ),
  ForbiddenState: () => (
    <ForbiddenState title="Access is restricted" detail="Your role or tenant does not allow this." />
  ),
  InlineAlert: () => <InlineAlert tone="danger" title="Save failed" detail="Check the highlighted fields." />,
  EmptyState: () => (
    <EmptyState
      title="No records yet"
      detail="Create your first record to get started."
      action={<Button variant="primary">New record</Button>}
    />
  ),
  Skeleton: () => <Skeleton lines={4} />,
  Toast: ToastExample,
  ToastRegion: ToastRegionExample,

  // Navigation
  AppShell: () => (
    <div className="example-shell-frame">
      <AppShell
        viewportBounded
        brand={<PdsHealthLogo width={134} height={25} />}
        navigation={
          <>
            <NavigationItem
              href="#records"
              label="Records"
              icon={<Glyph />}
              current
            />
            <NavigationItem
              href="#department-requests"
              label="Department requests"
              icon={<Glyph />}
              trailing={<Badge tone="accent">100</Badge>}
            />
            <NavigationItem
              href="#settings"
              label="Settings"
              icon={<Glyph />}
            />
          </>
        }
        topBar={<Badge tone="accent">Workspace</Badge>}
        responsiveCollapse
        sidebarResizable
      >
        <PageHeader eyebrow="Workspace" title="Records" subtitle="128 records" />
      </AppShell>
    </div>
  ),
  ConnectedFabric: () => (
    <Surface
      title="Connected Fabric is active behind this catalog"
      subtitle="A single decorative canvas responds to pointer and appearance settings."
    >
      <p className="example-note">
        Products configure placement selectors; PDS preserves geometry, motion,
        reduced-motion behavior, and non-semantic presentation.
      </p>
    </Surface>
  ),
  AppearanceProvider: () => {
    // Scope the demo provider's data-theme/data-visual-theme writes to this
    // element; the default documentElement target would fight the catalog's
    // own appearance controls whenever this example mounts.
    const [target, setTarget] = useState<HTMLElement | null>(null);
    return (
      <div ref={setTarget}>
        {target ? (
          <AppearanceProvider
            defaultColorMode="system"
            defaultVisualTheme="apple-like"
            persist={false}
            attributeTarget={target}
          >
            <Surface
              title="Appearance follows user preference"
              subtitle="System color mode with the Apple-like visual theme"
            >
              <p className="example-note">
                Products choose the persistence key and policy; PDS propagates the
                resolved appearance contract.
              </p>
            </Surface>
          </AppearanceProvider>
        ) : null}
      </div>
    );
  },
  NavigationItem: () => (
    <div className="example-stack">
      <NavigationItem
        href="#department-requests"
        label="Department requests requiring portfolio review"
        description="Wraps to two lines before disclosure is needed"
        icon={<Glyph />}
        trailing={<Badge tone="accent">100</Badge>}
        current
      />
      <NavigationItem
        label="Intentionally truncated destination with exceptional supporting context"
        labelBehavior="truncate"
        description="Overflow is disclosed on hover and focus"
        icon={<Glyph />}
      />
    </div>
  ),
  IconSlot: () => (
    <Row>
      <IconSlot size="sm" label="Add"><Glyph /></IconSlot>
      <IconSlot size="md" label="Add"><Glyph /></IconSlot>
      <IconSlot size="lg" label="Add"><Glyph /></IconSlot>
    </Row>
  ),
  Avatar: () => (
    <Row>
      <Avatar name="Avery Rivera" size="sm" />
      <Avatar name="Jordan Okafor" size="md" />
      <Avatar name="Morgan Chen" size="lg" />
    </Row>
  ),
  PdsHealthLogo: () => (
    <Row>
      <PdsHealthLogo variant="wordmark" width={214} height={40} />
      <PdsHealthLogo variant="mark" width={40} height={40} label="PDS Health compact mark" />
    </Row>
  ),
  IdentitySummary: () => (
    <IdentitySummary
      name="Avery Rivera"
      description="Regional partner"
      metadata="West"
      avatar={<Avatar name="Avery Rivera" />}
      trailing={<Badge tone="success">Online</Badge>}
    />
  ),
  Breadcrumbs: () => (
    <Breadcrumbs
      ariaLabel="Request navigation"
      items={[
        { id: "requests", label: "My requests", href: "#requests" },
        { id: "request", label: "Frisco West, TX", current: true }
      ]}
    />
  ),
  PageHeader: () => (
    <PageHeader
      eyebrow="Workspace"
      title="Records"
      subtitle="128 records"
      actions={<Button variant="primary">New</Button>}
    />
  ),
  Surface: () => (
    <div className="example-card-variants">
      <Surface variant="elevated" title="Elevated" subtitle="Level 1 surface">
        <p className="example-note">Related work with subtle elevation.</p>
      </Surface>
      <Surface variant="filled" title="Filled" subtitle="Tonal containment">
        <p className="example-note">Grouped content without elevation.</p>
      </Surface>
      <Surface variant="outlined" title="Outlined" subtitle="Boundary only">
        <p className="example-note">Lightweight visual containment.</p>
      </Surface>
      <Surface
        variant="outlined"
        overflow="visible"
        title="Overlay host"
        subtitle="Explicitly allows anchored controls to escape the surface boundary"
      >
        <SearchBar
          label="Search within a surface"
          placeholder="Focus to show results"
          items={[
            {
              id: "surface-overlay-result",
              label: "Overlay result",
              detail: "Paints beyond the surface without a product override"
            }
          ]}
        />
      </Surface>
    </div>
  ),
  InteractiveCard: () => (
    <InteractiveCard
      eyebrow="Workspace"
      title="Review open work"
      description="A whole-card command with a separately layered action."
      metadata="12 records"
      activation={{ kind: "button", onActivate: () => undefined }}
      trailingAction={<Button size="sm">Pin</Button>}
      tone="accent"
    />
  ),
  CardLink: () => (
    <CardLink
      href="#request-detail"
      eyebrow="Request"
      title="Open request details"
      description="Native link semantics across the complete card surface."
      footer="Updated 2 minutes ago"
      tone="accent"
    />
  ),
  WorkQueueItem: () => (
    <WorkQueueItem
      as="div"
      eyebrow="Assigned to you"
      title="Review location readiness"
      description="Scan-friendly queue row with independent trailing actions."
      metadata="Due today"
      status={<Badge tone="warning">Attention</Badge>}
      activation={{ kind: "button", onActivate: () => undefined }}
      trailingAction={<Button size="sm">Complete</Button>}
      tone="warning"
    />
  ),
  List: () => (
    <List variant="segmented">
      <ListItem headline="Record Alpha" supportingText="Recently updated" trailing="12" />
      <ListItem headline="Record Beta" supportingText="Approval pending" trailing="3" selected />
      <ListItem headline="Record Gamma" supportingText="Ready" trailing="8" />
    </List>
  ),
  ListItem: () => (
    <List>
      <ListItem leading={<Glyph />} overline="Suggested" headline="Review evidence" supportingText="Two sources need attention" trailing="2" />
    </List>
  ),
  SearchBar: SearchBarExample,
  NarrativeWorkspace: NarrativeWorkspaceExample,
  ExplorationWorkspace: ExplorationWorkspaceExample,
  Tabs: TabsExample,
  SegmentedControl: SegmentedControlExample,
  Badge: () => (
    <Row>
      <Badge tone="neutral">Neutral</Badge>
      <Badge tone="accent">Accent</Badge>
      <Badge tone="success">Active</Badge>
      <Badge tone="warning">Pending</Badge>
      <Badge tone="danger">Blocked</Badge>
    </Row>
  ),

  // Process
  ProcessStepper: ProcessStepperExample,
  ProcessProgress: ProcessProgressExample,

  // Analytics
  TimelineRangeSelector: TimelineRangeSelectorExample,
  RelationshipAtlas: RelationshipAtlasExample,
  RelationshipAtlasTable: RelationshipAtlasTableExample,
  RelationshipExplorer: RelationshipExplorerExample,
  KpiTile: () => (
    <KpiTile
      label="Open work"
      value="1,248"
      detail="vs. last quarter"
      tone="success"
      trend={<MetricTrend value="+12%" direction="up" tone="positive" />}
    />
  ),
  MetricTrend: () => (
    <Row>
      <MetricTrend value="+12%" label="QoQ" direction="up" tone="positive" />
      <MetricTrend value="-4%" label="WoW" direction="down" tone="negative" />
      <MetricTrend value="0%" label="Flat" direction="flat" />
    </Row>
  ),
  ChartShell: ChartVariationsExample,
  ChartLegend: () => (
    <ChartLegend
      items={[
        { id: "ready", label: "Ready", tone: "success", value: "42%" },
        { id: "review", label: "Review", tone: "accent", value: "38%" },
        { id: "blocked", label: "Blocked", tone: "danger", value: "20%" }
      ]}
    />
  ),
  BarChart: () => (
    <BarChart
      ariaLabel="Records by stage"
      data={[
        { label: "Draft", value: 12, tone: "neutral" },
        { label: "Review", value: 28, tone: "accent" },
        { label: "Ready", value: 19, tone: "success" },
        { label: "Done", value: 34, tone: "success" }
      ]}
    />
  ),
  LineChart: () => (
    <LineChart
      ariaLabel="Throughput over time"
      data={[
        { label: "Mon", value: 8 },
        { label: "Tue", value: 14 },
        { label: "Wed", value: 11 },
        { label: "Thu", value: 22 },
        { label: "Fri", value: 18 },
        { label: "Sat", value: 27 }
      ]}
    />
  ),
  AreaChart: () => (
    <AreaChart
      ariaLabel="Cumulative completion"
      data={[
        { label: "W1", value: 10 },
        { label: "W2", value: 24 },
        { label: "W3", value: 31 },
        { label: "W4", value: 48 },
        { label: "W5", value: 60 }
      ]}
    />
  ),
  DonutChart: () => (
    <DonutChart
      ariaLabel="Completion mix"
      centerValue="68%"
      centerLabel="Complete"
      data={[
        { label: "Done", value: 68, tone: "success" },
        { label: "Active", value: 22, tone: "accent" },
        { label: "Blocked", value: 10, tone: "danger" }
      ]}
    />
  )
};

export function exampleFor(name: string): FC | undefined {
  return componentExamples[name];
}
