import { useEffect, useState } from 'react';
import { fetchClients } from '../../../lib/clientDirectory';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { Client, Role } from '../types';

// Same role -> backend-user mapping as useDocumentTypeCatalogue.ts / useEntityDirectory.ts.
const BACKEND_USER_NAME: Record<Role, string> = { standard: 'alex', admin: 'dana' };

let cache: Client[] | null = null;
let inflight: Promise<Client[]> | null = null;

async function load(userName: string): Promise<Client[]> {
  if (cache) return cache;
  if (!inflight) {
    inflight = fetchClients(userName)
      .then((clients) => {
        cache = clients;
        return clients;
      })
      .catch((err) => {
        inflight = null;
        throw err;
      });
  }
  return inflight;
}

/** The full active client list, for reference-data admin views (business spec 5.4). */
export function useClientDirectory() {
  const { role } = useTaxRouting();
  const [clients, setClients] = useState<Client[]>(cache ?? []);
  const [error, setError] = useState(false);

  useEffect(() => {
    let alive = true;
    load(BACKEND_USER_NAME[role])
      .then((list) => {
        if (alive) setClients(list);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [role]);

  return { ready: cache !== null, error, clients };
}
