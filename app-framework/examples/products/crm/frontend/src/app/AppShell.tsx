import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction
} from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router";
import {
  BarChart3,
  Check,
  ChevronsLeft,
  ChevronsRight,
  Command,
  Database,
  Info,
  LayoutDashboard,
  Moon,
  ShieldCheck,
  Sun,
  Workflow,
  X
} from "lucide-react";
import pdsIconUrl from "../../pdshealthicon.jpg";
import pdsLogoUrl from "../../../../../../admin_ui/src/assets/pdsh-logo-nav-2.svg";
import { crmUiContract } from "../generated/appfw-ui-contract";
import {
  CommandPalette,
  Dialog,
  IconButton,
  ToastRegion,
  type CommandPaletteItem,
  type ToastItem
} from "../components/ui";
import { useAuth, useTenant } from "./providers";

const themeStorageKey = "crm-theme-mode";
type ThemeMode = "light" | "dark";

// Standardized app shell (ADR 0009): sidebar navigation derived from the
// contract, an identity/tenant bar, and an outlet for routed screens.
export function AppShell() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  return (
    <div className={`crm-shell ${sidebarCollapsed ? "is-sidebar-collapsed" : ""}`}>
      <Sidebar collapsed={sidebarCollapsed} onToggle={() => setSidebarCollapsed((current) => !current)} />
      <div className="crm-content">
        <WorkspaceHeader />
        <main className="crm-main">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  return (
    <aside className={`crm-sidebar ${collapsed ? "is-collapsed" : ""}`}>
      <div className="crm-sidebar-accent" />
      <div className="crm-brand-block">
        <div className="crm-brand-logo-card">
          <img src={pdsLogoUrl} alt="PDS Health" className="crm-brand-logo is-full" />
          <img src={pdsIconUrl} alt="" aria-hidden="true" className="crm-brand-logo is-icon" />
        </div>
        <div className="crm-brand-meta">
          <div className="crm-brand-title">CRM</div>
          <div className="crm-brand-subtitle">Reference product frontend</div>
        </div>
        <button
          type="button"
          className="crm-sidebar-toggle"
          aria-label={collapsed ? "Expand navigation" : "Collapse navigation"}
          title={collapsed ? "Expand navigation" : "Collapse navigation"}
          onClick={onToggle}
        >
          {collapsed ? <ChevronsRight size={15} /> : <ChevronsLeft size={15} />}
        </button>
      </div>

      <div className="crm-sidebar-nav">
        <NavSection label="Workflows">
          <NavItem to="/" label="Dashboard" sublabel="Metrics and operating telemetry" end icon={<LayoutDashboard size={16} />} />
          {crmUiContract.workflows.map((workflow) => (
            <NavItem
              key={workflow.id}
              to={`/${workflow.id}`}
              label={workflow.label}
              sublabel={workflow.primaryEntity}
              icon={<Workflow size={16} />}
            />
          ))}
        </NavSection>

        <NavSection label={`Entity model (${crmUiContract.entities.length})`}>
          {crmUiContract.entities.map((entity) => (
            <NavItem
              key={entity.typeName}
              to={`/data/${entity.routeSegment}`}
              label={entity.caption.plural}
              sublabel={`${entity.schemaName}.${entity.typeName}`}
              icon={<Database size={16} />}
            />
          ))}
        </NavSection>
      </div>

      <div className="crm-sidebar-footer">
        <div className="crm-footer-label">Scaffold coverage</div>
        <div className="crm-footer-row">
          <ShieldCheck size={16} />
          <span>{crmUiContract.workflows.length} workflows</span>
          <strong>{crmUiContract.entities.length} entities</strong>
        </div>
      </div>
    </aside>
  );
}

function NavSection({ label, children }: { label: string; children: ReactNode }) {
  return (
    <nav className="crm-nav-section">
      <div className="crm-nav-section-label">
        {label}
      </div>
      {children}
    </nav>
  );
}

function NavItem({
  to,
  label,
  sublabel,
  badge,
  icon,
  end
}: {
  to: string;
  label: string;
  sublabel?: string;
  badge?: string;
  icon?: ReactNode;
  end?: boolean;
}) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) => `crm-nav-item ${isActive ? "is-active" : ""}`}
      aria-label={label}
      data-caption={label}
      title={label}
    >
      <span className="crm-nav-icon">{icon ?? <BarChart3 size={16} />}</span>
      <span className="crm-nav-text">
        <span className="crm-nav-label">{label}</span>
        {sublabel ? <span className="crm-nav-sublabel">{sublabel}</span> : null}
      </span>
      {badge ? <span className="crm-nav-badge">{badge}</span> : null}
    </NavLink>
  );
}

