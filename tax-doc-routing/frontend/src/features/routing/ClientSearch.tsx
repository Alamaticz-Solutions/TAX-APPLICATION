import { useEffect, useMemo, useState } from 'react';
import { ComboboxField } from '@appfw/pds-health-components/forms';
import type { PdsOption } from '@appfw/pds-health-components/types';
import { searchClients } from '../../lib/clientDirectory';
import { useTaxRouting } from '../shared/state/TaxRoutingProvider';
import type { Client, Role } from '../shared/types';

// Same role -> backend-user mapping as useDocumentTypeCatalogue.ts / useEntityDirectory.ts.
const BACKEND_USER_NAME: Record<Role, string> = { standard: 'alex', admin: 'dana' };

const optionLabel = (c: Client) => `${c.fullName} — ${c.officeLocation} (${c.id})`;

/**
 * Client lookup over the reference list (business spec 5.4). Selecting a result hands the whole
 * client to the parent in one update. An empty search shows suggestions so a client can be picked
 * without typing.
 *
 * `loading` is deliberately not passed to the PDS combobox: it disables the input while true, which
 * would drop focus on every keystroke. The previous results stay visible while a search runs.
 */
export function ClientSearch({ selected, onSelect }: { selected: Client | null; onSelect: (client: Client | null) => void }) {
  const { role } = useTaxRouting();
  const [input, setInput] = useState(selected ? optionLabel(selected) : '');
  const [found, setFound] = useState<Client[]>([]);

  // Keep the text in step with a selection made elsewhere ("Use sample client", "Change client").
  const selectedId = selected?.id;
  useEffect(() => {
    setInput(selected ? optionLabel(selected) : '');
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only when the chosen client changes
  }, [selectedId]);

  useEffect(() => {
    let active = true;
    const timer = setTimeout(async () => {
      const results = await searchClients(BACKEND_USER_NAME[role], selected && input === optionLabel(selected) ? '' : input);
      if (active) {
        setFound(results);
      }
    }, 200);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [input, selected, role]);

  // The selected client must stay in the option list or the combobox would drop the selection.
  const clientsById = useMemo(() => {
    const map = new Map<string, Client>();
    if (selected) map.set(selected.id, selected);
    found.forEach((c) => map.set(c.id, c));
    return map;
  }, [found, selected]);
  const options = useMemo<PdsOption[]>(() => [...clientsById.values()].map((c) => ({ value: c.id, label: optionLabel(c) })), [clientsById]);

  return (
    <ComboboxField
      id="client-search"
      label="Client Name"
      placeholder="Search by name or ID"
      options={options}
      value={selected?.id ?? null}
      onValueChange={(id) => {
        const client = id ? clientsById.get(id) ?? null : null;
        if (client) setInput(optionLabel(client));
        onSelect(client);
      }}
      inputValue={input}
      onInputValueChange={setInput}
      emptyMessage="No active clients match this search."
    />
  );
}
