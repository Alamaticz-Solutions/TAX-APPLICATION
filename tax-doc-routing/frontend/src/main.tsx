import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router';
import {
  AppearanceProvider,
  AppShell,
  Badge,
  Button,
  CommandPalette,
  DataGridShell,
  Dialog,
  FormLayout,
  KpiTile,
  PageHeader,
  Surface,
  TextField,
  type CommandPaletteItem,
  type PdsDataGridColumn
} from '@appfw/pds-health-components';
import { AppProviders } from './app/providers';
import { ErrorBoundary } from './app/ErrorBoundary';
import { AppRoot } from './app/App';
import { taxRoutingUiContract } from './generated/appfw-ui-contract';
import './styles.css';

// ---------------------------------------------------------------------------
// The product SPA. Screens live under `src/features/**`; the shell, routing and providers under
// `src/app`; shared presentational pieces under `src/components`. `ScaffoldReference` below is the
// framework's PDS component reference screen (the scaffold check looks for these components
// here), reachable at `/scaffold` in development only.
// ---------------------------------------------------------------------------

type StarterRow = Record<string, unknown> & { id: string; item: string; owner: string; state: string };

const starterRows: StarterRow[] = [
  { id: 'REF-1', item: 'Design system reference', owner: 'Platform', state: 'Ready' },
  { id: 'REF-2', item: 'Token-only styling', owner: 'Platform', state: 'Ready' }
];

const starterColumns: PdsDataGridColumn<StarterRow>[] = [
  { key: 'item', header: 'Reference', width: '50%' },
  { key: 'owner', header: 'Owner', width: '25%' },
  { key: 'state', header: 'State', width: '25%', render: (row) => <Badge tone="success">{row.state}</Badge> }
];

const referenceCommands: CommandPaletteItem[] = [
  { id: 'ref:contract', group: 'Reference', label: 'Generated UI contract', detail: 'src/generated/appfw-ui-contract.ts' }
];

/** Product-owned UI kit reference. Dev-only: created only when `import.meta.env.DEV` is true. */
function ScaffoldReference() {
  const [reviewOpen, setReviewOpen] = useState(false);
  return (
    <AppShell
      brand={<strong>TaxPro Enterprise Platform</strong>}
      navigation={<div aria-label="Reference sections">Components</div>}
      topBar={<CommandPalette items={referenceCommands} triggerLabel="Search reference" searchPlaceholder="Search design-system reference" />}
    >
      <PageHeader
        eyebrow="Reference"
        title="UI kit reference"
        subtitle={`${taxRoutingUiContract.entities.length} entities in contract v${taxRoutingUiContract.version}`}
        actions={<Badge tone="neutral">{taxRoutingUiContract.provider.dataSourceType}</Badge>}
      />
      <KpiTile label="Design system" value="PDS package" detail="@appfw/pds-health-components" tone="accent" />
      <Surface title="Form primitives" density="compact">
        <FormLayout
          columns="two"
          footer={
            <Button variant="primary" onClick={() => setReviewOpen(true)}>
              Open dialog
            </Button>
          }
        >
          <TextField label="Example text" defaultValue="Reference" />
        </FormLayout>
      </Surface>
      <Surface title="Grid primitives" density="compact">
        <DataGridShell columns={starterColumns} rows={starterRows} rowKey="id" ariaLabel="Reference grid" />
      </Surface>
      <Dialog
        open={reviewOpen}
        title="Reference dialog"
        description="Shared overlay primitive."
        onClose={() => setReviewOpen(false)}
        footer={
          <Button variant="primary" onClick={() => setReviewOpen(false)}>
            Close
          </Button>
        }
      >
        <p>This screen is a UI kit reference, not part of the product navigation.</p>
      </Dialog>
    </AppShell>
  );
}

const container = document.getElementById('root');
if (!container) throw new Error('Root element #root is missing from index.html');

createRoot(container).render(
  <React.StrictMode>
    <ErrorBoundary>
      <BrowserRouter>
        <AppearanceProvider persist storageKey="tax-doc-routing.frontend.appearance">
          <AppProviders>
            <AppRoot scaffoldReference={import.meta.env.DEV ? <ScaffoldReference /> : null} />
          </AppProviders>
        </AppearanceProvider>
      </BrowserRouter>
    </ErrorBoundary>
  </React.StrictMode>
);
