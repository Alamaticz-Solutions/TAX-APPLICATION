export type PdsComponentFamilyName =
  | "Actions"
  | "Forms"
  | "Data Grid"
  | "Overlays"
  | "Feedback"
  | "Navigation"
  | "Conversation"
  | "Ambient AI"
  | "Process"
  | "Analytics";

export type PdsComponentMaturity = "foundation" | "enterprise-ready" | "release-gated";

export type PdsComponentFamily = {
  slug: string;
  name: PdsComponentFamilyName;
  maturity: PdsComponentMaturity;
  components: readonly string[];
};

export type PdsAgentDecisionRecipe = {
  slug: string;
  intent: string;
  startWith: readonly string[];
  productOwns: string;
  evidence: readonly string[];
};

export type PdsPlannedComponentFamilySlot = {
  slug: string;
  name: string;
  status: "planned";
  maturity: PdsComponentMaturity;
  reason: string;
  evidence: readonly string[];
};

export const pdsComponentFamilies = [
  {
    slug: "actions",
    name: "Actions",
    maturity: "release-gated",
    components: [
      "Button",
      "ButtonLink",
      "ToggleButton",
      "FloatingActionButton",
      "ButtonGroup",
      "IconButton",
      "MenuButton",
      "Toolbar",
      "IntentPreview",
      "ActionAudit",
      "UndoCompensationState",
      "CommandBar",
      "CommandPalette"
    ]
  },
  {
    slug: "forms",
    name: "Forms",
    maturity: "release-gated",
    components: [
      "Field",
      "FieldMetadata",
      "FormLoadingPreview",
      "FieldGroup",
      "FormLayout",
      "TextField",
      "ComboboxField",
      "TextArea",
      "SelectField",
      "DateField",
      "TimeField",
      "DateTimeField",
      "DatePicker",
      "TimePicker",
      "InputGroup",
      "MultiSelect",
      "FileUpload",
      "CheckboxField",
      "SwitchField",
      "LookupSelect",
      "ValidationSummary"
    ]
  },
  {
    slug: "data-grid",
    name: "Data Grid",
    maturity: "release-gated",
    components: [
      "DataGrid",
      "DataGridShell",
      "DataGridLoadingPreview",
      "DataGridToolbar",
      "DataGridColumnChooser",
      "DataGridColumnChooserTrigger",
      "DataGridColumnResizeHandle",
      "DataGridControlPopover",
      "DataGridDensityControl",
      "DataGridFilterPanel",
      "DataGridFilterGroup",
      "DataGridFilterRule",
      "DataGridFilterEmpty",
      "DataGridFilterTrigger",
      "DataGridSortButton",
      "DataGridPagination"
    ]
  },
  {
    slug: "overlays",
    name: "Overlays",
    maturity: "release-gated",
    components: [
      "Dialog",
      "Drawer",
      "Popover",
      "PopoverTrigger",
      "Tooltip",
      "ConfirmDialog"
    ]
  },
  {
    slug: "feedback",
    name: "Feedback",
    maturity: "release-gated",
    components: [
      "Alert",
      "Banner",
      "FeedbackState",
      "OperationState",
      "LoadingState",
      "ErrorState",
      "ForbiddenState",
      "InlineAlert",
      "EmptyState",
      "Skeleton",
      "Toast",
      "ToastRegion"
    ]
  },
  {
    slug: "navigation",
    name: "Navigation",
    maturity: "release-gated",
    components: [
      "AppShell",
      "NarrativeWorkspace",
      "ConnectedFabric",
      "AppearanceProvider",
      "PdsHealthLogo",
      "NavigationItem",
      "IconSlot",
      "Avatar",
      "IdentitySummary",
      "Breadcrumbs",
      "PageHeader",
      "Surface",
      "List",
      "ListItem",
      "SearchBar",
      "ExplorationWorkspace",
      "Tabs",
      "SegmentedControl",
      "Badge",
      "InteractiveCard",
      "CardLink",
      "WorkQueueItem"
    ]
  },
  {
    slug: "conversation",
    name: "Conversation",
    maturity: "enterprise-ready",
    components: [
      "MessageThread",
      "Message",
      "MessageComposer",
      "ConversationWorkspace",
      "StreamingText",
      "ToolCallStatus",
      "EntityRefCard",
      "CitationList",
      "ConfidenceSignal",
      "AgentTimeline",
      "FlowGraphShell"
    ]
  },
  {
    slug: "ambient-ai",
    name: "Ambient AI",
    maturity: "enterprise-ready",
    components: [
      "GeneratedViewShell",
      "SuggestedAction",
      "RecommendationCard",
      "EvidenceSummary",
      "InsightSummary",
      "FreshnessIndicator",
      "AttentionMarker",
      "AiAttributionAffordance",
      "AssistLevelControl",
      "MemoryChip",
      "EvidenceDisclosure",
      "ResolvedContextDisclosure",
      "WorkStatus",
      "ProgressiveResponse",
      "PdsIxRecipePresentation"
    ]
  },
  {
    slug: "process",
    name: "Process",
    maturity: "enterprise-ready",
    components: [
      "ProcessStepper",
      "ProcessProgress"
    ]
  },
  {
    slug: "analytics",
    name: "Analytics",
    maturity: "enterprise-ready",
    components: [
      "KpiTile",
      "MetricTrend",
      "ChartShell",
      "ChartLegend",
      "TimelineRangeSelector",
      "RelationshipAtlas",
      "RelationshipAtlasTable",
      "RelationshipExplorer",
      "BarChart",
      "LineChart",
      "AreaChart",
      "DonutChart"
    ]
  }
] as const satisfies readonly PdsComponentFamily[];

