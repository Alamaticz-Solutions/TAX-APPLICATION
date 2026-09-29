import type { DocumentTypeConfig, DocumentTypeName, EntityType } from '../types';

// Config-driven document types (business spec 3.4, 5.5, BR-06..BR-13).
// UI, validation and routing all read from here; no per-type `if` chains elsewhere.
//   allowsEntity        - the "Does this belong to an Entity?" question is offered
//   k1Attachments       - K-1 attachment options (shown for Partnership entities, BR-07)
//   form8308Attachments - Form 8308 attachment options (BR-09)
//   externalOnly        - routes to the external folder only (BR-13)
const base = { allowsEntity: true, k1Attachments: false, form8308Attachments: false, supportsPasswordProtection: true, externalOnly: false };

export const DOCUMENT_TYPES: readonly DocumentTypeConfig[] = [
  { ...base, type: 'K-1', businessName: 'Schedule K-1', k1Attachments: true, externalFolder: 'K-1s', internalFolder: '4. Tax Documents', note: 'Batches of 3,000+ possible' },
  { ...base, type: 'Draft K-1', businessName: 'Draft Schedule K-1', k1Attachments: true, externalFolder: 'K-1s', internalFolder: '4. Tax Documents', note: 'Preliminary K-1' },
  { ...base, type: 'Tax Return', businessName: 'Tax Return (1040/1065/1120)', externalFolder: 'Tax Returns', internalFolder: '7. Tax Returns', note: 'Personal, partnership, or corporate' },
  { ...base, type: 'Income Statement', businessName: 'Income Statement', externalFolder: 'Loan Assistance', internalFolder: 'Loan Assistance', note: 'Financial statement for loans' },
  { ...base, type: 'Balance Sheet', businessName: 'Balance Sheet', externalFolder: 'Loan Assistance', internalFolder: 'Loan Assistance', note: 'Financial statement for loans' },
  { ...base, type: 'S-Corp Election', businessName: 'S-Corp Election', externalOnly: true, externalFolder: 'Signature Requests', internalFolder: null, note: 'Requires signature' },
  { ...base, type: 'S-Corp', businessName: 'S-Corporation', externalOnly: true, externalFolder: 'Signature Requests', internalFolder: null, note: 'Distinct from S-Corp Election (added Jan 2025)' },
  { ...base, type: 'Form 8308', businessName: 'Form 8308', form8308Attachments: true, externalOnly: true, externalFolder: 'K-1s', internalFolder: null, note: 'Partnership interest sale/exchange' },
  { ...base, type: 'Extensions', businessName: 'Tax Extension', externalOnly: true, externalFolder: 'Tax Returns', internalFolder: null, note: 'Filing extension documents' }
];

export const getDocumentType = (type: DocumentTypeName | string): DocumentTypeConfig | undefined =>
  DOCUMENT_TYPES.find((d) => d.type === type);

export const ENTITY_TYPES: readonly EntityType[] = ['Partnership', 'S-Corp', 'Personal Corporation'];
export const DOCUMENT_YEARS: readonly number[] = [2025, 2024, 2023, 2022, 2021, 2020];

export type AttachmentOption = { key: string; label: string };
export const K1_ATTACHMENT_OPTIONS: readonly AttachmentOption[] = [
  { key: 'k1Statement', label: 'K-1 Statement' },
  { key: 'k1Schedule', label: 'Supporting Schedule' }
];
export const FORM_8308_ATTACHMENT_OPTIONS: readonly AttachmentOption[] = [
  { key: 'form8308Attachment', label: 'Form 8308 attachment' }
];
