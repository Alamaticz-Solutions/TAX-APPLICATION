import type { Client, TaxDocument } from '../types';

// Destination filename convention (business spec 14.11 and 6.4).
//   Personal: "Last, First - {Year} {DocType}"
//   Entity:   "Last, First - {EntityNumber} {EntityName} - {Year} {DocType}"
// The filename is the only metadata stored with the file, so keep this in one place.
export function buildFileName(
  client: Pick<Client, 'firstName' | 'lastName'>,
  doc: Pick<TaxDocument, 'year' | 'type' | 'entity' | 'passwordProtected'>
): string {
  const person = `${client.lastName}, ${client.firstName}`;
  const tail = `${doc.year} ${doc.type}`;
  const core = doc.entity ? `${person} - ${doc.entity.number} ${doc.entity.name} - ${tail}` : `${person} - ${tail}`;
  return `${core} ${doc.passwordProtected ? '(SECURED)' : '(UNSECURED)'}.pdf`;
}
