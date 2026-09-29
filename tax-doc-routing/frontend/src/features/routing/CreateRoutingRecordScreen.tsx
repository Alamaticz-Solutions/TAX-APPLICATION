import { Navigate, useParams } from 'react-router';
import { PageHeader } from '@appfw/pds-health-components/layout';
import { ProcessStepper, type ProcessStepItem } from '@appfw/pds-health-components/process';
import { STATUS } from '../shared/config/statuses';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { ClientInfoStep } from './ClientInfoStep';
import { CompletedStep } from './CompletedStep';
import { ProcessingStep } from './ProcessingStep';
import { UploadDocumentsStep } from './UploadDocumentsStep';

// Screens 1 and 2 are the user's work; Processing and Completed follow automatically after Submit.
const FLOW_STEPS = [
  { id: 'client', label: 'Client Info' },
  { id: 'documents', label: 'Upload Documents' },
  { id: 'processing', label: 'Processing' },
  { id: 'completed', label: 'Completed' }
] as const;

/**
 * One route (/new/:step) so the draft (client + documents) survives moving between steps.
 * Guards stop users from deep-linking past steps whose data doesn't exist yet.
 */
export function CreateRoutingRecordScreen() {
  const { step = '' } = useParams();
  const { draft, getCase } = useTaxRouting();
  const index = FLOW_STEPS.findIndex((s) => s.id === step);

  if (index < 0) return <Navigate to="/new/client" replace />;
  // Once submitted the record is processing: don't allow going back and submitting a duplicate.
  if (draft.recordId && index < 2) return <Navigate to="/new/processing" replace />;
  if (index >= 1 && !draft.client) return <Navigate to="/new/client" replace />;
  if (index >= 2 && !draft.recordId) return <Navigate to="/new/documents" replace />;
  if (step === 'completed' && getCase(draft.recordId ?? '')?.status !== STATUS.COMPLETED) return <Navigate to="/new/processing" replace />;

  const steps: ProcessStepItem[] = FLOW_STEPS.map((s, i) => ({
    id: s.id,
    label: s.label,
    status: i < index || (step === 'completed' && i === index) ? 'complete' : i === index ? 'current' : 'upcoming'
  }));

  return (
    <>
      <PageHeader title="Create New Request" subtitle="Capture the client details, then upload documents for AI to read." />
      <div className="tax-stepper-wide">
        <ProcessStepper ariaLabel="New request progress" steps={steps} currentStepId={step === 'completed' ? undefined : step} />
      </div>
      {step === 'client' ? <ClientInfoStep /> : null}
      {step === 'documents' ? <UploadDocumentsStep /> : null}
      {step === 'processing' && draft.recordId ? <ProcessingStep recordId={draft.recordId} /> : null}
      {step === 'completed' && draft.recordId ? <CompletedStep recordId={draft.recordId} /> : null}
    </>
  );
}
