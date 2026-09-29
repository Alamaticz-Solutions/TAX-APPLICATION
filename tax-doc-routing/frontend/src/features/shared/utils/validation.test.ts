import { describe, expect, it } from 'vitest';
import { isEmail, validateDocument, validateDocumentName, type DocumentFormValues } from './validation';

const file = new File(['%PDF'], 'K1_2024.pdf', { type: 'application/pdf' });
const valid: DocumentFormValues = {
  type: 'K-1',
  year: '2024',
  name: 'K1_2024.pdf',
  file,
  belongsToEntity: 'no',
  entityType: '',
  entityName: ''
};

describe('validateDocumentName', () => {
  it('requires a name, a pdf extension and legal characters', () => {
    expect(validateDocumentName('')).toBe('Enter a document name.');
    expect(validateDocumentName('a/b.pdf')).toContain('cannot contain');
    expect(validateDocumentName('notes.docx')).toBe('Only PDF files (.pdf) are allowed.');
    expect(validateDocumentName('K1_2024.pdf')).toBeNull();
    expect(validateDocumentName('K1_2024')).toBeNull();
  });
});

describe('validateDocument (BR-06)', () => {
  it('accepts a complete personal document', () => {
    expect(validateDocument(valid)).toEqual({});
  });

  it('requires the core fields and the attachment', () => {
    const errors = validateDocument({ ...valid, type: '', year: '', name: '', file: null });
    expect(Object.keys(errors).sort()).toEqual(['file', 'name', 'type', 'year']);
  });

  it('requires entity type and name once an entity is chosen', () => {
    const errors = validateDocument({ ...valid, belongsToEntity: 'yes' });
    expect(errors.entityType).toBeDefined();
    expect(errors.entityName).toBeDefined();
    expect(validateDocument({ ...valid, belongsToEntity: 'yes', entityType: 'Partnership', entityName: 'Oakwood Medical Partners LLC' })).toEqual({});
  });
});

describe('isEmail', () => {
  it('checks basic email shape', () => {
    expect(isEmail('a@b.co')).toBe(true);
    expect(isEmail('nope')).toBe(false);
  });
});
