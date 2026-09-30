import { describe, expect, it } from 'vitest';
import { CASE_NUMBER_MAX, CASE_NUMBER_MIN, formatCaseNumber, matchesCaseSearch, pickCaseNumber } from './caseNumber';

describe('pickCaseNumber', () => {
  it('returns a four-digit number', () => {
    for (const r of [0, 0.5, 0.999999]) {
      const n = pickCaseNumber(new Set(), () => r)!;
      expect(n).toBeGreaterThanOrEqual(CASE_NUMBER_MIN);
      expect(n).toBeLessThanOrEqual(CASE_NUMBER_MAX);
    }
  });

  it('skips numbers already taken, wrapping at the top of the range', () => {
    expect(pickCaseNumber(new Set([1000]), () => 0)).toBe(1001);
    expect(pickCaseNumber(new Set([CASE_NUMBER_MAX]), () => 0.999999)).toBe(CASE_NUMBER_MIN);
  });

  it('returns null when the range is exhausted', () => {
    const all = new Set<number>();
    for (let n = CASE_NUMBER_MIN; n <= CASE_NUMBER_MAX; n += 1) all.add(n);
    expect(pickCaseNumber(all)).toBeNull();
  });
});

describe('formatCaseNumber', () => {
  it('shows the bare number, and dashes a missing one', () => {
    expect(formatCaseNumber(1024)).toBe('1024');
    expect(formatCaseNumber(null)).toBe('—');
  });
});

describe('matchesCaseSearch', () => {
  const c = { caseNumber: 1024, client: { fullName: 'John Smith' } } as Parameters<typeof matchesCaseSearch>[0];
  it('matches by number with or without a leading hash, or by client name', () => {
    expect(matchesCaseSearch(c, `#${1024}`)).toBe(true);
    expect(matchesCaseSearch(c, '102')).toBe(true);
    expect(matchesCaseSearch(c, 'smith')).toBe(true);
    expect(matchesCaseSearch(c, '9999')).toBe(false);
    expect(matchesCaseSearch(c, '')).toBe(true);
  });
});
