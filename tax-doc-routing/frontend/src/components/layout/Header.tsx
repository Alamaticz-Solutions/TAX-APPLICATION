import { useNavigate } from 'react-router';
import { Bell, LogOut } from 'lucide-react';
import { useAppearance, type AppearanceColorMode, type AppearanceVisualTheme } from '@appfw/pds-health-components/foundation';
import { IconButton, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { useSession } from '../../app/SessionProvider';
import { useTaxRouting } from '../../features/shared/state/TaxRoutingProvider';
import { WorkspaceCommandPalette } from './commandPalette';

/**
 * Top bar content, rendered into the PDS `AppShell`'s `topBar` slot: command palette,
 * exception notifications, log out, and the two PDS appearance controls (visual style and
 * colour mode, persisted by the PDS `AppearanceProvider` mounted in main.tsx).
 */
export function Header() {
  const navigate = useNavigate();
  const { exceptionQueue, draftApi } = useTaxRouting();
  const { signOut } = useSession();
  const { colorMode, setColorMode, visualTheme, setVisualTheme } = useAppearance();
  const count = exceptionQueue.length;

  return (
    <>
      <WorkspaceCommandPalette />

      <IconButton
        icon={<Bell size={20} aria-hidden="true" />}
        ariaLabel="Exception notifications"
        tooltip={count > 0 ? `${count} exception${count === 1 ? '' : 's'} need attention` : 'No exceptions'}
        variant="quiet"
        onClick={() => navigate('/exceptions')}
      />

      <div className="tax-topbar-controls" role="group" aria-label="Appearance">
        <SegmentedControl<AppearanceVisualTheme>
          ariaLabel="Visual style"
          size="sm"
          value={visualTheme}
          onValueChange={setVisualTheme}
          options={[
            { value: 'apple-like', label: 'Apple-like' },
            { value: 'material-like', label: 'Material-like' }
          ]}
        />
        <SegmentedControl<AppearanceColorMode>
          ariaLabel="Colour mode"
          size="sm"
          value={colorMode}
          onValueChange={setColorMode}
          options={[
            { value: 'system', label: 'System' },
            { value: 'light', label: 'Light' },
            { value: 'dark', label: 'Dark' }
          ]}
        />
      </div>

      <IconButton
        icon={<LogOut size={20} aria-hidden="true" />}
        ariaLabel="Log out"
        tooltip="Log out"
        variant="quiet"
        onClick={() => {
          // A signed-out browser must not reopen the previous user's half-finished request.
          draftApi.reset();
          signOut();
          navigate('/sign-in', { replace: true });
        }}
      />
    </>
  );
}
