#!/usr/bin/env node
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const designSystemRoot = path.join(repoRoot, 'appfw_ui/pds_health');
const componentRoot = path.join(repoRoot, 'appfw_ui/pds_health/components');
const srcRoot = path.join(componentRoot, 'src');
const referenceRoot = path.join(designSystemRoot, 'reference');
const interactiveCatalogRoot = path.join(designSystemRoot, 'catalog-app');
const artifactPathArgIndex = process.argv.indexOf('--artifact-path');
// Demos and docs-check examples redirect the artifact so intentionally
// failing runs can never clobber the canonical retained evidence.
const artifactPath = artifactPathArgIndex >= 0 && process.argv[artifactPathArgIndex + 1]
  ? path.resolve(process.argv[artifactPathArgIndex + 1])
  : path.join(repoRoot, 'target/appfw/pds-component-check.json');
const chatMarkdownSanitizerDocPath = path.join(repoRoot, 'docs/frontend/chat-markdown-sanitizer.md');
const jsonOutput = process.argv.includes('--json');
const enforceGovernedAction = process.argv.includes('--enforce-governed-action');

function optionValue(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || index + 1 >= process.argv.length) {
    return null;
  }
  return process.argv[index + 1];
}

const evidencePathOverrides = new Map([
  [
    'g1-governed-write-posture',
    optionValue('--governed-write-posture-evidence')
      ?? process.env.APPFW_PDS_COMPONENT_GOVERNED_WRITE_POSTURE_EVIDENCE
      ?? null
  ],
  [
    'g1-governed-write-live-evidence',
    optionValue('--governed-write-live-evidence')
      ?? process.env.APPFW_PDS_COMPONENT_GOVERNED_WRITE_LIVE_EVIDENCE
      ?? null
  ],
  [
    'u2-agent-harness-profile',
    optionValue('--agent-harness-evidence')
      ?? process.env.APPFW_PDS_COMPONENT_AGENT_HARNESS_EVIDENCE
      ?? null
  ]
].filter(([, value]) => typeof value === 'string' && value.trim().length > 0));

const requiredFiles = [
  '.gitignore',
  'package.json',
  'package-lock.json',
  'README.md',
  'connected-fabric.accepted-baseline.json',
  'connected-fabric-visual-language.md',
  'nexus-readiness.json',
  'scripts/serve-pds-reference.mjs',
  'src/index.ts',
  'src/catalog.ts',
  'src/ambient.tsx',
  'src/charts.tsx',
  'src/conversation.tsx',
  'src/conversation-workspace.tsx',
  'src/connected-fabric.tsx',
  'src/exploration-workspace.tsx',
  'src/types.ts',
  'src/primitives.tsx',
  'src/forms.tsx',
  'src/foundation.tsx',
  'src/experience.tsx',
  'src/layout.tsx',
  'src/narrative-workspace.tsx',
  'src/process.tsx',
  'src/data.tsx',
  'src/data-grid.tsx',
  'src/surfaces.tsx',
  'src/timeline.tsx',
  'src/relationship-atlas.tsx',
  'src/work-surfaces.tsx',
  'src/copy.ts',
  'src/styles.css'
];

const requiredComponents = [
  'Alert',
  'AppearanceProvider',
  'AppShell',
  'AreaChart',
  'Badge',
  'Avatar',
  'Banner',
  'BarChart',
  'Breadcrumbs',
  'Button',
  'ButtonLink',
  'ButtonGroup',
  'CardLink',
  'ActionAudit',
  'CheckboxField',
  'ComboboxField',
  'ChartLegend',
  'ChartShell',
  'CommandBar',
  'CommandPalette',
  'ConfirmDialog',
  'ConnectedFabric',
  'DataGridColumnChooser',
  'DataGridColumnChooserTrigger',
  'DataGridColumnResizeHandle',
  'DataGridControlPopover',
  'DataGridDensityControl',
  'DataGridFilterEmpty',
  'DataGridFilterGroup',
  'DataGridFilterPanel',
  'DataGridFilterRule',
  'DataGridFilterTrigger',
  'DataGridLoadingPreview',
  'DataGridPagination',
  'DataGrid',
  'DataGridShell',
  'DataGridSortButton',
  'DataGridToolbar',
  'DateField',
  'DatePicker',
  'DateTimeField',
  'Dialog',
  'DonutChart',
  'Drawer',
  'EmptyState',
  'ErrorState',
  'ExplorationWorkspace',
  'FeedbackState',
  'OperationState',
  'Field',
  'FieldGroup',
  'FieldMetadata',
  'FileUpload',
  'FormLoadingPreview',
  'FormLayout',
  'ForbiddenState',
  'IconButton',
  'IconSlot',
  'IdentitySummary',
  'FloatingActionButton',
  'IntentPreview',
  'InlineAlert',
  'InteractiveCard',
  'InputGroup',
  'KpiTile',
  'LineChart',
  'List',
  'ListItem',
  'LoadingState',
  'LookupSelect',
  'MenuButton',
  'MetricTrend',
  'MultiSelect',
  'NavigationItem',
  'NarrativeWorkspace',
  'PdsHealthLogo',
  'PageHeader',
  'Popover',
  'PopoverTrigger',
  'ProcessProgress',
  'ProcessStepper',
  'SelectField',
  'SegmentedControl',
  'Skeleton',
  'Surface',
  'SearchBar',
  'SwitchField',
  'Tabs',
  'TextArea',
  'TextField',
  'TimeField',
  'TimePicker',
  'ToggleButton',
  'Toolbar',
  'Tooltip',
  'Toast',
  'ToastRegion',
  'TimelineRangeSelector',
  'RelationshipAtlas',
  'RelationshipAtlasTable',
  'RelationshipExplorer',
  'UndoCompensationState',
  'ValidationSummary',
  'WorkQueueItem',
  'MessageThread',
  'Message',
  'MessageComposer',
  'ConversationWorkspace',
  'StreamingText',
  'ToolCallStatus',
  'EntityRefCard',
  'CitationList',
  'ConfidenceSignal',
  'AgentTimeline',
  'FlowGraphShell',
  'GeneratedViewShell',
  'SuggestedAction',
  'RecommendationCard',
  'EvidenceSummary',
  'InsightSummary',
  'FreshnessIndicator',
  'AttentionMarker',
  'AiAttributionAffordance',
  'AssistLevelControl',
  'MemoryChip',
  'EvidenceDisclosure',
  'ResolvedContextDisclosure',
  'WorkStatus',
  'ProgressiveResponse',
  'PdsIxRecipePresentation'
];

const requiredCssClasses = [
  '.pds-alert',
  '.pds-app-shell',
  '.pds-narrative-workspace',
  '.pds-avatar',
  '.pds-area-chart',
  '.pds-badge',
  '.pds-banner',
  '.pds-bar-chart',
  '.pds-button',
  '.pds-button-link',
  '.pds-button-group',
  '.pds-action-audit',
  '.pds-chart-legend',
  '.pds-chart-shell',
  '.pds-command-bar',
  '.pds-command-palette',
  '.pds-card-link',
  '.pds-confirm-dialog',
  '.pds-data-grid',
  '.pds-data-grid-column-chooser',
  '.pds-data-grid-column-trigger',
  '.pds-data-grid-control-popover',
  '.pds-data-grid__resizer',
  '.pds-data-grid-density-control',
  '.pds-data-grid-filter-empty',
  '.pds-data-grid-filter-group',
  '.pds-data-grid-filter-panel',
  '.pds-data-grid-filter-rule',
  '.pds-data-grid-filter-trigger',
  '.pds-data-grid-loading-preview',
  '.pds-data-grid-pagination',
  '.pds-data-grid-sort-button',
  '.pds-data-grid-toolbar',
  '.pds-date-input',
  '.pds-date-picker',
  '.pds-datetime-input',
  '.pds-dialog',
  '.pds-donut-chart',
  '.pds-drawer',
  '.pds-empty-state',
  '.pds-exploration-workspace',
  '.pds-feedback-state',
  '.pds-operation-state',
  '.pds-field',
  '.pds-field-group',
  '.pds-field-metadata',
  '.pds-file-upload',
  '.pds-form-loading-preview',
  '.pds-form-layout',
  '.pds-icon-button',
  '.pds-icon-slot',
  '.pds-identity-summary',
  '.pds-floating-action-button',
  '.pds-intent-preview',
  '.pds-input',
  '.pds-input-group',
  '.pds-interactive-card',
  '.pds-inline-alert',
  '.pds-kpi-tile',
  '.pds-line-chart',
  '.pds-list',
  '.pds-list-item',
  '.pds-lookup-select',
  '.pds-menu-button',
  '.pds-metric-trend',
  '.pds-multi-select',
  '.pds-navigation-item',
  '.pds-popover',
  '.pds-popover-trigger',
  '.pds-process-progress',
  '.pds-process-progress__segments',
  '.pds-process-stepper',
  '.pds-process-stepper-adaptive',
  '.pds-segmented-control',
  '.pds-skeleton',
  '.pds-surface',
  '.pds-search-bar',
  '.pds-switch',
  '.pds-tabs',
  '.pds-time-input',
  '.pds-time-picker',
  '.pds-toggle-button',
  '.pds-toolbar',
  '.pds-tooltip',
  '.pds-toast',
  '.pds-timeline-range',
  '.pds-atlas',
  '.pds-atlas-table',
  '.pds-atlas-explorer',
  '.pds-work-queue-item',
  '.pds-toast-region',
  '.pds-undo-compensation-state',
  '.pds-validation-summary',
  '.pds-message-thread',
  '.pds-message',
  '.pds-message-composer',
  '.pds-conversation-workspace',
  '.pds-streaming-text',
  '.pds-tool-call-status',
  '.pds-entity-ref-card',
  '.pds-citation-list',
  '.pds-confidence-signal',
  '.pds-agent-timeline',
  '.pds-flow-graph-shell',
  '.pds-generated-view-shell',
  '.pds-suggested-action',
  '.pds-recommendation-card',
  '.pds-evidence-summary',
  '.pds-insight-summary',
  '.pds-freshness-indicator',
  '.pds-attention-marker',
  '.pds-ai-attribution',
  '.pds-assist-level-control',
  '.pds-memory-chip'
];

const referenceFiles = [
  'index.html',
  'reference.css',
  'catalog.json'
];

const interactiveCatalogFiles = [
  '.gitignore',
  'package.json',
  'package-lock.json',
  'index.html',
  'tsconfig.json',
  'tsconfig.node.json',
  'vite.config.ts',
  'src/App.tsx',
  'src/main.tsx',
  'src/examples.tsx',
  'src/app.css',
  'src/lib/catalogData.ts',
  'src/lib/snippets.ts',
  'src/lib/theme.ts'
];

const referenceCoverageClasses = [
  'pds-alert',
  'pds-app-shell',
  'pds-area-chart',
  'pds-badge',
  'pds-banner',
  'pds-bar-chart',
  'pds-button',
  'pds-action-audit',
  'pds-chart-legend',
  'pds-chart-shell',
  'pds-command-bar',
  'pds-command-palette',
  'pds-confirm-dialog',
  'pds-data-grid',
  'pds-data-grid-column-chooser',
  'pds-data-grid-column-trigger',
  'pds-data-grid-control-popover',
  'pds-data-grid__resizer',
  'pds-data-grid-density-control',
  'pds-data-grid-filter-empty',
  'pds-data-grid-filter-group',
  'pds-data-grid-filter-panel',
  'pds-data-grid-filter-rule',
  'pds-data-grid-filter-trigger',
  'pds-data-grid-loading-preview',
  'pds-data-grid-pagination',
  'pds-data-grid-sort-button',
  'pds-data-grid-toolbar',
  'pds-date-input',
  'pds-datetime-input',
  'pds-dialog',
  'pds-donut-chart',
  'pds-drawer',
  'pds-empty-state',
  'pds-feedback-state',
  'pds-field',
  'pds-field-group',
  'pds-field-metadata',
  'pds-file-upload',
  'pds-form-loading-preview',
  'pds-form-layout',
  'pds-input',
  'pds-input-group',
  'pds-intent-preview',
  'pds-inline-alert',
  'pds-kpi-tile',
  'pds-line-chart',
  'pds-lookup-select',
  'pds-menu-button',
  'pds-metric-trend',
  'pds-multi-select',
  'pds-page-header',
  'pds-popover',
  'pds-popover-trigger',
  'pds-process-progress',
  'pds-process-stepper',
  'pds-select',
  'pds-segmented-control',
  'pds-skeleton',
  'pds-surface',
  'pds-switch',
  'pds-tabs',
  'pds-time-input',
  'pds-tooltip',
  'pds-toast',
  'pds-toast-region',
  'pds-undo-compensation-state',
  'pds-validation-summary',
  'pds-message-thread',
  'pds-message',
  'pds-message-composer',
  'pds-streaming-text',
  'pds-tool-call-status',
  'pds-entity-ref-card',
  'pds-citation-list',
  'pds-confidence-signal',
  'pds-agent-timeline',
  'pds-flow-graph-shell',
  'pds-generated-view-shell',
  'pds-suggested-action',
  'pds-recommendation-card',
  'pds-evidence-summary',
  'pds-insight-summary',
  'pds-freshness-indicator',
  'pds-attention-marker',
  'pds-ai-attribution',
  'pds-assist-level-control',
  'pds-memory-chip'
];

const referenceCatalogFamilies = [
  'Actions',
  'Forms',
  'Data Grid',
  'Overlays',
  'Feedback',
  'Navigation',
  'Process',
  'Analytics',
  'Conversation',
  'Ambient AI'
];

const referenceCatalogSections = [
  'Props',
  'States',
  'Density',
  'Accessibility',
  'Usage',
  'Verification'
];

const referenceFamilyContractParts = [
  'props',
  'states',
  'density',
  'accessibility',
  'usage',
  'verification'
];

const referencePlannedFamilySlots = [];

const referenceAgentRecipeSlugs = [
  'generated-entity-workspace',
  'generated-entity-form',
  'governed-action',
  'decision-support-dashboard',
  'guided-process-flow',
  'workspace-navigation',
  'conversational-answer',
  'ambient-assistance',
  'service-feedback'
];

const referenceAgentRecipeEvidence = [
  'source-export-catalog-drift',
  'reference-agent-api-source-map',
  'reference-agent-readiness-ledger',
  'reference-consumer-wiring',
  'density-default-policy',
  'g1-governed-write-posture',
  'g1-governed-write-live-evidence',
  'u2-agent-harness-profile',
  'answer-envelope-contract',
  'view-registry-contract',
  'chat-eval-posture',
  'agentic-ux-grounding'
];

const requiredRecipeEvidence = {
  'governed-action': [
    'g1-governed-write-posture',
    'g1-governed-write-live-evidence',
    'u2-agent-harness-profile'
  ]
};

const referenceMaturityLevels = [
  'foundation',
  'enterprise-ready',
  'release-gated'
];

const referenceEvidenceRequirements = [
  'catalog-manifest',
  'visible-catalog',
  'token-backed-css',
  'accessibility-patterns',
  'consumer-wiring',
  'scaffold-proof',
  'g1-governed-write-posture',
  'g1-governed-write-live-evidence',
  'u2-agent-harness-profile',
  'answer-envelope-contract',
  'view-registry-contract',
  'chat-eval-posture',
  'agentic-ux-grounding'
];

const baselineFamilyEvidence = [
  'catalog-manifest',
  'visible-catalog',
  'token-backed-css',
  'accessibility-patterns'
];

// Evidence a family must have to legitimately claim each maturity level, and the
// check that actually backs each evidence category. A maturity claim is only
// honored when its required evidence is backed by *passing* checks (audit F-8):
// labels are evidence-gated, not self-asserted. Basis is retained local/CI
// evidence, not live managed-environment certification.
const maturityEvidenceRequirements = {
  foundation: baselineFamilyEvidence,
  'enterprise-ready': baselineFamilyEvidence,
  'release-gated': [...baselineFamilyEvidence, 'consumer-wiring']
};
const evidenceVerifiedBy = {
  'catalog-manifest': 'reference-agent-catalog-manifest',
  'visible-catalog': 'reference-component-catalog',
  'token-backed-css': 'token-backed-css',
  'accessibility-patterns': 'accessibility-patterns',
  'consumer-wiring': 'reference-consumer-wiring',
  'scaffold-proof': 'external:intake-proof',
  'g1-governed-write-posture': 'external:target/appfw/governed-write-posture.json',
  'g1-governed-write-live-evidence': 'external:target/appfw/governed-write-evidence.json',
  'u2-agent-harness-profile': 'external:.appfw/target/appfw/harness-check.json',
  'answer-envelope-contract': 'app_gen/_config/chat_evals/_schemas/answer-envelope-v1.schema.json',
  'view-registry-contract': 'examples/products/crm/frontend/src/generated/appfw-ui-contract.ts',
  'chat-eval-posture': 'target/appfw/chat-eval.json',
  'agentic-ux-grounding': 'docs/frontend/agentic-ux.md'
};
const maturityEvidenceBasis = 'retained-local-ci';

const governedActionLiveEvidenceRequirements = [
  {
    id: 'g1-governed-write-posture',
    description: 'G1 posture evidence proves governed writes are fail-closed unless certified.',
    paths: ['target/appfw/governed-write-posture.json']
  },
  {
    id: 'g1-governed-write-live-evidence',
    description: 'G1 live evidence proves an external API provider write path through provider-test.',
    paths: ['target/appfw/governed-write-evidence.json']
  },
  {
    id: 'u2-agent-harness-profile',
    description: 'U2 harness evidence proves product-agent governed-write capabilities are fail-closed.',
    paths: [
      'examples/products/crm/.appfw/target/appfw/harness-check.json',
      '.appfw/target/appfw/harness-check.json'
    ]
  }
].map((requirement) => {
  const override = evidencePathOverrides.get(requirement.id);
  return override ? { ...requirement, paths: [override] } : requirement;
});

const governedWriteExternalApiProviders = new Set([
  'servicenow',
  'workday',
  'icims',
  'salesforce',
  'anaplan',
  'oracle_financials'
]);
const governedWriteAuditSources = new Set(['http', 'mcp', 'kafka']);

