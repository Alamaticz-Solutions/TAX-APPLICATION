import { lazy, type ReactNode } from 'react';
import { Link, Navigate, Route, Routes } from 'react-router';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { FeedbackState } from '@appfw/pds-health-components/surfaces';
import { SignInScreen } from '../features/auth/SignInScreen';
import { AppShell } from './AppShell';
import { RequireAuth } from './RequireAuth';

// Every screen is route-level code-split; none is needed for the first paint of another.
const DashboardScreen = lazy(() => import('../features/dashboard/DashboardScreen').then((m) => ({ default: m.DashboardScreen })));
const BulkRequestScreen = lazy(() => import('../features/bulk/BulkRequestScreen').then((m) => ({ default: m.BulkRequestScreen })));
const CreateRoutingRecordScreen = lazy(() =>
  import('../features/routing/CreateRoutingRecordScreen').then((m) => ({ default: m.CreateRoutingRecordScreen }))
);
const MyCasesScreen = lazy(() => import('../features/records/MyCasesScreen').then((m) => ({ default: m.MyCasesScreen })));
const RecordDetailsScreen = lazy(() => import('../features/records/RecordDetailsScreen').then((m) => ({ default: m.RecordDetailsScreen })));
const ExceptionQueueScreen = lazy(() =>
  import('../features/exceptions/ExceptionQueueScreen').then((m) => ({ default: m.ExceptionQueueScreen }))
);

/**
 * SPA route table. `/sign-in` (and the `/login` alias) is the one public route; everything else
 * renders inside `AppShell` behind `RequireAuth`. Real enforcement is the backend RBAC policy, never the UI.
 */
export function AppRoot({ scaffoldReference = null }: { scaffoldReference?: ReactNode }) {
  return (
    <Routes>
      {/* framework UI-kit reference: dev builds only (import.meta.env.DEV in main.tsx) */}
      {scaffoldReference ? <Route path="/scaffold" element={scaffoldReference} /> : null}
      <Route path="sign-in" element={<SignInScreen />} />
      <Route path="login" element={<Navigate to="/sign-in" replace />} />
      <Route
        element={
          <RequireAuth>
            <AppShell />
          </RequireAuth>
        }
      >
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardScreen />} />
        <Route path="new" element={<Navigate to="/new/client" replace />} />
        <Route path="new/:step" element={<CreateRoutingRecordScreen />} />
        <Route path="bulk" element={<BulkRequestScreen />} />
        <Route path="cases" element={<MyCasesScreen />} />
        <Route path="records/:recordId" element={<RecordDetailsScreen />} />
        <Route path="exceptions" element={<ExceptionQueueScreen />} />
        {/* Retired pages: All Cases is the cases screen for admins, Administration is hidden until editing exists. */}
        <Route path="admin/cases" element={<Navigate to="/cases" replace />} />
        <Route path="admin" element={<Navigate to="/dashboard" replace />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  );
}

function NotFound() {
  return (
    <>
      <PageHeader title="Not found" />
      <FeedbackState kind="error" title="No such screen" detail="Check the URL or return to the dashboard." />
      <p>
        <Link to="/dashboard">← Back to dashboard</Link>
      </p>
    </>
  );
}