export const pdsPlannedComponentFamilySlots = [] as const satisfies readonly PdsPlannedComponentFamilySlot[];

export const pdsAgentDecisionGuide = [
  {
    slug: "generated-entity-workspace",
    intent: "Build a dense generated entity list with query controls, column control, density, loading, and pagination.",
    startWith: [
      "DataGrid",
      "DataGridShell",
      "DataGridToolbar",
      "DataGridFilterTrigger",
      "DataGridColumnChooserTrigger",
      "DataGridDensityControl",
      "DataGridLoadingPreview",
      "DataGridPagination"
    ],
    productOwns: "Columns, row routes, server query bindings, permission checks, persisted preferences, and row action meaning.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-consumer-wiring",
      "density-default-policy"
    ]
  },
  {
    slug: "generated-entity-form",
    intent: "Build a generated create or edit form with labels, hints, validation, lookup choices, and stable loading rhythm.",
    startWith: [
      "FormLayout",
      "FieldGroup",
      "FieldMetadata",
      "TextField",
      "SelectField",
      "LookupSelect",
      "ValidationSummary",
      "FormLoadingPreview"
    ],
    productOwns: "Generated field metadata, validation rules, submit behavior, save conflict handling, and workflow-specific copy.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-consumer-wiring"
    ]
  },
  {
    slug: "governed-action",
    intent: "Build a destructive, policy-sensitive, or release-relevant action that needs confirmation and traceable feedback.",
    startWith: [
      "Button",
      "IconButton",
      "MenuButton",
      "IntentPreview",
      "ActionAudit",
      "UndoCompensationState",
      "ConfirmDialog",
      "InlineAlert",
      "ToastRegion"
    ],
    productOwns: "Action labels, authorization result, business consequence, confirmation copy, undo or compensation behavior, and recovery path.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-agent-readiness-ledger",
      "g1-governed-write-posture",
      "g1-governed-write-live-evidence",
      "u2-agent-harness-profile"
    ]
  },
  {
    slug: "decision-support-dashboard",
    intent: "Build an operational dashboard with metric summaries, chart chrome, trends, empty states, and loading states.",
    startWith: [
      "KpiTile",
      "MetricTrend",
      "ChartShell",
      "ChartLegend",
      "EmptyState",
      "Skeleton"
    ],
    productOwns: "Measures, calculations, chart renderer, thresholds, and data interpretation.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-consumer-wiring"
    ]
  },
  {
    slug: "guided-process-flow",
    intent: "Build a multi-step intake, approval, certification, or release workflow with visible progress and governed transitions.",
    startWith: [
      "ProcessStepper",
      "ProcessProgress",
      "Button",
      "InlineAlert",
      "FeedbackState"
    ],
    productOwns: "Step labels, workflow ordering, branching rules, completion criteria, authorization, and transition side effects.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-agent-readiness-ledger"
    ]
  },
  {
    slug: "workspace-navigation",
    intent: "Build product navigation, page context, command access, and view switching with predictable keyboard behavior.",
    startWith: [
      "AppShell",
      "NarrativeWorkspace",
      "AppearanceProvider",
      "PdsHealthLogo",
      "NavigationItem",
      "IconSlot",
      "IdentitySummary",
      "Breadcrumbs",
      "PageHeader",
      "CommandBar",
      "CommandPalette",
      "Tabs",
      "SegmentedControl",
      "Badge"
    ],
    productOwns: "Route identity, navigation labels, selected view, role-aware visibility, and URL behavior.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-agent-readiness-ledger"
    ]
  },
  {
    slug: "conversational-answer",
    intent: "Build an invoked AI answer surface that shows grounded messages, citations, confidence, tool status, entity refs, timeline, and graph context without granting write execution.",
    startWith: [
      "MessageThread",
      "Message",
      "StreamingText",
      "ToolCallStatus",
      "EntityRefCard",
      "CitationList",
      "ConfidenceSignal",
      "AgentTimeline",
      "FlowGraphShell",
      "IntentPreview"
    ],
    productOwns: "Prompt copy, model/gateway calls, answer-envelope data, retrieval policy, write-gate decisions, and product-specific graph content.",
    evidence: [
      "answer-envelope-contract",
      "view-registry-contract",
      "chat-eval-posture",
      "source-export-catalog-drift",
      "reference-agent-api-source-map"
    ]
  },
  {
    slug: "ambient-assistance",
    intent: "Build in-workflow AI-generated views, suggestions, cues, and summaries that are grounded, preview-gated, attributable, and non-disruptive.",
    startWith: [
      "GeneratedViewShell",
      "SuggestedAction",
      "RecommendationCard",
      "EvidenceSummary",
      "InsightSummary",
      "FreshnessIndicator",
      "AttentionMarker",
      "AiAttributionAffordance",
      "AssistLevelControl",
      "MemoryChip",
      "IntentPreview"
    ],
    productOwns: "Resolved refs, viewRegistry bindings, answer-envelope claims, assist-level behavior, memory storage/correction, policy decisions, and all write execution.",
    evidence: [
      "answer-envelope-contract",
      "view-registry-contract",
      "chat-eval-posture",
      "source-export-catalog-drift",
      "reference-agent-api-source-map"
    ]
  },
  {
    slug: "service-feedback",
    intent: "Build request-aware loading, empty, denied, error, success, warning, and retry feedback near the affected workflow.",
    startWith: [
      "FeedbackState",
      "OperationState",
      "LoadingState",
      "ErrorState",
      "ForbiddenState",
      "Alert",
      "Banner",
      "Toast"
    ],
    productOwns: "Error category, retry behavior, request and correlation identifiers, and support escalation path.",
    evidence: [
      "source-export-catalog-drift",
      "reference-agent-api-source-map",
      "reference-agent-readiness-ledger"
    ]
  }
] as const satisfies readonly PdsAgentDecisionRecipe[];