const requiredFlowGraphTokenBridgeVariables = [
  '--xy-background-color-default',
  '--xy-node-background-color-default',
  '--xy-node-color-default',
  '--xy-node-border-default',
  '--xy-node-border-radius-default',
  '--xy-node-boxshadow-default',
  '--xy-edge-stroke-default',
  '--xy-edge-stroke-width-default',
  '--xy-connectionline-stroke-default',
  '--xy-controls-button-background-color-default',
  '--xy-controls-button-color-default',
  '--xy-minimap-background-color-default',
  '--xy-attribution-background-color-default'
];

const chatMarkdownSanitizerRequiredClauses = [
  {
    id: 'renderer-react-markdown',
    pattern: 'react-markdown',
    description: 'Decision chooses react-markdown as the client markdown renderer.'
  },
  {
    id: 'sanitizer-rehype-sanitize',
    pattern: 'rehype-sanitize',
    description: 'Decision chooses rehype-sanitize as the sanitizer.'
  },
  {
    id: 'raw-html-disabled',
    pattern: 'Raw HTML is disabled',
    description: 'Decision keeps model-authored raw HTML disabled.'
  },
  {
    id: 'no-rehype-raw',
    pattern: 'Do not enable `rehype-raw`',
    description: 'Decision forbids rehype-raw for model-authored content.'
  },
  {
    id: 'url-policy-allowlist',
    pattern: 'URL policy is allowlist-based',
    description: 'Decision requires allowlist URL handling.'
  },
  {
    id: 'refs-as-pointers',
    pattern: 'refs-as-pointers',
    description: 'Decision preserves generated-contract ref resolution.'
  },
  {
    id: 'incremental-streaming-sanitized',
    pattern: 'sanitize each rendered snapshot',
    description: 'Decision covers incremental streaming snapshots.'
  },
  {
    id: 'live-ready-false',
    pattern: 'live_ready:false',
    description: 'Decision keeps live markdown readiness false until adapter evidence exists.'
  }
];

const accessibilityPatterns = [
  {
    id: 'button-loading-state',
    file: 'src/primitives.tsx',
    pattern: 'aria-busy',
    description: 'Button exposes loading state to assistive technology.'
  },
  {
    id: 'icon-button-name',
    file: 'src/primitives.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'IconButton requires an accessible name.'
  },
  {
    id: 'menu-button-menu-role',
    file: 'src/primitives.tsx',
    pattern: 'role="menu"',
    description: 'MenuButton exposes grouped overflow actions through menu semantics.'
  },
  {
    id: 'menu-button-item-role',
    file: 'src/primitives.tsx',
    pattern: 'role="menuitem"',
    description: 'MenuButton items expose menuitem semantics for action discovery.'
  },
  {
    id: 'menu-button-disabled-state',
    file: 'src/primitives.tsx',
    pattern: 'aria-disabled={item.disabled || undefined}',
    description: 'MenuButton items expose disabled state to assistive technology.'
  },
  {
    id: 'intent-preview-named-region',
    file: 'src/primitives.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'IntentPreview exposes a named review region before a governed action is confirmed.'
  },
  {
    id: 'intent-preview-change-list',
    file: 'src/primitives.tsx',
    pattern: 'aria-label="Proposed changes"',
    description: 'IntentPreview exposes proposed changes as a named list.'
  },
  {
    id: 'action-audit-status-state',
    file: 'src/primitives.tsx',
    pattern: 'data-status={status}',
    description: 'ActionAudit exposes stable status state for audit review.'
  },
  {
    id: 'undo-compensation-live-status',
    file: 'src/primitives.tsx',
    pattern: 'aria-live="polite"',
    description: 'UndoCompensationState announces compensation and undo-window changes.'
  },
  {
    id: 'undo-compensation-status-role',
    file: 'src/primitives.tsx',
    pattern: 'role="status"',
    description: 'UndoCompensationState exposes non-blocking status semantics.'
  },
  {
    id: 'field-invalid-state',
    file: 'src/forms.tsx',
    pattern: 'aria-invalid',
    description: 'Fields expose validation state.'
  },
  {
    id: 'field-describedby',
    file: 'src/forms.tsx',
    pattern: 'aria-describedby',
    description: 'Fields connect hints and errors to controls.'
  },
  {
    id: 'date-field-native',
    file: 'src/forms.tsx',
    pattern: 'inputType="date"',
    description: 'DateField uses the native date input contract.'
  },
  {
    id: 'time-field-native',
    file: 'src/forms.tsx',
    pattern: 'inputType="time"',
    description: 'TimeField uses the native time input contract.'
  },
  {
    id: 'datetime-field-native',
    file: 'src/forms.tsx',
    pattern: 'inputType="datetime-local"',
    description: 'DateTimeField uses the native datetime-local input contract.'
  },
  {
    id: 'input-group-focus-state',
    file: 'src/styles.css',
    pattern: '.pds-input-group:focus-within',
    description: 'InputGroup exposes a group-level focus affordance while preserving native controls.'
  },
  {
    id: 'multi-select-native',
    file: 'src/forms.tsx',
    pattern: 'multiple',
    description: 'MultiSelect uses native multiple-selection semantics.'
  },
  {
    id: 'multi-select-selected-options',
    file: 'src/forms.tsx',
    pattern: 'selectedOptions',
    description: 'MultiSelect reports selected options through the platform selectedOptions collection.'
  },
  {
    id: 'file-upload-native',
    file: 'src/forms.tsx',
    pattern: 'type="file"',
    description: 'FileUpload preserves the native file input contract.'
  },
  {
    id: 'file-upload-describedby',
    file: 'src/forms.tsx',
    pattern: 'className={composeClassNames("pds-file-upload__input", className)}',
    description: 'FileUpload keeps generated hints and errors attached to the native file input.'
  },
  {
    id: 'lookup-select-native',
    file: 'src/forms.tsx',
    pattern: 'export function LookupSelect',
    description: 'LookupSelect exposes lookup values through a native select control.'
  },
  {
    id: 'lookup-select-multiple',
    file: 'src/forms.tsx',
    pattern: 'selectedOptions',
    description: 'LookupSelect supports native multiple selection for list-valued filters.'
  },
  {
    id: 'lookup-select-state',
    file: 'src/forms.tsx',
    pattern: 'data-state={state}',
    description: 'LookupSelect exposes a stable state hook for loading, empty, ready, and error states.'
  },
  {
    id: 'lookup-select-live-status',
    file: 'src/forms.tsx',
    pattern: 'aria-live={error ? "assertive" : "polite"}',
    description: 'LookupSelect announces loading, empty, and error status changes.'
  },
  {
    id: 'switch-role',
    file: 'src/forms.tsx',
    pattern: 'role="switch"',
    description: 'Switch uses the platform switch role.'
  },
  {
    id: 'segmented-control-group',
    file: 'src/primitives.tsx',
    pattern: 'export function SegmentedControl',
    description: 'SegmentedControl exposes grouped mutually exclusive commands.'
  },
  {
    id: 'segmented-control-pressed',
    file: 'src/primitives.tsx',
    pattern: 'aria-pressed={selected}',
    description: 'SegmentedControl options expose selected state to assistive technology.'
  },
  {
    id: 'tabs-roles',
    file: 'src/layout.tsx',
    pattern: '<AriaTabList',
    description: 'Tabs delegate tablist semantics and keyboard mechanics to the approved interaction substrate.'
  },
  {
    id: 'command-palette-shortcut',
    file: 'src/layout.tsx',
    pattern: 'event.key.toLowerCase() !== "k"',
    description: 'CommandPalette exposes a keyboard shortcut for unified command access.'
  },
  {
    id: 'command-palette-search',
    file: 'src/layout.tsx',
    pattern: 'type="search"',
    description: 'CommandPalette uses a native search input.'
  },
  {
    id: 'command-palette-dialog',
    file: 'src/layout.tsx',
    pattern: 'aria-label={searchLabel}',
    description: 'CommandPalette exposes a named popover dialog.'
  },
  {
    id: 'dialog-semantics',
    file: 'src/layout.tsx',
    pattern: 'aria-modal="true"',
    description: 'Dialog exposes modal semantics.'
  },
  {
    id: 'dialog-escape',
    file: 'src/layout.tsx',
    pattern: 'event.key === "Escape"',
    description: 'Dialog handles escape dismissal.'
  },
  {
    id: 'overlay-focus-return',
    file: 'src/layout.tsx',
    pattern: 'previousFocus.focus()',
    description: 'Modal overlays return focus to the invoking element when they close.'
  },
  {
    id: 'overlay-focus-trap',
    file: 'src/layout.tsx',
    pattern: '<AriaModalOverlay',
    description: 'Dialog delegates modal focus containment and restoration to the approved interaction substrate.'
  },
  {
    id: 'overlay-body-portal',
    file: 'src/layout.tsx',
    pattern: 'createPortal',
    description: 'Modal overlays render at document body scope instead of inheriting local stacking contexts.'
  },
  {
    id: 'dialog-scroll-body-focus',
    file: 'src/layout.tsx',
    pattern: 'className="pds-dialog__body" tabIndex={0}',
    description: 'Dialog scrollable body regions remain keyboard focusable.'
  },
  {
    id: 'drawer-semantics',
    file: 'src/layout.tsx',
    pattern: 'export function Drawer',
    description: 'Drawer exposes modal dialog semantics for side-panel workflows.'
  },
  {
    id: 'drawer-focus-trap',
    file: 'src/layout.tsx',
    pattern: 'trapOverlayFocus(event, drawerRef.current)',
    description: 'Drawer traps keyboard focus while open.'
  },
  {
    id: 'drawer-scroll-body-focus',
    file: 'src/layout.tsx',
    pattern: 'className="pds-drawer__body" tabIndex={0}',
    description: 'Drawer scrollable body regions remain keyboard focusable.'
  },
  {
    id: 'popover-native-compatibility',
    file: 'src/layout.tsx',
    pattern: 'popover: nativePopover',
    description: 'Popover supports the native popover attribute when consumers can use it.'
  },
  {
    id: 'popover-trigger-state',
    file: 'src/layout.tsx',
    pattern: 'aria-expanded={open}',
    description: 'PopoverTrigger exposes expanded state to assistive technology.'
  },
  {
    id: 'tooltip-role',
    file: 'src/layout.tsx',
    pattern: 'role="tooltip"',
    description: 'Tooltip exposes tooltip semantics and a stable described-by target.'
  },
  {
    id: 'tooltip-describedby',
    file: 'src/layout.tsx',
    pattern: 'aria-describedby',
    description: 'Tooltip connects trigger content to the tooltip body.'
  },
  {
    id: 'confirm-dialog-danger-action',
    file: 'src/layout.tsx',
    pattern: 'variant={tone === "danger" ? "danger" : "primary"}',
    description: 'ConfirmDialog reserves danger styling for destructive confirmation actions.'
  },
  {
    id: 'grid-region',
    file: 'src/data.tsx',
    pattern: 'role="region"',
    description: 'Data grid shell exposes a named region.'
  },
  {
    id: 'ag-grid-pds-adapter-boundary',
    file: 'src/data-grid.tsx',
    pattern: 'export function DataGrid',
    description: 'AG Grid Community is exposed only through the PDS-owned DataGrid adapter.'
  },
  {
    id: 'ag-grid-community-registration',
    file: 'src/data-grid.tsx',
    pattern: 'ClientSideRowModelModule',
    description: 'The PDS adapter registers only the Community capabilities required by its public contract.'
  },
  {
    id: 'ag-grid-visual-theme-adaptation',
    file: 'src/data-grid.tsx',
    pattern: 'resolvedVisualTheme === "material-like"',
    description: 'One semantic adapter resolves the Apple-like and Material-like render grammars.'
  },
  {
    id: 'ag-grid-enter-keyboard-activation',
    file: 'src/data-grid.tsx',
    pattern: 'event.key !== "Enter"',
    description: 'The PDS adapter binds Enter to row activation without consuming Space selection.'
  },
  {
    id: 'grid-row-selected-state',
    file: 'src/data.tsx',
    pattern: 'aria-selected={isSelected || undefined}',
    description: 'Data grid rows expose selected state to assistive technology.'
  },
  {
    id: 'grid-row-keyboard-action',
    file: 'src/data.tsx',
    pattern: 'event.key === "Enter" || event.key === " "',
    description: 'Data grid actionable rows can be activated from the keyboard.'
  },
  {
    id: 'grid-toolbar-role',
    file: 'src/data.tsx',
    pattern: 'role="toolbar"',
    description: 'Data grid toolbar exposes toolbar semantics.'
  },
  {
    id: 'grid-density-control-fieldset',
    file: 'src/data.tsx',
    pattern: '<fieldset',
    description: 'Data grid density control groups compact and comfortable options with native fieldset semantics.'
  },
  {
    id: 'grid-density-control-radios',
    file: 'src/data.tsx',
    pattern: 'type="radio"',
    description: 'Data grid density control uses native radio options for exclusive density modes.'
  },
  {
    id: 'grid-density-control-accessible-labels',
    file: 'src/data.tsx',
    pattern: 'pds-data-grid-density-control__sr-label',
    description: 'Data grid density control keeps icon-only radio options accessible with screen-reader labels.'
  },
  {
    id: 'grid-column-resize-button',
    file: 'src/data.tsx',
    pattern: 'export function DataGridColumnResizeHandle',
    description: 'Data grid column resizing is exposed as a shared button control.'
  },
  {
    id: 'grid-column-resize-pointer',
    file: 'src/data.tsx',
    pattern: 'onPointerDown',
    description: 'Data grid column resize handle uses pointer events for mouse and pen input.'
  },
  {
    id: 'grid-filter-trigger-button',
    file: 'src/data.tsx',
    pattern: 'export function DataGridFilterTrigger',
    description: 'Data grid filters expose a shared trigger control for active and dirty query states.'
  },
  {
    id: 'grid-filter-trigger-state',
    file: 'src/data.tsx',
    pattern: 'data-state={resolvedState}',
    description: 'Data grid filter trigger exposes stable state hooks for idle, active, and dirty states.'
  },
  {
    id: 'grid-filter-panel-region',
    file: 'src/data.tsx',
    pattern: 'aria-label={panelLabel}',
    description: 'Data grid filter builders expose a named panel structure for title, summary, controls, body, and actions.'
  },
  {
    id: 'grid-filter-rule-layout',
    file: 'src/data.tsx',
    pattern: 'export function DataGridFilterRule',
    description: 'Data grid filter rules use a shared row structure for field, operator, value, and row actions.'
  },
  {
    id: 'grid-filter-empty-status',
    file: 'src/data.tsx',
    pattern: 'role="status"',
    description: 'Data grid empty filter states expose non-blocking status semantics.'
  },
  {
    id: 'grid-control-popover-dialog',
    file: 'src/data.tsx',
    pattern: 'export const DataGridControlPopover',
    description: 'Data grid control popovers expose a shared dialog shell for filters and column menus.'
  },
  {
    id: 'grid-control-popover-label',
    file: 'src/data.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'Data grid control popovers require an accessible dialog name.'
  },
  {
    id: 'grid-control-popover-native',
    file: 'src/styles.css',
    pattern: '.pds-data-grid-control-popover[popover]:popover-open',
    description: 'Data grid control popovers define native popover open and closed states.'
  },
  {
    id: 'grid-column-trigger-button',
    file: 'src/data.tsx',
    pattern: 'export function DataGridColumnChooserTrigger',
    description: 'Data grid columns expose a shared trigger control for selected and customized column states.'
  },
  {
    id: 'grid-column-trigger-count',
    file: 'src/data.tsx',
    pattern: 'columns selected',
    description: 'Data grid column chooser trigger exposes an accessible selected-column count.'
  },
  {
    id: 'grid-sort-button-name',
    file: 'src/data.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'Data grid sort buttons require an accessible action name.'
  },
  {
    id: 'grid-sort-button-state',
    file: 'src/data.tsx',
    pattern: 'data-direction={direction ?? undefined}',
    description: 'Data grid sort buttons expose stable active direction state.'
  },
  {
    id: 'grid-column-chooser-search',
    file: 'src/data.tsx',
    pattern: 'type="search"',
    description: 'Data grid column chooser uses a native search input.'
  },
  {
    id: 'grid-column-chooser-checkboxes',
    file: 'src/data.tsx',
    pattern: 'type="checkbox"',
    description: 'Data grid column chooser uses native checkbox selection.'
  },
  {
    id: 'grid-column-chooser-grouping',
    file: 'src/data.tsx',
    pattern: '<fieldset',
    description: 'Data grid column chooser groups related columns with native fieldset semantics.'
  },
  {
    id: 'grid-pagination-nav',
    file: 'src/data.tsx',
    pattern: '<nav',
    description: 'Data grid pagination exposes navigation semantics.'
  },
  {
    id: 'field-group-legend',
    file: 'src/surfaces.tsx',
    pattern: '<legend',
    description: 'FieldGroup uses native fieldset and legend semantics.'
  },
  {
    id: 'kpi-tile-tone-state',
    file: 'src/surfaces.tsx',
    pattern: 'data-tone={tone}',
    description: 'KpiTile exposes stable tone hooks without hard-coding product palettes.'
  },
  {
    id: 'metric-trend-direction-state',
    file: 'src/surfaces.tsx',
    pattern: 'data-direction={direction}',
    description: 'MetricTrend exposes up, down, and flat states without relying only on color.'
  },
  {
    id: 'chart-shell-figure-region',
    file: 'src/surfaces.tsx',
    pattern: 'className="pds-chart-shell__figure"',
    description: 'ChartShell separates chart figure content from title, actions, and footer regions.'
  },
  {
    id: 'chart-legend-list',
    file: 'src/surfaces.tsx',
    pattern: 'role="list"',
    description: 'ChartLegend exposes legend entries through list semantics.'
  },
  {
    id: 'chart-image-role',
    file: 'src/charts.tsx',
    pattern: 'role="img"',
    description: 'Chart renderers expose an accessible image role for assistive technology.'
  },
  {
    id: 'chart-accessible-name',
    file: 'src/charts.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'Chart renderers require a caller-provided accessible name describing the series.'
  },
  {
    id: 'process-stepper-current-step',
    file: 'src/process.tsx',
    pattern: '"aria-current": isCurrent ? "step"',
    description: 'ProcessStepper exposes the current workflow step to assistive technology.'
  },
  {
    id: 'process-stepper-controls',
    file: 'src/process.tsx',
    pattern: '"aria-controls": step.controls',
    description: 'ProcessStepper can connect actionable steps to the controlled workflow content region.'
  },
  {
    id: 'process-stepper-compact-contract',
    file: 'src/process.tsx',
    pattern: 'compactPresentation?: "segments"',
    description: 'ProcessStepper exposes an opt-in compact presentation without product-owned descendant styling.'
  },
  {
    id: 'process-progress-segment-contract',
    file: 'src/process.tsx',
    pattern: 'variant?: "bar" | "dots" | "segments"',
    description: 'ProcessProgress exposes segmented compact workflow progress through its public API.'
  },
  {
    id: 'process-stepper-container-query',
    file: 'src/styles.css',
    pattern: '@container (max-width: 47.5rem)',
    description: 'ProcessStepper responsive composition follows its available container width.'
  },
  {
    id: 'process-progress-role',
    file: 'src/process.tsx',
    pattern: 'role="progressbar"',
    description: 'ProcessProgress exposes compact workflow progress through progressbar semantics.'
  },
  {
    id: 'process-progress-valuenow',
    file: 'src/process.tsx',
    pattern: 'aria-valuenow={normalizedCurrent}',
    description: 'ProcessProgress exposes current and total step counts to assistive technology.'
  },
  {
    id: 'validation-summary-alert',
    file: 'src/surfaces.tsx',
    pattern: 'role="alert"',
    description: 'ValidationSummary announces blocking validation feedback.'
  },
  {
    id: 'empty-state-status',
    file: 'src/surfaces.tsx',
    pattern: 'role="status"',
    description: 'EmptyState exposes non-blocking status semantics.'
  },
  {
    id: 'feedback-state-live-region',
    file: 'src/surfaces.tsx',
    pattern: 'aria-live={isBlocking ? "assertive" : "polite"}',
    description: 'FeedbackState announces blocking and non-blocking workflow states.'
  },
  {
    id: 'feedback-state-request-metadata',
    file: 'src/surfaces.tsx',
    pattern: 'requestId',
    description: 'FeedbackState can expose request metadata near errors.'
  },
  {
    id: 'operation-state-live-region',
    file: 'src/surfaces.tsx',
    pattern: 'className={composeClassNames("pds-operation-state", className)}',
    description: 'OperationState exposes the shared full-state vocabulary through status and alert live regions.'
  },
  {
    id: 'operation-state-request-correlation',
    file: 'src/surfaces.tsx',
    pattern: 'request?: RequestCorrelation',
    description: 'OperationState accepts typed request and correlation metadata without creating persistence or telemetry behavior.'
  },
  {
    id: 'operation-state-reduced-motion',
    file: 'src/styles.css',
    pattern: '.pds-operation-state[data-state="pending"] .pds-operation-state__indicator',
    description: 'OperationState removes its pending animation when reduced motion is requested.'
  },
  {
    id: 'inline-alert-live-region',
    file: 'src/surfaces.tsx',
    pattern: 'aria-live={blocking ? "assertive" : "polite"}',
    description: 'InlineAlert and Toast announce blocking and non-blocking feedback states.'
  },
  {
    id: 'banner-dismiss-action',
    file: 'src/surfaces.tsx',
    pattern: 'className="pds-banner__dismiss"',
    description: 'Banner exposes an optional dismiss action with an accessible name.'
  },
  {
    id: 'toast-region-named-region',
    file: 'src/surfaces.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'ToastRegion exposes a named notification region.'
  },
  {
    id: 'toast-dismiss-action',
    file: 'src/surfaces.tsx',
    pattern: 'onDismiss(id)',
    description: 'Toast exposes a dismiss action tied to the notification ID.'
  },
  {
    id: 'toast-request-metadata',
    file: 'src/surfaces.tsx',
    pattern: 'feedbackMetadataEntries(metadata)',
    description: 'Toast can expose request and correlation metadata near transient feedback.'
  },
  {
    id: 'skeleton-busy-state',
    file: 'src/surfaces.tsx',
    pattern: 'aria-busy="true"',
    description: 'Skeleton exposes loading state to assistive technology.'
  },
  {
    id: 'skeleton-variant-hook',
    file: 'src/surfaces.tsx',
    pattern: 'data-variant={variant}',
    description: 'Skeleton exposes stable variants for card and text loading states.'
  },
  {
    id: 'data-grid-loading-preview-status',
    file: 'src/data.tsx',
    pattern: 'className={composeClassNames("pds-data-grid-loading-preview", className)}',
    description: 'DataGridLoadingPreview exposes a stable, named loading preview for dense grids.'
  },
  {
    id: 'form-loading-preview-status',
    file: 'src/forms.tsx',
    pattern: 'className={composeClassNames("pds-form-loading-preview", className)}',
    description: 'FormLoadingPreview exposes a stable, named loading preview for generated forms.'
  },
  {
    id: 'message-thread-named-region',
    file: 'src/conversation.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'MessageThread exposes a named conversation region.'
  },
  {
    id: 'message-thread-message-list',
    file: 'src/conversation.tsx',
    pattern: 'aria-label="Messages"',
    description: 'MessageThread exposes messages as a named ordered list.'
  },
  {
    id: 'message-composer-native-form',
    file: 'src/conversation.tsx',
    pattern: 'onSubmit={submit}',
    description: 'MessageComposer uses native form submission semantics.'
  },
  {
    id: 'streaming-text-live-status',
    file: 'src/conversation.tsx',
    pattern: 'aria-live="polite"',
    description: 'StreamingText announces non-blocking streamed response changes.'
  },
  {
    id: 'tool-call-status-live-region',
    file: 'src/conversation.tsx',
    pattern: 'aria-live={status === "failed" || status === "blocked" ? "assertive" : "polite"}',
    description: 'ToolCallStatus escalates blocked or failed tool states.'
  },
  {
    id: 'entity-ref-card-addressable-link',
    file: 'src/conversation.tsx',
    pattern: 'href ? <a href={href}>{title}</a> : title',
    description: 'EntityRefCard supports route-safe generated view deep links.'
  },
  {
    id: 'citation-list-ordered',
    file: 'src/conversation.tsx',
    pattern: '<ol className="pds-citation-list__items">',
    description: 'CitationList exposes citations as ordered evidence.'
  },
  {
    id: 'confidence-signal-status',
    file: 'src/conversation.tsx',
    pattern: 'data-confidence={value}',
    description: 'ConfidenceSignal exposes stable confidence state without color-only meaning.'
  },
  {
    id: 'agent-timeline-named-region',
    file: 'src/conversation.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'AgentTimeline exposes the persistent agent workflow panel as a named region.'
  },
  {
    id: 'flow-graph-shell-image-role',
    file: 'src/conversation.tsx',
    pattern: 'role="img"',
    description: 'FlowGraphShell exposes graph context through a named image region.'
  },
  {
    id: 'generated-view-named-region',
    file: 'src/ambient.tsx',
    pattern: 'aria-label={ariaLabel}',
    description: 'GeneratedViewShell exposes AI-composed views as a named governed region.'
  },
  {
    id: 'generated-view-grounding-state',
    file: 'src/ambient.tsx',
    pattern: 'data-grounding={grounding}',
    description: 'GeneratedViewShell exposes stable grounding state for grounded, partial, unverified, and policy-denied views.'
  },
  {
    id: 'ai-attribution-live-status',
    file: 'src/ambient.tsx',
    pattern: 'aria-live="polite"',
    description: 'AiAttributionAffordance announces model-derived status without interrupting workflow.'
  },
  {
    id: 'freshness-indicator-state',
    file: 'src/ambient.tsx',
    pattern: 'data-freshness={value}',
    description: 'FreshnessIndicator exposes current, stale, and unknown state without relying only on color.'
  },
  {
    id: 'attention-marker-risk-alert',
    file: 'src/ambient.tsx',
    pattern: 'role={risk === "high" ? "alert" : "status"}',
    description: 'AttentionMarker escalates only high-risk cues to alert semantics.'
  },
  {
    id: 'suggested-action-preview-language',
    file: 'src/ambient.tsx',
    pattern: 'Preview required before any write.',
    description: 'SuggestedAction keeps ambient recommendations preview-gated before any write execution.'
  },
  {
    id: 'evidence-summary-ordered-claims',
    file: 'src/ambient.tsx',
    pattern: 'aria-label="Grounded claims"',
    description: 'EvidenceSummary exposes grounded claims as an ordered evidence list.'
  },
  {
    id: 'assist-level-fieldset',
    file: 'src/ambient.tsx',
    pattern: '<fieldset',
    description: 'AssistLevelControl groups autonomy choices with native fieldset semantics.'
  },
  {
    id: 'assist-level-native-radios',
    file: 'src/ambient.tsx',
    pattern: 'type="radio"',
    description: 'AssistLevelControl uses native exclusive radio options for Suggest, Co-pilot, and Autopilot.'
  },
  {
    id: 'assist-level-conservative-default',
    file: 'src/ambient.tsx',
    pattern: 'value = "suggest"',
    description: 'AssistLevelControl defaults to the conservative Suggest tier.'
  },
  {
    id: 'memory-chip-named-group',
    file: 'src/ambient.tsx',
    pattern: 'role="group"',
    description: 'MemoryChip exposes personalized context as a named group rather than silent state.'
  },
  {
    id: 'memory-chip-correction-buttons',
    file: 'src/ambient.tsx',
    pattern: 'onCorrect',
    description: 'MemoryChip exposes correction and reset affordances for personalized memory.'
  }
];

