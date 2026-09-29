import { useEffect, useState } from 'react';
import { fetchStaffDirectory } from '../../../lib/routingRecords';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { Role } from '../types';

const BACKEND_USER_NAME: Record<Role, string> = { standard: 'alex', admin: 'dana' };

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
  const { role } = useTaxRouting();
  const [staff, setStaff] = useState<{ id: string; name: string }[]>(cache ?? []);

  useEffect(() => {
    let alive = true;
    load(BACKEND_USER_NAME[role]).then((list) => {
      if (alive) setStaff(list);
    });
    return () => {
      alive = false;
    };
  }, [role]);

  const staffName = (id: string): string => staff.find((s) => s.id === id)?.name ?? (id === 'system' ? 'System' : 'Unassigned');

  return { staff, staffName };
}
