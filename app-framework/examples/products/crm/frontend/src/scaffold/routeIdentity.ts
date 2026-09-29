import type { AppfwUiEntityContract } from "../generated/appfw-ui-contract";
import type { AppfwRecord } from "../lib/appfwClient";

export const DEFAULT_RECORD_LOCATOR_FIELD = "record_locator";

const legacyRecordRefStorageKey = "crm-record-route-refs-v1";
const legacyRecordRefPrefix = "rec_";
const draftRefStorageKey = "crm-new-record-draft-refs-v1";
const draftRefPrefix = "draft_";

type LegacyRecordRouteStore = Record<string, {
  toInternal: Record<string, string>;
  toRoute: Record<string, string>;
}>;

type DraftRouteStore = Record<string, Record<string, AppfwRecord>>;

let memoryDraftRouteStore: DraftRouteStore = {};

export function getOrCreateRecordRouteRef(_entity: AppfwUiEntityContract, routeKey: string) {
  return routeKey.trim();
}

export function resolveRecordRouteRef(entity: AppfwUiEntityContract, routeRef: string) {
  const normalizedRef = routeRef.trim();
  if (!normalizedRef) return null;
  if (normalizedRef.startsWith(legacyRecordRefPrefix)) {
    return readLegacyRecordRouteStore()[entityNamespace(entity)]?.toInternal[normalizedRef] ?? null;
  }
  return normalizedRef;
}

export function recordRouteRef(entity: AppfwUiEntityContract, row: AppfwRecord) {
  const locator = row[routeIdField(entity)];
  if (typeof locator === "string" && locator.trim()) return locator.trim();
  const internalKey = String(row[entity.primaryKey] ?? "");
  return getOrCreateRecordRouteRef(entity, internalKey);
}

export function routeIdField(entity: AppfwUiEntityContract) {
  const routeKind = entity.addressing?.idKinds.find((kind) => kind.kind === entity.addressing.routeIdKind);
  return routeKind?.field ?? DEFAULT_RECORD_LOCATOR_FIELD;
}

export function isRecordLocator(value: string | null | undefined) {
  return Boolean(value?.startsWith("rl_"));
}

export function createNewRecordDraftRef(entity: AppfwUiEntityContract, initialValues: AppfwRecord) {
  const values = compactRecord(initialValues);
  if (!Object.keys(values).length) return null;

  const namespace = entityNamespace(entity);
  const store = readDraftRouteStore();
  const bucket = store[namespace] ?? {};
  const draftRef = newOpaqueRef(draftRefPrefix);
  bucket[draftRef] = values;
  store[namespace] = bucket;
  writeDraftRouteStore(store);
  return draftRef;
}

export function resolveNewRecordDraftRef(entity: AppfwUiEntityContract, draftRef: string | null) {
  if (!draftRef?.startsWith(draftRefPrefix)) return null;
  return readDraftRouteStore()[entityNamespace(entity)]?.[draftRef] ?? null;
}

function entityNamespace(entity: AppfwUiEntityContract) {
  return `${entity.schemaName}.${entity.typeName}`;
}

function newOpaqueRef(prefix: string) {
  if (globalThis.crypto?.randomUUID) {
    return `${prefix}${globalThis.crypto.randomUUID().replace(/-/g, "").slice(0, 20)}`;
  }
  const bytes = new Uint8Array(12);
  globalThis.crypto?.getRandomValues?.(bytes);
  const random = Array.from(bytes, (byte) => byte.toString(36).padStart(2, "0")).join("");
  return `${prefix}${Date.now().toString(36)}${random}`;
}

function compactRecord(record: AppfwRecord) {
  return Object.fromEntries(
    Object.entries(record).filter(([, value]) => value !== null && value !== undefined && value !== "")
  );
}

function readLegacyRecordRouteStore(): LegacyRecordRouteStore {
  return readJson<LegacyRecordRouteStore>(legacyRecordRefStorageKey) ?? {};
}

function readDraftRouteStore(): DraftRouteStore {
  const stored = readJson<DraftRouteStore>(draftRefStorageKey);
  if (stored) {
    memoryDraftRouteStore = stored;
    return stored;
  }
  return memoryDraftRouteStore;
}

function writeDraftRouteStore(store: DraftRouteStore) {
  memoryDraftRouteStore = store;
  writeJson(draftRefStorageKey, store);
}

function readJson<T>(key: string): T | null {
  try {
    const raw = globalThis.localStorage?.getItem(key);
    return raw ? JSON.parse(raw) as T : null;
  } catch (_error) {
    return null;
  }
}

function writeJson(key: string, value: unknown) {
  try {
    globalThis.localStorage?.setItem(key, JSON.stringify(value));
  } catch (_error) {
    // In private or locked-down browser contexts the in-memory store still
    // preserves opaque refs for the current session.
  }
}