const referenceConsumerFiles = [
  {
    id: 'crm-ui-import',
    path: 'examples/products/crm/frontend/src/components/ui.tsx',
    pattern: '@appfw/pds-health-components',
    description: 'CRM reference UI re-exports at least one shared PDS component.'
  },
  {
    id: 'crm-style-import',
    path: 'examples/products/crm/frontend/src/styles/index.css',
    pattern: '@appfw/pds-health-components/styles.css',
    description: 'CRM reference frontend imports shared PDS component styles.'
  },
  {
    id: 'crm-date-field-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/RecordFormFields.tsx',
    pattern: 'DateField',
    description: 'CRM generated record forms consume the shared PDS date field.'
  },
  {
    id: 'crm-time-field-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/RecordFormFields.tsx',
    pattern: 'TimeField',
    description: 'CRM generated record forms consume the shared PDS time field.'
  },
  {
    id: 'crm-datetime-field-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/RecordFormFields.tsx',
    pattern: 'DateTimeField',
    description: 'CRM generated record forms consume the shared PDS datetime field.'
  },
  {
    id: 'crm-overlay-bridge',
    path: 'examples/products/crm/frontend/src/components/ui.tsx',
    pattern: 'ConfirmDialog',
    description: 'CRM reference UI bridge exposes shared PDS overlay components for migration.'
  },
  {
    id: 'crm-about-dialog-consumer',
    path: 'examples/products/crm/frontend/src/app/AppShell.tsx',
    pattern: '<Dialog',
    description: 'CRM about surface consumes the shared PDS dialog shell.'
  },
  {
    id: 'crm-delete-confirm-dialog-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/EditPanel.tsx',
    pattern: '<ConfirmDialog',
    description: 'CRM destructive delete confirmation consumes the shared PDS confirmation shell.'
  },
  {
    id: 'crm-kpi-tile-consumer',
    path: 'examples/products/crm/frontend/src/features/dashboard/DashboardWidgets.tsx',
    pattern: '<KpiTile',
    description: 'CRM dashboard KPI cards consume the shared PDS analytics tile.'
  },
  {
    id: 'crm-chart-shell-consumer',
    path: 'examples/products/crm/frontend/src/features/dashboard/DashboardScreen.tsx',
    pattern: '<ChartShell',
    description: 'CRM dashboard analytics panels consume the shared PDS chart shell while keeping product chart logic local.'
  },
  {
    id: 'crm-empty-state-consumer',
    path: 'examples/products/crm/frontend/src/features/dashboard/DashboardWidgets.tsx',
    pattern: '<EmptyState',
    description: 'CRM dashboard empty panel states consume the shared PDS empty-state primitive.'
  },
  {
    id: 'crm-account-health-chart-shell-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: '<ChartShell',
    description: 'CRM account-health analytics panels consume the shared PDS chart shell.'
  },
  {
    id: 'crm-account-health-kpi-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: '<KpiTile',
    description: 'CRM account-health metrics consume the shared PDS KPI tile.'
  },
  {
    id: 'crm-account-health-empty-state-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: '<EmptyState',
    description: 'CRM account-health empty panel states consume the shared PDS empty-state primitive.'
  },
  {
    id: 'crm-account-health-surface-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: '<Surface',
    description: 'CRM account-health hero consumes the shared PDS work surface shell.'
  },
  {
    id: 'crm-edit-panel-surface-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/EditPanel.tsx',
    pattern: '<Surface',
    description: 'CRM generated record edit forms consume the shared PDS work surface shell.'
  },
  {
    id: 'crm-edit-panel-form-loading-preview-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/EditPanel.tsx',
    pattern: '<FormLoadingPreview',
    description: 'CRM generated record loading state consumes the shared PDS form loading preview.'
  },
  {
    id: 'crm-entity-list-grid-loading-preview-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/EntityListView.tsx',
    pattern: '<DataGridLoadingPreview',
    description: 'CRM generated list loading state consumes the shared PDS data grid loading preview.'
  },
  {
    id: 'crm-edit-panel-button-consumer',
    path: 'examples/products/crm/frontend/src/scaffold/EditPanel.tsx',
    pattern: '<Button',
    description: 'CRM generated record form actions consume the shared PDS button primitive.'
  },
  {
    id: 'crm-audit-surface-consumer',
    path: 'examples/products/crm/frontend/src/features/audit/AuditScreen.tsx',
    pattern: '<Surface',
    description: 'CRM audit placeholder consumes the shared PDS work surface shell.'
  },
  {
    id: 'crm-account-health-inline-alert-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: '<InlineAlert',
    description: 'CRM account-health warnings consume the shared PDS inline alert.'
  },
  {
    id: 'crm-dashboard-inline-alert-consumer',
    path: 'examples/products/crm/frontend/src/features/dashboard/DashboardScreen.tsx',
    pattern: '<InlineAlert',
    description: 'CRM dashboard warnings consume the shared PDS inline alert.'
  },
  {
    id: 'crm-dashboard-skeleton-consumer',
    path: 'examples/products/crm/frontend/src/features/dashboard/DashboardWidgets.tsx',
    pattern: 'variant="card"',
    description: 'CRM dashboard loading metrics consume the shared PDS card skeleton.'
  },
  {
    id: 'crm-account-health-skeleton-consumer',
    path: 'examples/products/crm/frontend/src/features/accounts/AccountHealthDashboard.tsx',
    pattern: 'variant="card"',
    description: 'CRM account-health loading state consumes the shared PDS card skeleton.'
  },
  {
    id: 'crm-ts-path',
    path: 'examples/products/crm/frontend/tsconfig.json',
    pattern: '@appfw/pds-health-components',
    description: 'CRM TypeScript config resolves the local PDS component package.'
  },
  {
    id: 'crm-vite-alias',
    path: 'examples/products/crm/frontend/vite.config.ts',
    pattern: '@appfw/pds-health-components',
    description: 'CRM Vite config resolves the local PDS component package.'
  },
  {
    id: 'product-intake-pds-import',
    path: 'app_gen/src/bin/appfw_introspect.rs',
    pattern: '@appfw/pds-health-components',
    description: 'Product-intake frontend starter imports shared PDS components.'
  },
  {
    id: 'product-intake-overlay-example',
    path: 'app_gen/src/bin/appfw_introspect.rs',
    pattern: 'pds-overlay-example',
    description: 'Product-intake scaffold check requires a shared PDS overlay example.'
  },
  {
    id: 'product-intake-analytics-example',
    path: 'app_gen/src/bin/appfw_introspect.rs',
    pattern: 'pds-analytics-example',
    description: 'Product-intake scaffold check requires shared PDS analytics starter chrome.'
  },
  {
    id: 'product-intake-data-grid-example',
    path: 'app_gen/src/bin/appfw_introspect.rs',
    pattern: 'pds-data-grid-example',
    description: 'Product-intake scaffold check requires a shared PDS data-grid starter.'
  },
  {
    id: 'product-intake-generated-form-example',
    path: 'app_gen/src/bin/appfw_introspect.rs',
    pattern: 'pds-generated-form-example',
    description: 'Product-intake scaffold check requires shared PDS generated-form controls.'
  },
  {
    id: 'admin-ui-import',
    path: 'admin_ui/src/components/RecordsPanel.tsx',
    pattern: '@appfw/pds-health-components',
    description: 'Admin UI records chrome consumes shared PDS components.'
  },
  {
    id: 'admin-shell-button-consumer',
    path: 'admin_ui/src/App.tsx',
    pattern: '<Button',
    description: 'Admin UI app shell consumes the shared PDS Button primitive.'
  },
  {
    id: 'admin-shell-icon-button-consumer',
    path: 'admin_ui/src/App.tsx',
    pattern: '<IconButton',
    description: 'Admin UI app shell consumes the shared PDS IconButton primitive.'
  },
  {
    id: 'admin-shell-badge-consumer',
    path: 'admin_ui/src/App.tsx',
    pattern: '<Badge',
    description: 'Admin UI app shell consumes the shared PDS Badge primitive.'
  },
  {
    id: 'admin-record-drawer-button-consumer',
    path: 'admin_ui/src/components/RecordDrawer.tsx',
    pattern: '<Button',
    description: 'Admin record editing consumes the shared PDS Button primitive for its actions.'
  },
  {
    id: 'admin-filter-button-consumer',
    path: 'admin_ui/src/components/AdvancedFilterBuilder.tsx',
    pattern: '<Button',
    description: 'Admin advanced filter builder consumes the shared PDS Button primitive.'
  },
  {
    id: 'admin-filter-icon-button-consumer',
    path: 'admin_ui/src/components/AdvancedFilterBuilder.tsx',
    pattern: '<IconButton',
    description: 'Admin advanced filter builder consumes the shared PDS IconButton primitive.'
  },
  {
    id: 'admin-record-drawer-consumer',
    path: 'admin_ui/src/components/RecordDrawer.tsx',
    pattern: '<Drawer',
    description: 'Admin record editing consumes the shared PDS drawer shell.'
  },
  {
    id: 'admin-model-drawer-consumer',
    path: 'admin_ui/src/components/EntityModelDrawer.tsx',
    pattern: '<Drawer',
    description: 'Admin entity model details consume the shared PDS drawer shell.'
  },
  {
    id: 'admin-console-drawer-consumer',
    path: 'admin_ui/src/components/DeveloperConsole.tsx',
    pattern: '<Drawer',
    description: 'Admin developer diagnostics consume the shared PDS drawer shell.'
  },
  {
    id: 'admin-about-dialog-consumer',
    path: 'admin_ui/src/components/AboutModal.tsx',
    pattern: '<Dialog',
    description: 'Admin about surface consumes the shared PDS dialog shell.'
  },
  {
    id: 'admin-style-import',
    path: 'admin_ui/src/main.tsx',
    pattern: '@appfw/pds-health-components/styles.css',
    description: 'Admin UI imports shared PDS component styles.'
  },
  {
    id: 'admin-ts-path',
    path: 'admin_ui/tsconfig.json',
    pattern: '@appfw/pds-health-components',
    description: 'Admin UI TypeScript config resolves the local PDS component package.'
  },
  {
    id: 'admin-vite-alias',
    path: 'admin_ui/vite.config.ts',
    pattern: '@appfw/pds-health-components',
    description: 'Admin UI Vite config resolves the local PDS component package.'
  }
];

