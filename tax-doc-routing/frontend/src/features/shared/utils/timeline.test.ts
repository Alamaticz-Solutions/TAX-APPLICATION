import { describe, expect, it } from 'vitest';
import { PROCESSING_STEPS } from '../config/processingSteps';
import { STATUS } from '../config/statuses';
import { buildTimeline, progressPercent } from './timeline';

describe('buildTimeline', () => {
  it('marks completed steps done and the running step current', () => {
    const steps = buildTimeline({ progress: 4, status: STATUS.PROCESSING });
    expect(steps.slice(0, 4).every((s) => s.state === 'done')).toBe(true);
    expect(steps[4].state).toBe('current');
    expect(steps[5].state).toBe('pending');
  });

  it('marks the failing step as failed for exceptions', () => {
    const steps = buildTimeline({ progress: 3, status: STATUS.EXCEPTION });
    expect(steps[3].state).toBe('failed');
  });

  it('shows everything done when completed, and skipped work when cancelled', () => {
    expect(buildTimeline({ progress: 9, status: STATUS.COMPLETED }).every((s) => s.state === 'done')).toBe(true);
    const cancelled = buildTimeline({ progress: 2, status: STATUS.CANCELLED });
    expect(cancelled[1].state).toBe('done');
    expect(cancelled[2].state).toBe('skipped');
  });
});

describe('progressPercent', () => {
  it('is 100 when completed and proportional otherwise', () => {
    expect(progressPercent({ progress: 0, status: STATUS.COMPLETED })).toBe(100);
    expect(progressPercent({ progress: PROCESSING_STEPS.length / 3, status: STATUS.PROCESSING })).toBe(33);
  });
});
