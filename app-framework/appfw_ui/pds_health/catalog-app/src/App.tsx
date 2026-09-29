import { useEffect, useMemo, useState, type FC } from "react";
import {
  AppShell,
  Badge,
  Button,
  ConnectedFabric,
  Drawer,
  NavigationItem,
  PageHeader,
  PdsHealthLogo,
  SegmentedControl,
  Surface
} from "@appfw/pds-health-components";
import {
  agentContract,
  agentDecisionGuide,
  catalogBrand,
  catalogPurpose,
  componentCount,
  families,
  familyMaturity,
  lifecycleFor,
  matchesQuery,
  packageVersion,
  recipesForComponent,
  type CatalogFamily,
  type ComponentStatus
} from "./lib/catalogData";
import { exampleFor } from "./examples";
import { importSnippet, usageSnippet } from "./lib/snippets";
import {
  applyTheme,
  applyVisualTheme,
  readStoredTheme,
  readStoredVisualTheme,
  resolvedScheme,
  THEME_MODES,
  VISUAL_THEMES,
  type ThemeMode,
  type VisualTheme
} from "./lib/theme";

const CONTRACT_PARTS: ReadonlyArray<{ key: keyof CatalogFamily["contract"]; label: string }> = [
  { key: "props", label: "Props" },
  { key: "states", label: "States" },
  { key: "density", label: "Density" },
  { key: "accessibility", label: "Accessibility" },
  { key: "usage", label: "Usage" },
  { key: "verification", label: "Verification" }
];

type DesignSystemViewId =
  | "overview"
  | "brand"
  | "floor-plans"
  | "elements"
  | "components"
  | "patterns"
  | "data-visualization"
  | "interactions"
  | "neutral-work"
  | "trusted-task"
  | "intelligent-experience";

type DesignSystemView = {
  id: DesignSystemViewId;
  label: string;
  description: string;
  shortLabel: string;
  group: "system" | "reference";
  src?: string;
};

const DESIGN_SYSTEM_VIEWS: readonly DesignSystemView[] = [
  {
    id: "overview",
    label: "Overview",
    description: "Doctrine, coverage, and the route through the system",
    shortLabel: "01",
    group: "system"
  },
  {
    id: "brand",
    label: "Brand",
    description: "Identity, color, provenance, and usage",
    shortLabel: "02",
    group: "system"
  },
  {
    id: "floor-plans",
    label: "Floor plans",
    description: "Page-level enterprise compositions",
    shortLabel: "03",
    group: "system"
  },
  {
    id: "elements",
    label: "Elements",
    description: "Color, type, space, shape, motion, and access",
    shortLabel: "04",
    group: "system"
  },
  {
    id: "components",
    label: "Components",
    description: "Reusable controls and assemblies",
    shortLabel: "05",
    group: "system"
  },
  {
    id: "patterns",
    label: "Patterns",
    description: "Goal-oriented interaction recipes",
    shortLabel: "06",
    group: "system"
  },
  {
    id: "data-visualization",
    label: "Data visualization",
    description: "Metrics, trends, legends, and charts",
    shortLabel: "07",
    group: "system"
  },
  {
    id: "interactions",
    label: "Representative interactions",
    description: "Field, menu, validation, and input behavior",
    shortLabel: "I",
    group: "reference",
    src: "./f1-representative-interactions.html"
  },
  {
    id: "neutral-work",
    label: "Neutral work",
    description: "Queue, state, recovery, and detail patterns",
    shortLabel: "W",
    group: "reference",
    src: "./pds-n0-neutral-my-work.html"
  },
  {
    id: "trusted-task",
    label: "Trusted task",
    description: "Governed task preview and receipt patterns",
    shortLabel: "T",
    group: "reference",
    src: "./pds-n1-trusted-task.html"
  },
  {
    id: "intelligent-experience",
    label: "Intelligent Experience",
    description: "Eight inspectable, user-steerable intelligence reference recipes",
    shortLabel: "IX",
    group: "reference",
    src: "./ix-reference.html"
  }
] as const;

