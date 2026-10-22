import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { AlertTriangle, Briefcase, FilePlus2, FileText, LayoutDashboard, Search } from 'lucide-react';
import { CommandPalette, type CommandPaletteItem } from '@appfw/pds-health-components/layout';
import { useTaxRouting } from '../../features/shared/state/TaxRoutingProvider';
import { formatCaseNumber } from '../../features/shared/utils/caseNumber';

/**
 * Workspace command access: the shared PDS `CommandPalette`. Product owns the command labels,
 * routes and role visibility; PDS owns the shortcut, search and popover. Records are limited to
 * those the current role may open.
 */
export function WorkspaceCommandPalette() {
  const navigate = useNavigate();
  const { visibleCases, hasPermission } = useTaxRouting();

  const items = useMemo<CommandPaletteItem[]>(() => {
    const go = (to: string) => () => navigate(to);
    const icon = { size: 15, 'aria-hidden': true as const };
    const pages: CommandPaletteItem[] = [
      { id: 'page:dashboard', group: 'Pages', label: 'Dashboard', detail: 'Work I need to do', icon: <LayoutDashboard {...icon} />, onSelect: go('/dashboard') },
      { id: 'page:cases', group: 'Pages', label: hasPermission('routing_record.view_assignee') ? 'Request Management' : 'My Cases', icon: <Briefcase {...icon} />, onSelect: go('/cases') },
      { id: 'page:exceptions', group: 'Pages', label: 'Exception Task', detail: 'Records needing attention', icon: <AlertTriangle {...icon} />, onSelect: go('/exceptions') }
    ];
    const actions: CommandPaletteItem[] = [
      { id: 'action:new-record', group: 'Actions', label: 'Create new request', detail: 'Start a request and upload documents for a client', keywords: ['create', 'start'], icon: <FilePlus2 {...icon} />, onSelect: go('/new/client') },
      { id: 'action:bulk-request', group: 'Actions', label: 'Create bulk request', detail: 'Create requests for many clients from an Excel file and a ZIP', keywords: ['bulk', 'upload', 'excel', 'zip'], icon: <FilePlus2 {...icon} />, onSelect: go('/bulk') }
    ];
    const records: CommandPaletteItem[] = visibleCases.slice(0, 100).map((c) => ({
      id: `record:${c.id}`,
      group: 'Records',
      label: formatCaseNumber(c.caseNumber, c.id),
      detail: c.client.fullName,
      keywords: [c.id, c.client.id, c.client.fullName],
      icon: <FileText {...icon} />,
      onSelect: go(`/records/${c.id}`)
    }));
    return [...actions, ...pages, ...records];
  }, [navigate, hasPermission, visibleCases]);

  return (
    <CommandPalette
      items={items}
      triggerLabel="Search"
      triggerIcon={<Search size={14} aria-hidden="true" />}
      searchLabel="Search pages, records and actions"
      searchPlaceholder="Search pages, records and actions"
      shortcutLabel={null}
      emptyMessage="No matching pages, records or actions."
      maxResults={12}
      clearOnSelect
    />
  );
}
