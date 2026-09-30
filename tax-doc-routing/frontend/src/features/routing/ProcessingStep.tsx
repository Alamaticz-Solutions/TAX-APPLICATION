import { useEffect } from 'react';
import { Link, useNavigate } from 'react-router';
import { Badge, Button } from '@appfw/pds-health-components/primitives';
import { ProcessProgress } from '@appfw/pds-health-components/process';
import { InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import { ProcessingTimeline } from '../../components/ProcessingTimeline';
import { ESTIMATED_MINUTES, PROCESSING_STEPS } from '../shared/config/processingSteps';
import { formatCaseNumber } from '../shared/utils/caseNumber';
import { STATUS } from '../shared/config/statuses';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import { progressPercent } from '../shared/utils/timeline';

/** Step 4: simulated processing. The state machine lives in the provider so it survives leaving this page. */
export function ProcessingStep({ recordId }: { recordId: string }) {
  const { getCase, retryException } = useTaxRouting();
  const navigate = useNavigate();
  const record = getCase(recordId);
  const completed = record?.status === STATUS.COMPLETED;

  useEffect(() => {
    if (!completed) return undefined;
    const timer = setTimeout(() => navigate('/new/completed', { replace: true }), 900);
    return () => clearTimeout(timer);
  }, [completed, navigate]);

  if (!record) return null;
  const pct = progressPercent(record);
  const current = PROCESSING_STEPS[Math.min(record.progress, PROCESSING_STEPS.length - 1)];
  const remaining = Math.max(1, Math.round(ESTIMATED_MINUTES * (1 - pct / 100)));

  return (
    <div className="tax-stack">
      <div>
        <h2 className="tax-section-title">Processing Tax Documents</h2>
        <p className="tax-muted">Case {formatCaseNumber(record.caseNumber)} · You can leave this page; processing continues and the record stays on your dashboard.</p>
      </div>

      {record.status === STATUS.EXCEPTION ? (
        <InlineAlert
          tone="danger"
          title="Processing needs attention"
          detail={record.exception?.detail}
          action={
            <Button size="sm" variant="secondary" onClick={() => retryException(record.id)}>
              Retry
            </Button>
          }
        />
      ) : null}

      <div className="tax-two-col tax-wide-left">
        <Surface title="Processing steps" density="compact">
          <ProcessingTimeline record={record} />
        </Surface>

        <div className="tax-stack">
          <Surface title="Overall progress" density="compact">
            <ProcessProgress
              ariaLabel="Overall processing progress"
              label={completed ? 'Finished' : current.active}
              detail={`${pct}%`}
              currentStep={completed ? PROCESSING_STEPS.length : record.progress}
              totalSteps={PROCESSING_STEPS.length}
            />
            <p className="tax-muted">
              Estimated time remaining: ~{completed ? 0 : remaining} min <span className="tax-subtle">(simulated)</span>
            </p>
          </Surface>
          <Surface title={`Documents (${record.documents.length})`} density="compact">
            <ul className="tax-plain-list">
              {record.documents.map((d) => (
                <li key={d.id} className="tax-spread">
                  <span>
                    {d.type} <span className="tax-subtle">{d.year}</span>
                  </span>
                  <Badge tone={completed ? 'success' : 'neutral'}>{completed ? 'Filed' : d.status}</Badge>
                </li>
              ))}
            </ul>
          </Surface>
          <Link to={`/records/${record.id}`}>Open record details →</Link>
        </div>
      </div>
    </div>
  );
}
