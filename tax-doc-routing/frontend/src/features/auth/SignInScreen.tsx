import { useState } from 'react';
import { Navigate, useLocation, useNavigate } from 'react-router';
import { AlertTriangle, FilePlus2, ListChecks, ShieldCheck } from 'lucide-react';
import { TextField } from '@appfw/pds-health-components/forms';
import { IconSlot, PdsHealthLogo } from '@appfw/pds-health-components/foundation';
import { Button } from '@appfw/pds-health-components/primitives';
import { FormLayout, InlineAlert, Surface } from '@appfw/pds-health-components/surfaces';
import { useSession } from '../../app/SessionProvider';
import { fetchMyPermissions } from '../../lib/permissions';
import { normalizeUserName } from '../../lib/session';

/**
 * Local sign-in (ENV_NAME=local), laid out like the Project Governance sign-in: a brand panel on
 * wide windows and the form beside it. Production identity is Okta/JWT through the backend; this
 * screen checks the entered user name against the real database-backed users (the same
 * `myPermissions` lookup the app uses everywhere) and only then starts the session. Role and
 * display name come from the database, so there is nothing else to pick here. It is the one public
 * route: every other route sits behind <RequireAuth>.
 */

const FEATURES = [
  { icon: FilePlus2, label: 'Guided requests: client, documents, processing' },
  { icon: ListChecks, label: 'Every case tracked from start to completion' },
  { icon: AlertTriangle, label: 'Exception tasks for anything that needs attention' },
  { icon: ShieldCheck, label: 'Role-based access for Tax Staff and Tax Admin' }
];

type Failure = 'unknown' | null;

export function SignInScreen() {
  const { userName: signedIn, signIn } = useSession();
  const navigate = useNavigate();
  const location = useLocation();
  const from = (location.state as { from?: string } | null)?.from ?? '/dashboard';

  const [userName, setUserName] = useState('');
  const [touched, setTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<Failure>(null);

  if (signedIn) return <Navigate to={from} replace />;

  const name = normalizeUserName(userName);
  const fieldError = touched && !name ? 'Enter your user name.' : undefined;

  const submit = async () => {
    setTouched(true);
    setFailure(null);
    if (!name) return;
    setBusy(true);
    const principal = await fetchMyPermissions(name);
    if (!principal.user_id) {
      setFailure('unknown');
      setBusy(false);
      return;
    }
    signIn(name);
    navigate(from, { replace: true });
  };

  return (
    <div className="tax-signin">
      {/* Brand panel: a hero, not content, so it is dropped below the large window class. */}
      <aside className="tax-signin-brand" aria-label="About this application">
        <div className="tax-signin-brand__identity">
          <PdsHealthLogo variant="wordmark" label="PDS Health" className="tax-signin-brand__logo" />
          <div>
            <p className="tax-signin-brand__title">TaxPro Enterprise Platform</p>
            <p className="tax-signin-brand__muted">Tax document routing</p>
          </div>
        </div>

        <div className="tax-signin-brand__content">
          <h2 className="tax-signin-brand__headline">Route every client document to the right place</h2>
          <p className="tax-signin-brand__muted">One workflow for collecting, filing and tracking tax documents for owner doctors.</p>
          <ul className="tax-signin-brand__features">
            {FEATURES.map((f) => (
              <li key={f.label}>
                <IconSlot size="md">
                  <f.icon aria-hidden="true" />
                </IconSlot>
                <span>{f.label}</span>
              </li>
            ))}
          </ul>
        </div>
      </aside>

      <main className="tax-signin-panel">
        <div className="tax-signin-card">
          <PdsHealthLogo variant="wordmark" label="PDS Health" className="tax-signin-mobile-logo" />
          <Surface variant="elevated">
            <div className="tax-signin-heading">
              <h1 className="tax-signin-title">Welcome back</h1>
              <p className="tax-signin-subtitle">Sign in to your tax workspace</p>
            </div>

            <form
              noValidate
              onSubmit={(event) => {
                event.preventDefault();
                void submit();
              }}
            >
              <FormLayout
                columns="one"
                footer={
                  <Button variant="primary" type="submit" isLoading={busy} className="tax-signin-submit">
                    Sign in
                  </Button>
                }
              >
                <TextField
                  label="User name"
                  required
                  autoComplete="username"
                  autoFocus
                  value={userName}
                  error={fieldError}
                  hint="Your assigned user name. Your role is read from your account."
                  onChange={(event) => {
                    setUserName(event.target.value);
                    setFailure(null);
                  }}
                />
                {failure === 'unknown' ? (
                  <InlineAlert
                    tone="danger"
                    title="We could not sign you in"
                    detail="That user name is not recognised, has no access, or the service could not be reached. Check it and try again."
                  />
                ) : null}
              </FormLayout>
            </form>
            <p className="tax-signin-footnote">Your access is checked again on every request.</p>
          </Surface>
        </div>
      </main>
    </div>
  );
}
