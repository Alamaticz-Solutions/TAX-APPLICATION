import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';
import { normalizeUserName, readSessionUser, writeSessionUser } from '../lib/session';

type SessionContextValue = {
  /** The signed-in user name, or null when signed out. */
  userName: string | null;
  /** True after an explicit log out, so the next sign-in starts fresh instead of resuming the old page. */
  signedOutByUser: boolean;
  signIn: (userName: string) => void;
  signOut: () => void;
};

const SessionContext = createContext<SessionContextValue | null>(null);

/** Holds who is signed in for this tab. The data providers below it read the user from here. */
export function SessionProvider({ children }: { children: ReactNode }) {
  const [userName, setUserName] = useState<string | null>(() => readSessionUser());
  const [signedOutByUser, setSignedOutByUser] = useState(false);

  const value = useMemo<SessionContextValue>(
    () => ({
      userName,
      signedOutByUser,
      signIn: (next) => {
        const name = normalizeUserName(next);
        writeSessionUser(name);
        setSignedOutByUser(false);
        setUserName(name);
      },
      signOut: () => {
        writeSessionUser(null);
        setSignedOutByUser(true);
        setUserName(null);
      }
    }),
    [userName, signedOutByUser]
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession(): SessionContextValue {
  const ctx = useContext(SessionContext);
  if (!ctx) throw new Error('useSession must be used inside SessionProvider');
  return ctx;
}
