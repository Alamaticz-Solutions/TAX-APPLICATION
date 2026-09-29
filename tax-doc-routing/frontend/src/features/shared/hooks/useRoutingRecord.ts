import { useTaxRouting } from '../state/TaxRoutingProvider';

// The in-progress routing record (client + documents) that survives moving between steps.
export function useRoutingRecord() {
  const { draft, draftApi, submitDraft } = useTaxRouting();
  return { draft, ...draftApi, submit: submitDraft, hasClient: draft.client !== null, hasDocuments: draft.documents.length > 0 };
}
