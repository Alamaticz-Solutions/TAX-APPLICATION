import { useState } from 'react';
import { useNavigate } from 'react-router';
import { ArrowRight } from 'lucide-react';
import { WorkingSpinner } from '../../components/WorkingSpinner';
import { TextField } from '@appfw/pds-health-components/forms';
import { Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { FieldGroup, FormLayout, InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import { useClientDirectory } from '../shared/hooks/useClientDirectory';
import { useRoutingRecord } from '../shared/hooks/useRoutingRecord';
import { isEmail } from '../shared/utils/validation';
import { ClientSearch } from './ClientSearch';

/**
 * Screen 1 of "Create New Request". Everything the request needs before documents are uploaded:
 * the client (looked up from the reference list, so its fields are read-only), the contact
 * emails, and the routing folders. Additional Email and Send Notification are the only fields the
 * user types into (business spec 5.4).
 */
export function ClientInfoStep() {
  const navigate = useNavigate();
  const { draft, setClient, setAdditionalEmail, setNotifyClient, createRecord, discard } = useRoutingRecord();
  const { clients } = useClientDirectory();
  const [emailError, setEmailError] = useState('');
  const [missingClient, setMissingClient] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState('');
  const client = draft.client;
  // Once the request exists (it gets its number and storage folder), its client can no longer change.
  const locked = draft.recordId !== null;

  const next = async () => {
    if (!client) {
      setMissingClient(true);
      return;
    }
    if (draft.additionalEmail && !isEmail(draft.additionalEmail)) {
      setEmailError('Enter a valid email address.');
      return;
    }
    setCreateError('');
    setCreating(true);
    try {
      // Creates the request: its TD number and its temporary storage folder, before any upload.
      await createRecord();
      navigate('/new/documents');
    } catch (err) {
      setCreateError(err instanceof Error && err.message ? err.message : 'The request could not be created.');
      setCreating(false);
    }
  };

  return (
    <div className="tax-stack">
      <Surface
        title="Client Info"
        subtitle="Choose the client for this request. Their details fill in from the reference database."
        density="compact"
        actions={
          client ? (
            locked ? undefined : (
              <Button size="sm" variant="quiet" onClick={() => setClient(null)}>
                Change client
              </Button>
            )
          ) : (
            <Button
              size="sm"
              variant="secondary"
              disabled={clients.length === 0}
              onClick={() => {
                if (clients[0]) {
                  setClient(clients[0]);
                  setMissingClient(false);
                }
              }}
            >
              Use sample client
            </Button>
          )
        }
      >
        {missingClient && !client ? (
          <InlineAlert tone="warning" title="Select a client to continue" detail="Search by name or ID in Client Name." />
        ) : null}
        {createError ? <InlineAlert tone="danger" title="The request could not be created" detail={createError} /> : null}
        {locked ? (
          <InlineAlert
            tone="neutral"
            title="This request has been created"
            detail="Its client can no longer be changed. Cancel the request to start over with a different client."
          />
        ) : null}
        <FormLayout columns="auto" className="tax-form-sections">
          <FieldGroup legend="Client Profile">
            <FormLayout columns="one">
            {locked ? (
              <TextField label="Client Name" value={client?.fullName ?? ''} readOnly />
            ) : (
              <ClientSearch
                selected={client}
                onSelect={(c) => {
                  setClient(c);
                  setEmailError('');
                  if (c) setMissingClient(false);
                }}
              />
            )}
            <TextField label="First Name" value={client?.firstName ?? ''} readOnly />
            <TextField label="Last Name" value={client?.lastName ?? ''} readOnly />
            </FormLayout>
          </FieldGroup>

          <FieldGroup legend="Contact Emails">
            <FormLayout columns="one">
            <TextField label="Client Personal Email" type="email" value={client?.personalEmail ?? ''} readOnly />
            <TextField label="Client PDS Email" type="email" value={client?.pdsEmail ?? ''} readOnly />
            <TextField
              label="Additional Email"
              type="email"
              value={draft.additionalEmail}
              readOnly={locked}
              error={emailError}
              onChange={(e) => {
                setAdditionalEmail(e.target.value);
                setEmailError('');
              }}
            />
            </FormLayout>
          </FieldGroup>

          <FieldGroup legend="Intelligence Routing">
            <FormLayout columns="one">
            <TextField label="Internal Folder ID" value={client?.internalFolder ?? ''} readOnly />
            <TextField label="Client Folder ID" value={client?.folderName ?? ''} readOnly />
            <TextField
              label="Read Write Password"
              type="password"
              value={client ? 'â€¢â€¢â€¢â€¢â€¢â€¢â€¢â€¢â€¢â€¢' : ''}
              readOnly
              hint={client ? 'Password on file. It is never displayed and only protects PDFs when required.' : undefined}
            />
            <FieldGroup legend="Send Notification?" description="Emails the client when routing completes.">
              <SegmentedControl<'yes' | 'no'>
                ariaLabel="Send notification to client"
                disabled={locked}
                value={draft.notifyClient ? 'yes' : 'no'}
                onValueChange={(v) => setNotifyClient(v === 'yes')}
                options={[
                  { value: 'yes', label: 'Yes' },
                  { value: 'no', label: 'No' }
                ]}
              />
            </FieldGroup>
            </FormLayout>
          </FieldGroup>
        </FormLayout>
      </Surface>

      <div className="tax-actions tax-spread">
        <Button
          variant="quiet"
          disabled={creating}
          onClick={() => {
            // Cancelling after the request exists removes it and its temporary folder.
            void discard().then(() => navigate('/dashboard'));
          }}
        >
          Cancel
        </Button>
        <Button variant="primary" disabled={creating} onClick={() => void next()}>
          {creating ? (
            <>
              <WorkingSpinner /> Creating Request…
            </>
          ) : (
            <>
              {locked ? 'Continue' : 'Create Request'} <ArrowRight size={16} aria-hidden="true" />
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
