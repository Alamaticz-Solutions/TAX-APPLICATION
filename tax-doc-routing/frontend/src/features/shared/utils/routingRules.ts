import { getDocumentType } from '../config/documentTypes';
import type { Destination, DocumentTypeName, EntityType, TaxDocument } from '../types';

// Where a document is written and whether each copy is encrypted
// (business spec 1.3 / 6.2 / 6.4, BR-11..BR-13).
export function destinationsFor(doc: Pick<TaxDocument, 'type' | 'passwordProtected'>): Destination[] {
  const cfg = getDocumentType(doc.type);
  if (!cfg) return [];
  const list: Destination[] = [];
  if (!cfg.externalOnly) {
    // Internal Quick View is never encrypted.
    list.push({ key: 'quickView', label: 'Internal Quick View', folder: cfg.internalFolder, encrypted: false });
  }
  list.push({ key: 'external', label: 'External Client Copy', folder: cfg.externalFolder, encrypted: doc.passwordProtected });
  if (!cfg.externalOnly) {
    list.push({ key: 'internal', label: 'Internal Client Copy', folder: cfg.internalFolder, encrypted: doc.passwordProtected });
  }
  return list;
}

export const isExternalOnly = (type: DocumentTypeName): boolean => Boolean(getDocumentType(type)?.externalOnly);

export type AttachmentGroup = 'k1' | 'form8308';

// Which attachment groups the Add Document dialog should show for this selection.
export function attachmentGroups(type: DocumentTypeName | '', entityType: EntityType | ''): AttachmentGroup[] {
  const cfg = type ? getDocumentType(type) : undefined;
  if (!cfg) return [];
  const groups: AttachmentGroup[] = [];
  // BR-07: K-1 attachment list appears for K-1 / Draft K-1 belonging to a Partnership.
  if (cfg.k1Attachments && entityType === 'Partnership') groups.push('k1');
  // BR-09: Form 8308 attachment list.
  if (cfg.form8308Attachments) groups.push('form8308');
  return groups;
}
