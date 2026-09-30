import { useState } from 'react';
import { useNavigate } from 'react-router';
import { ArrowRight } from 'lucide-react';
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
  const { draft, setClient, setAdditionalEmail, setNotifyClient, reset } = useRoutingRecord();
  const { clients } = useClientDirectory();
  const [emailError, setEmailError] = useState('');
  const [missingClient, setMissingClient] = useState(false);
  const client = draft.client;

  const next = () => {
    if (!client) {
      setMissingClient(true);
      return;
    }
    if (draft.additionalEmail && !isEmail(draft.additionalEmail)) {
      setEmailError('Enter a valid email address.');
      return;
    }
    navigate('/new/documents');
  };

  return (
    <div className="tax-stack">
      <Surface
        title="Client Info"
        subtitle="Choose the client for this request. Their details fill in from the reference database."
        density="compact"
        actions={
          client ? (
            <Button size="sm" variant="quiet" onClick={() => setClient(null)}>
              Change client
            </Button>
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
        <FormLayout columns="auto" className="tax-form-sections">
          <FieldGroup legend="Client Profile">
            <FormLayout columns="one">
              <ClientSearch
                selected={client}
                onSelect={(c) => {
                  setClient(c);
                  setEmailError('');
                  if (c) setMissingClient(false);
                }}
              />
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
              <TextField label="Internal Folder" value={client?.internalFolder ?? ''} readOnly />
              <TextField label="Client Folder" value={client?.folderName ?? ''} readOnly />
              <TextField
                label="Read Write Password"
                type="password"
                value={client ? '••••••••••' : ''}
                readOnly
                hint={client ? 'Password on file. It is never displayed and only protects PDFs when required.' : undefined}
              />
              <FieldGroup legend="Send Notification?" description="Emails the client when routing completes.">
                <SegmentedControl<'yes' | 'no'>
                  ariaLabel="Send notification to client"
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
          onClick={() => {
            reset();
            navigate('/dashboard');
          }}
        >
          Cancel
        </Button>
        <Button variant="primary" onClick={next}>
          Create Request <ArrowRight size={16} aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
