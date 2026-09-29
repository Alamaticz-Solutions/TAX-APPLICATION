import { useState } from 'react';
import { DataGridShell } from '@appfw/pds-health-components/data';
import { PageHeader, Tabs } from '@appfw/pds-health-components/layout';
import { Badge } from '@appfw/pds-health-components/primitives';
import { InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import type { PdsDataGridColumn } from '@appfw/pds-health-components/types';
import { useDocumentTypeCatalogue } from '../shared/hooks/useDocumentTypeCatalogue';
import { useEntityDirectory } from '../shared/hooks/useEntityDirectory';
import { useClientDirectory } from '../shared/hooks/useClientDirectory';

type RouteRow = { type: string; businessName: string; externalFolder: string; internalFolder: string; routing: string; note: string };
type ClientRow = { id: string; fullName: string; officeLocation: string; pdsEmail: string; folderName: string; active: string };
type EntityRow = { number: string; name: string; type: string };

const routeColumns: PdsDataGridColumn<RouteRow>[] = [
  { key: 'type', header: 'Document Type', width: 150 },
  { key: 'businessName', header: 'Business Name', width: 220 },
  { key: 'externalFolder', header: 'External Folder', width: 170 },
  { key: 'internalFolder', header: 'Internal Folder', width: 190 },
  { key: 'routing', header: 'Routing', width: 170, render: (row) => <Badge tone="neutral">{row.routing}</Badge> },
  { key: 'note', header: 'Note', width: 300 }
];

const clientColumns: PdsDataGridColumn<ClientRow>[] = [
  { key: 'id', header: 'Client ID', width: 120 },
  { key: 'fullName', header: 'Full Name', width: 170 },
  { key: 'officeLocation', header: 'Office', width: 150 },
  { key: 'pdsEmail', header: 'PDS Email', width: 260 },
  { key: 'folderName', header: 'Folder', width: 160 },
  { key: 'active', header: 'Active', width: 90 }
];

const entityColumns: PdsDataGridColumn<EntityRow>[] = [
  { key: 'number', header: 'Oracle ID', width: 120 },
  { key: 'name', header: 'Entity Name', width: 300 },
  { key: 'type', header: 'Entity Type', width: 180 }
];

/** Read-only views of the reference data that drives the routing workflow, read live from the database. */
export function AdministrationScreen() {
  const [tab, setTab] = useState('routing');
  const { documentTypes } = useDocumentTypeCatalogue();
  const { entities } = useEntityDirectory();
  const { clients } = useClientDirectory();

  const routeRows: RouteRow[] = documentTypes.map((d) => ({
    type: d.type,
    businessName: d.businessName,
    externalFolder: d.externalFolder,
    internalFolder: d.internalFolder ?? '—',
    routing: d.externalOnly ? 'External only' : 'External + Internal',
    note: d.note
  }));
  const clientRows: ClientRow[] = clients.map((c) => ({
    id: c.id,
    fullName: c.fullName,
    officeLocation: c.officeLocation,
    pdsEmail: c.pdsEmail,
    folderName: c.folderName,
    active: c.active ? 'Yes' : 'No'
  }));
  const entityRows: EntityRow[] = entities.map((e) => ({ number: e.number, name: e.name, type: e.type }));

  return (
    <>
      <PageHeader title="Administration" subtitle="Reference data used by the routing workflow." />
      <InlineAlert tone="neutral" title="Read-only in this prototype" detail="Editing reference data is out of scope for the static UI." />
      <Tabs
        ariaLabel="Reference data"
        selectedId={tab}
        onChange={setTab}
        items={[
          {
            id: 'routing',
            label: 'Document Routing',
            content: (
              <Surface density="compact">
                <DataGridShell<RouteRow> ariaLabel="Document type routing" columns={routeColumns} rows={routeRows} rowKey="type" density="comfortable" />
              </Surface>
            )
          },
          {
            id: 'taxpayers',
            label: 'Taxpayers',
            content: (
              <Surface density="compact">
                <DataGridShell<ClientRow> ariaLabel="Taxpayers" columns={clientColumns} rows={clientRows} rowKey="id" density="comfortable" />
              </Surface>
            )
          },
          {
            id: 'entities',
            label: 'Entities',
            content: (
              <Surface density="compact">
                <DataGridShell<EntityRow> ariaLabel="Entities" columns={entityColumns} rows={entityRows} rowKey="number" density="comfortable" />
              </Surface>
            )
          }
        ]}
      />
    </>
  );
}
