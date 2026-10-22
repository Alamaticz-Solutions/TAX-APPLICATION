import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router';
import { Button } from '@appfw/pds-health-components/primitives';
import { FeedbackState } from '@appfw/pds-health-components/surfaces';
import { useTaxRouting } from '../features/shared/state/TaxRoutingProvider';
import { useSession } from './SessionProvider';

/**
 * Shell-level auth guard: the UI fails closed. A visitor who is not signed in is sent to
 * /sign-in and returns to the page they asked for afterwards. A signed-in user whose identity the
 * backend cannot resolve (unknown user, access removed, service rejecting the session) gets an
 * explicit "session could not be verified" state, never a half-working screen.
 *
 * This only decides what to show; the backend RBAC policy is the authority on every request.
 */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { userName, signedOutByUser, signOut } = useSession();
  const { permissionsStatus, draftApi } = useTaxRouting();
  const location = useLocation();

  if (!userName) {
    // After an explicit log out the next person starts at the dashboard, not on the last page.
    const from = signedOutByUser ? undefined : `${location.pathname}${location.search}`;
    return <Navigate to="/sign-in" replace state={{ from }} />;
  }

  if (permissionsStatus === 'error') {
    return (
      <FeedbackState
        kind="error"
        title="Your session could not be verified"
        detail="The service did not recognise this user or could not be reached. Sign in again to continue."
        action={
          <Button
            variant="primary"
            onClick={() => {
              draftApi.reset();
              signOut();
            }}
          >
            Back to sign in
          </Button>
        }
      />
    );
  }

  return <>{children}</>;
}
