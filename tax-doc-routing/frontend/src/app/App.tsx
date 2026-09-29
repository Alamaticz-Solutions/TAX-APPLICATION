import { lazy, type ReactNode } from 'react';
import { Link, Navigate, Route, Routes } from 'react-router';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { FeedbackState } from '@appfw/pds-health-components/surfaces';
import { AppShell } from './AppShell';
import { RequireAdmin } from './RequireAdmin';

// Every screen is route-level code-split; none is needed for the first paint of another.
const DashboardScreen = lazy(() => import('../features/dashboard/DashboardScreen').then((m) => ({ default: m.DashboardScreen })));
const CreateRoutingRecordScreen = lazy(() =>
  import('../features/routing/CreateRoutingRecordScreen').then((m) => ({ default: m.CreateRoutingRecordScreen }))
);
const MyCasesScreen = lazy(() => import('../features/records/MyCasesScreen').then((m) => ({ default: m.MyCasesScreen })));
const AllCasesScreen = lazy(() => import('../features/records/AllCasesScreen').then((m) => ({ default: m.AllCasesScreen })));
const RecordDetailsScreen = lazy(() => import('../features/records/RecordDetailsScreen').then((m) => ({ default: m.RecordDetailsScreen })));
const ExceptionQueueScreen = lazy(() =>
  import('../features/exceptions/ExceptionQueueScreen').then((m) => ({ default: m.ExceptionQueueScreen }))
);
const AdministrationScreen = lazy(() =>
  import('../features/admin/AdministrationScreen').then((m) => ({ default: m.AdministrationScreen }))
);

/**
 * SPA route table. Everything renders inside `AppShell`. Admin-only routes are guarded here for
 * navigation only; real enforcement is the backend RBAC policy, never the UI.
 */
export function AppRoot({ scaffoldReference = null }: { scaffoldReference?: ReactNode }) {
  return (
    <Routes>
      {/* framework UI-kit reference: dev builds only (import.meta.env.DEV in main.tsx) */}
      {scaffoldReference ? <Route path="/scaffold" element={scaffoldReference} /> : null}
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardScreen />} />
        <Route path="new" element={<Navigate to="/new/client" replace />} />
        <Route path="new/:step" element={<CreateRoutingRecordScreen />} />
        <Route path="cases" element={<MyCasesScreen />} />
        <Route path="records/:recordId" element={<RecordDetailsScreen />} />
        <Route path="exceptions" element={<ExceptionQueueScreen />} />
        <Route
          path="admin/cases"
          element={
            <RequireAdmin permission="routing_record.view_assignee">
              <AllCasesScreen />
            </RequireAdmin>
          }
        />
        <Route
          path="admin"
          element={
            <RequireAdmin permission="rbac.manage">
              <AdministrationScreen />
            </RequireAdmin>
          }
        />
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
