import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router';
import { describe, expect, it, vi } from 'vitest';

const taxRouting = vi.hoisted(() => ({ permissionsStatus: 'ready' as 'loading' | 'ready' | 'error' }));
const session = vi.hoisted(() => ({ userName: null as string | null }));

vi.mock('../features/shared/state/TaxRoutingProvider', () => ({
  useTaxRouting: () => ({ permissionsStatus: taxRouting.permissionsStatus, draftApi: { reset: () => undefined } })
}));
vi.mock('./SessionProvider', () => ({
  useSession: () => ({ userName: session.userName, signedOutByUser: false, signIn: () => undefined, signOut: () => undefined })
}));

import { RequireAuth } from './RequireAuth';

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/sign-in" element={<div>sign in page</div>} />
        <Route
          path="/dashboard"
          element={
            <RequireAuth>
              <div>protected page</div>
            </RequireAuth>
          }
        />
      </Routes>
    </MemoryRouter>
  );
}

describe('RequireAuth', () => {
  it('sends a signed-out visitor to /sign-in', () => {
    session.userName = null;
    renderAt('/dashboard');
    expect(screen.getByText('sign in page')).toBeInTheDocument();
  });

  it('shows the page for a signed-in user', () => {
    session.userName = 'alex';
    taxRouting.permissionsStatus = 'ready';
    renderAt('/dashboard');
    expect(screen.getByText('protected page')).toBeInTheDocument();
  });

  it('fails closed with an explicit state when the session cannot be verified', () => {
    session.userName = 'ghost';
    taxRouting.permissionsStatus = 'error';
    renderAt('/dashboard');
    expect(screen.getByText('Your session could not be verified')).toBeInTheDocument();
    expect(screen.queryByText('protected page')).not.toBeInTheDocument();
  });
});
