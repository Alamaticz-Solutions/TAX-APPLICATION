import { useNavigate, useLocation } from 'react-router';
import { AlertTriangle, Briefcase, LayoutDashboard, type LucideIcon } from 'lucide-react';
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
      <div className="tax-sidebar-brand-title">TaxPro Enterprise Platform</div>
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
  // "Create New Request" is deliberately not a nav item: it lives on the Dashboard and All Cases
  // headers (and the command palette), so the rail only holds places to look at records.
  const { exceptionQueue, hasPermission } = useTaxRouting();
  // The label follows the real grant fetched from the backend (docs/architecture/rbac-design.md):
  // staff see their own cases, a Tax Admin sees every case on the same screen.
  const seesEveryone = hasPermission('routing_record.view_assignee');

  const home: NavItem[] = [{ label: 'Dashboard', icon: LayoutDashboard, route: '/dashboard', end: true }];
  const requests: NavItem[] = [
    { label: seesEveryone ? 'Request Management' : 'My Cases', icon: Briefcase, route: '/cases' },
    {
      label: 'Exception Task',
      icon: AlertTriangle,
      route: '/exceptions',
      badge: exceptionQueue.length
    }
  ];

  return (
    <div className="tax-sidebar-nav-sections">
      <ul className="tax-sidebar-nav-list">
        {home.map((item) => (
          <NavRow key={item.label} item={item} />
        ))}
      </ul>
      {/* A small gap sets the request screens apart from the dashboard (the sections container's own spacing). */}
      <ul className="tax-sidebar-nav-list">
        {requests.map((item) => (
          <NavRow key={item.label} item={item} />
        ))}
      </ul>
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
