import type { Client, TaxDocument } from '../types';

// The name written at every destination folder: "Last, First - OriginalFileName.pdf" (the Pega
// convention the client confirmed). The file keeps the name it was uploaded with in the temporary
// folder; only the permanent copies are renamed. The backend applies the same rule
// (services/document_routing.rs: target_file_name), so this preview matches what is filed.
const FORBIDDEN = '/\\:*?"<>|';
const clean = (value: string) =>
  Array.from(value)
    .map((c) => (c.charCodeAt(0) < 32 || FORBIDDEN.includes(c) ? '_' : c))
    .join('')
    .trim()
    .replace(/\.+$/, '')
    .trim();

export function buildFileName(client: Pick<Client, 'firstName' | 'lastName'> & { fullName?: string }, doc: Pick<TaxDocument, 'fileName'>): string {
  const last = clean(client.lastName ?? '');
  const first = clean(client.firstName ?? '');
  const who = last && first ? `${last}, ${first}` : clean(client.fullName ?? '') || 'Client';
  return `${who} - ${clean(doc.fileName)}`;
}
