import { createAppfwClient } from './appfwClient';
import type { DocumentTypeConfig, DocumentTypeName } from '../features/shared/types';

// Reads the document-type catalogue and managed option lists live from the backend
// (docs/architecture/dynamic-form-engine-design.md) instead of the hardcoded arrays that used to
// live in features/shared/config/documentTypes.ts. Same local-auth mechanism as lib/permissions.ts.

export type OptionValue = { code: string; label: string };

function localAuthHeader(userName: string, tenantId = 't1'): string {
  return `Bearer appfw-local:user=${userName};tenant=${tenantId};roles=n/a`;
}

function client(userName: string) {
  return createAppfwClient({ authorization: localAuthHeader(userName), tenantId: 't1' });
}

type RawDocumentTypeRoute = {
  document_type: string | null;
  business_name: string | null;
  external_folder: string | null;
  internal_folder: string | null;
  note: string | null;
  allows_entity: boolean | null;
  requires_k1_attachments: boolean | null;
  requires_form_8308_attachments: boolean | null;
  supports_password_protection: boolean | null;
  is_external_only: boolean | null;
};

const DOCUMENT_TYPE_ROUTES_QUERY = `{
  queryDocumentTypeRoutes(limit: 100) {
    items {
      document_type
      business_name
      external_folder
      internal_folder
      note
      allows_entity
      requires_k1_attachments
      requires_form_8308_attachments
      supports_password_protection
      is_external_only
    }
  }
}`;

/** The document-type requirements table (business spec 3.4), admin-managed, not hardcoded. */
export async function fetchDocumentTypeCatalogue(userName: string): Promise<DocumentTypeConfig[]> {
  const result = await client(userName).graphql<{ queryDocumentTypeRoutes: { items: RawDocumentTypeRoute[] } }, Record<string, never>>({
    schemaName: 'tax_routing',
    operationName: 'query_document_type_routes',
    query: DOCUMENT_TYPE_ROUTES_QUERY,
    variables: {}
  });
  return result.data.queryDocumentTypeRoutes.items.map((row) => ({
    type: (row.document_type ?? '') as DocumentTypeName,
    businessName: row.business_name ?? '',
    allowsEntity: row.allows_entity ?? false,
    k1Attachments: row.requires_k1_attachments ?? false,
    form8308Attachments: row.requires_form_8308_attachments ?? false,
    supportsPasswordProtection: row.supports_password_protection ?? true,
    externalOnly: row.is_external_only ?? false,
    externalFolder: row.external_folder ?? '',
    internalFolder: row.internal_folder,
    note: row.note ?? ''
  }));
}

type RawOptionValue = { code: string | null; label: string | null; sort_order: number | null };

const OPTION_LIST_QUERY = `query($name: JSON) {
  queryFieldOptionLists(filter: $name, limit: 1) {
    items { id }
  }
}`;

const OPTION_VALUES_QUERY = `query($filter: JSON) {
  queryFieldOptionValues(filter: $filter, sort: { sort_order: "asc" }, limit: 200) {
    items { code label sort_order }
  }
}`;

/** A managed drop-down list (document_year, entity_type, ...), admin-editable, not hardcoded. */
export async function fetchOptionList(userName: string, listName: string): Promise<OptionValue[]> {
  const appfw = client(userName);
  const listResult = await appfw.graphql<{ queryFieldOptionLists: { items: { id: string }[] } }, { name: unknown }>({
    schemaName: 'tax_routing',
    operationName: 'query_field_option_lists',
    query: OPTION_LIST_QUERY,
    variables: { name: { name: { _eq: listName } } }
  });
  const listId = listResult.data.queryFieldOptionLists.items[0]?.id;
  if (!listId) return [];
  const valuesResult = await appfw.graphql<{ queryFieldOptionValues: { items: RawOptionValue[] } }, { filter: unknown }>({
    schemaName: 'tax_routing',
    operationName: 'query_field_option_values',
    query: OPTION_VALUES_QUERY,
    variables: { filter: { _and: [{ option_list_id: { _eq: listId } }, { is_active: { _eq: true } }] } }
  });
  return valuesResult.data.queryFieldOptionValues.items.map((v) => ({ code: v.code ?? '', label: v.label ?? v.code ?? '' }));
}
