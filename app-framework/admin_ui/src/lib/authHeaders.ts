const ADMIN_AUTHORIZATION_KEY = "appfw.admin.authorization";

export function adminAuthHeaders(): Record<string, string> {
  if (typeof window === "undefined") {
    return {};
  }

  const authorization = window.sessionStorage.getItem(ADMIN_AUTHORIZATION_KEY)?.trim();
  return authorization ? { Authorization: authorization } : {};
}

export { ADMIN_AUTHORIZATION_KEY };
