import { useTaxRouting } from '../state/TaxRoutingProvider';

// The in-progress routing record (client + documents) that survives moving between steps.
export function useRoutingRecord() {
  const { draft, draftApi, createDraftRecord, uploadFile, submitDraft, discardDraft } = useTaxRouting();
  return { draft, ...draftApi, createRecord: createDraftRecord, uploadFile, submit: submitDraft, discard: discardDraft, hasClient: draft.client !== null, hasDocuments: draft.documents.length > 0 };
}