function viewFromLocation(): DesignSystemViewId {
  if (typeof window === "undefined") return "overview";
  const requestedView = new URL(window.location.href).searchParams.get("view");
  return DESIGN_SYSTEM_VIEWS.some((view) => view.id === requestedView)
    ? requestedView as DesignSystemViewId
    : "overview";
}

function ViewIcon({ children }: { children: string }) {
  return <span className="design-system-app__view-icon">{children}</span>;
}

type FloorPlan = {
  title: string;
  summary: string;
  adaptation: string;
  status: "available" | "contract-defined" | "planned";
  priority: "P0" | "P1" | "P2";
  disposition: "own" | "adapt" | "evaluate";
  components: readonly string[];
};

const FLOOR_PLANS: readonly FloorPlan[] = [
  {
    title: "Narrative knowledge workspace",
    summary: "Long-form strategy, policy, research, and role-aware knowledge with persistent orientation and exact proof close to the story.",
    adaptation: "A compact sticky header and horizontal experience dock remain visible; contextual controls return to normal flow on narrow screens while the document stays the single scroll owner.",
    status: "available",
    priority: "P0",
    disposition: "own",
    components: ["NarrativeWorkspace", "SegmentedControl", "CommandPalette", "PageHeader", "ExplorationWorkspace"]
  },
  {
    title: "Intranet and role home",
    summary: "Personalized news, tools, work, notifications, and intelligent information packs without portal fragmentation.",
    adaptation: "Wide dashboards become prioritized feed and task sections on compact screens.",
    status: "contract-defined",
    priority: "P1",
    disposition: "own",
    components: ["AppShell", "SearchBar", "KpiTile", "List", "RecommendationCard"]
  },
  {
    title: "My Work queue and detail",
    summary: "A scannable work queue connected to stable detail, status, approval, and governed action feedback.",
    adaptation: "Wide screens preserve queue and detail; compact screens use focused detail routes or drawers.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["AppShell", "PageHeader", "SearchBar", "DataGridShell", "Drawer", "OperationState"]
  },
  {
    title: "Service and request center",
    summary: "Discover services, submit or resume requests, and understand cross-system status in one surface.",
    adaptation: "Search and recent work take priority on compact screens; taxonomy stays progressively disclosed.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["AppShell", "SearchBar", "List", "FormLayout", "ProcessProgress", "FeedbackState"]
  },
  {
    title: "Entity record 360",
    summary: "A durable record workspace for summary, relationships, editing, validation, evidence, and traceable actions.",
    adaptation: "Identity and status persist while tabs, sections, and actions reflow by window class.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["AppShell", "PageHeader", "Tabs", "FormLayout", "ValidationSummary", "ActionAudit"]
  },
  {
    title: "Administrative console",
    summary: "Dense configuration, permissions, audit, validation, and bulk work for authorized operators.",
    adaptation: "Desktop density yields to explicit detail and guarded actions on touch devices.",
    status: "contract-defined",
    priority: "P1",
    disposition: "own",
    components: ["AppShell", "DataGridShell", "FormLayout", "CommandBar", "ActionAudit"]
  },
  {
    title: "Decision workspace",
    summary: "Operational signals, trends, supporting records, and grounded insight arranged for comparison and action.",
    adaptation: "Metrics and charts reflow without losing decision hierarchy.",
    status: "available",
    priority: "P0",
    disposition: "own",
    components: ["AppShell", "KpiTile", "MetricTrend", "ChartShell", "DataGridShell", "InsightSummary"]
  },
  {
    title: "Guided task and approval",
    summary: "A bounded multi-step task with visible progress, validation, preview, evidence, and recoverable completion.",
    adaptation: "Desktop may show process context beside work; mobile keeps the current step dominant.",
    status: "available",
    priority: "P0",
    disposition: "own",
    components: ["ProcessStepper", "ProcessProgress", "FormLayout", "IntentPreview", "FeedbackState"]
  },
  {
    title: "Calendar and planner",
    summary: "Agenda and operational time views for due work, availability, conflicts, and governed scheduling.",
    adaptation: "Mobile uses agenda or list as the accessible alternative to spatial calendar views.",
    status: "contract-defined",
    priority: "P1",
    disposition: "evaluate",
    components: ["PageHeader", "Tabs", "DataGridToolbar", "Drawer", "FeedbackState"]
  },
  {
    title: "Task plan and delivery roadmap",
    summary: "Work breakdown, owners, dependencies, dates, milestones, blockers, variance, and plan health.",
    adaptation: "Wide screens pair an outline with inspection; compact screens focus one item at a time.",
    status: "planned",
    priority: "P1",
    disposition: "own",
    components: ["AppShell", "PageHeader", "List", "DataGridShell", "FlowGraphShell", "Drawer"]
  },
  {
    title: "Team board and Kanban",
    summary: "Backlog, swimlanes, WIP limits, blocked work, policy transitions, and flow metrics.",
    adaptation: "Compact screens use grouped lists with explicit keyboard and touch movement.",
    status: "contract-defined",
    priority: "P1",
    disposition: "adapt",
    components: ["AppShell", "SearchBar", "Badge", "Drawer", "MetricTrend"]
  },
  {
    title: "Workflow monitor",
    summary: "Definition and run context, live node state, exceptions, retries, audit, ownership, and continuation.",
    adaptation: "Graph, timeline, and structured list remain equivalent views of the same run state.",
    status: "contract-defined",
    priority: "P1",
    disposition: "adapt",
    components: ["FlowGraphShell", "AgentTimeline", "FeedbackState", "ActionAudit", "Drawer"]
  },
  {
    title: "Workflow designer",
    summary: "Typed node-edge authoring with palette, inspector, validation, versions, comparison, and governed publication.",
    adaptation: "Compact and accessible alternatives expose the same graph as a structured outline.",
    status: "planned",
    priority: "P2",
    disposition: "evaluate",
    components: ["AppShell", "Toolbar", "FlowGraphShell", "Drawer", "ValidationSummary"]
  },
  {
    title: "Advanced query designer",
    summary: "Provider-neutral nested conditions, relationships, typed operators, preview, and saved views.",
    adaptation: "Compact screens edit one group at a time with a readable query summary.",
    status: "planned",
    priority: "P1",
    disposition: "evaluate",
    components: ["AppShell", "List", "FormLayout", "DataGridShell", "ValidationSummary"]
  },
  {
    title: "Enterprise search and discovery",
    summary: "Unified search, facets, grounded answers, source metadata, and actions into authoritative systems.",
    adaptation: "Search remains stable while results shift between list, answer, and generated views.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["SearchBar", "GeneratedViewShell", "CitationList", "EntityRefCard", "FeedbackState"]
  },
  {
    title: "Intelligence workspace",
    summary: "Generated views, grounded answers, evidence, recommendations, and agent progress embedded in work context.",
    adaptation: "Intelligence becomes a contextual layer without displacing source-of-truth work.",
    status: "available",
    priority: "P0",
    disposition: "own",
    components: ["GeneratedViewShell", "MessageThread", "AgentTimeline", "CitationList", "EvidenceSummary"]
  },
  {
    title: "Contextual agentic chat",
    summary: "A bounded conversational surface that inspects context, proposes actions, shows progress, and returns receipts.",
    adaptation: "Chat docks, overlays, or expands full-screen while underlying work stays addressable.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["MessageThread", "MessageComposer", "ToolCallStatus", "IntentPreview", "ActionAudit"]
  },
  {
    title: "Native mobile task flow",
    summary: "Concise task list and detail optimized for interruption, resumption, approval, and notification entry.",
    adaptation: "Native controls preserve shared semantics, task state, and evidence expectations.",
    status: "contract-defined",
    priority: "P0",
    disposition: "own",
    components: ["PageHeader", "List", "ListItem", "ProcessProgress", "Drawer", "ToastRegion"]
  },
  {
    title: "Embedded clinical work",
    summary: "A SMART on FHIR-hosted experience preserving app-fabric identity, context, and authoritative handoff.",
    adaptation: "Embedded constraints remove redundant chrome while preserving deep links and continuation.",
    status: "contract-defined",
    priority: "P1",
    disposition: "own",
    components: ["PageHeader", "List", "GeneratedViewShell", "IntentPreview", "FeedbackState"]
  }
];

