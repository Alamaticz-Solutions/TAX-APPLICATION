import type { TaxDocument } from '../types';
import { delay } from './delay';

export const mockDocumentService = {
  // Pretends to stage the PDF in the temporary folder.
  async uploadDocument(file: File): Promise<{ fileName: string; size: number }> {
    await delay(600);
    return { fileName: file.name, size: file.size };
  },

  // BR-05: same type + year + name (or same type + same file) already on this record.
  async checkDuplicate(
    existing: TaxDocument[],
    candidate: Pick<TaxDocument, 'type' | 'year' | 'name' | 'fileName'>
  ): Promise<TaxDocument | null> {
    await delay(250);
    return (
      existing.find(
        (d) =>
          d.type === candidate.type &&
          (d.fileName === candidate.fileName || (d.year === candidate.year && d.name === candidate.name))
      ) ?? null
    );
  }
};
