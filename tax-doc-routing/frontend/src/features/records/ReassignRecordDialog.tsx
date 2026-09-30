import { useState } from 'react';
import { ConfirmDialog } from '@appfw/pds-health-components/layout';
import { InlineAlert } from '@appfw/pds-health-components/surfaces';
import { SingleSelectField } from '../../components/ui';
import { useStaffDirectory } from '../shared/hooks/useStaffDirectory';
import type { RoutingCase } from '../shared/types';
import { formatCaseNumber } from '../shared/utils/caseNumber';

/**
 * Admin-only reassignment (business spec 12.3). Real enforcement belongs to the backend's
 * reassign_record custom mutation; the control that opens this is hidden for other roles.
 */
export function ReassignRecordDialog({
  record,
  onClose,
  onConfirm
}: {
  record: RoutingCase;
  onClose: () => void;
  onConfirm: (newAssignedUserId: string) => Promise<void>;
}) {
  const { staff, staffName } = useStaffDirectory();
  const [assignee, setAssignee] = useState(record.assignedTo);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const options = staff.map((s) => ({ value: s.id, label: s.name }));

  const submit = async () => {
    if (!assignee || assignee === record.assignedTo) {
      onClose();
      return;
    }
    setBusy(true);
    try {
      await onConfirm(assignee);
    } catch {
      setError('Could not reassign this record. Try again.');
      setBusy(false);
    }
  };

  return (
    <ConfirmDialog
      open
      size="sm"
      title="Reassign Record"
      description={`Case ${formatCaseNumber(record.caseNumber)} is currently assigned to ${staffName(record.assignedTo)}.`}
      confirmLabel="Reassign"
      cancelLabel="Cancel"
      isConfirming={busy}
      onConfirm={submit}
      onCancel={onClose}
    >
      <div className="tax-stack">
        <SingleSelectField label="Assign to" required value={assignee} onValueChange={setAssignee} options={options} placeholder="Select a staff member" />
        {error ? <InlineAlert tone="danger" title={error} /> : null}
      </div>
    </ConfirmDialog>
  );
}
