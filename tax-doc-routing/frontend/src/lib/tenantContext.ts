export const CRM_TENANT_STORAGE_KEY = "crm.frontend.tenantId";

export type CrmTenantContextSource = "unset" | "explicit" | "session";

export type CrmTenantContext = {
  tenantId?: string;
  source: CrmTenantContextSource;
};

export function createTenantContext(tenantId?: string | null): CrmTenantContext {
  const normalized = normalizeTenantId(tenantId);
  return {
    tenantId: normalized,
    source: normalized ? "explicit" : "unset"
  };
}

export function readSessionTenantContext(storage = browserSessionStorage()): CrmTenantContext {
  const tenantId = storage?.getItem(CRM_TENANT_STORAGE_KEY) ?? undefined;
  const context = createTenantContext(tenantId);
  return {
    ...context,
    source: context.tenantId ? "session" : "unset"
  };
}

export function writeSessionTenantId(tenantId: string | null, storage = browserSessionStorage()) {
  if (!storage) return;
  const normalized = normalizeTenantId(tenantId);
  if (normalized) {
    storage.setItem(CRM_TENANT_STORAGE_KEY, normalized);
  } else {
    storage.removeItem(CRM_TENANT_STORAGE_KEY);
  }
}

function browserSessionStorage() {
  return typeof window === "undefined" ? undefined : window.sessionStorage;
}

function normalizeTenantId(tenantId: string | null | undefined) {
  const normalized = tenantId?.trim();
  return normalized ? normalized : undefined;
}