const ELEMENT_FOUNDATIONS = [
  ["Color roles", "Semantic roles carry hierarchy, state, intelligence, and emphasis across themes."],
  ["Typography", "A deliberate hierarchy supports fast scanning, data reading, and code-like metadata."],
  ["Spacing and layout", "Shared spacing and responsive regions create consistent density without freezing composition."],
  ["Shape and surface", "Elevation, borders, translucency, and shape adapt to the selected experience grammar."],
  ["Motion", "Transitions communicate continuity and intent with reduced-motion parity."],
  ["Accessibility", "Focus, contrast, target size, keyboard behavior, and non-color cues are part of every contract."]
] as const;

function maturityTone(level: string): "success" | "accent" | "neutral" {
  if (level === "release-gated") return "success";
  if (level === "enterprise-ready") return "accent";
  return "neutral";
}

function statusTone(status: ComponentStatus): "success" | "accent" | "warning" | "danger" {
  if (status === "stable") return "success";
  if (status === "beta") return "accent";
  if (status === "deprecated") return "danger";
  return "warning";
}

function floorPlanTone(status: FloorPlan["status"]): "success" | "warning" | "neutral" {
  if (status === "available") return "success";
  if (status === "contract-defined") return "warning";
  return "neutral";
}

const CopyButton: FC<{ label: string; value: string }> = ({ label, value }) => {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return undefined;
    const timer = window.setTimeout(() => setCopied(false), 1600);
    return () => window.clearTimeout(timer);
  }, [copied]);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  return (
    <Button size="sm" variant="quiet" onClick={copy} aria-label={copied ? `${label} copied` : label}>
      {copied ? "Copied" : label}
    </Button>
  );
};

