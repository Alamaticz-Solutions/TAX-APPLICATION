import { findEntity } from '../data/entities';
import type { AiSuggestion, DocumentTypeName } from '../types';
import { delay } from './delay';

// Frontend-only simulation of document classification. Output is a *suggestion*: the user
// must press "Apply Suggestions" and can still change every field afterwards.
const RULES: { test: RegExp; type: DocumentTypeName; entity: boolean }[] = [
  { test: /draft.?k-?1/, type: 'Draft K-1', entity: true },
  { test: /k-?1/, type: 'K-1', entity: true },
  { test: /8308/, type: 'Form 8308', entity: true },
  { test: /s-?corp.?election/, type: 'S-Corp Election', entity: false },
  { test: /s-?corp/, type: 'S-Corp', entity: false },
  { test: /extension/, type: 'Extensions', entity: false },
  { test: /(return|1040|1065|1120)/, type: 'Tax Return', entity: false },
  { test: /income/, type: 'Income Statement', entity: false },
  { test: /balance/, type: 'Balance Sheet', entity: false }
];

export const mockAIService = {
  async analyzeDocument(fileName: string): Promise<AiSuggestion> {
    await delay(1300);
    const name = fileName.toLowerCase();
    const rule = RULES.find((r) => r.test.test(name));
    const yearMatch = name.match(/20(2[0-5])/);
    const year = yearMatch ? Number(yearMatch[0]) : 2024;
    if (!rule) return { documentType: null, year, entity: null, confidence: 41 };
    const entity = rule.entity ? findEntity('Oakwood Medical Partners LLC') ?? null : null;
    return { documentType: rule.type, year, entity, confidence: rule.entity ? 96 : 88 };
  }
};
