import type { ReactNode } from 'react';
import { Navigate } from 'react-router';
import { useTaxRouting } from '../features/shared/state/TaxRoutingProvider';

/**
 * Navigation guard for admin-only routes, driven by the caller's real permission grants
 * (fetched live from the backend's `myPermissions` query — see docs/architecture/rbac-design.md)
 * rather than a role literal. The backend RBAC policy is still the real enforcement; this only
 * decides whether the nav link is worth showing.
 *
 * While permissions are loading, nothing is shown yet rather than a flash of the wrong screen.
 */
export function RequireAdmin({ permission, children }: { permission: string; children: ReactNode }) {
  const { hasPermission, permissionsStatus } = useTaxRouting();
  if (permissionsStatus === 'loading') return null;
  return hasPermission(permission) ? <>{children}</> : <Navigate to="/dashboard" replace />;
}
