import { describe, expect, it } from 'vitest';
import type { DocumentTypeConfig } from '../types';
import { attachmentGroups, destinationsFor, isExternalOnly } from './routingRules';

// A stand-in for the catalogue the app loads from `document_type_routes`: the rules must work
// from whatever rows the database returns, so the tests supply their own.
const row = (type: string, over: Partial<DocumentTypeConfig> = {}): DocumentTypeConfig =>
  ({ type, businessName: type, allowsEntity: true, k1Attachments: false, form8308Attachments: false, supportsPasswordProtection: true, externalOnly: false, externalFolder: 'External', internalFolder: 'Internal', note: '', ...over }) as DocumentTypeConfig;
const CATALOGUE: DocumentTypeConfig[] = [
  row('K-1', { k1Attachments: true }),
  row('Draft K-1', { k1Attachments: true }),
  row('Tax Return'),
  row('S-Corp Election', { externalOnly: true, internalFolder: null }),
  row('S-Corp', { externalOnly: true, internalFolder: null }),
  row('Form 8308', { externalOnly: true, internalFolder: null, form8308Attachments: true }),
  row('Extensions', { externalOnly: true, internalFolder: null })
];
const lookup = (type: string) => CATALOGUE.find((d) => d.type === type);
const K1_ENTITY_TYPES = ['Partnership'];

describe('destinationsFor (business spec 1.3 / 6.4)', () => {
  it('writes three copies for a non-external-only type and never encrypts Quick View', () => {
    const dests = destinationsFor({ type: 'K-1', year: 2025, passwordProtected: true }, lookup);
    expect(dests.map((d) => d.key)).toEqual(['quickView', 'external', 'internal']);
    expect(dests.find((d) => d.key === 'quickView')?.encrypted).toBe(false);
    expect(dests.find((d) => d.key === 'external')?.encrypted).toBe(true);
    expect(dests.find((d) => d.key === 'internal')?.encrypted).toBe(true);
  });

  it('builds the Pega folder paths: year, then the type folder, with Released to Client inside the year', () => {
    const dests = destinationsFor({ type: 'K-1', year: 2025, passwordProtected: false }, (type) => (type === 'K-1' ? row('K-1', { internalFolder: '4. Tax Documents', externalFolder: 'K-1s' }) : undefined));
    expect(dests.map((d) => d.folder)).toEqual(['2025/4. Tax Documents', '2025/K-1s', '2025/6. Released to Client/K-1s']);
  });

  it('does not encrypt any copy when the document is not password protected', () => {
    const dests = destinationsFor({ type: 'Tax Return', year: 2025, passwordProtected: false }, lookup);
    expect(dests.every((d) => !d.encrypted)).toBe(true);
  });

  it('routes external-only types to the external folder only (BR-13)', () => {
    for (const type of ['S-Corp Election', 'S-Corp', 'Form 8308', 'Extensions'] as const) {
      const dests = destinationsFor({ type, year: 2025, passwordProtected: true }, lookup);
      expect(dests.map((d) => d.key)).toEqual(['external']);
      expect(isExternalOnly(type, lookup)).toBe(true);
    }
  });

  it('covers every configured document type', () => {
    for (const cfg of CATALOGUE) {
      expect(destinationsFor({ type: cfg.type, year: 2025, passwordProtected: false }, lookup).length).toBeGreaterThan(0);
    }
  });
});

describe('attachmentGroups (BR-07, BR-09)', () => {
  it('shows K-1 attachments only for K-1 types owned by a Partnership', () => {
    expect(attachmentGroups('K-1', 'Partnership', lookup, K1_ENTITY_TYPES)).toEqual(['k1']);
    expect(attachmentGroups('Draft K-1', 'Partnership', lookup, K1_ENTITY_TYPES)).toEqual(['k1']);
    expect(attachmentGroups('K-1', '', lookup, K1_ENTITY_TYPES)).toEqual([]);
    expect(attachmentGroups('K-1', 'S-Corp', lookup, K1_ENTITY_TYPES)).toEqual([]);
  });

  it('shows Form 8308 attachments for Form 8308 regardless of entity', () => {
    expect(attachmentGroups('Form 8308', '', lookup, K1_ENTITY_TYPES)).toEqual(['form8308']);
  });

  it('follows the managed entity-type list rather than a fixed name', () => {
    expect(attachmentGroups('K-1', 'S-Corp', lookup, ['Partnership', 'S-Corp'])).toEqual(['k1']);
    expect(attachmentGroups('K-1', 'Partnership', lookup, [])).toEqual([]);
  });

  it('shows nothing for an unknown type', () => {
    expect(destinationsFor({ type: 'Unknown' as never, year: 2025, passwordProtected: true }, lookup)).toEqual([]);
  });

  it('shows nothing for other types or no selection', () => {
    expect(attachmentGroups('Tax Return', 'Partnership', lookup, K1_ENTITY_TYPES)).toEqual([]);
    expect(attachmentGroups('', '', lookup, K1_ENTITY_TYPES)).toEqual([]);
  });
});
