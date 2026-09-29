export const pdsIxPresentationSchema: "pds.ix.presentation@1";

export * from "./recipes.js";

export type PdsIxPresentationIdentity = {
  schemaVersion: typeof pdsIxPresentationSchema;
  presentationId: string;
  revision: number;
};

export type EvidenceDisclosureItemModel = {
  id?: string;
  sourceRef?: string;
  label: string;
  value: string;
};

export type EvidenceDisclosureModel = {
  summary: string;
  items: readonly EvidenceDisclosureItemModel[];
};

export type ResolvedContextPresentationModel = {
  eyebrow: string;
  title: string;
  detail?: string;
  announcement: string;
  gaps: readonly string[];
  evidence: EvidenceDisclosureModel;
  metaLabel?: string;
  actionLabel?: string;
};

export type WorkStatusPresentationModel = {
  label: string;
  detail?: string;
  active: boolean;
  actionLabel?: string;
};

export type ProgressiveResponseRegionModel = {
  id: string;
  status: "partial" | "ready" | "stale" | "failed";
  label?: string;
  title: string;
  body: string;
  whyItMatters?: string;
  evidence?: EvidenceDisclosureModel;
  actionLabel?: string;
  editable?: boolean;
  changed?: boolean;
};

export type ProgressiveResponsePresentationModel = {
  eyebrow: string;
  title: string;
  metaLabel?: string;
  announcement: string;
  active: boolean;
  regions: readonly ProgressiveResponseRegionModel[];
  emptyState: string;
  footerText?: string;
};

export type PdsIxPresentationEnvelope = {
  identity: PdsIxPresentationIdentity;
  announcement: string;
  context?: ResolvedContextPresentationModel;
  workStatus?: WorkStatusPresentationModel;
  response: ProgressiveResponsePresentationModel;
};

export type PdsIxPresentationValidation =
  | { ok: true; value: PdsIxPresentationEnvelope; errors: readonly [] }
  | { ok: false; value: null; errors: readonly string[] };

export function validatePdsIxPresentation(value: unknown): PdsIxPresentationValidation;
export function assertPdsIxPresentation(value: unknown): PdsIxPresentationEnvelope;
