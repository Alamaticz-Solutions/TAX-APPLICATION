// The signed-in user for this browser tab. Local sign-in (ENV_NAME=local): the backend accepts
// `Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=n/a` and resolves the user's
// real roles and grants from the database (see lib/permissions.ts), so the only thing the browser
// has to remember is who signed in. Production identity is Okta/JWT through the backend; swapping
// it in replaces this file and the sign-in screen, not the screens behind them.
//
// Kept in sessionStorage on purpose: it ends with the tab, like the Project Governance session.

export const SESSION_STORAGE_KEY = 'tax-doc-routing.frontend.session';

export function normalizeUserName(value: string | null | undefined): string | null {
  const name = value?.trim().toLowerCase();
  return name ? name : null;
}

export function readSessionUser(storage: Storage | undefined = browserSessionStorage()): string | null {
  try {
    return normalizeUserName(storage?.getItem(SESSION_STORAGE_KEY));
  } catch {
    return null;
  }
}

export function writeSessionUser(userName: string | null, storage: Storage | undefined = browserSessionStorage()): void {
  try {
    const name = normalizeUserName(userName);
    if (!storage) return;
    if (name) storage.setItem(SESSION_STORAGE_KEY, name);
    else storage.removeItem(SESSION_STORAGE_KEY);
  } catch {
    /* storage blocked: the session just won't survive a reload */
  }
}

function browserSessionStorage(): Storage | undefined {
  try {
    return typeof window === 'undefined' ? undefined : window.sessionStorage;
  } catch {
    return undefined;
  }
}
