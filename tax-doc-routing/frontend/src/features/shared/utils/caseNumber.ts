import type { RoutingCase } from '../types';

/** Case numbers are four digits (1000-9999): short enough to quote on a call. The record's uuid
 * stays the internal key; the database's UNIQUE constraint on `case_number` is the real guard. */
export const CASE_NUMBER_MIN = 1000;
export const CASE_NUMBER_MAX = 9999;

/** "1024", or a dash for a record created before case numbers existed. */
export const formatCaseNumber = (n: number | null | undefined): string => (n ? String(n) : '—');

/** Picks a random four-digit number that is not in `taken`. `taken` only holds what the caller
 * can see (a staff user sees their own records), so a clash with a record they cannot see is still
 * possible; the caller retries when the database refuses the number. Returns null when every
 * number in range is taken. */
export function pickCaseNumber(taken: ReadonlySet<number>, random: () => number = Math.random): number | null {
  const size = CASE_NUMBER_MAX - CASE_NUMBER_MIN + 1;
  if (taken.size >= size) return null;
  let candidate = CASE_NUMBER_MIN + Math.floor(random() * size);
  // Walk forward from the random start so a mostly-full range still terminates.
  for (let i = 0; i < size; i += 1) {
    if (!taken.has(candidate)) return candidate;
    candidate = candidate === CASE_NUMBER_MAX ? CASE_NUMBER_MIN : candidate + 1;
  }
  return null;
}

/** Search match on the case number ("1024" or a typed "#" before it) or the client's name. */
export function matchesCaseSearch(c: Pick<RoutingCase, 'caseNumber' | 'client'>, query: string): boolean {
  const term = query.trim().toLowerCase().replace(/^#/, '');
  if (!term) return true;
  return (c.caseNumber !== null && String(c.caseNumber).includes(term)) || c.client.fullName.toLowerCase().includes(term);
}
