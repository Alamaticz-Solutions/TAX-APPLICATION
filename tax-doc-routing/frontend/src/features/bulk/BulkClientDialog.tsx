import { useState } from 'react';
import { TextField } from '@appfw/pds-health-components/forms';
import { Dialog } from '@appfw/pds-health-components/layout';
import { Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { FieldGroup, FormLayout, InlineAlert } from '@appfw/pds-health-components/surfaces';
import type { BulkClientValues, BulkItem } from '../../lib/bulkRequests';

/** Edits one client's details for this bulk upload (the client directory is not changed). */
export function BulkClientDialog({ item, onSave, onClose }: { item: BulkItem; onSave: (values: BulkClientValues) => void; onClose: () => void }) {
  const c = item.client;
  const [form, setForm] = useState<BulkClientValues>({
    firstName: c.first_name,
    lastName: c.last_name,
    fullName: c.full_name,
    personalEmail: c.personal_email,
    pdsEmail: c.pds_email,
    additionalEmail: c.additional_email,
    officeLocation: c.office_location,
    internalFolder: c.internal_folder,
    clientFolder: c.client_folder,
    password: '',
    notify: c.notify
  });
  const [error, setError] = useState('');
  const set = (patch: Partial<BulkClientValues>) => setForm((f) => ({ ...f, ...patch }));

  const save = () => {
    if (!form.firstName.trim() || !form.lastName.trim() || !form.internalFolder.trim() || !form.clientFolder.trim()) {
      setError('First Name, Last Name, Internal Folder ID and Client Folder ID are required.');
      return;
    }
    onSave(form);
  };

  return (
    <Dialog
      open
      size="md"
      title={`Edit Client ${item.client_key}`}
      onClose={onClose}
      footer={
        <>
          <Button variant="quiet" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={save}>
            Save Changes
          </Button>
        </>
      }
    >
      <FormLayout columns="one">
        <TextField label="Client Name" value={form.fullName} onChange={(e) => set({ fullName: e.target.value })} />
        <TextField label="First Name" required value={form.firstName} onChange={(e) => set({ firstName: e.target.value })} />
        <TextField label="Last Name" required value={form.lastName} onChange={(e) => set({ lastName: e.target.value })} />
        <TextField label="Client Personal Email" value={form.personalEmail} onChange={(e) => set({ personalEmail: e.target.value })} />
        <TextField label="Client PDS Email" value={form.pdsEmail} onChange={(e) => set({ pdsEmail: e.target.value })} />
        <TextField label="Additional Email" value={form.additionalEmail} onChange={(e) => set({ additionalEmail: e.target.value })} />
        <TextField label="Office Location" value={form.officeLocation} onChange={(e) => set({ officeLocation: e.target.value })} />
        <TextField label="Internal Folder ID" required value={form.internalFolder} onChange={(e) => set({ internalFolder: e.target.value })} />
        <TextField label="Client Folder ID" required value={form.clientFolder} onChange={(e) => set({ clientFolder: e.target.value })} />
        <TextField
          label="Read Write Password"
          type="password"
          autoComplete="new-password"
          value={form.password}
          hint={c.password_on_file ? 'A password is on file. Leave blank to keep it.' : 'No password on file. Documents cannot be password protected without one.'}
          onChange={(e) => set({ password: e.target.value })}
        />
        <FieldGroup legend="Send Notification?" description="Emails the client when routing completes.">
          <SegmentedControl<'yes' | 'no'>
            ariaLabel="Send notification to client"
            value={form.notify ? 'yes' : 'no'}
            onValueChange={(v) => set({ notify: v === 'yes' })}
            options={[
              { value: 'yes', label: 'Yes' },
              { value: 'no', label: 'No' }
            ]}
          />
        </FieldGroup>
      </FormLayout>
      {error ? <InlineAlert tone="danger" title="Check the details" detail={error} /> : null}
    </Dialog>
  );
}
