import type { Entity, EntityType } from '../types';

// Mock entity reference list (TaxEntityList). `number` is the Oracle ID join key.
export const entities: Entity[] = [
  { number: '001', name: 'Oakwood Medical Partners LLC', type: 'Partnership' },
  { number: '002', name: 'Riverside Surgical Associates LP', type: 'Partnership' },
  { number: '003', name: 'Summit Orthopedic Group LLC', type: 'Partnership' },
  { number: '014', name: 'Smith Family Medicine PC', type: 'Personal Corporation' },
  { number: '015', name: 'Lakeview Radiology PC', type: 'Personal Corporation' },
  { number: '021', name: 'Harbor Health Holdings Inc.', type: 'S-Corp' },
  { number: '022', name: 'Pinecrest Clinics Inc.', type: 'S-Corp' }
];

export const entitiesOfType = (type: EntityType | ''): Entity[] => entities.filter((e) => e.type === type);
export const findEntity = (name: string): Entity | undefined => entities.find((e) => e.name === name);
