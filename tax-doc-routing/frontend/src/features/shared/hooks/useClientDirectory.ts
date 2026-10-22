import { useEffect, useState } from 'react';
import { fetchClients } from '../../../lib/clientDirectory';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { Client } from '../types';


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
  const { userName } = useTaxRouting();
  const [clients, setClients] = useState<Client[]>(cache ?? []);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!userName) return undefined;
    let alive = true;
    load(userName)
      .then((list) => {
        if (alive) setClients(list);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [userName]);

  return { ready: cache !== null, error, clients };
}