const ComponentCard: FC<{ name: string }> = ({ name }) => {
  const Example = exampleFor(name);
  const recipes = recipesForComponent(name);
  const lifecycle = lifecycleFor(name);
  return (
    <article
      className={name === "DataGrid" || name === "NarrativeWorkspace" ? "component-card component-card--wide" : "component-card"}
      id={`component-${name}`}
      data-component={name}
    >
      <header className="component-card__head">
        <div className="component-card__title">
          <div className="component-card__name">
            <h3>{name}</h3>
            {lifecycle ? (
              <Badge tone={statusTone(lifecycle.status)} title={`Since v${lifecycle.since}`}>
                {lifecycle.status}
              </Badge>
            ) : null}
          </div>
          {recipes.length ? (
            <div className="component-card__recipes">
              {recipes.map((recipe) => (
                <Badge key={recipe.slug} tone="neutral">{recipe.slug}</Badge>
              ))}
            </div>
          ) : null}
        </div>
        <div className="component-card__actions">
          <CopyButton label="Copy import" value={importSnippet(name)} />
          <CopyButton label="Copy usage" value={usageSnippet(name)} />
        </div>
      </header>
      <div className="component-card__preview">
        {Example ? <Example /> : <p className="example-note">Preview pending — see component source.</p>}
      </div>
    </article>
  );
};

