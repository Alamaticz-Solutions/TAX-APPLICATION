import { useEffect, useState } from 'react';
import { fetchDocumentTypeCatalogue, fetchOptionList } from '../../../lib/formEngine';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { DocumentTypeConfig, DocumentTypeName, EntityType } from '../types';


export type AttachmentOption = { key: string; label: string };

type Catalogue = {
  documentTypes: DocumentTypeConfig[];
  entityTypes: EntityType[];
  documentYears: number[];
  k1Attachments: AttachmentOption[];
  k1AttachmentEntityTypes: string[];
  form8308Attachments: AttachmentOption[];
};

// Module-level cache: the catalogue is reference data (business spec 3.4), the same for every
// user of a given role, and changes only when an administrator edits it — not on every render.
let cache: Catalogue | null = null;
let inflight: Promise<Catalogue> | null = null;

async function load(userName: string): Promise<Catalogue> {
  if (cache) return cache;
  if (!inflight) {
    inflight = Promise.all([fetchDocumentTypeCatalogue(userName), fetchOptionList(userName, 'entity_type'), fetchOptionList(userName, 'document_year'),
      fetchOptionList(userName, 'k1_attachment'),
      fetchOptionList(userName, 'k1_attachment_entity_type'),
      fetchOptionList(userName, 'form_8308_attachment')
    ])
      .then(([documentTypes, entityTypeValues, yearValues, k1Values, k1EntityValues, form8308Values]) => {
        const resolved: Catalogue = {
          documentTypes,
          entityTypes: entityTypeValues.map((v) => v.code) as EntityType[],
          documentYears: yearValues.map((v) => Number(v.code)).sort((a, b) => b - a),
          k1Attachments: k1Values.map((v) => ({ key: v.code, label: v.label })),
          k1AttachmentEntityTypes: k1EntityValues.map((v) => v.code),
          form8308Attachments: form8308Values.map((v) => ({ key: v.code, label: v.label }))
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
 * static arrays that used to live in features/shared/config/documentTypes.ts (now removed). `ready` is false only until
 * the first load for this session resolves; after that every caller shares the same cache.
 */
export function useDocumentTypeCatalogue() {
  const { userName } = useTaxRouting();
  const [state, setState] = useState<Catalogue | null>(cache);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!userName) return undefined;
    let alive = true;
    load(userName)
      .then((c) => {
        if (alive) setState(c);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [userName]);

  const documentTypes = state?.documentTypes ?? [];
  const getDocumentType = (type: DocumentTypeName | string): DocumentTypeConfig | undefined => documentTypes.find((d) => d.type === type);

  return {
    ready: state !== null,
    error,
    documentTypes,
    entityTypes: state?.entityTypes ?? [],
    documentYears: state?.documentYears ?? [],
    k1Attachments: state?.k1Attachments ?? [],
    k1AttachmentEntityTypes: state?.k1AttachmentEntityTypes ?? [],
    form8308Attachments: state?.form8308Attachments ?? [],
    getDocumentType
  };
}
