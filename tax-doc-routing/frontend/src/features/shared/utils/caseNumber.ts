// The human-facing case number (TD-1001, TD-1002, ...). The backend assigns the number and the
// UUID `id` stays the key behind routes and mutations; this is only how a record is labelled
// for people. The prefix lives here, in one place, so changing it is a one-line edit.
export const CASE_NUMBER_PREFIX = 'TD';

/**
 * The label to show for a record. Records created before case numbers existed are backfilled by
 * database/backfill/001_case_number.sql; until that has run (or for an exception raised on a
 * record this user may not open) there is no number, so fall back to the start of the id rather
 * than showing a blank.
 */
export function formatCaseNumber(caseNumber: number | null | undefined, id = ''): string {
  if (typeof caseNumber === 'number') return `${CASE_NUMBER_PREFIX}-${caseNumber}`;
  return id.slice(0, 8);
}

/** True when `term` (already lower-cased) matches either the label or the full id. */
export function matchesCase(term: string, c: { id: string; caseNumber: number | null }): boolean {
  return formatCaseNumber(c.caseNumber, c.id).toLowerCase().includes(term) || c.id.toLowerCase().includes(term);
}
