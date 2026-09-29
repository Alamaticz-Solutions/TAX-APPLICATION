import { describe, expect, it } from 'vitest';
import { DOCUMENT_TYPES } from '../config/documentTypes';
import { attachmentGroups, destinationsFor, isExternalOnly } from './routingRules';

describe('destinationsFor (business spec 1.3 / 6.4)', () => {
  it('writes three copies for a non-external-only type and never encrypts Quick View', () => {
    const dests = destinationsFor({ type: 'K-1', passwordProtected: true });
    expect(dests.map((d) => d.key)).toEqual(['quickView', 'external', 'internal']);
    expect(dests.find((d) => d.key === 'quickView')?.encrypted).toBe(false);
    expect(dests.find((d) => d.key === 'external')?.encrypted).toBe(true);
    expect(dests.find((d) => d.key === 'internal')?.encrypted).toBe(true);
  });

  it('does not encrypt any copy when the document is not password protected', () => {
    const dests = destinationsFor({ type: 'Tax Return', passwordProtected: false });
    expect(dests.every((d) => !d.encrypted)).toBe(true);
  });

  it('routes external-only types to the external folder only (BR-13)', () => {
    for (const type of ['S-Corp Election', 'S-Corp', 'Form 8308', 'Extensions'] as const) {
      const dests = destinationsFor({ type, passwordProtected: true });
      expect(dests.map((d) => d.key)).toEqual(['external']);
      expect(isExternalOnly(type)).toBe(true);
    }
  });

  it('covers every configured document type', () => {
    for (const cfg of DOCUMENT_TYPES) {
      expect(destinationsFor({ type: cfg.type, passwordProtected: false }).length).toBeGreaterThan(0);
    }
  });
});

describe('attachmentGroups (BR-07, BR-09)', () => {
  it('shows K-1 attachments only for K-1 types owned by a Partnership', () => {
    expect(attachmentGroups('K-1', 'Partnership')).toEqual(['k1']);
    expect(attachmentGroups('Draft K-1', 'Partnership')).toEqual(['k1']);
    expect(attachmentGroups('K-1', '')).toEqual([]);
    expect(attachmentGroups('K-1', 'S-Corp')).toEqual([]);
  });

  it('shows Form 8308 attachments for Form 8308 regardless of entity', () => {
    expect(attachmentGroups('Form 8308', '')).toEqual(['form8308']);
  });

  it('shows nothing for other types or no selection', () => {
    expect(attachmentGroups('Tax Return', 'Partnership')).toEqual([]);
    expect(attachmentGroups('', '')).toEqual([]);
  });
});
