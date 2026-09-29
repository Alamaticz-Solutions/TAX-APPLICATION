import { PROCESSING_STEPS, type ProcessingStep } from '../config/processingSteps';
import { STATUS } from '../config/statuses';
import type { ProcessingStepState, RoutingCase } from '../types';

export type TimelineStep = ProcessingStep & { state: ProcessingStepState };

// Turns a case's `progress` (number of completed steps) and status into per-step states.
export function buildTimeline(c: Pick<RoutingCase, 'progress' | 'status'>): TimelineStep[] {
  const p = c.progress;
  return PROCESSING_STEPS.map((step, i) => {
    let state: ProcessingStepState = 'pending';
    if (c.status === STATUS.COMPLETED) state = 'done';
    else if (i < p) state = 'done';
    else if (c.status === STATUS.CANCELLED) state = 'skipped';
    else if (i === p && c.status === STATUS.EXCEPTION) state = 'failed';
    else if (i === p && c.status === STATUS.PROCESSING) state = 'current';
    return { ...step, state };
  });
}

export const progressPercent = (c: Pick<RoutingCase, 'progress' | 'status'>): number =>
  c.status === STATUS.COMPLETED ? 100 : Math.round((c.progress / PROCESSING_STEPS.length) * 100);
