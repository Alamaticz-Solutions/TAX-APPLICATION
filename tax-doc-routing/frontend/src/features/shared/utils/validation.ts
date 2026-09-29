import type { DocumentTypeName, EntityType } from '../types';

// Client-side validation for the Add Document form (business spec 5.5a, BR-05..BR-09).
const ILLEGAL_NAME_CHARS = /[\\/:*?"<>|]/;

export function validateDocumentName(name: string): string | null {
  const value = name.trim();
  if (!value) return 'Enter a document name.';
  if (ILLEGAL_NAME_CHARS.test(value)) return 'Document name cannot contain \\ / : * ? " < > |';
  const dot = value.lastIndexOf('.');
  if (dot > 0 && value.slice(dot).toLowerCase() !== '.pdf') return 'Only PDF files (.pdf) are allowed.';
  return null;
}

export type DocumentFormValues = {
  type: DocumentTypeName | '';
  year: string;
  name: string;
  file: File | null;
  belongsToEntity: 'yes' | 'no';
  entityType: EntityType | '';
  entityName: string;
};

export type DocumentFormErrors = Partial<Record<'type' | 'year' | 'name' | 'file' | 'entityType' | 'entityName', string>>;

export function validateDocument(form: DocumentFormValues): DocumentFormErrors {
  const errors: DocumentFormErrors = {};
  if (!form.type) errors.type = 'Select a document type.';
  if (!form.year) errors.year = 'Select a document year.';
  const nameError = validateDocumentName(form.name);
  if (nameError) errors.name = nameError;
  if (!form.file) errors.file = 'Attach a PDF file.';
  if (form.belongsToEntity === 'yes') {
    if (!form.entityType) errors.entityType = 'Select an entity type.';
    if (!form.entityName) errors.entityName = 'Select an entity.';
  }
  return errors;
}

export const isEmail = (value: string): boolean => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
