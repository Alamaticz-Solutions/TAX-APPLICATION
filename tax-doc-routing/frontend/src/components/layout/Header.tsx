import { useNavigate } from 'react-router';
import { Bell } from 'lucide-react';
import { useAppearance, type AppearanceColorMode, type AppearanceVisualTheme } from '@appfw/pds-health-components/foundation';
import { IconButton, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { useTaxRouting } from '../../features/shared/state/TaxRoutingProvider';
import type { Role } from '../../features/shared/types';
import { WorkspaceCommandPalette } from './commandPalette';

/**
 * Top bar content, rendered into the PDS `AppShell`'s `topBar` slot: command palette,
 * exception notifications, the prototype role switcher, and the two PDS appearance controls
 * (visual style and colour mode, persisted by the PDS `AppearanceProvider` mounted in main.tsx).
 */
export function Header() {
  const navigate = useNavigate();
  const { role, setRole, exceptionQueue } = useTaxRouting();
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

      <div className="tax-topbar-controls" role="group" aria-label="Prototype role">
        <SegmentedControl<Role>
          ariaLabel="Prototype role"
          size="sm"
          value={role}
          onValueChange={(next) => {
            setRole(next);
            navigate('/dashboard');
          }}
          options={[
            { value: 'standard', label: 'Tax Staff' },
            { value: 'admin', label: 'Tax Admin' }
          ]}
        />
      </div>

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
    </>
  );
}
