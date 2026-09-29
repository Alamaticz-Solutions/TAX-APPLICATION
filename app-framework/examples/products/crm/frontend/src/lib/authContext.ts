export const CRM_AUTHORIZATION_STORAGE_KEY = "crm.frontend.authorization";

export type CrmAuthContextSource = "anonymous" | "explicit" | "session";

export type CrmAuthContext = {
  authorization?: string;
  userName?: string;
  roles: string[];
  source: CrmAuthContextSource;
};

export type CrmAuthContextInput = {
  authorization?: string | null;
  userName?: string | null;
  roles?: readonly string[] | null;
};

export function createAuthContext(input: CrmAuthContextInput = {}): CrmAuthContext {
  const authorization = normalizeOptional(input.authorization);
  return {
    authorization,
    userName: normalizeOptional(input.userName),
    roles: normalizeRoles(input.roles),
    source: authorization ? "explicit" : "anonymous"
  };
}

export function readSessionAuthContext(storage = browserSessionStorage()): CrmAuthContext {
  const authorization = storage?.getItem(CRM_AUTHORIZATION_STORAGE_KEY) ?? undefined;
  const context = createAuthContext({ authorization });
  return {
    ...context,
    source: context.authorization ? "session" : "anonymous"
  };
}

export function writeSessionAuthorization(authorization: string | null, storage = browserSessionStorage()) {
  if (!storage) return;
  const normalized = normalizeOptional(authorization);
  if (normalized) {
    storage.setItem(CRM_AUTHORIZATION_STORAGE_KEY, normalized);
  } else {
    storage.removeItem(CRM_AUTHORIZATION_STORAGE_KEY);
  }
}

function browserSessionStorage() {
  return typeof window === "undefined" ? undefined : window.sessionStorage;
}

function normalizeOptional(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

function normalizeRoles(roles: readonly string[] | null | undefined) {
  return Array.from(new Set((roles ?? []).map((role) => role.trim()).filter(Boolean))).sort();
}
