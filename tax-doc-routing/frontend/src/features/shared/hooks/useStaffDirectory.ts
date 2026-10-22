import { useEffect, useState } from 'react';
import { fetchStaffDirectory } from '../../../lib/routingRecords';
import { useTaxRouting } from '../state/TaxRoutingProvider';


let cache: { id: string; name: string }[] | null = null;
let inflight: Promise<{ id: string; name: string }[]> | null = null;

async function load(userName: string): Promise<{ id: string; name: string }[]> {
  if (cache) return cache;
  if (!inflight) {
    inflight = fetchStaffDirectory(userName)
      .then((staff) => {
        cache = staff;
        return staff;
      })
      .catch((err) => {
        inflight = null;
        throw err;
      });
  }
  return inflight;
}

/** The active operator directory (business spec 12.3), read live instead of the hardcoded
 * `staffDirectory` array `features/shared/data/users.ts` used to export. */
export function useStaffDirectory() {
  const { userName } = useTaxRouting();
  const [staff, setStaff] = useState<{ id: string; name: string }[]>(cache ?? []);

  useEffect(() => {
    if (!userName) return undefined;
    let alive = true;
    load(userName).then((list) => {
      if (alive) setStaff(list);
    });
    return () => {
      alive = false;
    };
  }, [userName]);

  const staffName = (id: string): string => staff.find((s) => s.id === id)?.name ?? (id === 'system' ? 'System' : 'Unassigned');

  return { staff, staffName };
}