// Dev identity + tenant bar. Real Okta/JWT replaces this (ADR 0010); for now it
// lets you set a bearer token and tenant id for local exploration.
function WorkspaceHeader() {
  const { auth, setAuthorization } = useAuth();
  const { tenant, setTenantId } = useTenant();
  const location = useLocation();
  const [themeMode, setThemeMode] = useThemeMode();
  const [token, setToken] = useState("");
  const [tenantInput, setTenantInput] = useState("");
  const [aboutOpen, setAboutOpen] = useState(false);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const breadcrumb = useMemo(() => currentBreadcrumb(location.pathname), [location.pathname]);
  const commands = useMemo(() => workspaceCommands(), []);

  function switchTheme() {
    const nextTheme = themeMode === "dark" ? "light" : "dark";
    setThemeMode(nextTheme);
    setToasts([
      {
        id: `theme-${nextTheme}`,
        tone: "success",
        title: "Display preference saved",
        detail: `Theme switched to ${nextTheme} mode.`
      }
    ]);
  }

  return (
    <header className="crm-identity-bar">
      <div className="crm-breadcrumb" aria-label="Breadcrumb">
        <span>{breadcrumb.group}</span>
        <span>/</span>
        <strong>{breadcrumb.label}</strong>
      </div>

      <div className="crm-command-shell">
        <CommandPalette
          items={commands}
          triggerIcon={<Command size={14} />}
          triggerLabel="Search operations"
          searchPlaceholder="Search workflows, entities, and records"
          shortcutLabel="⌘K"
          emptyMessage="No matching workspace commands."
          renderItem={(item, children, props) => (
            <Link to={item.href ?? "#"} {...props}>
              {children}
            </Link>
          )}
        />
        <IconButton
          ariaLabel={themeMode === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          icon={themeMode === "dark" ? <Sun size={15} /> : <Moon size={15} />}
          onClick={switchTheme}
          size="sm"
          tooltip="Persist this browser display preference"
          variant="secondary"
        />
        <IconButton
          ariaLabel="About PDS Health CRM"
          icon={<Info size={15} />}
          onClick={() => setAboutOpen(true)}
          size="sm"
          tooltip="View generated contract and runtime summary"
          variant="secondary"
        />
        <ToastRegion
          toasts={toasts}
          onDismiss={(id) => setToasts((current) => current.filter((toast) => toast.id !== id))}
          position="bottom-end"
        />
      </div>

      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} />

      <div className="crm-identity-region">
        <div className="crm-identity-controls">
          <div className="crm-identity-status">
            {auth.authorization ? (
              <span>
                Signed in{auth.userName ? ` as ${auth.userName}` : ""} ·{" "}
                <span style={{ color: "var(--pds-color-text-quiet)" }}>
                  {auth.roles.length ? auth.roles.join(", ") : "no roles"}
                </span>
              </span>
            ) : (
              <span>Not signed in (anonymous)</span>
            )}
            {tenant.tenantId ? <span> · tenant {tenant.tenantId}</span> : null}
          </div>
          <input
            aria-label="Bearer token"
            placeholder="Bearer token"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            className="crm-identity-input"
          />
          <input
            aria-label="Tenant id"
            placeholder="Tenant id"
            value={tenantInput}
            onChange={(event) => setTenantInput(event.target.value)}
            className="crm-identity-input compact"
          />
          <button
            type="button"
            onClick={() => {
              setAuthorization(token ? `Bearer ${token.replace(/^Bearer\s+/i, "")}` : null);
              setTenantId(tenantInput || null);
            }}
            className="crm-button primary compact"
          >
            <Check size={14} />
            Apply
          </button>
          <button
            type="button"
            onClick={() => {
              setToken("");
              setTenantInput("");
              setAuthorization(null);
              setTenantId(null);
            }}
            className="crm-button secondary compact"
          >
            <X size={14} />
            Clear
          </button>
        </div>
      </div>
    </header>
  );
}

function AboutDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const auditedEntities = crmUiContract.entities.filter((entity) => entity.audited).length;
  const mutationCount = crmUiContract.entities.reduce(
    (total, entity) => total + entity.operations.filter((operation) => operation.kind === "mutation").length,
    0
  );
  const queryCount = crmUiContract.entities.reduce(
    (total, entity) => total + entity.operations.filter((operation) => operation.kind === "query").length,
    0
  );

  return (
    <Dialog
      open={open}
      className="crm-about-dialog"
      title="PDS Health CRM"
      description="Enterprise CRM reference frontend generated from the App Framework UI contract, with human-owned workflow screens and model-driven entity workspaces."
      size="lg"
      closeLabel="Close about dialog"
      onClose={onClose}
    >
      <div className="crm-about-surface">
        <div className="crm-about-header">
          <img src={pdsLogoUrl} alt="PDS Health" />
        </div>
        <div className="crm-about-intro">
          <div className="crm-kicker">About</div>
        </div>
        <div className="crm-about-grid" aria-label="Application summary">
          <AboutMetric label="Version" value={`v${__APP_VERSION__}`} />
          <AboutMetric label="Schema" value={crmUiContract.schemaName} />
          <AboutMetric label="Provider" value={crmUiContract.provider.dataSourceType} detail={crmUiContract.provider.dataSourceName} />
          <AboutMetric label="Workflows" value={crmUiContract.workflows.length.toString()} />
          <AboutMetric label="Entities" value={crmUiContract.entities.length.toString()} />
          <AboutMetric label="Relationships" value={crmUiContract.relationships.length.toString()} />
          <AboutMetric label="Audited Entities" value={auditedEntities.toString()} />
        </div>
        <div className="crm-about-sections">
          <section>
            <h3>Runtime Contract</h3>
            <dl>
              <div>
                <dt>Source</dt>
                <dd>{crmUiContract.source}</dd>
              </div>
              <div>
                <dt>Contract version</dt>
                <dd>{crmUiContract.version}</dd>
              </div>
              <div>
                <dt>Default page size</dt>
                <dd>{crmUiContract.pagination.defaultPageSize}</dd>
              </div>
              <div>
                <dt>Max page size</dt>
                <dd>{crmUiContract.pagination.maxPageSize}</dd>
              </div>
            </dl>
          </section>
          <section>
            <h3>Capabilities</h3>
            <dl>
              <div>
                <dt>Queries</dt>
                <dd>{queryCount}</dd>
              </div>
              <div>
                <dt>Mutations</dt>
                <dd>{mutationCount}</dd>
              </div>
              <div>
                <dt>Auth required</dt>
                <dd>{crmUiContract.auth.required ? "Yes" : "No"}</dd>
              </div>
              <div>
                <dt>Tenant required</dt>
                <dd>{crmUiContract.auth.tenantRequired ? "Yes" : "No"}</dd>
              </div>
            </dl>
          </section>
        </div>
        <div className="crm-about-tags">
          {crmUiContract.provider.capabilityHints.slice(0, 6).map((hint) => (
            <span key={hint}>{hint.replace(/_/g, " ")}</span>
          ))}
        </div>
      </div>
    </Dialog>
  );
}

function AboutMetric({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="crm-about-metric">
      <span>{label}</span>
      <strong>{value}</strong>
      {detail ? <small>{detail}</small> : null}
    </div>
  );
}

function useThemeMode(): [ThemeMode, Dispatch<SetStateAction<ThemeMode>>] {
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => initialThemeMode());

  useEffect(() => {
    document.documentElement.dataset.theme = themeMode;
    document.documentElement.style.colorScheme = themeMode;
    window.localStorage.setItem(themeStorageKey, themeMode);
  }, [themeMode]);

  return [themeMode, setThemeMode];
}

function initialThemeMode(): ThemeMode {
  const stored = window.localStorage.getItem(themeStorageKey);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function workspaceCommands(): CommandPaletteItem[] {
  return [
    {
      id: "workflow:dashboard",
      group: "Workflow",
      label: "Dashboard",
      detail: "Metrics and operating telemetry",
      href: "/",
      icon: <LayoutDashboard size={15} />
    },
    ...crmUiContract.workflows.map((workflow) => ({
      id: `workflow:${workflow.id}`,
      group: "Workflow",
      label: workflow.label,
      detail: workflow.primaryEntity,
      href: `/${workflow.id}`,
      keywords: [workflow.id, workflow.primaryEntity],
      icon: <Workflow size={15} />
    })),
    ...crmUiContract.entities.map((entity) => ({
      id: `entity:${entity.schemaName}:${entity.typeName}`,
      group: "Entity",
      label: entity.caption.plural,
      detail: `${entity.schemaName}.${entity.typeName}`,
      href: `/data/${entity.routeSegment}`,
      keywords: [entity.routeSegment, entity.caption.singular, entity.typeName],
      icon: <Database size={15} />
    }))
  ];
}

function currentBreadcrumb(pathname: string) {
  if (pathname === "/") return { group: "Workflows", label: "Dashboard" };
  const dataMatch = pathname.match(/^\/data\/([^/]+)/);
  if (dataMatch) {
    const entity = crmUiContract.entities.find((candidate) => candidate.routeSegment === dataMatch[1]);
    return { group: "Entity model", label: entity?.caption.plural ?? "Records" };
  }
  const workflow = crmUiContract.workflows.find((candidate) => pathname === `/${candidate.id}`);
  if (workflow) return { group: "Workflows", label: workflow.label };
  return { group: "CRM", label: "Workspace" };
}
