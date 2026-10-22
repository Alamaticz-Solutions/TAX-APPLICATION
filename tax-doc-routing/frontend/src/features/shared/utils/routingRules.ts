import { RELEASED_TO_CLIENT_FOLDER } from '../config/processingSteps';
import type { Destination, DocumentTypeConfig, DocumentTypeName, TaxDocument } from '../types';

// Where a document is written and whether each copy is encrypted
// (business spec 1.3 / 6.2 / 6.4, BR-11..BR-13). Every rule reads the document-type catalogue
// loaded from the database (useDocumentTypeCatalogue); nothing is hardcoded here.
export type DocumentTypeLookup = (type: DocumentTypeName | string) => DocumentTypeConfig | undefined;

// Each destination is the folder path below the client's Internal or Client folder, as Pega builds it:
//   Internal Quick View   <year>/<internal type folder>               e.g. 2025/4. Tax Documents
//   Internal Client Copy  <year>/6. Released to Client/<type folder>  e.g. 2025/6. Released to Client/K-1s
//   External Client Copy  <year>/<client type folder>                 e.g. 2025/K-1s
export function destinationsFor(
  doc: Pick<TaxDocument, 'type' | 'year' | 'passwordProtected'>,
  lookup: DocumentTypeLookup,
  releasedFolder: string = RELEASED_TO_CLIENT_FOLDER
): Destination[] {
  const cfg = lookup(doc.type);
  if (!cfg) return [];
  const list: Destination[] = [];
  if (!cfg.externalOnly) {
    // Internal Quick View is never encrypted.
    list.push({ key: 'quickView', label: 'Internal Quick View', folder: `${doc.year}/${cfg.internalFolder}`, encrypted: false });
  }
  list.push({ key: 'external', label: 'External Client Copy', folder: `${doc.year}/${cfg.externalFolder}`, encrypted: doc.passwordProtected });
  if (!cfg.externalOnly) {
    list.push({ key: 'internal', label: 'Internal Client Copy', folder: `${doc.year}/${releasedFolder}/${cfg.externalFolder}`, encrypted: doc.passwordProtected });
  }
  return list;
}

export const isExternalOnly = (type: DocumentTypeName, lookup: DocumentTypeLookup): boolean => Boolean(lookup(type)?.externalOnly);

export type AttachmentGroup = 'k1' | 'form8308';

// Which attachment groups the Add Document dialog should show for this selection.
// `k1EntityTypes` is the managed `k1_attachment_entity_type` list (BR-07).
export function attachmentGroups(
  type: DocumentTypeName | '',
  entityType: string,
  lookup: DocumentTypeLookup,
  k1EntityTypes: readonly string[]
): AttachmentGroup[] {
  const cfg = type ? lookup(type) : undefined;
  if (!cfg) return [];
  const groups: AttachmentGroup[] = [];
  // BR-07: K-1 attachment list appears for K-1 / Draft K-1 belonging to a listed entity type.
  if (cfg.k1Attachments && entityType !== '' && k1EntityTypes.includes(entityType)) groups.push('k1');
  // BR-09: Form 8308 attachment list.
  if (cfg.form8308Attachments) groups.push('form8308');
  return groups;
}
