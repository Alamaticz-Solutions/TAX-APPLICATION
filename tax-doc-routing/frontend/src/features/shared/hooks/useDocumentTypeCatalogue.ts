import { useEffect, useState } from 'react';
import { fetchDocumentTypeCatalogue, fetchOptionList } from '../../../lib/formEngine';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { DocumentTypeConfig, DocumentTypeName, EntityType, Role } from '../types';

// Same role -> backend-user mapping as TaxRoutingProvider's permissions fetch (BACKEND_USER_NAME
// there isn't exported; duplicated here rather than adding a cross-module dependency for two
// lines — both must stay in step with .appfw/model/schemas/tax_routing/seeds/04_app_user.yaml).
const BACKEND_USER_NAME: Record<Role, string> = { standard: 'alex', admin: 'dana' };

type Catalogue = {
  documentTypes: DocumentTypeConfig[];
  entityTypes: EntityType[];
  documentYears: number[];
};

// Module-level cache: the catalogue is reference data (business spec 3.4), the same for every
// user of a given role, and changes only when an administrator edits it — not on every render.
let cache: Catalogue | null = null;
let inflight: Promise<Catalogue> | null = null;

async function load(userName: string): Promise<Catalogue> {
  if (cache) return cache;
  if (!inflight) {
    inflight = Promise.all([fetchDocumentTypeCatalogue(userName), fetchOptionList(userName, 'entity_type'), fetchOptionList(userName, 'document_year')])
      .then(([documentTypes, entityTypeValues, yearValues]) => {
        const resolved: Catalogue = {
          documentTypes,
          entityTypes: entityTypeValues.map((v) => v.code) as EntityType[],
          documentYears: yearValues.map((v) => Number(v.code)).sort((a, b) => b - a)
        };
        cache = resolved;
        return resolved;
      })
      .catch((err) => {
        inflight = null; // allow a retry on the next call rather than caching a failure forever
        throw err;
      });
  }
  return inflight;
}

/**
 * The document-type catalogue and its managed option lists (business spec 3.4, 5.5), read live
 * from the database (docs/architecture/dynamic-form-engine-design.md) instead of the hardcoded
 * arrays `features/shared/config/documentTypes.ts` used to export. `ready` is false only until
 * the first load for this session resolves; after that every caller shares the same cache.
 */
export function useDocumentTypeCatalogue() {
  const { role } = useTaxRouting();
  const [state, setState] = useState<Catalogue | null>(cache);
  const [error, setError] = useState(false);

  useEffect(() => {
    let alive = true;
    load(BACKEND_USER_NAME[role])
      .then((c) => {
        if (alive) setState(c);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [role]);

  const documentTypes = state?.documentTypes ?? [];
  const getDocumentType = (type: DocumentTypeName | string): DocumentTypeConfig | undefined => documentTypes.find((d) => d.type === type);

  return {
    ready: state !== null,
    error,
    documentTypes,
    entityTypes: state?.entityTypes ?? [],
    documentYears: state?.documentYears ?? [],
    getDocumentType
  };
}