const densityDefaultFiles = [
  {
    id: 'shared-data-grid-default-density',
    path: 'appfw_ui/pds_health/components/src/data.tsx',
    pattern: 'density = "compact"',
    description: 'Shared data grid shell and toolbar default to compact density.'
  },
  {
    id: 'generator-ui-contract-default-density',
    path: 'app_gen/src/frontend.rs',
    pattern: 'density: "compact"',
    description: 'Generated product UI contracts default data grids to compact density.'
  },
  {
    id: 'crm-generated-contract-default-density',
    path: 'examples/products/crm/frontend/src/generated/appfw-ui-contract.ts',
    pattern: '"density": "compact"',
    description: 'CRM reference contract reflects the generated compact grid default.'
  },
  {
    id: 'crm-grid-preference-fallback-density',
    path: 'examples/products/crm/frontend/src/scaffold/gridPreferences.ts',
    pattern: 'defaultDensity: PdsDensity = "compact"',
    description: 'CRM reference grid preference fallback stays aligned with compact defaults.'
  },
  {
    id: 'admin-grid-default-density',
    path: 'admin_ui/src/components/RecordsPanel.tsx',
    pattern: 'useState<PdsDensity>("compact")',
    description: 'Admin UI records grid starts with the same compact density.'
  }
];

const bannedSourceTerms = [
  'crm',
  'account',
  'accounts',
  'activity',
  'activities',
  'contact',
  'contacts',
  'lead',
  'leads',
  'opportunity',
  'opportunities',
  'pipeline',
  'pipelines',
  'quote',
  'quotes'
];

const checks = [];

function relativePath(absolutePath) {
  return path.relative(repoRoot, absolutePath).split(path.sep).join('/');
}

function read(relative) {
  return fs.readFileSync(path.join(componentRoot, relative), 'utf8');
}

function readDesignSystem(relative) {
  return fs.readFileSync(path.join(designSystemRoot, relative), 'utf8');
}

function addCheck(id, ok, detail = {}) {
  checks.push({ id, ok, ...detail });
}

function readJsonCandidate(relativePaths) {
  const candidates = relativePaths.map((relative) => ({
    relative,
    absolute: path.isAbsolute(relative) ? relative : path.join(repoRoot, relative)
  }));
  const selected = candidates.find((candidate) => fs.existsSync(candidate.absolute));
  if (!selected) {
    return {
      present: false,
      selected_path: null,
      candidate_paths: relativePaths,
      json_valid: false,
      value: null,
      error: 'missing'
    };
  }
  try {
    return {
      present: true,
      selected_path: selected.relative,
      candidate_paths: relativePaths,
      json_valid: true,
      value: JSON.parse(fs.readFileSync(selected.absolute, 'utf8')),
      error: null
    };
  } catch (error) {
    return {
      present: true,
      selected_path: selected.relative,
      candidate_paths: relativePaths,
      json_valid: false,
      value: null,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

function validateEvidenceValue(id, value) {
  const checks = [];
  const record = (name, ok, detail = {}) => {
    checks.push({ name, ok, ...detail });
  };
  const checkNames = Array.isArray(value?.checks)
    ? value.checks.map((check) => check?.name).filter(Boolean)
    : [];

  if (id === 'g1-governed-write-posture') {
    record('command-governed-write-check', value?.command === 'governed-write-check', {
      actual: value?.command ?? null
    });
    record('lane-g1', value?.lane === 'G1', { actual: value?.lane ?? null });
    record('ok-true', value?.ok === true, { actual: value?.ok ?? null });
    record('gate-enforced', value?.gate?.enforced === true, {
      actual: value?.gate?.enforced ?? null
    });
    record('gate-ready-to-enforce', value?.gate?.ready_to_enforce === true, {
      actual: value?.gate?.ready_to_enforce ?? null
    });
    record('provider-graduation-ok', value?.provider_graduation?.ok === true, {
      actual: value?.provider_graduation?.ok ?? null
    });
  } else if (id === 'g1-governed-write-live-evidence') {
    record('command-provider-test', value?.command === 'provider-test', {
      actual: value?.command ?? null
    });
    record('lane-g1', value?.lane === 'G1', { actual: value?.lane ?? null });
    record('ok-true', value?.ok === true, { actual: value?.ok ?? null });
    record(
      'provider-external-api-supported',
      governedWriteExternalApiProviders.has(value?.provider),
      { actual: value?.provider ?? null }
    );
    record('operation-named-mutation', value?.operation === 'named_mutation', {
      actual: value?.operation ?? null
    });

    const delegatedActor = value?.delegated_actor_context;
    record('delegated-actor-object', Boolean(delegatedActor && typeof delegatedActor === 'object'), {
      actual: delegatedActor ? typeof delegatedActor : null
    });
    record('delegated-actor-tenant', typeof delegatedActor?.tenant === 'string' && delegatedActor.tenant.trim().length > 0, {
      actual: delegatedActor?.tenant ?? null
    });
    record(
      'delegated-actor-on-behalf-of',
      typeof delegatedActor?.on_behalf_of === 'string' && delegatedActor.on_behalf_of.trim().length > 0,
      { actual: delegatedActor?.on_behalf_of ?? null }
    );

    const tokenStore = value?.token_store_isolation;
    const partitionKey = Array.isArray(tokenStore?.partition_key)
      ? tokenStore.partition_key
      : [];
    record('token-store-object', Boolean(tokenStore && typeof tokenStore === 'object'), {
      actual: tokenStore ? typeof tokenStore : null
    });
    record(
      'token-store-partition-key',
      ['user', 'tenant', 'provider'].every((key) => partitionKey.includes(key)),
      { actual: partitionKey }
    );
    record('token-store-revocation-checked', tokenStore?.revocation_checked === true, {
      actual: tokenStore?.revocation_checked ?? null
    });

    const mutationRegistry = value?.mutation_registry;
    record('mutation-registry-object', Boolean(mutationRegistry && typeof mutationRegistry === 'object'), {
      actual: mutationRegistry ? typeof mutationRegistry : null
    });
    record(
      'mutation-registry-name',
      typeof mutationRegistry?.name === 'string' && mutationRegistry.name.trim().length > 0,
      { actual: mutationRegistry?.name ?? null }
    );
    record('mutation-registry-mcp-disabled', mutationRegistry?.mcp_enabled === false, {
      actual: mutationRegistry?.mcp_enabled ?? null
    });
    record(
      'mutation-registry-policy-scope',
      typeof mutationRegistry?.policy_scope === 'string' && mutationRegistry.policy_scope.trim().length > 0,
      { actual: mutationRegistry?.policy_scope ?? null }
    );

    const idempotency = value?.idempotency;
    record('idempotency-object', Boolean(idempotency && typeof idempotency === 'object'), {
      actual: idempotency ? typeof idempotency : null
    });
    record(
      'idempotency-key-source',
      typeof idempotency?.key_source === 'string' && idempotency.key_source.trim().length > 0,
      { actual: idempotency?.key_source ?? null }
    );
    record('idempotency-replay-rejected', idempotency?.replay_rejected === true, {
      actual: idempotency?.replay_rejected ?? null
    });

    const audit = value?.audit;
    record('audit-object', Boolean(audit && typeof audit === 'object'), {
      actual: audit ? typeof audit : null
    });
    record('audit-source-supported', governedWriteAuditSources.has(audit?.source), {
      actual: audit?.source ?? null
    });
    for (const field of ['correlation_id', 'provider_request_id', 'sink']) {
      record(
        `audit-${field.replaceAll('_', '-')}`,
        typeof audit?.[field] === 'string' && audit[field].trim().length > 0,
        { actual: audit?.[field] ?? null }
      );
    }
  } else if (id === 'u2-agent-harness-profile') {
    record('command-harness-check', value?.command === 'harness-check', {
      actual: value?.command ?? null
    });
    record('lane-u2', value?.lane === 'U2', { actual: value?.lane ?? null });
    record('ok-true', value?.ok === true, { actual: value?.ok ?? null });
    record(
      'governed-write-gate-present',
      checkNames.includes('governed-writes-disabled-unless-g1-evidence'),
      { actual: checkNames }
    );
  } else {
    record('known-evidence-id', false, { actual: id });
  }

  return {
    valid: checks.every((check) => check.ok),
    checks
  };
}

function externalEvidenceStatus(requirement) {
  const loaded = readJsonCandidate(requirement.paths);
  if (!loaded.present || !loaded.json_valid) {
    return {
      id: requirement.id,
      description: requirement.description,
      candidate_paths: loaded.candidate_paths,
      selected_path: loaded.selected_path,
      present: loaded.present,
      json_valid: loaded.json_valid,
      valid: false,
      error: loaded.error,
      checks: []
    };
  }
  const validation = validateEvidenceValue(requirement.id, loaded.value);
  return {
    id: requirement.id,
    description: requirement.description,
    candidate_paths: loaded.candidate_paths,
    selected_path: loaded.selected_path,
    present: true,
    json_valid: true,
    valid: validation.valid,
    error: null,
    checks: validation.checks
  };
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function taggedBlock(html, attribute, value) {
  const escapedAttribute = escapeRegExp(attribute);
  const escapedValue = escapeRegExp(value);
  const pattern = new RegExp(`<[^>]+${escapedAttribute}="${escapedValue}"[\\s\\S]*?</(?:article|section)>`);
  return html.match(pattern)?.[0] ?? '';
}

function collectComponentExports(filesByPath) {
  const componentExports = [];
  for (const [relative, text] of filesByPath) {
    if (!relative.startsWith('appfw_ui/pds_health/components/src/')) continue;
    if (!/\.(tsx|ts)$/.test(relative)) continue;
    if (relative.endsWith('/types.ts') || relative.endsWith('/copy.ts')) continue;

    const source = relative.replace('appfw_ui/pds_health/components/', '');
    const exportPattern = /^export\s+(?:function|const)\s+([A-Z][A-Za-z0-9]*)\b/gm;
    let match;
    while ((match = exportPattern.exec(text)) !== null) {
      const component = match[1];
      const propsType = `${component}Props`;
      const propsPattern = new RegExp(`^export\\s+type\\s+${escapeRegExp(propsType)}\\b`, 'm');
      componentExports.push({
        component,
        source,
        source_path: relative,
        props_type: propsType,
        props_exported: propsPattern.test(text)
      });
    }
  }
  return componentExports.sort((left, right) => left.component.localeCompare(right.component));
}

function collectSourceFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectSourceFiles(absolute));
    } else if (entry.isFile() && /\.(ts|tsx|css)$/.test(entry.name)) {
      files.push(absolute);
    }
  }
  return files;
}

function collectAgGridPolicyFiles(dir) {
  const files = [];
  const excludedDirectories = new Set([
    '.git', '.codex', 'node_modules', 'target', 'dist', 'build', 'coverage', '.next'
  ]);
  if (!fs.existsSync(dir)) return files;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory() && excludedDirectories.has(entry.name)) continue;
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectAgGridPolicyFiles(absolute));
    } else if (entry.isFile() && /\.(ts|tsx|js|jsx|mjs|cjs|css|json)$/.test(entry.name)) {
      files.push(absolute);
    }
  }
  return files;
}

const missingFiles = requiredFiles.filter((relative) => {
  const root = relative.startsWith('scripts/') ? repoRoot : componentRoot;
  return !fs.existsSync(path.join(root, relative));
});
addCheck('required-files', missingFiles.length === 0, {
  required: requiredFiles,
  missing: missingFiles
});

let sourceFiles = [];
if (fs.existsSync(srcRoot)) {
  sourceFiles = collectSourceFiles(srcRoot);
}

const sourceTextByFile = new Map(
  sourceFiles.map((absolute) => [relativePath(absolute), fs.readFileSync(absolute, 'utf8')])
);
const combinedSource = [...sourceTextByFile.values()].join('\n');
const packageCatalogText = sourceTextByFile.get('appfw_ui/pds_health/components/src/catalog.ts') ?? '';
const sourceComponentExports = collectComponentExports(sourceTextByFile);
const sourceComponentNames = sourceComponentExports.map((entry) => entry.component);
const packageJsonPath = path.join(componentRoot, 'package.json');
let packageJson = {};
if (fs.existsSync(packageJsonPath)) {
  packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
}

const catalogPackageJsonPath = path.join(interactiveCatalogRoot, 'package.json');
const catalogPackageJson = fs.existsSync(catalogPackageJsonPath)
  ? JSON.parse(fs.readFileSync(catalogPackageJsonPath, 'utf8'))
  : {};
const componentLockPath = path.join(componentRoot, 'package-lock.json');
const catalogLockPath = path.join(interactiveCatalogRoot, 'package-lock.json');
const componentLock = fs.existsSync(componentLockPath)
  ? JSON.parse(fs.readFileSync(componentLockPath, 'utf8'))
  : {};
const catalogLock = fs.existsSync(catalogLockPath)
  ? JSON.parse(fs.readFileSync(catalogLockPath, 'utf8'))
  : {};

const dataGridSourcePath = 'appfw_ui/pds_health/components/src/data-grid.tsx';
const dataGridSourceText = sourceTextByFile.get(dataGridSourcePath) ?? '';
const approvedCommunityModules = [
  'ClientSideRowModelModule',
  'CellStyleModule',
  'TextFilterModule',
  'TooltipModule',
  'NumberFilterModule',
  'DateFilterModule',
  'QuickFilterModule',
  'RowSelectionModule',
  'PaginationModule',
  'RenderApiModule'
];
const registeredModuleBody = dataGridSourceText.match(/const communityModules\s*=\s*\[(?<body>[\s\S]*?)\];/)?.groups?.body ?? '';
const registeredCommunityModules = registeredModuleBody
  .split(',')
  .map((value) => value.trim())
  .filter(Boolean);
const unexpectedCommunityModules = registeredCommunityModules.filter(
  (moduleName) => !approvedCommunityModules.includes(moduleName)
);
const missingCommunityModules = approvedCommunityModules.filter(
  (moduleName) => !registeredCommunityModules.includes(moduleName)
);
addCheck('ag-grid-community-module-allowlist', unexpectedCommunityModules.length === 0 && missingCommunityModules.length === 0, {
  approved: approvedCommunityModules,
  registered: registeredCommunityModules,
  unexpected: unexpectedCommunityModules,
  missing: missingCommunityModules
});

