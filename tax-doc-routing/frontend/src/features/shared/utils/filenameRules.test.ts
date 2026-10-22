import { describe, expect, it } from 'vitest';
import { buildFileName } from './filenameRules';

const client = { firstName: 'Andrea', lastName: 'Moore', fullName: 'Andrea Moore, DDS INC.' };

describe('buildFileName ("Last, First - OriginalFileName.pdf")', () => {
  it('prefixes the uploaded name with the client name', () => {
    expect(buildFileName(client, { fileName: 'Pega AI.pdf' })).toBe('Moore, Andrea - Pega AI.pdf');
  });

  it('falls back to the full name when a part is missing', () => {
    expect(buildFileName({ firstName: '', lastName: 'Moore', fullName: 'Andrea Moore' }, { fileName: 'a.pdf' })).toBe('Andrea Moore - a.pdf');
    expect(buildFileName({ firstName: '', lastName: '' }, { fileName: 'a.pdf' })).toBe('Client - a.pdf');
  });

  it('replaces characters a storage folder will not accept', () => {
    expect(buildFileName({ firstName: 'Pat', lastName: "O/Brien" }, { fileName: 'K1: 2025?.pdf' })).toBe('O_Brien, Pat - K1_ 2025_.pdf');
  });
});
