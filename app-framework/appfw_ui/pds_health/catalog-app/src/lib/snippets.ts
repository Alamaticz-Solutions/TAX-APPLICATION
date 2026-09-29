// Copy-ready snippets for each component. The import line is generated; the
// usage line is a minimal, paste-able starting point an agent can adapt. These
// are documentation strings, not compiled — they intentionally show the props a
// product is expected to bind (labels, data, handlers) rather than full state.
const PACKAGE = "@appfw/pds-health-components";

export function importSnippet(name: string): string {
  return `import { ${name} } from "${PACKAGE}";`;
}

const usageSnippets: Record<string, string> = {
  // Actions
  Button: `<Button variant="primary" onClick={onSave}>Save</Button>`,
  ButtonLink: `<ButtonLink href="/records" variant="primary">Open records</ButtonLink>`,
  ToggleButton: `<ToggleButton selected={selected} onSelectedChange={setSelected}>Filter results</ToggleButton>`,
  FloatingActionButton: `<FloatingActionButton icon={<AddIcon />} label="Create" onClick={onCreate} />`,
  ButtonGroup: `<ButtonGroup variant="connected" ariaLabel="Record view">{/* buttons */}</ButtonGroup>`,
  IconButton: `<IconButton ariaLabel="Edit" icon={<EditIcon />} onClick={onEdit} />`,
  MenuButton: `<MenuButton label="Actions" items={[{ id: "export", label: "Export", onSelect: onExport }]} />`,
  Toolbar: `<Toolbar ariaLabel="Editing tools" variant="floating">{/* controls */}</Toolbar>`,
  IntentPreview: `<IntentPreview\n  title="Preview generated write"\n  actor={actorLabel}\n  policy={policyResult}\n  changes={proposedChanges}\n  actions={<Button variant="primary">Confirm</Button>}\n/>`,
  ActionAudit: `<ActionAudit\n  actor={principalLabel}\n  action="Approve generated write"\n  status="approved"\n  requestId={requestId}\n  correlationId={correlationId}\n/>`,
  UndoCompensationState: `<UndoCompensationState\n  title="Undo window is open"\n  state="available"\n  deadline={undoDeadline}\n  action={<Button>Undo</Button>}\n/>`,
  MessageThread: `<MessageThread title="Ask generated data" messages={messages} />`,
  ConversationWorkspace: `<ConversationWorkspace\n  title="Ask workspace"\n  messages={messages}\n  suggestedPrompts={prompts}\n  onSuggestedPromptSelect={submitPrompt}\n  composerProps={{ value: draft, onValueChange: setDraft, onSubmitMessage: sendMessage }}\n/>`,
  Message: `<Message author="Assistant" role="assistant" timestamp={timestamp}>\n  {answerText}\n</Message>`,
  MessageComposer: `<MessageComposer\n  value={draft}\n  onValueChange={setDraft}\n  onSubmitMessage={sendMessage}\n  helperText="Write actions require preview and approval."\n/>`,
  StreamingText: `<StreamingText isStreaming={isStreaming}>{answerText}</StreamingText>`,
  ToolCallStatus: `<ToolCallStatus label="Retrieve generated refs" status="running" requestId={requestId} />`,
  EntityRefCard: `<EntityRefCard title={caption} entityType={entityType} recordLocator={recordLocator} href={detailHref} />`,
  CitationList: `<CitationList citations={answerEnvelope.citations} />`,
  ConfidenceSignal: `<ConfidenceSignal value="medium" detail="Grounded in generated refs" />`,
  AgentTimeline: `<AgentTimeline items={agentEvents} description="Persistent workflow panel" />`,
  FlowGraphShell: `<FlowGraphShell nodes={nodes} edges={edges} renderGraph={renderFlowGraph} />`,
  GeneratedViewShell: `<GeneratedViewShell\n  title="Generated view"\n  grounding="grounded"\n  freshness={<FreshnessIndicator value="current" detail={freshnessLabel} />}\n>\n  {registeredView}\n</GeneratedViewShell>`,
  SuggestedAction: `<SuggestedAction\n  title="Review next step"\n  reason={whyNow}\n  evidence={evidenceSummary}\n  preview={<IntentPreview title="Preview generated write" changes={changes} />}\n/>`,
  RecommendationCard: `<RecommendationCard rank="1" title="Recommended next step" reason={whyNow} risk="medium" />`,
  EvidenceSummary: `<EvidenceSummary title="What changed" claims={claims} citations={<CitationList citations={citations} />} />`,
  InsightSummary: `<InsightSummary title="Risk summary" insightTone="warning" claims={claims} />`,
  FreshnessIndicator: `<FreshnessIndicator value="current" detail="Resolved from source data" timestamp={freshnessTime} />`,
  AttentionMarker: `<AttentionMarker label="Needs review" detail="Confidence changed" risk="medium" tone="warning" />`,
  AiAttributionAffordance: `<AiAttributionAffordance label="AI-assisted" detail="Composed from resolved records" sourceCount={3} />`,
  AssistLevelControl: `<AssistLevelControl value={assistLevel} onValueChange={setAssistLevel} />`,
  MemoryChip: `<MemoryChip detail="Using remembered preferences" scope="tenant-scoped" onCorrect={openMemorySettings} onReset={resetMemory} />`,
  EvidenceDisclosure: `<EvidenceDisclosure model={presentationEvidence} />`,
  ResolvedContextDisclosure: `<ResolvedContextDisclosure model={resolvedContextPresentation} />`,
  WorkStatus: `<WorkStatus model={workStatusPresentation} action={pauseAction} />`,
  ProgressiveResponse: `<ProgressiveResponse model={progressiveResponsePresentation} renderAction={renderRegionAction} />`,
  PdsIxRecipePresentation: `<PdsIxRecipePresentation registration={recipeRegistration} presentation={presentation} />`,
  CommandBar: `<CommandBar\n  filters={<SearchField />}\n  resultSummary="128 records"\n  primaryAction={<Button variant="primary">New</Button>}\n/>`,
  CommandPalette: `<CommandPalette items={[{ id: "go-records", group: "Navigate", label: "Records", onSelect: goToRecords }]} />`,

  // Forms
  Field: `<Field id="email" label="Email" hint="Work address" error={errors.email}>\n  <input className="pds-input" id="email" />\n</Field>`,
  FieldMetadata: `<FieldMetadata tags={["required", "indexed"]}>Synced from source system</FieldMetadata>`,
  FormLoadingPreview: `<FormLoadingPreview fieldCount={6} actionCount={2} />`,
  FieldGroup: `<FieldGroup legend="Identity" description="Core generated fields for this record">\n  {/* fields */}\n</FieldGroup>`,
  FormLayout: `<FormLayout columns="two" footer={<Button variant="primary">Save</Button>}>\n  {/* fields */}\n</FormLayout>`,
  TextField: `<TextField label="Record label" name="name" required value={value} onChange={onChange} />`,
  ComboboxField: `<ComboboxField\n  id="stage"\n  label="Stage"\n  options={stageOptions}\n  value={stage}\n  onValueChange={setStage}\n  inputValue={stageInput}\n  onInputValueChange={setStageInput}\n/>`,
  TextArea: `<TextArea label="Notes" name="notes" rows={4} value={value} onChange={onChange} />`,
  SelectField: `<SelectField label="Stage" name="stage" options={stageOptions} placeholder="Select a stage" />`,
  DateField: `<DateField label="Close date" name="closeDate" value={value} onChange={onChange} />`,
  TimeField: `<TimeField label="Reminder" name="reminder" value={value} onChange={onChange} />`,
  DateTimeField: `<DateTimeField label="Follow up" name="followUp" value={value} onChange={onChange} />`,
  DatePicker: `<DatePicker label="Select date" value={date} onValueChange={setDate} variant="modal" />`,
  TimePicker: `<TimePicker label="Select time" value={time} onValueChange={setTime} variant="dial" />`,
  InputGroup: `<InputGroup prefix="$" suffix="USD">\n  <input className="pds-input" inputMode="decimal" />\n</InputGroup>`,
  MultiSelect: `<MultiSelect label="Tags" options={tagOptions} value={tags} onValueChange={setTags} />`,
  FileUpload: `<FileUpload label="Attachment" name="file" selectedLabel="contract.pdf" />`,
  CheckboxField: `<CheckboxField label="Subscribed" name="subscribed" detail="Receives updates" />`,
  SwitchField: `<SwitchField id="active" label="Active" checked={active} onCheckedChange={setActive} />`,
  LookupSelect: `<LookupSelect options={owners} value={ownerId} onValueChange={setOwnerId} loading={isLoading} />`,
  ValidationSummary: `<ValidationSummary validation={{ Email: ["is required"], Stage: ["is invalid"] }} />`,

  // Data Grid
  DataGrid: `<DataGrid\n  ariaLabel="Records"\n  columns={columns}\n  rows={rows}\n  rowKey="id"\n  selectionMode="multiple"\n  pagination\n/>`,
  DataGridShell: `<DataGridShell\n  ariaLabel="Records"\n  columns={columns}\n  rows={rows}\n  rowKey="id"\n  density="compact"\n  onRowSelect={openRecord}\n/>`,
  DataGridLoadingPreview: `<DataGridLoadingPreview columnCount={5} rowCount={7} density="compact" />`,
  DataGridToolbar: `<DataGridToolbar ariaLabel="Record controls" search={<SearchField />} summary="128 records" actions={<Button>New</Button>} />`,
  DataGridColumnChooser: `<DataGridColumnChooser groups={columnGroups} selectedIds={visibleColumns} onToggle={toggleColumn} />`,
  DataGridColumnChooserTrigger: `<DataGridColumnChooserTrigger selectedCount={6} totalCount={9} onClick={openColumnChooser} />`,
  DataGridColumnResizeHandle: `<DataGridColumnResizeHandle ariaLabel="Resize Name" onResize={setWidth} onReset={resetWidth} />`,
  DataGridControlPopover: `<DataGridControlPopover ariaLabel="Column settings">{/* controls */}</DataGridControlPopover>`,
  DataGridDensityControl: `<DataGridDensityControl value={density} onChange={setDensity} />`,
  DataGridFilterPanel: `<DataGridFilterPanel title="Filters" summary="2 active">{/* groups */}</DataGridFilterPanel>`,
  DataGridFilterGroup: `<DataGridFilterGroup isRoot>{/* rules */}</DataGridFilterGroup>`,
  DataGridFilterRule: `<DataGridFilterRule>{/* field + operator + value controls */}</DataGridFilterRule>`,
  DataGridFilterEmpty: `<DataGridFilterEmpty>No filters applied yet.</DataGridFilterEmpty>`,
  DataGridFilterTrigger: `<DataGridFilterTrigger activeCount={2} onClick={openFilters} />`,
  DataGridSortButton: `<DataGridSortButton label="Updated" ariaLabel="Sort by updated" direction="desc" onSort={toggleSort} />`,
  DataGridPagination: `<DataGridPagination\n  pageIndex={1}\n  pageSize={25}\n  startRow={1}\n  endRow={25}\n  totalRows={128}\n  onNextPage={next}\n  onPreviousPage={prev}\n/>`,

  // Overlays
  Dialog: `<Dialog open={open} title="Edit record" onClose={close}>\n  {/* body */}\n</Dialog>`,
  Drawer: `<Drawer open={open} title="Record details" side="right" onClose={close}>\n  {/* body */}\n</Drawer>`,
  Popover: `<Popover ariaLabel="Quick filters" open={open} onClose={close}>{/* body */}</Popover>`,
  PopoverTrigger: `<PopoverTrigger controls="filters-popover" open={open} onClick={toggle}>Filters</PopoverTrigger>`,
  Tooltip: `<Tooltip content="Archived records stay searchable"><IconButton ariaLabel="Help" icon={<HelpIcon />} /></Tooltip>`,
  ConfirmDialog: `<ConfirmDialog open={open} title="Delete record?" tone="danger" onConfirm={remove} onCancel={close} />`,

  // Feedback
  Alert: `<Alert tone="warning" title="Sync delayed" detail="Last updated 8 minutes ago." />`,
  Banner: `<Banner tone="accent" title="New release" detail="Filters now persist per view." onDismiss={dismiss} />`,
  FeedbackState: `<FeedbackState kind="empty" title="No records found" detail="Adjust filters to see results." />`,
  OperationState: `<OperationState\n  state={operationState}\n  title={stateTitle}\n  result={{ value: resultLabel }}\n  request={{ requestId, correlationId }}\n  action={<Button>Try again</Button>}\n/>`,
  LoadingState: `<LoadingState title="Loading records" skeletonLines={4} metadata={{ requestId }} />`,
  ErrorState: `<ErrorState title="Request failed" detail="Try again." metadata={{ requestId, correlationId }} action={<Button>Retry</Button>} />`,
  ForbiddenState: `<ForbiddenState title="Access is restricted" detail="Your role does not allow this." />`,
  InlineAlert: `<InlineAlert tone="danger" title="Save failed" detail="Check the highlighted fields." />`,
  EmptyState: `<EmptyState title="No records yet" detail="Create the first record." action={<Button variant="primary">New</Button>} />`,
  Skeleton: `<Skeleton lines={4} />`,
  Toast: `<Toast id="saved" tone="success" title="Saved" onDismiss={dismiss} />`,
  ToastRegion: `<ToastRegion toasts={toasts} onDismiss={dismiss} position="top-end" />`,

  // Navigation
  AppShell: `<AppShell\n  brand={<Brand />}\n  navigation={<Nav />}\n  topBar={<TopBar />}\n  responsiveCollapse\n  sidebarResizable\n  defaultSidebarWidth={288}\n  sidebarMinWidth={240}\n  sidebarMaxWidth={360}\n  onSidebarWidthCommit={persistSidebarWidth}\n>\n  {/* page */}\n</AppShell>`,
  NarrativeWorkspace: `<NarrativeWorkspace
  brand={<ProductIdentity />}
  utilities={<HeaderUtilities />}
  primaryNavigation={<ExperienceModes />}
  primaryNavigationLabel="Product experience"
  contextBand={<RoleContext />}
  contextBandLabel="Reader context"
>
  {/* document-scrolling knowledge experience */}
</NarrativeWorkspace>`,
  ConnectedFabric: `<ConnectedFabric\n  theme={appearance.colorMode}\n  visualTheme={appearance.visualTheme}\n  contentRootSelector=".app-main"\n  panelSelector=".pds-surface"\n  scrollRootSelector=".app-main"\n/>`,
  AppearanceProvider: `<AppearanceProvider\n  defaultColorMode="system"\n  defaultVisualTheme="apple-like"\n  storageKey="product-appearance"\n>\n  <App />\n</AppearanceProvider>`,
  PdsHealthLogo: `<PdsHealthLogo variant="wordmark" label="PDS Health" />`,
  NavigationItem: `<NavigationItem\n  href="/workspace"\n  label="Workspace"\n  description="Current destination"\n  icon={<WorkspaceIcon />}\n  trailing={<Badge tone="accent">12</Badge>}\n  current\n/>`,
  IconSlot: `<IconSlot size="md" label="Add record"><AddIcon /></IconSlot>`,
  Avatar: `<Avatar name="Avery Rivera" initials="AR" size="md" />`,
  IdentitySummary: `<IdentitySummary\n  name="Avery Rivera"\n  description="Regional partner"\n  metadata="West"\n  avatar={<Avatar name="Avery Rivera" />}\n  trailing={<Badge tone="success">Online</Badge>}\n/>`,
  Breadcrumbs: `<Breadcrumbs ariaLabel="Request navigation" items={[{ id: "requests", label: "My requests", href: "/my-requests" }, { id: "detail", label: "Request detail", current: true }]} />`,
  PageHeader: `<PageHeader eyebrow="Workspace" title="Records" subtitle="128 records" actions={<Button variant="primary">New</Button>} />`,
  Surface: `<Surface title="Operational summary" subtitle="This quarter" actions={<Button>Export</Button>}>\n  {/* content */}\n</Surface>`,
  InteractiveCard: `<InteractiveCard\n  title="Review open work"\n  description="12 records need review"\n  activation={{ kind: "button", onActivate: openQueue }}\n  trailingAction={<Button size="sm">Pin</Button>}\n/>`,
  CardLink: `<CardLink href="/requests/42" title="Open request details" description="Updated 2 minutes ago" />`,
  WorkQueueItem: `<WorkQueueItem\n  title="Review location readiness"\n  description="Assigned to you"\n  activation={{ kind: "link", href: "/tasks/42" }}\n  status={<Badge tone="warning">Due today</Badge>}\n  trailingAction={<Button size="sm">Complete</Button>}\n/>`,
  List: `<List variant="segmented">{/* list items */}</List>`,
  ListItem: `<ListItem headline="Record" supportingText="Recently updated" trailing="12" />`,
  SearchBar: `<SearchBar items={searchItems} value={query} onValueChange={setQuery} />`,
  ExplorationWorkspace: `<ExplorationWorkspace
  title="System explorer"
  controls={<SearchControls />}
  navigator={<ObjectNavigator />}
  focusRegion={<FocusView />}
  inspector={<ObjectInspector />}
  evidenceRail={<EvidenceRail />}
/>`,
  Tabs: `<Tabs ariaLabel="Record views" selectedId={tab} onChange={setTab} items={tabItems} />`,
  SegmentedControl: `<SegmentedControl ariaLabel="Density" value={value} onValueChange={setValue} options={options} />`,
  Badge: `<Badge tone="success">Active</Badge>`,

  // Process
  ProcessStepper: `<ProcessStepper\n  ariaLabel="Release process"\n  steps={steps}\n  currentStepId={currentStepId}\n  selectedStepId={selectedStepId}\n  variant="milestone"\n  onStepSelect={selectStep}\n/>`,
  ProcessProgress: `<ProcessProgress\n  label="Certification progress"\n  currentStep={3}\n  totalSteps={5}\n  detail="3 of 5"\n/>`,

  // Analytics
  TimelineRangeSelector: `<TimelineRangeSelector\n  ariaLabel="Delivery history window"\n  items={months}\n  startIndex={selection.startIndex}\n  width={selection.width}\n  selectionLabel={selectionLabel}\n  snapWidths={[1, 3, 6, 12]}\n  onSelectionChange={setSelection}\n/>`,
  RelationshipAtlas: `<RelationshipAtlas ariaLabel="Decision relationships" groups={groups} nodes={nodes} edges={edges} selectedId={selectedId} onSelectNode={setSelectedId} />`,
  RelationshipAtlasTable: `<RelationshipAtlasTable caption="Decision relationships" groups={groups} nodes={nodes} edges={edges} selectedId={selectedId} onSelectNode={setSelectedId} />`,
  RelationshipExplorer: `<RelationshipExplorer\n  ariaLabel="Decision relationships"\n  tableCaption="Decision relationships"\n  groups={groups}\n  nodes={nodes}\n  edges={edges}\n  selectedId={selectedId}\n  onSelectNode={setSelectedId}\n/>`,
  KpiTile: `<KpiTile label="Open work" value="1,248" detail="vs. last quarter" tone="success" trend={<MetricTrend value="+12%" direction="up" tone="positive" />} />`,
  MetricTrend: `<MetricTrend value="+12%" label="QoQ" direction="up" tone="positive" />`,
  ChartShell: `<ChartShell title="Trend" subtitle="Current scope" footer={<ChartLegend items={legend} />}>\n  {/* chart renderer or token-backed chart marks */}\n</ChartShell>`,
  ChartLegend: `<ChartLegend items={[{ id: "ready", label: "Ready", tone: "success", value: "42%" }]} />`,
  BarChart: `<BarChart ariaLabel="Records by stage" data={[{ label: "Draft", value: 12 }, { label: "Review", value: 28, tone: "accent" }, { label: "Done", value: 34, tone: "success" }]} />`,
  LineChart: `<LineChart ariaLabel="Throughput over time" data={[{ label: "Mon", value: 8 }, { label: "Tue", value: 14 }, { label: "Wed", value: 22 }]} />`,
  AreaChart: `<AreaChart ariaLabel="Cumulative completion" data={[{ label: "W1", value: 10 }, { label: "W2", value: 31 }, { label: "W3", value: 60 }]} />`,
  DonutChart: `<DonutChart ariaLabel="Completion mix" centerValue="68%" centerLabel="Complete" data={[{ label: "Done", value: 68, tone: "success" }, { label: "Active", value: 22, tone: "accent" }, { label: "Blocked", value: 10, tone: "danger" }]} />`
};

export function usageSnippet(name: string): string {
  return usageSnippets[name] ?? `<${name} />`;
}
