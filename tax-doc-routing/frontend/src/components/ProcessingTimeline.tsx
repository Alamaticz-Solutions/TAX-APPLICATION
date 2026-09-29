import { ProcessStepper, type ProcessStepItem, type ProcessStepStatus } from '@appfw/pds-health-components/process';
import type { ProcessingStepState, RoutingCase } from '../features/shared/types';
import { buildTimeline } from '../features/shared/utils/timeline';

const STATE_STATUS: Record<ProcessingStepState, ProcessStepStatus> = {
  done: 'complete',
  current: 'current',
  failed: 'blocked',
  pending: 'upcoming',
  skipped: 'upcoming'
};
const STATE_TEXT: Record<ProcessingStepState, string> = {
  done: 'Done',
  current: 'In progress',
  failed: 'Failed',
  pending: 'Pending',
  skipped: 'Not run'
};

/** Vertical processing timeline: the PDS ProcessStepper driven by the case's progress + status. */
export function ProcessingTimeline({ record }: { record: Pick<RoutingCase, 'progress' | 'status'> }) {
  const timeline = buildTimeline(record);
  const steps: ProcessStepItem[] = timeline.map((s) => ({
    id: s.key,
    label: s.label,
    metadata: STATE_TEXT[s.state],
    status: STATE_STATUS[s.state]
  }));
  const active = timeline.find((s) => s.state === 'current' || s.state === 'failed') ?? timeline.find((s) => s.state === 'pending');
  return <ProcessStepper ariaLabel="Processing steps" orientation="vertical" steps={steps} currentStepId={active?.key} density="compact" />;
}
