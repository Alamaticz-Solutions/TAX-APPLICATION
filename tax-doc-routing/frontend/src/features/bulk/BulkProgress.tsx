import { useEffect, useState } from 'react';
import { ProcessProgress } from '@appfw/pds-health-components/process';
import { LoadingState } from '@appfw/pds-health-components/surfaces';
import { WorkingSpinner } from '../../components/WorkingSpinner';

/** After this long, the panel says the work is taking longer than usual instead of looking stuck. */
const SLOW_AFTER_SECONDS = 20;

function formatElapsed(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return m > 0 ? `${m} min ${s} s` : `${s} s`;
}

/**
 * Shown while the server is working on a bulk upload (checking the files, or creating the
 * requests). It says what is happening, how far it has got, and how long it has been running, and
 * after a while says so plainly when the work is slow, so a long operation is never mistaken for a
 * frozen page.
 */
export function BulkProgress({
  title,
  step,
  done,
  total,
  startedAt,
  note
}: {
  title: string;
  /** What the server is doing right now. */
  step: string;
  done: number;
  total: number;
  startedAt: number;
  note?: string;
}) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  const seconds = Math.max(0, Math.round((now - startedAt) / 1000));
  const slow = seconds >= SLOW_AFTER_SECONDS;

  return (
    <div role="status" aria-live="polite" className="tax-stack">
      <LoadingState
        title={
          <span className="tax-inline">
            <WorkingSpinner size={20} /> {title}
          </span>
        }
        detail={
          <>
            {step ? `${step}. ` : ''}Running for {formatElapsed(seconds)}.
            {slow ? ' This is taking longer than usual. Large uploads can take a few minutes; keep this page open.' : ''}
          </>
        }
        skeletonLines={0}
      />
      {total > 0 ? <ProcessProgress ariaLabel={title} label={step} detail={`${Math.min(done, total)} of ${total}`} currentStep={Math.min(done, total)} totalSteps={total} variant="bar" /> : null}
      {note ? <p className="tax-muted">{note}</p> : null}
    </div>
  );
}
