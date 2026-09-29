import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import {
  readSessionAuthContext,
  writeSessionAuthorization,
  type CrmAuthContext
} from "../lib/authContext";
import {
  readSessionTenantContext,
  writeSessionTenantId,
  type CrmTenantContext
} from "../lib/tenantContext";
import { createAppfwClient } from "../lib/appfwClient";

type AuthContextValue = {
  auth: CrmAuthContext;
  setAuthorization: (authorization: string | null) => void;
};

type TenantContextValue = {
  tenant: CrmTenantContext;
  setTenantId: (tenantId: string | null) => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);
const TenantContext = createContext<TenantContextValue | null>(null);

export function AppProviders({ children }: { children: ReactNode }) {
  const [auth, setAuth] = useState<CrmAuthContext>(() => readSessionAuthContext());
  const [tenant, setTenant] = useState<CrmTenantContext>(() => readSessionTenantContext());

  const authValue = useMemo<AuthContextValue>(
    () => ({
      auth,
      setAuthorization: (authorization) => {
        writeSessionAuthorization(authorization);
        setAuth(readSessionAuthContext());
      }
    }),
    [auth]
  );

  const tenantValue = useMemo<TenantContextValue>(
    () => ({
      tenant,
      setTenantId: (tenantId) => {
        writeSessionTenantId(tenantId);
        setTenant(readSessionTenantContext());
      }
    }),
    [tenant]
  );

  return (
    <AuthContext.Provider value={authValue}>
      <TenantContext.Provider value={tenantValue}>{children}</TenantContext.Provider>
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const value = useContext(AuthContext);
  if (!value) throw new Error("useAuth must be used within AppProviders");
  return value;
}

export function useTenant(): TenantContextValue {
  const value = useContext(TenantContext);
  if (!value) throw new Error("useTenant must be used within AppProviders");
  return value;
}

// Builds an App Framework GraphQL client bound to the current auth + tenant.
// baseUrl is left undefined in dev so requests go same-origin through the Vite
// proxy; set VITE_BACKEND_URL to target a backend directly.
export function useAppfwClient() {
  const { auth } = useAuth();
  const { tenant } = useTenant();
  return useMemo(
    () =>
      createAppfwClient({
        baseUrl: import.meta.env.VITE_BACKEND_URL || undefined,
        auth,
        tenant
      }),
    [auth, tenant]
  );
}
