import { describe, expect, it } from 'vitest';
import { formatCaseNumber, matchesCase } from './caseNumber';

describe('formatCaseNumber', () => {
  it('formats a number as TD-<number>', () => {
    expect(formatCaseNumber(1001)).toBe('TD-1001');
    expect(formatCaseNumber(12345)).toBe('TD-12345');
  });

  it('keeps the formatted label when the id is also supplied', () => {
    expect(formatCaseNumber(1042, '6f0a0001-0000-4a00-9000-000000000001')).toBe('TD-1042');
  });

  it('falls back to the start of the id when there is no number', () => {
    expect(formatCaseNumber(null, '6f0a0001-0000-4a00-9000-000000000001')).toBe('6f0a0001');
    expect(formatCaseNumber(undefined, 'abcdef1234')).toBe('abcdef12');
  });

  it('is empty rather than throwing when neither is available', () => {
    expect(formatCaseNumber(null)).toBe('');
  });
});

describe('matchesCase', () => {
  const record = { id: '6f0a0001-0000-4a00-9000-000000000001', caseNumber: 1001 };

  it('matches on the label, case-insensitively and on a partial', () => {
    expect(matchesCase('td-1001', record)).toBe(true);
    expect(matchesCase('1001', record)).toBe(true);
  });

  it('still matches on the full id so existing links and pasted ids keep working', () => {
    expect(matchesCase('6f0a0001-0000', record)).toBe(true);
  });

  it('does not match unrelated text', () => {
    expect(matchesCase('td-2000', record)).toBe(false);
  });
});
