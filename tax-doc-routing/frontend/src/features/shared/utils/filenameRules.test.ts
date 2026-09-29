import { describe, expect, it } from 'vitest';
import { buildFileName } from './filenameRules';

const client = { firstName: 'John', lastName: 'Smith' };

describe('buildFileName (business spec 14.11, 6.4)', () => {
  it('builds a personal document name', () => {
    expect(buildFileName(client, { year: 2024, type: 'Tax Return', entity: null, passwordProtected: false })).toBe(
      'Smith, John - 2024 Tax Return (UNSECURED).pdf'
    );
  });

  it('includes the entity number and name for entity documents', () => {
    const entity = { type: 'Partnership' as const, name: 'Oakwood Medical Partners LLC', number: '001' };
    expect(buildFileName(client, { year: 2024, type: 'K-1', entity, passwordProtected: true })).toBe(
      'Smith, John - 001 Oakwood Medical Partners LLC - 2024 K-1 (SECURED).pdf'
    );
  });
});
