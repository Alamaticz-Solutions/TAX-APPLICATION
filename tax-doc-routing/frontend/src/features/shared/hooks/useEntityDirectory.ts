import { useEffect, useState } from 'react';
import { fetchEntities } from '../../../lib/clientDirectory';
import { useTaxRouting } from '../state/TaxRoutingProvider';
import type { Entity, EntityType } from '../types';


// Module-level cache: the entity list is reference data (business spec 5.4), the same for every
// user of a given role, and changes only when an administrator edits it.
let cache: Entity[] | null = null;
let inflight: Promise<Entity[]> | null = null;

async function load(userName: string): Promise<Entity[]> {
  if (cache) return cache;
  if (!inflight) {
    inflight = fetchEntities(userName)
      .then((entities) => {
        cache = entities;
        return entities;
      })
      .catch((err) => {
        inflight = null;
        throw err;
      });
  }
  return inflight;
}

/**
 * The active entity list (business spec 5.4), read live from the database instead of the
 * hardcoded `entities.ts` array `EditDocumentDialog.tsx` used to import.
 */
export function useEntityDirectory() {
  const { userName } = useTaxRouting();
  const [entities, setEntities] = useState<Entity[]>(cache ?? []);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!userName) return undefined;
    let alive = true;
    load(userName)
      .then((list) => {
        if (alive) setEntities(list);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [userName]);

  const entitiesOfType = (type: EntityType | ''): Entity[] => (type ? entities.filter((e) => e.type === type) : []);
  const findEntity = (name: string): Entity | undefined => entities.find((e) => e.name === name);

  return { ready: cache !== null, error, entities, entitiesOfType, findEntity };
}
