import type { DocEntity, TaxDocument } from '../types';

const OAKWOOD: DocEntity = { type: 'Partnership', name: 'Oakwood Medical Partners LLC', number: '001' };

// Prototype-only helper: the three documents from the design mockups, so the whole flow can be
// clicked through without uploading files.
export function sampleDraftDocuments(): TaxDocument[] {
  const base = { year: 2024, attachments: {}, status: 'Ready' as const };
  return [
    { ...base, id: 'doc-sample-k1', type: 'K-1', name: 'K1_2024.pdf', fileName: 'K1_2024.pdf', fileSize: 182000, entity: OAKWOOD, passwordProtected: true },
    { ...base, id: 'doc-sample-return', type: 'Tax Return', name: 'TaxReturn_2024.pdf', fileName: 'TaxReturn_2024.pdf', fileSize: 356000, entity: null, passwordProtected: false },
    { ...base, id: 'doc-sample-8308', type: 'Form 8308', name: '8308_2024.pdf', fileName: '8308_2024.pdf', fileSize: 94000, entity: OAKWOOD, passwordProtected: true }
  ];
}
