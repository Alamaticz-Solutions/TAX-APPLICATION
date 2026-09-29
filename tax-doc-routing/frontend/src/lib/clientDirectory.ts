import { createAppfwClient } from './appfwClient';
import type { Client, Entity, EntityType } from '../features/shared/types';

// Client and entity reference data (business spec 5.4), read live from the database instead of
// the hardcoded frontend/src/features/shared/data/clients.ts and entities.ts arrays. Same
// local-auth mechanism as lib/formEngine.ts and lib/permissions.ts.

function localAuthHeader(userName: string, tenantId = 't1'): string {
  return `Bearer appfw-local:user=${userName};tenant=${tenantId};roles=n/a`;
}

function client(userName: string) {
  return createAppfwClient({ authorization: localAuthHeader(userName), tenantId: 't1' });
}

type RawTaxPayerRefData = {
  id: string | null;
  first_name: string | null;
  last_name: string | null;
  full_name: string | null;
  personal_email: string | null;
  pds_email: string | null;
  office_location: string | null;
  folder_name: string | null;
  external_folder_id: string | null;
  internal_folder_path: string | null;
  is_active: boolean | null;
  read_write_password: string | null;
};

const toClient = (row: RawTaxPayerRefData): Client => ({
  id: row.external_folder_id ?? '',
  refId: row.id ?? '',
  firstName: row.first_name ?? '',
  lastName: row.last_name ?? '',
  fullName: row.full_name ?? '',
  officeLocation: row.office_location ?? '',
  pdsEmail: row.pds_email ?? '',
  personalEmail: row.personal_email ?? '',
  internalFolder: row.internal_folder_path ?? '',
  folderName: row.folder_name ?? '',
  passwordOnFile: Boolean(row.read_write_password),
  active: row.is_active ?? false,
  readWritePassword: row.read_write_password ?? ''
});

const TAXPAYER_QUERY = `query($filter: JSON, $limit: Int) {
  queryTaxPayerRefData(filter: $filter, limit: $limit) {
    items {
      id
      first_name
      last_name
      full_name
      personal_email
      pds_email
      office_location
      folder_name
      external_folder_id
      internal_folder_path
      is_active
      read_write_password
    }
  }
}`;

async function queryTaxpayers(userName: string, filter: unknown, limit: number): Promise<Client[]> {
  const result = await client(userName).graphql<{ queryTaxPayerRefData: { items: RawTaxPayerRefData[] } }, { filter: unknown; limit: number }>({
    schemaName: 'tax_routing',
    operationName: 'query_tax_payer_ref_data',
    query: TAXPAYER_QUERY,
    variables: { filter, limit }
  });
  return result.data.queryTaxPayerRefData.items.map(toClient);
}

/** Client lookup over the reference table (business spec 5.4), replacing mockClientService. */
export async function searchClients(userName: string, search: string): Promise<Client[]> {
  const q = search.trim();
  const filter = q
    ? {
        _and: [
          { is_active: { _eq: true } },
          { _or: [{ full_name: { _contains: q } }, { external_folder_id: { _contains: q } }, { pds_email: { _contains: q } }, { personal_email: { _contains: q } }] }
        ]
      }
    : { is_active: { _eq: true } };
  return queryTaxpayers(userName, filter, 10);
}

/** The full active client list, for reference-data admin views (business spec 5.4). */
export async function fetchClients(userName: string): Promise<Client[]> {
  return queryTaxpayers(userName, { is_active: { _eq: true } }, 100);
}

type RawTaxEntityList = { oracle_id: string | null; entity_name: string | null; entity_type: string | null; is_active: boolean | null };

const ENTITY_QUERY = `{
  queryTaxEntityLists(filter: { is_active: { _eq: true } }, limit: 100) {
    items { oracle_id entity_name entity_type is_active }
  }
}`;

/** The active entity list (business spec 5.4), replacing the static entities.ts lookup table. */
export async function fetchEntities(userName: string): Promise<Entity[]> {
  const result = await client(userName).graphql<{ queryTaxEntityLists: { items: RawTaxEntityList[] } }, Record<string, never>>({
    schemaName: 'tax_routing',
    operationName: 'query_tax_entity_lists',
    query: ENTITY_QUERY,
    variables: {}
  });
  return result.data.queryTaxEntityLists.items.map((row) => ({
    number: row.oracle_id ?? '',
    name: row.entity_name ?? '',
    type: (row.entity_type ?? '') as EntityType
  }));
}
