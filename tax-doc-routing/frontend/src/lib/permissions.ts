import { createAppfwClient } from './appfwClient';

// Reads the caller's real, database-driven permission grants from the backend's `myPermissions`
// query (see docs/architecture/rbac-design.md). This is the frontend half of that design: the
// grants shown here are exactly what `backend/src/services/authz.rs` resolved from the
// app_user/role/permission/role_permission tables for the identity the caller authenticates as —
// nothing here is a role name, and nothing here is decided in the frontend.
//
// Local dev auth (matches `docs/LOCAL_DEV_SETUP.md`): the backend accepts
// `Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=<ignored>` with
// `ENV_NAME=local`. `roles=` is accepted but no policy reads it any more — the backend looks the
// user up by name and tenant instead.

export type Grant = {
  code: string;
  resource: string | null;
  action: string | null;
  scope: string;
  owner_field: string | null;
};

export type Principal = {
  user_id: string | null;
  grants: Grant[];
};

const MY_PERMISSIONS_QUERY = '{ myPermissions }';

function localAuthHeader(userName: string, tenantId = 't1'): string {
  return `Bearer appfw-local:user=${userName};tenant=${tenantId};roles=n/a`;
}

/** Fetches the caller's grants. Never throws for an access-denied or unknown-user response — an
 * unresolved identity legitimately has no grants, which the caller should render as "no access",
 * not an error page. */
export async function fetchMyPermissions(userName: string, tenantId = 't1'): Promise<Principal> {
  const client = createAppfwClient({ authorization: localAuthHeader(userName, tenantId), tenantId });
  try {
    const result = await client.graphql<{ myPermissions: Principal }, Record<string, never>>({
      schemaName: 'tax_routing',
      operationName: 'my_permissions',
      query: MY_PERMISSIONS_QUERY,
      variables: {}
    });
    return result.data.myPermissions ?? { user_id: null, grants: [] };
  } catch {
    return { user_id: null, grants: [] };
  }
}

export function hasPermission(grants: Grant[], code: string): boolean {
  return grants.some((g) => g.code === code);
}

export function hasScope(grants: Grant[], code: string, scope: string): boolean {
  return grants.some((g) => g.code === code && g.scope === scope);
}