const agGridPolicyFiles = [
  'appfw_ui',
  'examples/products',
  'admin_ui',
  'tools',
  'app_gen'
].flatMap((relative) => collectAgGridPolicyFiles(path.join(repoRoot, relative)));
const directAgGridImportPattern = /\b(?:from|import\s*\(|require\s*\(|import)\s*["']ag-grid-(?:community|react|enterprise)(?:\/[^"']*)?["']/g;
const enterprisePattern = /ag-grid-enterprise|\bLicenseManager\s*\.\s*setLicenseKey\b|\bsetLicenseKey\s*\(/;
const legacyThemePattern = /ag-theme-[a-z0-9-]+/;
const directAgGridImportMatches = [];
const enterpriseMatches = [];
const legacyThemeMatches = [];

function isCodePosition(text, targetIndex) {
  let state = 'code';
  for (let index = 0; index < targetIndex; index += 1) {
    const current = text[index];
    const next = text[index + 1];
    if (state === 'line-comment') {
      if (current === '\n') state = 'code';
      continue;
    }
    if (state === 'block-comment') {
      if (current === '*' && next === '/') {
        state = 'code';
        index += 1;
      }
      continue;
    }
    if (state !== 'code') {
      if (current === '\\') {
        index += 1;
      } else if (
        (state === 'single-quote' && current === "'")
        || (state === 'double-quote' && current === '"')
        || (state === 'template' && current === '`')
      ) {
        state = 'code';
      }
      continue;
    }
    if (current === '/' && next === '/') {
      state = 'line-comment';
      index += 1;
    } else if (current === '/' && next === '*') {
      state = 'block-comment';
      index += 1;
    } else if (current === "'") {
      state = 'single-quote';
    } else if (current === '"') {
      state = 'double-quote';
    } else if (current === '`') {
      state = 'template';
    }
  }
  return state === 'code';
}

function findDirectAgGridImports(text) {
  return [...text.matchAll(directAgGridImportPattern)]
    .filter((match) => isCodePosition(text, match.index ?? 0))
    .map((match) => {
      const index = match.index ?? 0;
      const line = text.slice(0, index).split(/\r?\n/).length;
      const lineStart = text.lastIndexOf('\n', index - 1) + 1;
      const lineEnd = text.indexOf('\n', index);
      return {
        line,
        excerpt: text.slice(lineStart, lineEnd < 0 ? text.length : lineEnd).trim().slice(0, 180)
      };
    });
}

const agGridScannerFixtures = [
  { id: 'static-from', source: 'import { ModuleRegistry } from "ag-grid-community";', expected: 1 },
  { id: 'side-effect', source: 'import "ag-grid-community/styles.css";', expected: 1 },
  { id: 'spaced-dynamic', source: 'const grid = import ( "ag-grid-react" );', expected: 1 },
  { id: 'multiline-dynamic', source: 'const grid = import(\n  "ag-grid-community"\n);', expected: 1 },
  { id: 'require', source: 'const grid = require ( "ag-grid-community" );', expected: 1 },
  { id: 'line-comment', source: '// import("ag-grid-community")', expected: 0 },
  { id: 'block-comment', source: '/* require("ag-grid-react") */', expected: 0 },
  { id: 'quoted-example', source: 'const example = "import(\\"ag-grid-community\\")";', expected: 0 }
].map((fixture) => ({
  ...fixture,
  actual: findDirectAgGridImports(fixture.source).length
}));
const agGridEnterpriseScannerFixtures = [
  { id: 'enterprise-package', source: 'import "ag-grid-enterprise";', expected: true },
  { id: 'ag-license-manager', source: 'LicenseManager.setLicenseKey(key);', expected: true },
  { id: 'ag-license-function', source: 'setLicenseKey(key);', expected: true },
  { id: 'unrelated-property', source: 'const licenseKey = product.licenseKey;', expected: false },
  { id: 'unrelated-config', source: 'configure({ licenseKey: value });', expected: false }
].map((fixture) => ({
  ...fixture,
  actual: enterprisePattern.test(fixture.source)
}));
const agGridBoundaryFixtures = [
  { path: dataGridSourcePath, source: 'import "ag-grid-community";', expected_forbidden: false },
  { path: 'examples/products/example/grid.ts', source: 'import "ag-grid-community";', expected_forbidden: true }
].map((fixture) => ({
  ...fixture,
  actual_forbidden: findDirectAgGridImports(fixture.source).length > 0 && fixture.path !== dataGridSourcePath
}));
addCheck(
  'ag-grid-import-scanner-fixtures',
  agGridScannerFixtures.every((fixture) => fixture.actual === fixture.expected)
    && agGridBoundaryFixtures.every((fixture) => fixture.actual_forbidden === fixture.expected_forbidden)
    && agGridEnterpriseScannerFixtures.every((fixture) => fixture.actual === fixture.expected),
  {
    syntax: agGridScannerFixtures,
    boundary: agGridBoundaryFixtures,
    enterprise: agGridEnterpriseScannerFixtures
  }
);

for (const absolute of agGridPolicyFiles) {
  const relative = relativePath(absolute);
  const text = fs.readFileSync(absolute, 'utf8');
  if (relative !== dataGridSourcePath) {
    directAgGridImportMatches.push(...findDirectAgGridImports(text).map((match) => ({
      path: relative,
      ...match
    })));
  }
  text.split(/\r?\n/).forEach((line, index) => {
    if (enterprisePattern.test(line)) {
      enterpriseMatches.push({ path: relative, line: index + 1, excerpt: line.trim().slice(0, 180) });
    }
    if (legacyThemePattern.test(line)) {
      legacyThemeMatches.push({ path: relative, line: index + 1, excerpt: line.trim().slice(0, 180) });
    }
  });
}
addCheck('ag-grid-pds-import-boundary', directAgGridImportMatches.length === 0, {
  allowed: dataGridSourcePath,
  matches: directAgGridImportMatches
});
addCheck('ag-grid-community-only-policy', enterpriseMatches.length === 0 && legacyThemeMatches.length === 0, {
  enterprise_matches: enterpriseMatches,
  legacy_theme_matches: legacyThemeMatches
});

const expectedAgGridVersion = '36.0.1';
const agGridVersionEvidence = {
  component_community: packageJson.dependencies?.['ag-grid-community'] ?? null,
  component_react: packageJson.dependencies?.['ag-grid-react'] ?? null,
  catalog_community: catalogPackageJson.dependencies?.['ag-grid-community'] ?? null,
  catalog_react: catalogPackageJson.dependencies?.['ag-grid-react'] ?? null,
  component_lock_community: componentLock.packages?.['node_modules/ag-grid-community']?.version ?? null,
  component_lock_react: componentLock.packages?.['node_modules/ag-grid-react']?.version ?? null,
  catalog_lock_community: catalogLock.packages?.['node_modules/ag-grid-community']?.version ?? null,
  catalog_lock_react: catalogLock.packages?.['node_modules/ag-grid-react']?.version ?? null
};
const agGridLicenses = {
  component_lock_community: componentLock.packages?.['node_modules/ag-grid-community']?.license ?? null,
  component_lock_react: componentLock.packages?.['node_modules/ag-grid-react']?.license ?? null,
  catalog_lock_community: catalogLock.packages?.['node_modules/ag-grid-community']?.license ?? null,
  catalog_lock_react: catalogLock.packages?.['node_modules/ag-grid-react']?.license ?? null
};
addCheck(
  'ag-grid-version-and-license-alignment',
  [
    agGridVersionEvidence.component_community,
    agGridVersionEvidence.component_react,
    agGridVersionEvidence.component_lock_community,
    agGridVersionEvidence.component_lock_react
  ].every((version) => version === expectedAgGridVersion)
    && [
      agGridLicenses.component_lock_community,
      agGridLicenses.component_lock_react
    ].every((license) => license === 'MIT')
    && [
      agGridVersionEvidence.catalog_community,
      agGridVersionEvidence.catalog_react,
      agGridVersionEvidence.catalog_lock_community,
      agGridVersionEvidence.catalog_lock_react
    ].every((version) => version === null),
  {
    expected_version: expectedAgGridVersion,
    dependency_owner: 'appfw_ui/pds_health/components',
    catalog_dependency_posture: 'consume-through-pds-source-alias-no-duplicate-ag-grid-dependency',
    versions: agGridVersionEvidence,
    licenses: agGridLicenses
  }
);

const missingComponents = requiredComponents.filter((name) => {
  return !new RegExp(`export (function|const) ${name}\\b`).test(combinedSource);
});
addCheck('component-exports', missingComponents.length === 0, {
  required: requiredComponents,
  missing: missingComponents
});

const indexText = fs.existsSync(path.join(componentRoot, 'src/index.ts')) ? read('src/index.ts') : '';
const missingIndexExports = ['ambient', 'catalog', 'copy', 'conversation', 'data', 'data-grid', 'experience', 'forms', 'layout', 'process', 'primitives', 'surfaces', 'types'].filter(
  (moduleName) => !indexText.includes(`"./${moduleName}"`)
);
addCheck('index-barrel', missingIndexExports.length === 0, {
  missing_modules: missingIndexExports
});

const cssText = fs.existsSync(path.join(componentRoot, 'src/styles.css')) ? read('src/styles.css') : '';
const missingCssClasses = requiredCssClasses.filter((selector) => !cssText.includes(selector));
const tokenBacked = cssText.includes('@import "../../tokens/pdsTokens.css"')
  && cssText.includes('var(--pds-color')
  && cssText.includes('var(--pds-space')
  && cssText.includes('var(--pds-radius');
addCheck('token-backed-css', missingCssClasses.length === 0 && tokenBacked, {
  required_classes: requiredCssClasses,
  missing_classes: missingCssClasses,
  imports_tokens: cssText.includes('@import "../../tokens/pdsTokens.css"'),
  uses_pds_tokens: tokenBacked
});

const rawColorPattern = /#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(/;
const rawComponentColorMatches = [];
cssText.split(/\r?\n/).forEach((line, index) => {
  if (rawColorPattern.test(line)) {
    rawComponentColorMatches.push({
      path: 'appfw_ui/pds_health/components/src/styles.css',
      line: index + 1,
      excerpt: line.trim().slice(0, 160)
    });
  }
});
addCheck('token-only-component-css', rawComponentColorMatches.length === 0, {
  description: 'Framework-owned component CSS must consume named PDS tokens instead of raw color literals.',
  matches: rawComponentColorMatches
});

const flowGraphViewportMatch = cssText.match(/\.pds-flow-graph-shell__viewport\s*\{(?<body>[\s\S]*?)\n\}/);
const flowGraphViewportCss = flowGraphViewportMatch?.groups?.body ?? '';
const missingFlowGraphTokenVariables = requiredFlowGraphTokenBridgeVariables.filter(
  (variable) => !flowGraphViewportCss.includes(`${variable}:`)
);
const isPdsTokenBackedFlowGraphValue = (value) => Boolean(value && value.includes('var(--pds-'));
const flowGraphTokenBridgeNegativeProof = [
  {
    name: 'raw-color-mix',
    value: 'color-mix(in oklch, red 80%, transparent)',
    pds_token_backed: isPdsTokenBackedFlowGraphValue('color-mix(in oklch, red 80%, transparent)')
  },
  {
    name: 'raw-hex',
    value: '#ffffff',
    pds_token_backed: isPdsTokenBackedFlowGraphValue('#ffffff')
  }
].map((fixture) => ({
  ...fixture,
  ok: fixture.pds_token_backed === false
}));
const flowGraphTokenBridgeNegativeProofOk = flowGraphTokenBridgeNegativeProof.every((fixture) => fixture.ok);
const flowGraphTokenVariableEntries = requiredFlowGraphTokenBridgeVariables.map((variable) => {
  const pattern = new RegExp(`${variable.replaceAll('-', '\\-')}\\s*:\\s*([^;]+);`);
  const value = flowGraphViewportCss.match(pattern)?.[1]?.trim() ?? null;
  return {
    variable,
    value,
    pds_token_backed: isPdsTokenBackedFlowGraphValue(value)
  };
});
const nonPdsBackedFlowGraphVariables = flowGraphTokenVariableEntries
  .filter((entry) => entry.value && !entry.pds_token_backed)
  .map((entry) => entry.variable);
const flowGraphTokenBridgeOk =
  Boolean(flowGraphViewportMatch)
  && missingFlowGraphTokenVariables.length === 0
  && nonPdsBackedFlowGraphVariables.length === 0
  && flowGraphTokenBridgeNegativeProofOk
  && Boolean(packageJson.peerDependencies?.['@xyflow/react'])
  && packageJson.peerDependenciesMeta?.['@xyflow/react']?.optional === true;
const flowGraphTokenBridge = {
  status: 'local-token-bridge-proven',
  lane: 'CH5',
  adapter_ready: false,
  live_ready: false,
  shell_selector: '.pds-flow-graph-shell__viewport',
  renderer: '@xyflow/react',
  renderer_dependency: {
    package_path: 'appfw_ui/pds_health/components/package.json',
    optional_peer: true
  },
  variable_count: requiredFlowGraphTokenBridgeVariables.length,
  variables: flowGraphTokenVariableEntries,
  missing_variables: missingFlowGraphTokenVariables,
  non_pds_backed_variables: nonPdsBackedFlowGraphVariables,
  negative_proof: flowGraphTokenBridgeNegativeProof,
  remaining_evidence: [
    'framework-owned React Flow adapter',
    'keyboard/focus tests for graph nodes and controls',
    'catalog visual/a11y evidence with adapter mounted',
    'product-level visual/a11y evidence',
    'live chat/search/gateway evidence'
  ]
};
addCheck('flow-graph-token-bridge', flowGraphTokenBridgeOk, {
  description: 'FlowGraphShell publishes a PDS-token-backed --xy-* bridge for future @xyflow/react adapters.',
  status: flowGraphTokenBridge.status,
  adapter_ready: flowGraphTokenBridge.adapter_ready,
  live_ready: flowGraphTokenBridge.live_ready,
  missing_variables: missingFlowGraphTokenVariables,
  non_pds_backed_variables: nonPdsBackedFlowGraphVariables,
  negative_proof: flowGraphTokenBridgeNegativeProof
});

const missingAccessibilityPatterns = accessibilityPatterns.filter((item) => {
  const text = sourceTextByFile.get(`appfw_ui/pds_health/components/${item.file}`) ?? '';
  return !text.includes(item.pattern);
});
addCheck('accessibility-patterns', missingAccessibilityPatterns.length === 0, {
  required: accessibilityPatterns.map(({ id, description }) => ({ id, description })),
  missing: missingAccessibilityPatterns.map(({ id, description }) => ({ id, description }))
});

const residueMatches = [];
for (const [relative, text] of sourceTextByFile) {
  if (relative.endsWith('styles.css')) continue;
  text.split(/\r?\n/).forEach((line, index) => {
    const words = new Set(line.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean));
    for (const term of bannedSourceTerms) {
      if (words.has(term)) {
        residueMatches.push({
          path: relative,
          line: index + 1,
          term,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
  });
}
addCheck('product-neutral-source', residueMatches.length === 0, {
  terms: bannedSourceTerms,
  matches: residueMatches
});

const missingReferenceFiles = referenceFiles.filter((relative) => !fs.existsSync(path.join(referenceRoot, relative)));
const referenceTextByFile = new Map();
for (const relative of referenceFiles) {
  const absolute = path.join(referenceRoot, relative);
  if (fs.existsSync(absolute)) {
    referenceTextByFile.set(`appfw_ui/pds_health/reference/${relative}`, fs.readFileSync(absolute, 'utf8'));
  }
}
const referenceHtml = referenceTextByFile.get('appfw_ui/pds_health/reference/index.html') ?? '';
const referenceCss = referenceTextByFile.get('appfw_ui/pds_health/reference/reference.css') ?? '';
const referenceCatalogJson = referenceTextByFile.get('appfw_ui/pds_health/reference/catalog.json') ?? '';
let referenceCatalog = {};
let referenceCatalogParseError = null;
if (referenceCatalogJson) {
  try {
    referenceCatalog = JSON.parse(referenceCatalogJson);
  } catch (error) {
    referenceCatalogParseError = error instanceof Error ? error.message : String(error);
  }
}
const missingReferenceClasses = referenceCoverageClasses.filter(
  (className) => !referenceHtml.includes(className)
);
const referenceCatalogTitlePresent = referenceHtml.includes('PDS Component Catalog');
const missingReferenceCatalogFamilies = referenceCatalogFamilies.filter(
  (family) => !referenceHtml.includes(`data-pds-catalog-family="${family}"`)
);
const missingReferenceCatalogComponents = requiredComponents.filter(
  (component) => !referenceHtml.includes(`data-pds-component="${component}"`)
);
const missingReferenceCatalogSections = referenceCatalogSections.filter(
  (section) => !referenceHtml.includes(`data-pds-catalog-section="${section}"`)
);
const missingReferenceFamilyContracts = referenceCatalogFamilies.filter(
  (family) => !referenceHtml.includes(`data-pds-family-contract="${family}"`)
);
const missingReferenceFamilyContractParts = [];
for (const family of referenceCatalogFamilies) {
  const block = taggedBlock(referenceHtml, 'data-pds-family-contract', family);
  for (const part of referenceFamilyContractParts) {
    if (!block.includes(`data-pds-family-contract-part="${part}"`)) {
      missingReferenceFamilyContractParts.push({ family, part });
    }
  }
}
const referenceCatalogFamiliesManifest = Array.isArray(referenceCatalog.families)
  ? referenceCatalog.families
  : [];
const manifestFamilyNames = referenceCatalogFamiliesManifest
  .map((family) => family?.name)
  .filter(Boolean);
const manifestComponents = referenceCatalogFamiliesManifest
  .flatMap((family) => Array.isArray(family?.components) ? family.components : [])
  .filter(Boolean);
const plannedFamilySlotsManifest = Array.isArray(referenceCatalog.plannedFamilySlots)
  ? referenceCatalog.plannedFamilySlots
  : [];
const plannedFamilySlotSlugs = plannedFamilySlotsManifest
  .map((slot) => slot?.slug)
  .filter(Boolean);
const plannedFamilySlotSlugCounts = plannedFamilySlotSlugs.reduce((counts, slug) => {
  counts.set(slug, (counts.get(slug) ?? 0) + 1);
  return counts;
}, new Map());
const duplicatePlannedFamilySlots = [...plannedFamilySlotSlugCounts.entries()]
  .filter(([, count]) => count > 1)
  .map(([slug, count]) => ({ slug, count }));
const missingManifestPlannedFamilySlots = referencePlannedFamilySlots
  .filter((requiredSlot) => !plannedFamilySlotSlugCounts.has(requiredSlot.slug))
  .map((requiredSlot) => requiredSlot.slug);
const unexpectedManifestPlannedFamilySlots = plannedFamilySlotSlugs
  .filter((slug) => !referencePlannedFamilySlots.some((requiredSlot) => requiredSlot.slug === slug));
const missingManifestPlannedFamilySlotFields = [];
const missingManifestPlannedFamilySlotEvidence = [];
const unexpectedManifestPlannedFamilySlotEvidence = [];
for (const requiredSlot of referencePlannedFamilySlots) {
  const slot = plannedFamilySlotsManifest.find((candidate) => candidate?.slug === requiredSlot.slug);
  if (!slot) continue;
  for (const [field, expected] of [
    ['name', requiredSlot.name],
    ['status', requiredSlot.status],
    ['maturity', requiredSlot.maturity]
  ]) {
    if (slot?.[field] !== expected) {
      missingManifestPlannedFamilySlotFields.push({
        slug: requiredSlot.slug,
        field,
        expected,
        actual: slot?.[field] ?? null
      });
    }
  }
  if (!slot?.reason) {
    missingManifestPlannedFamilySlotFields.push({
      slug: requiredSlot.slug,
      field: 'reason',
      expected: 'non-empty',
      actual: slot?.reason ?? null
    });
  }
  const evidence = Array.isArray(slot?.evidence) ? slot.evidence : [];
  for (const evidenceId of requiredSlot.evidence) {
    if (!evidence.includes(evidenceId)) {
      missingManifestPlannedFamilySlotEvidence.push({
        slug: requiredSlot.slug,
        evidence: evidenceId
      });
    }
  }
  for (const evidenceId of evidence) {
    if (!requiredSlot.evidence.includes(evidenceId)) {
      unexpectedManifestPlannedFamilySlotEvidence.push({
        slug: requiredSlot.slug,
        evidence: evidenceId
      });
    }
  }
}
const manifestFamilyByComponent = new Map();
for (const family of referenceCatalogFamiliesManifest) {
  for (const component of Array.isArray(family?.components) ? family.components : []) {
    manifestFamilyByComponent.set(component, family?.name ?? null);
  }
}
const manifestComponentCounts = manifestComponents.reduce((counts, component) => {
  counts.set(component, (counts.get(component) ?? 0) + 1);
  return counts;
}, new Map());
const duplicateManifestComponents = [...manifestComponentCounts.entries()]
  .filter(([, count]) => count > 1)
  .map(([component, count]) => ({ component, count }));
const missingManifestFamilies = referenceCatalogFamilies.filter(
  (family) => !manifestFamilyNames.includes(family)
);
const missingManifestComponents = requiredComponents.filter(
  (component) => !manifestComponentCounts.has(component)
);
const unexpectedManifestComponents = manifestComponents.filter(
  (component) => !requiredComponents.includes(component)
);
const sourceComponentsMissingManifest = sourceComponentNames.filter(
  (component) => !manifestComponentCounts.has(component)
);
const manifestComponentsMissingSource = manifestComponents.filter(
  (component) => !sourceComponentNames.includes(component)
);
const sourceComponentsMissingProps = sourceComponentExports
  .filter((entry) => !entry.props_exported)
  .map(({ component, source, props_type }) => ({ component, source, props_type }));
const componentApiInventory = sourceComponentExports.map((entry) => ({
  component: entry.component,
  family: manifestFamilyByComponent.get(entry.component) ?? null,
  source: entry.source,
  props_type: entry.props_type,
  props_exported: entry.props_exported
}));
const apiContract = referenceCatalog.apiContract ?? {};
const apiContractSourceFiles = Array.isArray(apiContract.sourceFiles)
  ? apiContract.sourceFiles
  : [];
const missingManifestApiContractFields = [
  'componentExportRule',
  'sourceExportEvidence',
  'inventoryArtifactPath',
  'agentUse',
  'sourceFiles'
].filter((field) => {
  if (field === 'agentUse') return !Array.isArray(apiContract.agentUse) || apiContract.agentUse.length < 2;
  if (field === 'sourceFiles') return apiContractSourceFiles.length === 0;
  return !apiContract[field];
});
const missingApiContractSourceFiles = apiContractSourceFiles
  .filter((item) => !item?.path || !fs.existsSync(path.join(repoRoot, item.path)))
  .map((item) => item?.path ?? null);
const missingVisibleApiSourceFiles = apiContractSourceFiles
  .filter((item) => item?.path && !referenceHtml.includes(`data-pds-api-source="${item.path}"`))
  .map((item) => item.path);
const agentDecisionGuide = Array.isArray(referenceCatalog.agentDecisionGuide)
  ? referenceCatalog.agentDecisionGuide
  : [];
const agentRecipeSlugs = agentDecisionGuide
  .map((recipe) => recipe?.slug)
  .filter(Boolean);
const agentRecipeSlugCounts = agentRecipeSlugs.reduce((counts, slug) => {
  counts.set(slug, (counts.get(slug) ?? 0) + 1);
  return counts;
}, new Map());
const duplicateAgentRecipes = [...agentRecipeSlugCounts.entries()]
  .filter(([, count]) => count > 1)
  .map(([slug, count]) => ({ slug, count }));
const missingAgentRecipes = referenceAgentRecipeSlugs.filter(
  (slug) => !agentRecipeSlugCounts.has(slug)
);
const unexpectedAgentRecipes = agentRecipeSlugs.filter(
  (slug) => !referenceAgentRecipeSlugs.includes(slug)
);
const missingAgentRecipeFields = [];
const unexpectedAgentRecipeComponents = [];
const unexpectedAgentRecipeEvidence = [];
const missingAgentRecipeEvidence = [];
const missingVisibleAgentRecipes = [];
const missingVisibleAgentRecipeComponents = [];
const agentRecipeInventory = [];
for (const recipe of agentDecisionGuide) {
  const slug = recipe?.slug ?? null;
  if (!slug) {
    missingAgentRecipeFields.push({ slug, field: 'slug' });
    continue;
  }
  for (const field of ['intent', 'productOwns', 'avoid']) {
    if (!recipe?.[field]) {
      missingAgentRecipeFields.push({ slug, field });
    }
  }
  const startWith = Array.isArray(recipe?.startWith) ? recipe.startWith : [];
  if (startWith.length < 2) {
    missingAgentRecipeFields.push({ slug, field: 'startWith' });
  }
  for (const component of startWith) {
    if (!requiredComponents.includes(component)) {
      unexpectedAgentRecipeComponents.push({ slug, component });
    }
  }
  const evidence = Array.isArray(recipe?.evidence) ? recipe.evidence : [];
  if (evidence.length < 2) {
    missingAgentRecipeFields.push({ slug, field: 'evidence' });
  }
  for (const evidenceId of evidence) {
    if (!referenceAgentRecipeEvidence.includes(evidenceId)) {
      unexpectedAgentRecipeEvidence.push({ slug, evidence: evidenceId });
    }
  }
  for (const evidenceId of requiredRecipeEvidence[slug] ?? []) {
    if (!evidence.includes(evidenceId)) {
      missingAgentRecipeEvidence.push({ slug, evidence: evidenceId });
    }
  }
  const block = taggedBlock(referenceHtml, 'data-pds-agent-recipe', slug);
  if (!block) {
    missingVisibleAgentRecipes.push(slug);
  } else {
    for (const component of startWith) {
      if (!block.includes(`data-pds-agent-recipe-component="${component}"`)) {
        missingVisibleAgentRecipeComponents.push({ slug, component });
      }
    }
  }
  agentRecipeInventory.push({
    slug,
    component_count: startWith.length,
    evidence,
    start_with: startWith
  });
}
const missingPackageCatalogExports = [
  'pdsComponentCatalog',
  'pdsComponentFamilies',
  'pdsPlannedComponentFamilySlots',
  'pdsAgentDecisionGuide'
].filter((exportName) => !packageCatalogText.includes(`export const ${exportName}`));
const missingPackageCatalogFamilies = referenceCatalogFamilies.filter(
  (family) => !packageCatalogText.includes(`name: "${family}"`)
);
const missingPackageCatalogFamilySlugs = referenceCatalogFamiliesManifest
  .map((family) => family?.slug)
  .filter(Boolean)
  .filter((slug) => !packageCatalogText.includes(`slug: "${slug}"`));
const missingPackageCatalogComponents = requiredComponents.filter(
  (component) => !packageCatalogText.includes(`"${component}"`)
);
const missingPackageCatalogRecipes = referenceAgentRecipeSlugs.filter(
  (slug) => !packageCatalogText.includes(`slug: "${slug}"`)
);
const missingPackageCatalogMaturityLevels = referenceMaturityLevels.filter(
  (level) => !packageCatalogText.includes(`"${level}"`)
);
const missingPackageCatalogPlannedFamilySlots = referencePlannedFamilySlots
  .filter((slot) => !packageCatalogText.includes(`slug: "${slot.slug}"`)
    || !packageCatalogText.includes(`name: "${slot.name}"`)
    || !packageCatalogText.includes(`status: "${slot.status}"`)
    || !slot.evidence.every((evidenceId) => packageCatalogText.includes(`"${evidenceId}"`)))
  .map((slot) => slot.slug);
const missingManifestReadinessParts = referenceFamilyContractParts.filter(
  (part) => !Array.isArray(referenceCatalog.readinessContract)
    || !referenceCatalog.readinessContract.includes(part)
);
const missingManifestMaturityLevels = referenceMaturityLevels.filter(
  (level) => !Array.isArray(referenceCatalog.maturityLevels)
    || !referenceCatalog.maturityLevels.includes(level)
);
const unexpectedManifestMaturityLevels = Array.isArray(referenceCatalog.maturityLevels)
  ? referenceCatalog.maturityLevels.filter((level) => !referenceMaturityLevels.includes(level))
  : [];
const missingManifestEvidenceRequirements = referenceEvidenceRequirements.filter(
  (evidence) => !Array.isArray(referenceCatalog.evidenceRequirements)
    || !referenceCatalog.evidenceRequirements.includes(evidence)
);
const unexpectedManifestEvidenceRequirements = Array.isArray(referenceCatalog.evidenceRequirements)
  ? referenceCatalog.evidenceRequirements.filter((evidence) => !referenceEvidenceRequirements.includes(evidence))
  : [];
const missingManifestEntryPoints = [
  'tokens',
  'componentSource',
  'visibleCatalog',
  'interactiveCatalog',
  'catalogManifest',
  'evidenceCommand',
  'evidenceArtifact'
].filter((key) => !referenceCatalog.entrypoints?.[key]);
const missingManifestAgentSections = [
  'startHere',
  'do',
  'avoid'
].filter((key) => !Array.isArray(referenceCatalog.agentContract?.[key])
  || referenceCatalog.agentContract[key].length === 0);
const missingManifestFamilyContractParts = [];
for (const family of referenceCatalogFamiliesManifest) {
  for (const part of referenceFamilyContractParts) {
    if (!family?.contract?.[part]) {
      missingManifestFamilyContractParts.push({ family: family?.name ?? null, part });
    }
  }
}
const missingManifestFamilyReadiness = [];
const unexpectedManifestFamilyReadiness = [];
const missingManifestFamilyEvidence = [];
const unexpectedManifestFamilyEvidence = [];
const familyReadiness = [];
for (const family of referenceCatalogFamiliesManifest) {
  const readiness = family?.readiness ?? {};
  if (!readiness.level) {
    missingManifestFamilyReadiness.push({ family: family?.name ?? null, field: 'level' });
  } else if (!referenceMaturityLevels.includes(readiness.level)) {
    unexpectedManifestFamilyReadiness.push({
      family: family?.name ?? null,
      field: 'level',
      value: readiness.level
    });
  }
  if (!Array.isArray(readiness.agentUse) || readiness.agentUse.length < 2) {
    missingManifestFamilyReadiness.push({ family: family?.name ?? null, field: 'agentUse' });
  }
  if (!readiness.productBoundary) {
    missingManifestFamilyReadiness.push({ family: family?.name ?? null, field: 'productBoundary' });
  }
  const evidence = Array.isArray(readiness.evidence) ? readiness.evidence : [];
  if (evidence.length === 0) {
    missingManifestFamilyReadiness.push({ family: family?.name ?? null, field: 'evidence' });
  }
  for (const evidenceId of evidence) {
    if (!referenceEvidenceRequirements.includes(evidenceId)) {
      unexpectedManifestFamilyEvidence.push({
        family: family?.name ?? null,
        evidence: evidenceId
      });
    }
  }
  const requiredEvidence = maturityEvidenceRequirements[readiness.level] ?? baselineFamilyEvidence;
  for (const evidenceId of requiredEvidence) {
    if (!evidence.includes(evidenceId)) {
      missingManifestFamilyEvidence.push({
        family: family?.name ?? null,
        evidence: evidenceId
      });
    }
  }
  familyReadiness.push({
    family: family?.name ?? null,
    slug: family?.slug ?? null,
    level: readiness.level ?? null,
    component_count: Array.isArray(family?.components) ? family.components.length : 0,
    evidence_count: evidence.length,
    evidence
  });
}
const missingManifestVisibleFamilies = manifestFamilyNames.filter(
  (family) => !referenceHtml.includes(`data-pds-catalog-family="${family}"`)
);
const missingManifestVisibleComponents = manifestComponents.filter(
  (component) => !referenceHtml.includes(`data-pds-component="${component}"`)
);
const missingVisibleReadinessFamilies = [];
const missingVisibleReadinessLevels = [];
const missingVisibleReadinessEvidence = [];
for (const family of referenceCatalogFamiliesManifest) {
  const familyName = family?.name ?? null;
  const readiness = family?.readiness ?? {};
  if (!familyName) continue;
  const block = taggedBlock(referenceHtml, 'data-pds-readiness-family', familyName);
  if (!block) {
    missingVisibleReadinessFamilies.push(familyName);
    continue;
  }
  if (readiness.level && !block.includes(`data-pds-readiness-level="${readiness.level}"`)) {
    missingVisibleReadinessLevels.push({ family: familyName, level: readiness.level });
  }
  const evidence = Array.isArray(readiness.evidence) ? readiness.evidence : [];
  for (const evidenceId of evidence) {
    if (!block.includes(`data-pds-readiness-evidence="${evidenceId}"`)) {
      missingVisibleReadinessEvidence.push({ family: familyName, evidence: evidenceId });
    }
  }
}
const referenceImportsComponents = referenceHtml.includes('../components/src/styles.css');
const referenceUsesTokens = referenceCss.includes('var(--pds-color')
  && referenceCss.includes('var(--pds-space')
  && referenceCss.includes('var(--pds-radius');
const referenceResidueMatches = [];
for (const [relative, text] of referenceTextByFile) {
  text.split(/\r?\n/).forEach((line, index) => {
    const words = new Set(line.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean));
    for (const term of bannedSourceTerms) {
      if (words.has(term)) {
        referenceResidueMatches.push({
          path: relative,
          line: index + 1,
          term,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
  });
}

const missingInteractiveCatalogFiles = interactiveCatalogFiles.filter(
  (relative) => !fs.existsSync(path.join(interactiveCatalogRoot, relative))
);
const interactiveCatalogTextByFile = new Map();
for (const relative of interactiveCatalogFiles) {
  const absolute = path.join(interactiveCatalogRoot, relative);
  if (fs.existsSync(absolute)) {
    interactiveCatalogTextByFile.set(`appfw_ui/pds_health/catalog-app/${relative}`, fs.readFileSync(absolute, 'utf8'));
  }
}
const interactiveCatalogPackageText =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/package.json') ?? '';
let interactiveCatalogPackage = {};
let interactiveCatalogPackageParseError = null;
if (interactiveCatalogPackageText) {
  try {
    interactiveCatalogPackage = JSON.parse(interactiveCatalogPackageText);
  } catch (error) {
    interactiveCatalogPackageParseError = error instanceof Error ? error.message : String(error);
  }
}
const interactiveCatalogGitignore =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/.gitignore') ?? '';
const interactiveCatalogMain =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/src/main.tsx') ?? '';
const interactiveCatalogApp =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/src/App.tsx') ?? '';
const connectedFabricSource = fs.readFileSync(
  path.join(componentRoot, 'src/connected-fabric.tsx'),
  'utf8'
);
const interactiveCatalogData =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/src/lib/catalogData.ts') ?? '';
const interactiveCatalogExamples =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/src/examples.tsx') ?? '';
const interactiveCatalogSnippets =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/src/lib/snippets.ts') ?? '';
const interactiveCatalogVite =
  interactiveCatalogTextByFile.get('appfw_ui/pds_health/catalog-app/vite.config.ts') ?? '';
const missingInteractiveCatalogImports = [];
if (!interactiveCatalogMain.includes('@appfw/pds-health/tokens/pdsTokens.css')) {
  missingInteractiveCatalogImports.push('@appfw/pds-health/tokens/pdsTokens.css');
}
if (!interactiveCatalogMain.includes('@appfw/pds-health-components/styles.css')) {
  missingInteractiveCatalogImports.push('@appfw/pds-health-components/styles.css');
}
if (!interactiveCatalogData.includes('../../../reference/catalog.json')) {
  missingInteractiveCatalogImports.push('reference/catalog.json');
}
if (!interactiveCatalogData.includes('pdsComponentFamilies')) {
  missingInteractiveCatalogImports.push('pdsComponentFamilies');
}
if (!interactiveCatalogData.includes('pdsAgentDecisionGuide')) {
  missingInteractiveCatalogImports.push('pdsAgentDecisionGuide');
}
if (
  !interactiveCatalogVite.includes('../components/src')
  || !interactiveCatalogVite.includes('"@appfw/pds-health-components": path.resolve(pdsComponentsRoot, "index.ts")')
) {
  missingInteractiveCatalogImports.push('components source alias');
}
const missingInteractiveCatalogExamples = requiredComponents.filter(
  (component) => !new RegExp(`\\b${escapeRegExp(component)}\\s*:`).test(interactiveCatalogExamples)
);
const missingInteractiveCatalogSnippets = requiredComponents.filter(
  (component) => !new RegExp(`\\b${escapeRegExp(component)}\\s*:`).test(interactiveCatalogSnippets)
);
const missingInteractiveCatalogFamilies = referenceCatalogFamilies.filter(
  (family) => !interactiveCatalogApp.includes('FamilySection')
    || !interactiveCatalogData.includes('families = manifest.families')
    || !referenceCatalogJson.includes(`"name": "${family}"`)
);
const interactiveCatalogResidueMatches = [];
for (const [relative, text] of interactiveCatalogTextByFile) {
  text.split(/\r?\n/).forEach((line, index) => {
    const words = new Set(line.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean));
    for (const term of bannedSourceTerms) {
      if (words.has(term)) {
        interactiveCatalogResidueMatches.push({
          path: relative,
          line: index + 1,
          term,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
  });
}
const interactiveCatalogPackageOk = Boolean(
  interactiveCatalogPackage.name === '@appfw/pds-health-catalog-app'
    && interactiveCatalogPackage.private === true
    && interactiveCatalogPackage.scripts?.typecheck
    && interactiveCatalogPackage.scripts?.build
    && interactiveCatalogPackage.scripts?.['test:evidence']?.includes('check-pds-catalog-evidence.mjs')
    && interactiveCatalogPackage.dependencies?.react
    && interactiveCatalogPackage.dependencies?.['react-aria-components'] === '^1.19.0'
    && interactiveCatalogPackage.dependencies?.['react-dom']
    && interactiveCatalogPackage.devDependencies?.['@axe-core/playwright']
    && interactiveCatalogPackage.devDependencies?.['@playwright/test']
);
const interactiveCatalogGitignoreOk =
  interactiveCatalogGitignore.includes('node_modules/')
  && interactiveCatalogGitignore.includes('dist/');

const connectedFabricBaselinePath = path.join(
  componentRoot,
  'connected-fabric.accepted-baseline.json'
);
let connectedFabricBaseline = null;
let connectedFabricBaselineParseError = null;
if (fs.existsSync(connectedFabricBaselinePath)) {
  try {
    connectedFabricBaseline = JSON.parse(
      fs.readFileSync(connectedFabricBaselinePath, 'utf8')
    );
  } catch (error) {
    connectedFabricBaselineParseError =
      error instanceof Error ? error.message : String(error);
  }
}
const connectedFabricRequiredMarkers = Array.isArray(
  connectedFabricBaseline?.required_source_markers
)
  ? connectedFabricBaseline.required_source_markers
  : [];
const missingConnectedFabricMarkers = connectedFabricRequiredMarkers.filter(
  (marker) => !connectedFabricSource.includes(marker)
);

const nexusReadinessPath = path.join(componentRoot, 'nexus-readiness.json');
let nexusReadiness = null;
let nexusReadinessParseError = null;
if (fs.existsSync(nexusReadinessPath)) {
  try {
    nexusReadiness = JSON.parse(fs.readFileSync(nexusReadinessPath, 'utf8'));
  } catch (error) {
    nexusReadinessParseError =
      error instanceof Error ? error.message : String(error);
  }
}
const requiredNexusSlices = ['F0', 'F1', 'N0', 'N1', 'N2', 'N3', 'N4'];
const declaredNexusSlices = Array.isArray(nexusReadiness?.readiness_slices)
  ? nexusReadiness.readiness_slices.map((slice) => slice?.id).filter(Boolean)
  : [];
const missingNexusSlices = requiredNexusSlices.filter(
  (slice) => !declaredNexusSlices.includes(slice)
);

addCheck(
  'reference-preview',
  missingReferenceFiles.length === 0
    && missingReferenceClasses.length === 0
    && referenceImportsComponents
    && referenceUsesTokens
    && referenceResidueMatches.length === 0,
  {
    files: referenceFiles.map((relative) => `appfw_ui/pds_health/reference/${relative}`),
    missing_files: missingReferenceFiles,
    required_classes: referenceCoverageClasses,
    missing_classes: missingReferenceClasses,
    imports_component_styles: referenceImportsComponents,
    uses_pds_tokens: referenceUsesTokens,
    terms: bannedSourceTerms,
    matches: referenceResidueMatches
  }
);
addCheck(
  'interactive-catalog-app',
  missingInteractiveCatalogFiles.length === 0
    && !interactiveCatalogPackageParseError
    && interactiveCatalogPackageOk
    && interactiveCatalogGitignoreOk
    && missingInteractiveCatalogImports.length === 0
    && missingInteractiveCatalogExamples.length === 0
    && missingInteractiveCatalogSnippets.length === 0
    && missingInteractiveCatalogFamilies.length === 0
    && interactiveCatalogResidueMatches.length === 0,
  {
    description: 'Interactive PDS Component Catalog must render the same product-neutral manifest and real component source as the static catalog.',
    root: 'appfw_ui/pds_health/catalog-app',
    files: interactiveCatalogFiles.map((relative) => `appfw_ui/pds_health/catalog-app/${relative}`),
    missing_files: missingInteractiveCatalogFiles,
    package: {
      name: interactiveCatalogPackage.name ?? null,
      private: interactiveCatalogPackage.private ?? null,
      parse_error: interactiveCatalogPackageParseError,
      has_typecheck: Boolean(interactiveCatalogPackage.scripts?.typecheck),
      has_build: Boolean(interactiveCatalogPackage.scripts?.build),
      has_evidence_script: Boolean(
        interactiveCatalogPackage.scripts?.['test:evidence']?.includes('check-pds-catalog-evidence.mjs')
      ),
      react_aria_components:
        interactiveCatalogPackage.dependencies?.['react-aria-components'] ?? null,
      has_axe_dependency: Boolean(interactiveCatalogPackage.devDependencies?.['@axe-core/playwright']),
      has_playwright_dependency: Boolean(interactiveCatalogPackage.devDependencies?.['@playwright/test'])
    },
    gitignore_ok: interactiveCatalogGitignoreOk,
    missing_imports: missingInteractiveCatalogImports,
    missing_examples: missingInteractiveCatalogExamples,
    missing_snippets: missingInteractiveCatalogSnippets,
    missing_families: missingInteractiveCatalogFamilies,
    terms: bannedSourceTerms,
    matches: interactiveCatalogResidueMatches
  }
);
addCheck(
  'connected-fabric-accepted-baseline',
  Boolean(
    connectedFabricBaseline
      && !connectedFabricBaselineParseError
      && connectedFabricBaseline.contract_version === 'connected-fabric/1.0.0'
      && connectedFabricBaseline.status === 'protected'
      && connectedFabricBaseline.behavior?.routing?.edge_policy
        === 'shared-nodes-exclusive-edges'
      && connectedFabricBaseline.behavior?.accessibility?.reduced_motion_static === true
      && connectedFabricRequiredMarkers.length > 0
      && missingConnectedFabricMarkers.length === 0
      && interactiveCatalogApp.includes('<ConnectedFabric')
  ),
  {
    description: 'The Product Owner-accepted Connected Fabric implementation is protected as a coherent signature asset.',
    contract_version: connectedFabricBaseline?.contract_version ?? null,
    status: connectedFabricBaseline?.status ?? null,
    baseline: relativePath(connectedFabricBaselinePath),
    canonical_implementation:
      connectedFabricBaseline?.canonical_implementation ?? null,
    parse_error: connectedFabricBaselineParseError,
    required_source_markers: connectedFabricRequiredMarkers.length,
    missing_source_markers: missingConnectedFabricMarkers,
    mounted_by_catalog: interactiveCatalogApp.includes('<ConnectedFabric'),
    browser_evidence_command:
      connectedFabricBaseline?.browser_evidence_command ?? null
  }
);
addCheck(
  'nexus-experience-system-readiness',
  Boolean(
    nexusReadiness
      && !nexusReadinessParseError
      && nexusReadiness.schema_version === 'pds-nexus-readiness/1.0.0'
      && nexusReadiness.web_interaction_substrate === 'react-aria-components'
      && nexusReadiness.product_direct_substrate_imports_allowed === false
      && missingNexusSlices.length === 0
      && Array.isArray(nexusReadiness.interaction_proof)
      && nexusReadiness.interaction_proof.length >= 6
      && Array.isArray(nexusReadiness.required_evidence)
      && nexusReadiness.required_evidence.includes('package-only-consumer')
  ),
  {
    description: 'Nexus readiness is an executable layered journey contract, not a component-count claim.',
    manifest: relativePath(nexusReadinessPath),
    schema_version: nexusReadiness?.schema_version ?? null,
    status: nexusReadiness?.status ?? null,
    web_interaction_substrate:
      nexusReadiness?.web_interaction_substrate ?? null,
    product_direct_substrate_imports_allowed:
      nexusReadiness?.product_direct_substrate_imports_allowed ?? null,
    required_slices: requiredNexusSlices,
    declared_slices: declaredNexusSlices,
    missing_slices: missingNexusSlices,
    interaction_proof: nexusReadiness?.interaction_proof ?? [],
    required_evidence: nexusReadiness?.required_evidence ?? []
  }
);
addCheck(
  'reference-component-catalog',
  missingReferenceFiles.length === 0
    && referenceCatalogTitlePresent
    && missingReferenceCatalogFamilies.length === 0
    && missingReferenceCatalogComponents.length === 0,
  {
    title_present: referenceCatalogTitlePresent,
    required_families: referenceCatalogFamilies,
    missing_families: missingReferenceCatalogFamilies,
    required_components: requiredComponents,
    missing_components: missingReferenceCatalogComponents
  }
);
addCheck(
  'reference-enterprise-catalog-contract',
  missingReferenceFiles.length === 0
    && missingReferenceCatalogSections.length === 0
    && missingReferenceFamilyContracts.length === 0
    && missingReferenceFamilyContractParts.length === 0,
  {
    required_sections: referenceCatalogSections,
    missing_sections: missingReferenceCatalogSections,
    required_families: referenceCatalogFamilies,
    missing_family_contracts: missingReferenceFamilyContracts,
    required_family_parts: referenceFamilyContractParts,
    missing_family_parts: missingReferenceFamilyContractParts
  }
);
addCheck(
  'reference-agent-catalog-manifest',
  missingReferenceFiles.length === 0
    && !referenceCatalogParseError
    && referenceCatalog.version === 1
    && referenceCatalog.name === 'PDS Component Catalog'
    && missingManifestFamilies.length === 0
    && missingManifestComponents.length === 0
    && unexpectedManifestComponents.length === 0
    && duplicateManifestComponents.length === 0
    && missingManifestApiContractFields.length === 0
    && missingApiContractSourceFiles.length === 0
    && missingVisibleApiSourceFiles.length === 0
    && missingManifestReadinessParts.length === 0
    && missingManifestMaturityLevels.length === 0
    && unexpectedManifestMaturityLevels.length === 0
    && missingManifestEvidenceRequirements.length === 0
    && unexpectedManifestEvidenceRequirements.length === 0
    && missingManifestEntryPoints.length === 0
    && missingManifestAgentSections.length === 0
    && missingManifestFamilyContractParts.length === 0
    && missingManifestFamilyReadiness.length === 0
    && unexpectedManifestFamilyReadiness.length === 0
    && missingManifestFamilyEvidence.length === 0
    && unexpectedManifestFamilyEvidence.length === 0
    && missingManifestPlannedFamilySlots.length === 0
    && unexpectedManifestPlannedFamilySlots.length === 0
    && duplicatePlannedFamilySlots.length === 0
    && missingManifestPlannedFamilySlotFields.length === 0
    && missingManifestPlannedFamilySlotEvidence.length === 0
    && unexpectedManifestPlannedFamilySlotEvidence.length === 0
    && missingManifestVisibleFamilies.length === 0
    && missingManifestVisibleComponents.length === 0,
  {
    manifest_path: 'appfw_ui/pds_health/reference/catalog.json',
    parse_error: referenceCatalogParseError,
    version: referenceCatalog.version ?? null,
    name: referenceCatalog.name ?? null,
    required_families: referenceCatalogFamilies,
    manifest_families: manifestFamilyNames,
    missing_families: missingManifestFamilies,
    required_components: requiredComponents,
    manifest_components: manifestComponents,
    missing_components: missingManifestComponents,
    unexpected_components: unexpectedManifestComponents,
    duplicate_components: duplicateManifestComponents,
    api_contract: {
      component_export_rule: apiContract.componentExportRule ?? null,
      source_export_evidence: apiContract.sourceExportEvidence ?? null,
      inventory_artifact_path: apiContract.inventoryArtifactPath ?? null,
      source_files: apiContractSourceFiles
    },
    missing_api_contract_fields: missingManifestApiContractFields,
    missing_api_contract_source_files: missingApiContractSourceFiles,
    missing_visible_api_source_files: missingVisibleApiSourceFiles,
    required_readiness_parts: referenceFamilyContractParts,
    missing_readiness_parts: missingManifestReadinessParts,
    required_maturity_levels: referenceMaturityLevels,
    missing_maturity_levels: missingManifestMaturityLevels,
    unexpected_maturity_levels: unexpectedManifestMaturityLevels,
    required_evidence_requirements: referenceEvidenceRequirements,
    missing_evidence_requirements: missingManifestEvidenceRequirements,
    unexpected_evidence_requirements: unexpectedManifestEvidenceRequirements,
    missing_entrypoints: missingManifestEntryPoints,
    missing_agent_sections: missingManifestAgentSections,
    missing_family_contract_parts: missingManifestFamilyContractParts,
    family_readiness: familyReadiness,
    missing_family_readiness: missingManifestFamilyReadiness,
    unexpected_family_readiness: unexpectedManifestFamilyReadiness,
    missing_family_evidence: missingManifestFamilyEvidence,
    unexpected_family_evidence: unexpectedManifestFamilyEvidence,
    planned_family_slots: plannedFamilySlotsManifest,
    required_planned_family_slots: referencePlannedFamilySlots.map((slot) => slot.slug),
    missing_planned_family_slots: missingManifestPlannedFamilySlots,
    unexpected_planned_family_slots: unexpectedManifestPlannedFamilySlots,
    duplicate_planned_family_slots: duplicatePlannedFamilySlots,
    missing_planned_family_slot_fields: missingManifestPlannedFamilySlotFields,
    missing_planned_family_slot_evidence: missingManifestPlannedFamilySlotEvidence,
    unexpected_planned_family_slot_evidence: unexpectedManifestPlannedFamilySlotEvidence,
    missing_visible_families: missingManifestVisibleFamilies,
    missing_visible_components: missingManifestVisibleComponents
  }
);
addCheck(
  'source-export-catalog-drift',
  missingReferenceFiles.length === 0
    && sourceComponentsMissingManifest.length === 0
    && manifestComponentsMissingSource.length === 0
    && sourceComponentsMissingProps.length === 0,
  {
    description: 'Named component exports, matching props types, and catalog manifest component inventory must stay in lockstep.',
    source_export_count: sourceComponentExports.length,
    manifest_component_count: manifestComponents.length,
    missing_from_manifest: sourceComponentsMissingManifest,
    missing_from_source: manifestComponentsMissingSource,
    missing_props_types: sourceComponentsMissingProps,
    component_api_inventory: componentApiInventory
  }
);
addCheck(
  'package-catalog-api',
  missingFiles.length === 0
    && missingPackageCatalogExports.length === 0
    && missingPackageCatalogFamilies.length === 0
    && missingPackageCatalogFamilySlugs.length === 0
    && missingPackageCatalogComponents.length === 0
    && missingPackageCatalogRecipes.length === 0
    && missingPackageCatalogMaturityLevels.length === 0
    && missingPackageCatalogPlannedFamilySlots.length === 0,
  {
    description: 'The component package must export a typed catalog API that stays aligned with the reference manifest.',
    path: 'appfw_ui/pds_health/components/src/catalog.ts',
    required_exports: [
      'pdsComponentCatalog',
      'pdsComponentFamilies',
      'pdsPlannedComponentFamilySlots',
      'pdsAgentDecisionGuide'
    ],
    missing_exports: missingPackageCatalogExports,
    missing_families: missingPackageCatalogFamilies,
    missing_family_slugs: missingPackageCatalogFamilySlugs,
    missing_components: missingPackageCatalogComponents,
    missing_recipes: missingPackageCatalogRecipes,
    missing_maturity_levels: missingPackageCatalogMaturityLevels,
    missing_planned_family_slots: missingPackageCatalogPlannedFamilySlots
  }
);
addCheck(
  'reference-agent-api-source-map',
  missingReferenceFiles.length === 0
    && referenceHtml.includes('data-pds-api-contract="source-map"')
    && missingVisibleApiSourceFiles.length === 0
    && missingManifestApiContractFields.length === 0
    && missingApiContractSourceFiles.length === 0,
  {
    description: 'Visible catalog must expose the framework-owned source API map used by agents.',
    api_contract_present: referenceHtml.includes('data-pds-api-contract="source-map"'),
    source_files: apiContractSourceFiles.map((item) => item?.path).filter(Boolean),
    missing_visible_source_files: missingVisibleApiSourceFiles,
    missing_source_files: missingApiContractSourceFiles,
    missing_contract_fields: missingManifestApiContractFields
  }
);
addCheck(
  'reference-agent-decision-guide',
  missingReferenceFiles.length === 0
    && missingAgentRecipes.length === 0
    && unexpectedAgentRecipes.length === 0
    && duplicateAgentRecipes.length === 0
    && missingAgentRecipeFields.length === 0
    && missingAgentRecipeEvidence.length === 0
    && unexpectedAgentRecipeComponents.length === 0
    && unexpectedAgentRecipeEvidence.length === 0
    && missingVisibleAgentRecipes.length === 0
    && missingVisibleAgentRecipeComponents.length === 0,
  {
    description: 'Agent decision recipes must map common product workflows to existing PDS components and visible catalog examples.',
    required_recipes: referenceAgentRecipeSlugs,
    recipe_inventory: agentRecipeInventory,
    allowed_evidence: referenceAgentRecipeEvidence,
    missing_recipes: missingAgentRecipes,
    unexpected_recipes: unexpectedAgentRecipes,
    duplicate_recipes: duplicateAgentRecipes,
    missing_fields: missingAgentRecipeFields,
    unexpected_components: unexpectedAgentRecipeComponents,
    missing_evidence: missingAgentRecipeEvidence,
    unexpected_evidence: unexpectedAgentRecipeEvidence,
    missing_visible_recipes: missingVisibleAgentRecipes,
    missing_visible_components: missingVisibleAgentRecipeComponents
  }
);
addCheck(
  'reference-agent-readiness-ledger',
  missingReferenceFiles.length === 0
    && missingVisibleReadinessFamilies.length === 0
    && missingVisibleReadinessLevels.length === 0
    && missingVisibleReadinessEvidence.length === 0,
  {
    required_families: manifestFamilyNames,
    missing_families: missingVisibleReadinessFamilies,
    missing_levels: missingVisibleReadinessLevels,
    missing_evidence: missingVisibleReadinessEvidence
  }
);

const missingReferenceConsumers = referenceConsumerFiles.filter((item) => {
  const absolute = path.join(repoRoot, item.path);
  return !fs.existsSync(absolute) || !fs.readFileSync(absolute, 'utf8').includes(item.pattern);
});
addCheck('reference-consumer-wiring', missingReferenceConsumers.length === 0, {
  required: referenceConsumerFiles.map(({ id, path: filePath, description }) => ({
    id,
    path: filePath,
    description
  })),
  missing: missingReferenceConsumers.map(({ id, path: filePath, description }) => ({
    id,
    path: filePath,
    description
  }))
});

const missingDensityDefaults = densityDefaultFiles.filter((item) => {
  const absolute = path.join(repoRoot, item.path);
  return !fs.existsSync(absolute) || !fs.readFileSync(absolute, 'utf8').includes(item.pattern);
});
addCheck('density-default-policy', missingDensityDefaults.length === 0, {
  required: densityDefaultFiles.map(({ id, path: filePath, description }) => ({
    id,
    path: filePath,
    description
  })),
  missing: missingDensityDefaults.map(({ id, path: filePath, description }) => ({
    id,
    path: filePath,
    description
  }))
});

const packageContractOk = Boolean(
  packageJson.name === '@appfw/pds-health-components'
    && packageJson.private === true
    && packageJson.dependencies?.['react-aria-components'] === '^1.19.0'
    && packageJson.peerDependencies?.react
    && packageJson.exports?.['./styles.css']
);
addCheck('package-contract', packageContractOk, {
  name: packageJson.name ?? null,
  private: packageJson.private ?? null,
  react_aria_components: packageJson.dependencies?.['react-aria-components'] ?? null,
  has_react_peer: Boolean(packageJson.peerDependencies?.react),
  exports_styles: Boolean(packageJson.exports?.['./styles.css'])
});

// --- Versioning, lifecycle & release certification (governance) ---
const SEMVER_RE = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const lifecycleStatuses = ['stable', 'beta', 'experimental', 'deprecated'];
const packageVersion = packageJson.version ?? null;
const packageVersionValid = typeof packageVersion === 'string' && SEMVER_RE.test(packageVersion);

const changelogAbsolute = path.join(designSystemRoot, 'CHANGELOG.md');
const changelogText = fs.existsSync(changelogAbsolute)
  ? fs.readFileSync(changelogAbsolute, 'utf8')
  : '';
const changelogExists = changelogText.length > 0;
const changelogReferencesVersion = Boolean(packageVersion) && changelogText.includes(`[${packageVersion}]`);

const lifecycleExported = packageCatalogText.includes('export const pdsComponentLifecycle');
const packageVersionExported = Boolean(packageVersion)
  && packageCatalogText.includes(`pdsPackageVersion = "${packageVersion}"`);

const manifestVersioning = (referenceCatalog && referenceCatalog.versioning) || {};
const requiredVersioningFields = ['packageVersion', 'scheme', 'changelog', 'statuses', 'deprecationPolicy'];
const missingVersioningFields = requiredVersioningFields.filter((field) => !(field in manifestVersioning));
const manifestVersionMatches = manifestVersioning.packageVersion === packageVersion;

addCheck(
  'package-versioning',
  packageVersionValid
    && changelogExists
    && changelogReferencesVersion
    && lifecycleExported
    && packageVersionExported
    && manifestVersionMatches
    && missingVersioningFields.length === 0,
  {
    description: 'The component package must be SemVer-versioned with a CHANGELOG and a manifest versioning contract that stays aligned with package.json and catalog source.',
    package_version: packageVersion,
    semver_valid: packageVersionValid,
    changelog_path: 'appfw_ui/pds_health/CHANGELOG.md',
    changelog_exists: changelogExists,
    changelog_references_version: changelogReferencesVersion,
    lifecycle_exported: lifecycleExported,
    package_version_exported: packageVersionExported,
    manifest_version_matches: manifestVersionMatches,
    missing_versioning_fields: missingVersioningFields
  }
);

const manifestLifecycle = (referenceCatalog && referenceCatalog.componentLifecycle) || {};
const lifecycleMissing = requiredComponents.filter((component) => !manifestLifecycle[component]);
const lifecycleInvalidStatus = requiredComponents.filter(
  (component) => manifestLifecycle[component] && !lifecycleStatuses.includes(manifestLifecycle[component].status)
);
const lifecycleMissingSince = requiredComponents.filter(
  (component) => manifestLifecycle[component] && !manifestLifecycle[component].since
);
const lifecycleUnexpected = Object.keys(manifestLifecycle).filter(
  (component) => !requiredComponents.includes(component)
);
// A deprecated component must declare a replacement path and a removeBy version.
const lifecycleBadDeprecations = Object.entries(manifestLifecycle)
  .filter(([, entry]) => entry.status === 'deprecated' && !(entry.deprecated && entry.deprecated.removeBy && entry.deprecated.replacement))
  .map(([component]) => component);
const sourceLifecycleOverrides = Object.fromEntries(
  [...packageCatalogText.matchAll(/^  (\w+): \{\n    status: "([^"]+)"/gm)]
    .map(([, component, status]) => [component, status])
);
const sourceStatusByMaturity = {
  'release-gated': 'stable',
  'enterprise-ready': 'beta',
  foundation: 'experimental'
};
const lifecycleSourceManifestDrift = [];
for (const family of referenceCatalog?.families ?? []) {
  for (const component of family.components ?? []) {
    const sourceStatus = sourceLifecycleOverrides[component]
      ?? sourceStatusByMaturity[family.readiness?.level];
    const manifestStatus = manifestLifecycle[component]?.status;
    if (sourceStatus !== manifestStatus) {
      lifecycleSourceManifestDrift.push({ component, source_status: sourceStatus, manifest_status: manifestStatus });
    }
  }
}

addCheck(
  'component-lifecycle',
  lifecycleMissing.length === 0
    && lifecycleInvalidStatus.length === 0
    && lifecycleMissingSince.length === 0
    && lifecycleUnexpected.length === 0
    && lifecycleBadDeprecations.length === 0
    && lifecycleSourceManifestDrift.length === 0,
  {
    description: 'Every exported component must carry a lifecycle status and since-version, and deprecations must declare a replacement and removeBy.',
    required_components: requiredComponents.length,
    lifecycle_count: Object.keys(manifestLifecycle).length,
    valid_statuses: lifecycleStatuses,
    missing_lifecycle: lifecycleMissing,
    invalid_status: lifecycleInvalidStatus,
    missing_since: lifecycleMissingSince,
    unexpected: lifecycleUnexpected,
    incomplete_deprecations: lifecycleBadDeprecations,
    source_manifest_drift: lifecycleSourceManifestDrift
  }
);

const lifecycleStatusCounts = {};
for (const component of requiredComponents) {
  const status = manifestLifecycle[component]?.status ?? 'unknown';
  lifecycleStatusCounts[status] = (lifecycleStatusCounts[status] ?? 0) + 1;
}
const deprecatedComponents = Object.entries(manifestLifecycle)
  .filter(([, entry]) => entry.status === 'deprecated')
  .map(([component]) => component);

// --- Maturity evidence: a level claim is only honored when its required
// evidence is backed by passing checks (audit F-8). ---
const checkPassed = new Map(checks.map((check) => [check.id, check.ok]));
const manifestMaturityEvidence = (referenceCatalog && referenceCatalog.maturityEvidence) || {};
const maturityEvidenceContractOk = Boolean(
  manifestMaturityEvidence.basis === maturityEvidenceBasis
    && manifestMaturityEvidence.requiredByLevel
    && manifestMaturityEvidence.verifiedBy
);
const unsupportedMaturityClaims = [];
const maturityEvidenceFamilies = referenceCatalogFamiliesManifest.map((family) => {
  const level = family?.readiness?.level ?? null;
  const required = maturityEvidenceRequirements[level] ?? [];
  const evidenceChecks = required.map((category) => {
    const backing = evidenceVerifiedBy[category] ?? null;
    const external = typeof backing === 'string' && backing.startsWith('external:');
    return { category, backing, external, ok: external ? true : checkPassed.get(backing) === true };
  });
  const satisfied = evidenceChecks.every((entry) => entry.ok);
  if (level && !satisfied) {
    unsupportedMaturityClaims.push({
      family: family?.name ?? null,
      level,
      unmet: evidenceChecks.filter((entry) => !entry.ok).map((entry) => entry.category)
    });
  }
  return {
    family: family?.name ?? null,
    level,
    required_evidence: required,
    declared_evidence: Array.isArray(family?.readiness?.evidence) ? family.readiness.evidence : [],
    evidence_checks: evidenceChecks,
    satisfied
  };
});
addCheck(
  'maturity-evidence',
  maturityEvidenceContractOk && unsupportedMaturityClaims.length === 0,
  {
    description: 'Each family maturity claim must be backed by passing evidence checks for its level; basis is retained local/CI evidence, not live managed-environment certification.',
    basis: maturityEvidenceBasis,
    manifest_contract_ok: maturityEvidenceContractOk,
    unsupported_claims: unsupportedMaturityClaims
  }
);
const maturityEvidence = {
  basis: maturityEvidenceBasis,
  note: manifestMaturityEvidence.note ?? null,
  manifest_contract_ok: maturityEvidenceContractOk,
  unsupported_claims: unsupportedMaturityClaims,
  families: maturityEvidenceFamilies
};

const governedActionEvidence = governedActionLiveEvidenceRequirements.map(externalEvidenceStatus);
const governedActionMissingEvidence = governedActionEvidence
  .filter((evidence) => !evidence.present)
  .map((evidence) => evidence.id);
const governedActionInvalidEvidence = governedActionEvidence
  .filter((evidence) => evidence.present && !evidence.valid)
  .map((evidence) => evidence.id);
const governedActionLiveReady = governedActionEvidence.every((evidence) => evidence.valid);
const governedActionLiveReadiness = {
  basis: 'G2 consumes G1 governed-write posture/live evidence and U2 agent-harness evidence before action primitives may represent live write execution.',
  enforced: enforceGovernedAction,
  live_ready: governedActionLiveReady,
  missing_evidence: governedActionMissingEvidence,
  invalid_evidence: governedActionInvalidEvidence,
  evidence: governedActionEvidence
};
addCheck(
  'governed-action-live-readiness',
  !enforceGovernedAction || governedActionLiveReady,
  {
    description: 'Governed-action UI primitives are only live-write-ready when G1 posture, G1 live evidence, and U2 harness artifacts are present and valid.',
    enforced: enforceGovernedAction,
    live_ready: governedActionLiveReady,
    missing_evidence: governedActionMissingEvidence,
    invalid_evidence: governedActionInvalidEvidence
  }
);

const chatMarkdownSanitizerDocPresent = fs.existsSync(chatMarkdownSanitizerDocPath);
const chatMarkdownSanitizerDocText = chatMarkdownSanitizerDocPresent
  ? fs.readFileSync(chatMarkdownSanitizerDocPath, 'utf8')
  : '';
const chatMarkdownSanitizerClauses = chatMarkdownSanitizerRequiredClauses.map((clause) => ({
  ...clause,
  present: chatMarkdownSanitizerDocText.includes(clause.pattern)
}));
const chatMarkdownSanitizerOk = chatMarkdownSanitizerDocPresent
  && chatMarkdownSanitizerClauses.every((clause) => clause.present);
const conversationMarkdownSanitizer = {
  status: 'decision-recorded-not-wired',
  lane: 'CH5',
  decision: {
    renderer: 'react-markdown',
    sanitizer: 'rehype-sanitize',
    raw_html: 'disabled',
    server_dependency: false,
    product_direct_imports_allowed: false
  },
  live_ready: false,
  implementation_wired: false,
  decision_doc: relativePath(chatMarkdownSanitizerDocPath),
  decision_doc_present: chatMarkdownSanitizerDocPresent,
  required_clauses: chatMarkdownSanitizerClauses,
  remaining_evidence: [
    'PDS StreamingText markdown adapter implementation',
    'malicious HTML and URL-policy unit tests',
    'incremental streaming snapshot sanitizer tests',
    'catalog visual/a11y evidence for sanitized markdown states',
    'dependency/license/vulnerability/bundle review'
  ]
};
addCheck(
  'conversation-markdown-sanitizer-decision',
  chatMarkdownSanitizerOk,
  {
    description: 'CH5 StreamingText markdown rendering has a governed sanitizer decision before live markdown adapter work begins.',
    status: conversationMarkdownSanitizer.status,
    live_ready: conversationMarkdownSanitizer.live_ready,
    missing_clauses: chatMarkdownSanitizerClauses
      .filter((clause) => !clause.present)
      .map((clause) => clause.id)
  }
);

const tokenFiles = [
  'appfw_ui/pds_health/tokens/pdsTokens.css',
  'appfw_ui/pds_health/tokens/pdsTokens.ts'
];
const interactiveCatalogSourceFiles = interactiveCatalogFiles
  .map((relative) => path.join(interactiveCatalogRoot, relative))
  .filter((absolute) => fs.existsSync(absolute));
const ok = checks.every((check) => check.ok);
const designSystem = {
  name: packageJson.name ?? null,
  version: packageJson.version ?? null,
  package_path: relativePath(packageJsonPath),
  component_root: relativePath(componentRoot),
  token_root: 'appfw_ui/pds_health/tokens',
  interactive_catalog_root: relativePath(interactiveCatalogRoot),
  source_sha256: hashFiles(sourceFiles),
  token_sha256: hashFiles(tokenFiles.map((relative) => path.join(repoRoot, relative))),
  interactive_catalog_sha256: hashFiles(interactiveCatalogSourceFiles),
  status: {
    ok,
    required_components: requiredComponents.length,
    source_component_exports: sourceComponentExports.length,
    checks_passed: checks.filter((check) => check.ok).length,
    checks_failed: checks.filter((check) => !check.ok).length
  }
};
const referenceCatalogPath = path.join(referenceRoot, 'catalog.json');
const catalogManifest = {
  path: relativePath(referenceCatalogPath),
  sha256: fs.existsSync(referenceCatalogPath) ? hashFiles([referenceCatalogPath]) : null,
  family_count: manifestFamilyNames.length,
  component_count: manifestComponents.length,
  readiness_contract: Array.isArray(referenceCatalog.readinessContract)
    ? referenceCatalog.readinessContract
    : [],
  maturity_levels: Array.isArray(referenceCatalog.maturityLevels)
    ? referenceCatalog.maturityLevels
    : [],
  evidence_requirements: Array.isArray(referenceCatalog.evidenceRequirements)
    ? referenceCatalog.evidenceRequirements
    : [],
  api_contract: apiContract ? {
    source_export_evidence: apiContract.sourceExportEvidence ?? null,
    inventory_artifact_path: apiContract.inventoryArtifactPath ?? null,
    source_files: apiContractSourceFiles.map((item) => item?.path).filter(Boolean)
  } : null,
  agent_decision_guide: agentRecipeInventory,
  planned_family_slots: plannedFamilySlotsManifest.map((slot) => ({
    slug: slot?.slug ?? null,
    name: slot?.name ?? null,
    status: slot?.status ?? null,
    maturity: slot?.maturity ?? null,
    evidence: Array.isArray(slot?.evidence) ? slot.evidence : []
  })),
  family_readiness: familyReadiness
};
const packageCatalog = {
  path: 'appfw_ui/pds_health/components/src/catalog.ts',
  exported: packageCatalogText.includes('export const pdsComponentCatalog'),
  family_count: referenceCatalogFamiliesManifest.length,
  planned_slot_count: plannedFamilySlotsManifest.length,
  component_count: requiredComponents.length,
  recipe_count: agentRecipeInventory.length
};
const interactiveCatalog = {
  root: relativePath(interactiveCatalogRoot),
  package_path: 'appfw_ui/pds_health/catalog-app/package.json',
  sha256: hashFiles(interactiveCatalogSourceFiles),
  file_count: interactiveCatalogSourceFiles.length,
  component_example_count: requiredComponents.length - missingInteractiveCatalogExamples.length,
  usage_snippet_count: requiredComponents.length - missingInteractiveCatalogSnippets.length,
  product_neutral: interactiveCatalogResidueMatches.length === 0,
  renders_manifest: missingInteractiveCatalogImports.length === 0,
  source_files: interactiveCatalogSourceFiles.map(relativePath).sort()
};

// Release-certification summary: a single, retainable view of what version of
// the design system this is, how stable each component is, and where the
// supporting evidence lives. This is the design-system analogue of a release
// certificate for downstream/enterprise product teams.
const releaseCertification = {
  package: packageJson.name ?? null,
  version: packageVersion,
  semver_valid: packageVersionValid,
  manifest_sha256: catalogManifest.sha256,
  changelog: {
    path: 'appfw_ui/pds_health/CHANGELOG.md',
    present: changelogExists,
    references_version: changelogReferencesVersion
  },
  lifecycle: {
    total: requiredComponents.length,
    by_status: lifecycleStatusCounts,
    deprecated: deprecatedComponents
  },
  maturity: {
    basis: maturityEvidenceBasis,
    supported: unsupportedMaturityClaims.length === 0,
    unsupported_claims: unsupportedMaturityClaims
  },
  evidence: {
    component_check: relativePath(artifactPath),
    token_drift: 'target/appfw/pds-token-drift.json',
    catalog_accessibility: 'target/appfw/pds-catalog-evidence.json'
  }
};

const report = {
  command: 'pds-component-check',
  ok,
  generated_at: new Date().toISOString(),
  artifact: relativePath(artifactPath),
  design_system: designSystem,
  release_certification: releaseCertification,
  maturity_evidence: maturityEvidence,
  governed_action_live_readiness: governedActionLiveReadiness,
  flow_graph_token_bridge: flowGraphTokenBridge,
  conversation_markdown_sanitizer: conversationMarkdownSanitizer,
  component_root: relativePath(componentRoot),
  reference_root: relativePath(referenceRoot),
  catalog_manifest: catalogManifest,
  package_catalog: packageCatalog,
  interactive_catalog: interactiveCatalog,
  component_api_inventory: componentApiInventory,
  agent_decision_guide: agentRecipeInventory,
  source_files: sourceFiles.map(relativePath).sort(),
  reference_files: [...referenceTextByFile.keys()].sort(),
  interactive_catalog_files: [...interactiveCatalogTextByFile.keys()].sort(),
  required_components: requiredComponents,
  checks
};

fs.mkdirSync(path.dirname(artifactPath), { recursive: true });
fs.writeFileSync(artifactPath, `${JSON.stringify(report, null, 2)}\n`);

if (jsonOutput) {
  console.log(JSON.stringify(report, null, 2));
} else if (report.ok) {
  console.log(`PDS component check: OK\nArtifact: ${relativePath(artifactPath)}`);
} else {
  console.error(`PDS component check: FAILED\nArtifact: ${relativePath(artifactPath)}`);
}

if (!report.ok) {
  process.exitCode = 1;
}

function normalizeEol(text) {
  return text.replace(/\r\n/g, '\n');
}

function sha256(text) {
  return `sha256:${createHash('sha256').update(text).digest('hex')}`;
}

function hashFiles(files) {
  const hashInput = files
    .filter((absolute) => fs.existsSync(absolute))
    .map((absolute) => {
      const relative = relativePath(absolute);
      const text = normalizeEol(fs.readFileSync(absolute, 'utf8'));
      return `${relative}\n${text}`;
    })
    .sort()
    .join('\n--- appfw-file ---\n');
  return sha256(hashInput);
}
