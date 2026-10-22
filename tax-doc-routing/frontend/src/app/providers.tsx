import type { ReactNode } from 'react';
import { TaxRoutingProvider } from '../features/shared/state/TaxRoutingProvider';
import { SessionProvider } from './SessionProvider';

/**
 * App-level providers. Data currently comes from the prototype state in `TaxRoutingProvider`;
 * when the screens move to the GraphQL API this is where the auth/tenant contexts and the
 * `lib/appfwClient.ts` client are provided.
 */
export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <SessionProvider>
      <TaxRoutingProvider>{children}</TaxRoutingProvider>
    </SessionProvider>
  );
}
