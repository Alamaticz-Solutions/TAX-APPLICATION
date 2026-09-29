import { useState } from 'react';
import { TextArea } from '@appfw/pds-health-components/forms';
import { ConfirmDialog } from '@appfw/pds-health-components/layout';
import { InlineAlert } from '@appfw/pds-health-components/surfaces';
import { SingleSelectField } from '../../components/ui';
import { CANCEL_RESOLUTIONS } from '../shared/config/statuses';
import type { RoutingCase } from '../shared/types';

const RESOLUTION_OPTIONS = CANCEL_RESOLUTIONS.map((r) => ({ value: r, label: r }));

/**
 * Admin-only UI (business spec 9.1, BR-17/18). Real enforcement belongs to the backend; the
 * control that opens this is hidden for other roles. Comments are mandatory and are written to
 * the activity log.
 */
export function CancelRecordDialog({
  record,
  onClose,
  onConfirm
}: {
  record: RoutingCase;
  onClose: () => void;
  onConfirm: (resolution: string, comments: string) => void;
}) {
  const [resolution, setResolution] = useState<string>(CANCEL_RESOLUTIONS[0]);
  const [comments, setComments] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!comments.trim()) {
      setError('Enter a reason for closing this record.');
      return;
    }
    setBusy(true);
    await new Promise((resolve) => setTimeout(resolve, 500));
    onConfirm(resolution, comments.trim());
  };

  return (
    <ConfirmDialog
      open
      size="sm"
      tone="danger"
      title="Cancel / Withdraw Record"
      description={`Record ${record.id} will be permanently closed.`}
      confirmLabel="Confirm Closure"
      cancelLabel="Cancel"
      isConfirming={busy}
      onConfirm={submit}
      onCancel={onClose}
    >
      <div className="tax-stack">
        <SingleSelectField label="Resolution Status" required value={resolution} onValueChange={setResolution} options={RESOLUTION_OPTIONS} />
        <TextArea
          label="Comments"
          required
          value={comments}
          error={error}
          hint="Recorded permanently in the audit trail."
          onChange={(e) => {
            setComments(e.target.value);
            setError('');
          }}
        />
        <InlineAlert tone="danger" title="This action cannot be undone." detail="There is no reopen path; start a new record if the work must restart." />
      </div>
    </ConfirmDialog>
  );
}
