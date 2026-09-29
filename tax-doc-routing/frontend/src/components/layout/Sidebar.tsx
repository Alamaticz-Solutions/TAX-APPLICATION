import { useNavigate, useLocation } from 'react-router';
import { AlertTriangle, Briefcase, FilePlus2, LayoutDashboard, Settings, Users, type LucideIcon } from 'lucide-react';
import { Avatar, NavigationItem, PdsHealthLogo } from '@appfw/pds-health-components/foundation';
import { Badge } from '@appfw/pds-health-components/primitives';
import { useTaxRouting } from '../../features/shared/state/TaxRoutingProvider';
import { isModifiedClick } from '../../lib/navigation';

/**
 * Primary navigation rail content, passed into the PDS `AppShell`'s `brand` / `navigation` /
 * `footer` slots by `app/AppShell.tsx`. Width and narrow-screen behaviour are PDS's own.
 */

type NavItem = {
  label: string;
  icon: LucideIcon;
  route: string;
  end?: boolean;
  badge?: number;
};

function isActive(pathname: string, item: NavItem): boolean {
  if (item.end) return pathname === item.route;
  return pathname === item.route || pathname.startsWith(`${item.route}/`);
}

export function SidebarBrand() {
  return (
    <div className="tax-sidebar-brand">
      <PdsHealthLogo variant="mark" decorative className="tax-sidebar-logo" />
      <div className="tax-sidebar-brand-title">Tax Document Routing</div>
    </div>
  );
}

function NavRow({ item }: { item: NavItem }) {
  const navigate = useNavigate();
  const location = useLocation();
  return (
    <li>
      <NavigationItem
        href={item.route}
        onClick={(e) => {
          if (isModifiedClick(e)) return; // let Ctrl/middle-click open a new tab, as a link should
          e.preventDefault();
          navigate(item.route);
        }}
        icon={<item.icon size={19} aria-hidden="true" />}
        label={item.label}
        current={isActive(location.pathname, item)}
        trailing={item.badge ? <Badge tone="danger">{item.badge > 99 ? '99+' : item.badge}</Badge> : undefined}
      />
    </li>
  );
}

export function SidebarNav() {
  const { exceptionQueue, draft, draftApi, hasPermission } = useTaxRouting();
  const navigate = useNavigate();
  const location = useLocation();
  // Admin section visibility follows the real grants fetched from the backend, not the role
  // switcher directly — see docs/architecture/rbac-design.md.
  const canSeeAllCases = hasPermission('routing_record.view_assignee');
  const canSeeAdministration = hasPermission('rbac.manage');

  const workspace: NavItem[] = [
    {
      label: 'Dashboard',
      icon: LayoutDashboard,
      route: '/dashboard',
      end: true
    },
    { label: 'My Cases', icon: Briefcase, route: '/cases' },
    {
      label: 'Exception Queue',
      icon: AlertTriangle,
      route: '/exceptions',
      badge: exceptionQueue.length
    }
  ];
  const admin: NavItem[] = [
    ...(canSeeAllCases ? [{ label: 'All Cases', icon: Users, route: '/admin/cases' }] : []),
    ...(canSeeAdministration ? [{ label: 'Administration', icon: Settings, route: '/admin', end: true }] : [])
  ];
  const newRecord: NavItem = {
    label: 'Create New Request',
    icon: FilePlus2,
    route: '/new'
  };

  return (
    <div className="tax-sidebar-nav-sections">
      <ul className="tax-sidebar-nav-list">
        <li>
          <NavigationItem
            href="/new/client"
            onClick={(e) => {
              if (isModifiedClick(e)) return;
              e.preventDefault();
              // A finished record must not reopen its old draft.
              if (draft.recordId) draftApi.reset();
              navigate('/new/client');
            }}
            icon={<newRecord.icon size={19} aria-hidden="true" />}
            label={newRecord.label}
            current={location.pathname.startsWith('/new')}
          />
        </li>
        {workspace.map((item) => (
          <NavRow key={item.label} item={item} />
        ))}
      </ul>
      {admin.length > 0 ? (
        <div>
          <div className="tax-sidebar-section-label">Admin</div>
          <ul className="tax-sidebar-nav-list">
            {admin.map((item) => (
              <NavRow key={item.label} item={item} />
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

export function SidebarFooter() {
  const { user } = useTaxRouting();
  return (
    <ul className="tax-sidebar-nav-list">
      <li>
        <NavigationItem
          href="/dashboard"
          onClick={(e) => e.preventDefault()}
          icon={<Avatar name={user.name} initials={user.initials} size="md" />}
          label={user.name}
          description={user.roleLabel}
        />
      </li>
    </ul>
  );
}