const FamilySection: FC<{ family: CatalogFamily; query: string }> = ({ family, query }) => {
  const visible = family.components.filter((component) => matchesQuery(family, component, query));
  if (visible.length === 0) return null;
  const maturity = familyMaturity(family.name);

  return (
    <section className="family" id={family.slug} aria-labelledby={`${family.slug}-title`}>
      <header className="family__head">
        <div className="family__title">
          <h2 id={`${family.slug}-title`}>{family.name}</h2>
          <Badge tone={maturityTone(maturity)} title="Maturity basis: retained local/CI evidence (not live managed-environment certification)">{maturity}</Badge>
          <span className="family__count">{visible.length} of {family.components.length}</span>
        </div>
        <p className="family__purpose">{family.purpose}</p>
        <dl className="family__contract">
          {CONTRACT_PARTS.map((part) => (
            <div key={part.key} className="family__contract-item">
              <dt>{part.label}</dt>
              <dd>{family.contract[part.key]}</dd>
            </div>
          ))}
        </dl>
      </header>
      <div className="family__grid">
        {visible.map((component) => (
          <ComponentCard key={component} name={component} />
        ))}
      </div>
    </section>
  );
};

const App: FC = () => {
  const [theme, setTheme] = useState<ThemeMode>(() => readStoredTheme());
  const [visualTheme, setVisualTheme] = useState<VisualTheme>(() => readStoredVisualTheme());
  const [query, setQuery] = useState("");
  const [activeViewId, setActiveViewId] = useState<DesignSystemViewId>(() => viewFromLocation());
  const [navigationOpen, setNavigationOpen] = useState(false);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    applyVisualTheme(visualTheme);
  }, [visualTheme]);

  useEffect(() => {
    const handleHistoryChange = () => setActiveViewId(viewFromLocation());
    window.addEventListener("popstate", handleHistoryChange);
    return () => window.removeEventListener("popstate", handleHistoryChange);
  }, []);

  const activeView = DESIGN_SYSTEM_VIEWS.find((view) => view.id === activeViewId)
    ?? DESIGN_SYSTEM_VIEWS[0];

  useEffect(() => {
    document.title = `PDS Design System · ${activeView.label}`;
  }, [activeView.label]);

  function navigateToView(viewId: DesignSystemViewId) {
    if (viewId === activeViewId) {
      setNavigationOpen(false);
      return;
    }
    const next = new URL(window.location.href);
    next.search = "";
    next.searchParams.set("view", viewId);
    next.hash = "";
    window.history.pushState({ view: viewId }, "", next);
    setActiveViewId(viewId);
    setNavigationOpen(false);
  }

  const matchCount = useMemo(
    () =>
      families.reduce(
        (count, family) =>
          count + family.components.filter((component) => matchesQuery(family, component, query)).length,
        0
      ),
    [query]
  );

  const visibleComponentFamilies = useMemo(
    () => families.filter(
      (family) =>
        family.slug !== "analytics"
        && family.components.some((component) => matchesQuery(family, component, query))
    ),
    [query]
  );

  const navigation = (
    <div className="design-system-app__navigation">
      <p className="design-system-app__navigation-heading">Design system</p>
      {DESIGN_SYSTEM_VIEWS.filter((view) => view.group === "system").map((view) => (
        <NavigationItem
          key={view.id}
          href={`?view=${view.id}`}
          icon={<ViewIcon>{view.shortLabel}</ViewIcon>}
          label={view.label}
          current={view.id === activeViewId}
          onClick={(event) => {
            event.preventDefault();
            navigateToView(view.id);
          }}
        />
      ))}
      {activeViewId === "floor-plans" ? (
        <div className="design-system-app__subnavigation" aria-label="Floor plan contents">
          {FLOOR_PLANS.map((floorPlan) => (
            <a key={floorPlan.title} href={`#floor-plan-${floorPlan.title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`}>
              {floorPlan.title}
            </a>
          ))}
        </div>
      ) : null}
      <p className="design-system-app__navigation-heading design-system-app__navigation-heading--reference">
        Reference views
      </p>
      {DESIGN_SYSTEM_VIEWS.filter((view) => view.group === "reference").map((view) => (
        <NavigationItem
          key={view.id}
          href={`?view=${view.id}`}
          icon={<ViewIcon>{view.shortLabel}</ViewIcon>}
          label={view.label}
          current={view.id === activeViewId}
          onClick={(event) => {
            event.preventDefault();
            navigateToView(view.id);
          }}
        />
      ))}
    </div>
  );

  const referenceSource = activeView.src
    ? `${activeView.src}?theme=${resolvedScheme(theme)}&grammar=${visualTheme}`
    : null;

  return (
    <div className="design-system-app">
      <ConnectedFabric theme={theme} visualTheme={visualTheme} />
      <AppShell
        className="catalog"
        brand={
          <div className="catalog__brand">
            <span className="catalog__brand-mark" aria-hidden="true">PDS</span>
            <div>
              <strong>{catalogBrand}</strong>
              <span>Design System</span>
            </div>
          </div>
        }
        navigation={navigation}
        navigationLabel="PDS design-system views"
        footer={
          <div className="design-system-app__sidebar-footer">
            <strong>PDS Health</strong>
            <span>Framework-owned UI contracts</span>
            <Badge tone="neutral">v{packageVersion}</Badge>
          </div>
        }
        topBar={
          <div className="catalog__topbar">
            <Button
              className="design-system-app__views-button"
              size="sm"
              variant="secondary"
              onClick={() => setNavigationOpen(true)}
            >
              Views
            </Button>
            <div className="catalog__controls">
              {activeViewId === "components" || activeViewId === "data-visualization" ? (
                <label className="catalog__search">
                  <span className="visually-hidden">Search components</span>
                  <input
                    type="search"
                    className="pds-input"
                    placeholder="Search components…"
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    autoComplete="off"
                  />
                </label>
              ) : (
                <strong className="design-system-app__active-view">{activeView.label}</strong>
              )}
              <div className="catalog__theme-field">
                <span className="catalog__theme-label">Style</span>
                <SegmentedControl
                  ariaLabel="Visual theme"
                  value={visualTheme}
                  onValueChange={(value) => setVisualTheme(value as VisualTheme)}
                  options={VISUAL_THEMES.map((option) => ({
                    value: option,
                    label: option[0].toUpperCase() + option.slice(1)
                  }))}
                />
              </div>
              <div className="catalog__theme-field">
                <span className="catalog__theme-label">Mode</span>
                <SegmentedControl
                  ariaLabel="Color mode"
                  value={theme}
                  onValueChange={(value) => setTheme(value as ThemeMode)}
                  options={THEME_MODES.map((mode) => ({ value: mode, label: mode[0].toUpperCase() + mode.slice(1) }))}
                />
              </div>
            </div>
          </div>
        }
        responsiveCollapse
        sidebarResizable
        viewportBounded
      >
        {activeViewId === "overview" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="PDS experience system"
              title="PDS Design System"
              subtitle="One executable system from experience doctrine and foundations through components, compositions, and product journeys."
              actions={
                <span className="catalog__header-badges">
                  <Badge tone="neutral" title="Package version (SemVer)">v{packageVersion}</Badge>
                  <Badge tone="accent">{componentCount} components · {families.length} families</Badge>
                </span>
              }
            />

            <Surface title="For agents" subtitle="Read before generating product UI" className="catalog__contract">
              <div className="catalog__contract-cols">
                <div>
                  <h4>Start here</h4>
                  <ol>
                    {agentContract.startHere.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ol>
                </div>
                <div>
                  <h4>Do</h4>
                  <ul>
                    {agentContract.do.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h4>Avoid</h4>
                  <ul>
                    {agentContract.avoid.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </Surface>

            <section className="catalog-overview" aria-labelledby="catalog-overview-title">
              <header className="catalog-section-heading">
                <p className="catalog-section-heading__eyebrow">System map</p>
                <h2 id="catalog-overview-title">Move from foundations to outcomes</h2>
                <p>{catalogPurpose}</p>
              </header>
              <div className="catalog-overview__grid">
                {DESIGN_SYSTEM_VIEWS.filter(
                  (view) => view.group === "system" && view.id !== "overview"
                ).map((view) => (
                  <button key={view.id} type="button" onClick={() => navigateToView(view.id)}>
                    <span>{view.shortLabel}</span>
                    <strong>{view.label}</strong>
                    <small>{view.description}</small>
                  </button>
                ))}
              </div>
            </section>
          </div>
        ) : activeViewId === "brand" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="PDS Health identity"
              title="Brand"
              subtitle="Governed identity, color, provenance, and usage rules shared by every PDS product experience."
              actions={<Badge tone="accent">Framework-owned</Badge>}
            />
            <section className="brand-showcase" aria-label="PDS Health brand foundations">
              <Surface title="Primary identity" subtitle="Product and employee experiences">
                <div className="brand-showcase__lockup">
                  <PdsHealthLogo width={320} height={60} />
                </div>
                <p>Use supplied artwork and preserve its proportions, spacing, and semantic color roles.</p>
              </Surface>
              <Surface title="Identity palette" subtitle="Core brand roles">
                <div className="brand-showcase__palette">
                  <span><i className="brand-showcase__swatch brand-showcase__swatch--blue" />Health Blue</span>
                  <span><i className="brand-showcase__swatch brand-showcase__swatch--gray" />Health Gray</span>
                  <span><i className="brand-showcase__swatch brand-showcase__swatch--white" />Clean White</span>
                  <span><i className="brand-showcase__swatch brand-showcase__swatch--light" />Light Gray</span>
                </div>
              </Surface>
              <Surface title="Usage contract" subtitle="Stable across every grammar">
                <ul className="brand-showcase__rules">
                  <li>Use governed assets, never generated substitutes.</li>
                  <li>Preserve legibility across Apple-like and Material-like expressions.</li>
                  <li>Keep identity colors separate from semantic state colors.</li>
                  <li>Retain provenance when assets move into product delivery.</li>
                </ul>
              </Surface>
            </section>
          </div>
        ) : activeViewId === "floor-plans" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="Experience compositions"
              title="Floor plans"
              subtitle="Stable page-level arrangements that preserve hierarchy, context, and work continuity while products supply their own nouns and policy."
              actions={<Badge tone="accent">{FLOOR_PLANS.length} reference compositions</Badge>}
            />
            <div className="floor-plans__grid">
              {FLOOR_PLANS.map((floorPlan) => {
                const floorPlanId = `floor-plan-${floorPlan.title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
                return (
                  <Surface
                    key={floorPlan.title}
                    id={floorPlanId}
                    as="article"
                    className="floor-plan"
                    title={floorPlan.title}
                    subtitle={floorPlan.summary}
                  >
                    <div className="floor-plan__meta">
                      <Badge tone={floorPlanTone(floorPlan.status)}>{floorPlan.status}</Badge>
                      <span>{floorPlan.priority}</span>
                      <span>{floorPlan.disposition}</span>
                    </div>
                    <p><strong>Adaptation:</strong> {floorPlan.adaptation}</p>
                    <div className="recipes__components">
                      {floorPlan.components.map((component) => (
                        <button
                          key={component}
                          type="button"
                          className="recipes__chip"
                          onClick={() => {
                            navigateToView("components");
                            window.setTimeout(() => {
                              document.getElementById(`component-${component}`)?.scrollIntoView({ block: "start" });
                            }, 0);
                          }}
                        >
                          {component}
                        </button>
                      ))}
                    </div>
                  </Surface>
                );
              })}
            </div>
          </div>
        ) : activeViewId === "elements" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="Canonical foundations"
              title="Elements"
              subtitle="Semantic roles and adaptation rules give every experience grammar a shared foundation without forcing identical rendering."
              actions={<Badge tone="accent">{ELEMENT_FOUNDATIONS.length} foundations</Badge>}
            />
            <div className="elements-grid">
              {ELEMENT_FOUNDATIONS.map(([title, summary], index) => (
                <Surface key={title} as="article" title={title} subtitle={summary}>
                  <span className={`elements-grid__specimen elements-grid__specimen--${index + 1}`} aria-hidden="true" />
                  <strong>{String(index + 1).padStart(2, "0")}</strong>
                </Surface>
              ))}
            </div>
          </div>
        ) : activeViewId === "components" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="Published package"
              title="Components"
              subtitle="Reusable controls and assemblies backed by the framework-owned PDS package."
              actions={<Badge tone="accent">{componentCount} published components</Badge>}
            />
            {query ? (
              <p className="catalog__match" role="status">
                {matchCount} component{matchCount === 1 ? "" : "s"} match “{query}”
              </p>
            ) : null}

            {visibleComponentFamilies.length === 0 ? (
              <Surface title="No matches">
                <p className="example-note">No components match “{query}”.</p>
                <Button onClick={() => setQuery("")}>Clear search</Button>
              </Surface>
            ) : (
              visibleComponentFamilies.map((family) => (
                <FamilySection key={family.slug} family={family} query={query} />
              ))
            )}
          </div>
        ) : activeViewId === "patterns" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="Interaction recipes"
              title="Patterns"
              subtitle="Start from the workflow and trust contract, then compose the smallest set of PDS components that preserves it."
              actions={<Badge tone="accent">{agentDecisionGuide.length} executable recipes</Badge>}
            />
            <section className="recipes" id="agent-decision-guide" aria-labelledby="recipes-title">
              <h2 id="recipes-title">Agent decision guide</h2>
              <p className="family__purpose">
                Each recipe identifies the PDS components to reach for first and the meaning products must retain.
              </p>
              <div className="recipes__grid">
                {agentDecisionGuide.map((recipe) => (
                  <Surface key={recipe.slug} as="article" title={recipe.slug} subtitle={recipe.intent}>
                    <p className="recipes__owns"><strong>Product owns:</strong> {recipe.productOwns}</p>
                    <div className="recipes__components">
                      {recipe.startWith.map((component) => (
                        <a key={component} className="recipes__chip" href={`#component-${component}`}>{component}</a>
                      ))}
                    </div>
                  </Surface>
                ))}
              </div>
            </section>
          </div>
        ) : activeViewId === "data-visualization" ? (
          <div className="catalog__main">
            <PageHeader
              eyebrow="Operational evidence"
              title="Data visualization"
              subtitle="Metrics, trends, legends, status palettes, and chart renderers that keep operational meaning visible."
              actions={<Badge tone="accent">Chart-engine neutral</Badge>}
            />
            {families.filter((family) => family.slug === "analytics").map((family) => (
              <FamilySection key={family.slug} family={family} query={query} />
            ))}
          </div>
        ) : referenceSource ? (
          <section
            className={activeViewId === "intelligent-experience"
              ? "design-system-app__reference design-system-app__reference--ix"
              : "design-system-app__reference"}
            aria-labelledby="design-system-reference-title"
          >
            <h1 id="design-system-reference-title" className="visually-hidden">{activeView.label}</h1>
            {activeViewId === "intelligent-experience" ? (
              <aside className="design-system-app__reference-notice" aria-label="Reference fixture boundary">
                <strong>Synthetic reference exception.</strong> Names, values, and timed transitions demonstrate
                interaction behavior only. They are not reusable product copy, production data,
                domain defaults, or evidence of a live intelligence provider.
              </aside>
            ) : null}
            <iframe
              key={referenceSource}
              className="design-system-app__reference-frame"
              src={referenceSource}
              title={`${activeView.label} reference experience`}
            />
          </section>
        ) : null}
      </AppShell>

      <Drawer
        open={navigationOpen}
        title="PDS views"
        description="Explore the component system and its reference experiences."
        side="left"
        onClose={() => setNavigationOpen(false)}
      >
        {navigation}
      </Drawer>
    </div>
  );
};

export default App;