// ---------------------------------------------------------------------------
// Versioning & component lifecycle
//
// Enterprise product teams need to depend on, pin, and upgrade the design
// system as a versioned package. `pdsPackageVersion` mirrors the package.json
// version (SemVer); the component lifecycle exposes a per-component API
// stability status so consumers and agents know what is safe to build on and
// what may change. Status is derived from family maturity so every exported
// component is always covered (no drift), with room for explicit overrides
// (e.g. deprecations) layered on top.
// ---------------------------------------------------------------------------
export const pdsPackageVersion = "0.12.0";
const pdsInitialComponentVersion = "0.7.0";

export type PdsComponentStatus = "stable" | "beta" | "experimental" | "deprecated";

export type PdsComponentLifecycle = {
  status: PdsComponentStatus;
  since: string;
  deprecated?: {
    since: string;
    removeBy?: string;
    replacement?: string;
    note?: string;
  };
};

const statusByMaturity: Record<PdsComponentMaturity, PdsComponentStatus> = {
  "release-gated": "stable",
  "enterprise-ready": "beta",
  foundation: "experimental"
};

// Explicit overrides for individual components (deprecations, early-access, or
// status that diverges from its family's maturity).
const lifecycleOverrides: Record<string, PdsComponentLifecycle> = {
  AppearanceProvider: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  Avatar: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  ConnectedFabric: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  ComboboxField: {
    status: "experimental",
    since: pdsInitialComponentVersion
  },
  DataGrid: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  OperationState: {
    status: "experimental",
    since: pdsInitialComponentVersion
  },
  IconSlot: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  IdentitySummary: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  NavigationItem: {
    status: "beta",
    since: pdsInitialComponentVersion
  },
  PdsHealthLogo: {
    status: "beta",
    since: "0.9.0"
  },
  RelationshipAtlas: {
    status: "experimental",
    since: "0.8.0"
  },
  RelationshipAtlasTable: {
    status: "experimental",
    since: "0.8.0"
  },
  RelationshipExplorer: {
    status: "experimental",
    since: "0.8.0"
  },
  ExplorationWorkspace: {
    status: "experimental",
    since: "0.8.0"
  },
  NarrativeWorkspace: {
    status: "experimental",
    since: "0.9.0"
  },
  EvidenceDisclosure: {
    status: "experimental",
    since: "0.10.0"
  },
  ResolvedContextDisclosure: {
    status: "experimental",
    since: "0.10.0"
  },
  WorkStatus: {
    status: "experimental",
    since: "0.10.0"
  },
  ProgressiveResponse: {
    status: "experimental",
    since: "0.10.0"
  },
  PdsIxRecipePresentation: {
    status: "experimental",
    since: pdsPackageVersion
  }
};

export const pdsComponentLifecycle: Record<string, PdsComponentLifecycle> = Object.fromEntries(
  pdsComponentFamilies.flatMap((family) =>
    family.components.map((component) => [
      component,
      lifecycleOverrides[component] ?? {
        status: statusByMaturity[family.maturity],
        since: pdsInitialComponentVersion
      }
    ])
  )
);

export const pdsComponentCatalog = {
  version: 1,
  packageName: "@appfw/pds-health-components",
  packageVersion: pdsPackageVersion,
  families: pdsComponentFamilies,
  plannedFamilySlots: pdsPlannedComponentFamilySlots,
  agentDecisionGuide: pdsAgentDecisionGuide,
  lifecycle: pdsComponentLifecycle
} as const;
