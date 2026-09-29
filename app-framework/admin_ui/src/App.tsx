import { type FormEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  Database,
  Info,
  KeyRound,
  Loader2,
  Lock,
  Plus,
  RefreshCcw,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  Terminal
} from "lucide-react";
import { Badge, Button, IconButton } from "@appfw/pds-health-components";
import { AboutModal } from "./components/AboutModal";
import { PdsLogo } from "./components/Brand";
import { DeveloperConsole } from "./components/DeveloperConsole";
import { EntityModelDrawer } from "./components/EntityModelDrawer";
import { MetaChipGroup } from "./components/MetaChipGroup";
import { RecordDrawer } from "./components/RecordDrawer";
import { RecordsPanel } from "./components/RecordsPanel";
import { SchemaModelDrawer } from "./components/SchemaModelDrawer";
import { GraphqlRequestError, graphqlDetailed } from "./lib/api";
import { ADMIN_AUTHORIZATION_KEY, adminAuthHeaders } from "./lib/authHeaders";
import {
  EMPTY_ADVANCED_FILTER,
  advancedFilterToJson,
  normalizeAdvancedFilter,
  recordMatchesFilter
} from "./lib/filters";
import { defaultFormValues, formProps, missingConcurrencyProps, normalizeInput } from "./lib/form";
import { formatResponseTime } from "./lib/timing";
import {
  combineFilters,
  defaultGridColumnsForEntity,
  entityKey,
  findEntity,
  gridColumnsForEntity,
  hasMethod,
  isAuditEntity,
  keyProp,
  lookupOptionForRecord,
  metaGroupsForEntity,
  optionSelectionFor,
  pluralMethod,
  recordMatchesSearch,
  relationProps,
  relationshipAction,
  searchFilterForEntity,
  selectionFor,
  selectionForGridColumns,
  singularMethod,
  uniqueEntities
} from "./lib/entityModel";
import type {
  ActiveScope,
  AdvancedFilterState,
  AdminModel,
  AuditTimelineStatus,
  CustomMethodPreview,
  CustomMethodStatus,
  DrawerMode,
  EntityType,
  FormQueryPreview,
  FormActionStatus,
  GraphqlTrace,
  GridSort,
  LookupOption,
  PropertyType,
  QueryResult,
  RecordDiff,
  RecordValue,
  RelationshipAction,
  SavedGridView,
  SaveMutationPreview,
  StatusTone
} from "./types";

declare const __ADMIN_UI_VERSION__: string;

const PAGE_SIZE = 25;
const LOOKUP_LIMIT = 100;
const SYSTEM_SCHEMA = "system";

export function App() {
  const [model, setModel] = useState<AdminModel | null>(null);
  const [activeSchema, setActiveSchema] = useState<string>("");
  const [activeEntityId, setActiveEntityId] = useState<string>("");
  const [entitySearch, setEntitySearch] = useState("");
  const [recordSearch, setRecordSearch] = useState("");
  const [advancedFilter, setAdvancedFilter] = useState<AdvancedFilterState>(EMPTY_ADVANCED_FILTER);
  const [gridSort, setGridSort] = useState<GridSort | null>(null);
  const [visibleColumnIds, setVisibleColumnIds] = useState<string[]>([]);
  const [records, setRecords] = useState<RecordValue[]>([]);
  const [recordCount, setRecordCount] = useState<number | null>(null);
  const [pageCount, setPageCount] = useState<number | null>(null);
  const [skip, setSkip] = useState(0);
  const [status, setStatus] = useState("Loading entity model");
  const [statusTone, setStatusTone] = useState<StatusTone>("idle");
  const [authorizationInput, setAuthorizationInput] = useState(() =>
    typeof window === "undefined" ? "" : (window.sessionStorage.getItem(ADMIN_AUTHORIZATION_KEY) ?? "")
  );
  const [isLoading, setIsLoading] = useState(false);
  const [drawerRecord, setDrawerRecord] = useState<RecordValue | null>(null);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>("create");
  const [formValues, setFormValues] = useState<RecordValue>({});
  const [formActionStatus, setFormActionStatus] = useState<FormActionStatus | null>(null);
  const [customMethodStatuses, setCustomMethodStatuses] = useState<Record<string, CustomMethodStatus>>({});
  const [auditTimeline, setAuditTimeline] = useState<AuditTimelineStatus | null>(null);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [modelOpen, setModelOpen] = useState(false);
  const [schemaOpen, setSchemaOpen] = useState(false);
  const [consoleOpen, setConsoleOpen] = useState(false);
  const [graphqlTraces, setGraphqlTraces] = useState<GraphqlTrace[]>([]);
  const [activeScope, setActiveScope] = useState<ActiveScope | null>(null);
  const [lookupOptions, setLookupOptions] = useState<Record<string, LookupOption[]>>({});
  const [loadingLookups, setLoadingLookups] = useState<Record<string, boolean>>({});
  const recordsRequestRef = useRef(0);
  const formLoadRequestRef = useRef(0);
  const auditRequestRef = useRef(0);

  useEffect(() => {
    loadModel();
  }, []);

  function applyAuthorization(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    saveAuthorization(authorizationInput);
    loadModel();
  }

  function saveAuthorization(value: string) {
    setAuthorizationInput(value);
    const authorization = value.trim();
    if (authorization) {
      window.sessionStorage.setItem(ADMIN_AUTHORIZATION_KEY, authorization);
    } else {
      window.sessionStorage.removeItem(ADMIN_AUTHORIZATION_KEY);
    }
  }

  const schemas = model?.schemas ?? [];
  const schema = schemas.find((item) => item.name === activeSchema) ?? schemas[0] ?? null;
  const filterCapabilities = schema?.filter_capabilities ?? null;
  const schemaHasPersistence = hasSchemaPersistence(schema);
  const schemaEntities = useMemo(() => {
    if (!model || !schema) return [];
    return model.entity_types
      .filter((entity) => entity.schema_name === schema.name && !entity.is_union)
  }, [model, schema]);
  const entities = useMemo(() => {
    const needle = entitySearch.trim().toLowerCase();
    if (!needle) return schemaEntities;
    return schemaEntities.filter((entity) => (
      entity.caption_n.toLowerCase().includes(needle) ||
      entity.pascal_1.toLowerCase().includes(needle) ||
      entity.snake_n.toLowerCase().includes(needle)
    ));
  }, [schemaEntities, entitySearch]);
  const entity =
    entities.find((item) => item.id === activeEntityId) ??
    entities.find((item) => item.is_table) ??
    entities[0] ??
    null;
  const allTableColumns = useMemo(() => (entity ? gridColumnsForEntity(entity, model) : []), [entity, model]);
  const tableColumns = useMemo(
    () => allTableColumns.filter((column) => visibleColumnIds.includes(column.id)),
    [allTableColumns, visibleColumnIds]
  );
  const visibleColumnSelectionKey = tableColumns.map((column) => column.id).join("|");
  useEffect(() => {
    if (!entity) {
      setVisibleColumnIds([]);
      return;
    }
    const columns = gridColumnsForEntity(entity, model);
    const validIds = new Set(columns.map((column) => column.id));
    const defaultIds = defaultGridColumnsForEntity(entity, model).map((column) => column.id);
    const savedView = loadSavedGridView(entity);
    const savedVisibleIds = savedView?.visibleColumnIds.filter((id) => validIds.has(id)) ?? [];
    setVisibleColumnIds(savedVisibleIds.length ? savedVisibleIds : defaultIds);
    setAdvancedFilter(normalizeAdvancedFilter(entity, savedView?.advancedFilter, filterCapabilities));
    setRecordSearch(savedView?.recordSearch ?? "");
    const savedSort = savedView?.sort ?? null;
    const sortColumn = savedSort ? columns.find((column) => column.id === savedSort.propId) : null;
    setGridSort(sortColumn?.sortable ? savedSort : null);
    setSkip(0);
  }, [entity?.id, filterCapabilities, model]);
  useEffect(() => {
    if (!gridSort || tableColumns.some((prop) => prop.id === gridSort.propId)) return;
    setGridSort(null);
  }, [gridSort, tableColumns]);
  const activeScopeKey = activeScope ? JSON.stringify(activeScope) : "";
  const recordSearchTerm = recordSearch.trim();
  const advancedFilterJson = useMemo(
    () => advancedFilterToJson(entity, advancedFilter, filterCapabilities),
    [advancedFilter, entity, filterCapabilities]
  );
  const advancedFilterKey = useMemo(() => JSON.stringify(advancedFilterJson ?? null), [advancedFilterJson]);
  const entityMetaGroups = entity ? metaGroupsForEntity(entity) : [];
  const entityIsReadOnly = isAuditEntity(entity);
  const mainEntityMetaGroups = entityMetaGroups.filter((group) => group.label === "Identity");
  const relationshipActions =
    model && entity && drawerMode === "edit" && drawerRecord
      ? relationProps(entity).map((prop) => relationshipAction(model, entity, prop, drawerRecord))
      : [];
  const formDiffs = useMemo(
    () => {
      if (!entity) return [];
      if (isAuditEntity(entity)) return [];
      if (drawerMode === "create") return recordDiffs(entity, {}, formValues, true);
      return drawerRecord ? recordDiffs(entity, drawerRecord, formValues, false) : [];
    },
    [entity, drawerMode, drawerRecord, formValues]
  );
  const formIsDirty = formDiffs.length > 0;
  const formQueryPreview = useMemo(
    () => (entity && drawerMode === "edit" && drawerRecord ? buildFormQueryPreview(entity, drawerRecord, model) : null),
    [entity, drawerMode, drawerRecord, model]
  );
  const saveMutationPreview = useMemo(
    () => (entity && !isAuditEntity(entity) && Object.keys(formValues).length ? buildSaveMutationPreview(entity, formValues, drawerMode === "create") : null),
    [entity, formValues, drawerMode]
  );
  const customMethodPreviews = useMemo(
    () => (entity && drawerMode === "edit" && drawerRecord ? buildCustomMethodPreviews(entity, drawerRecord) : []),
    [entity, drawerMode, drawerRecord]
  );
  const schemaGraphqlTraces = useMemo(
    () => graphqlTraces.filter((trace) => trace.schemaName === activeSchema),
    [graphqlTraces, activeSchema]
  );

  async function runGraphql(
    operation: string,
    schemaName: string,
    query: string,
    variables: Record<string, unknown>
  ) {
    try {
      const result = await graphqlDetailed(schemaName, query, variables);
      addGraphqlTrace({
        id: crypto.randomUUID(),
        operation,
        schemaName,
        query,
        variables,
        data: result.data,
        payload: result.payload,
        responseMs: result.responseMs,
        httpStatus: result.httpStatus,
        requestId: result.requestId,
        correlationId: result.correlationId,
        responseRequestId: result.responseRequestId,
        responseCorrelationId: result.responseCorrelationId,
        createdAt: new Date().toISOString(),
        ok: true
      });
      return result.data;
    } catch (error) {
      if (error instanceof GraphqlRequestError) {
        addGraphqlTrace({
          id: crypto.randomUUID(),
          operation,
          schemaName,
          query,
          variables,
          payload: error.payload,
          error: error.message,
          responseMs: error.responseMs,
          httpStatus: error.httpStatus,
          requestId: error.requestId,
          correlationId: error.correlationId,
          responseRequestId: error.responseRequestId,
          responseCorrelationId: error.responseCorrelationId,
          createdAt: new Date().toISOString(),
          ok: false
        });
      }
      throw error;
    }
  }

  function addGraphqlTrace(trace: GraphqlTrace) {
    setGraphqlTraces((current) => [trace, ...current].slice(0, 30));
  }

  async function loadModel() {
    setStatus("Loading entity model");
    setStatusTone("idle");
    const response = await fetch("/admin/model", {
      headers: {
        ...adminAuthHeaders(),
        "x-timezone": Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC"
      }
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) {
      setStatus(payload.error ?? "Unable to load admin model");
      setStatusTone("error");
      return;
    }

    const nextModel = payload as AdminModel;
    const defaultSchema = nextModel.schemas.find((item) => item.name !== "system") ?? nextModel.schemas[0];
    const defaultEntity =
      nextModel.entity_types.find(
        (item) => item.schema_name === defaultSchema?.name && item.is_table && !item.is_union
      ) ??
      nextModel.entity_types.find((item) => item.schema_name === defaultSchema?.name && !item.is_union);

    setModel(nextModel);
    setActiveSchema(defaultSchema?.name ?? "");
    setActiveEntityId(defaultEntity?.id ?? "");
    setSkip(0);
    setStatus("Entity model loaded");
    setStatusTone("ok");
  }

  useEffect(() => {
    if (!entity) return;
    if (!hasEntityPersistence(entity)) {
      recordsRequestRef.current += 1;
      setRecords([]);
      setRecordCount(null);
      setPageCount(null);
      setIsLoading(false);
      setStatus(schemaPersistenceMessage(entity.schema_name));
      setStatusTone("idle");
      return;
    }
    if (allTableColumns.length > 0 && tableColumns.length === 0) return;
    const handle = window.setTimeout(
      () => loadRecords(entity, skip),
      recordSearchTerm ? 250 : 0
    );
    return () => window.clearTimeout(handle);
  }, [
    entity?.id,
    skip,
    activeScopeKey,
    recordSearchTerm,
    advancedFilterKey,
    gridSort?.propId,
    gridSort?.propName,
    gridSort?.direction,
    visibleColumnSelectionKey
  ]);

  useEffect(() => {
    if (!entity || !hasEntityPersistence(entity)) return;
    loadLookupsForEntity(entity);
  }, [entity?.id]);

  async function loadRecords(target = entity, nextSkip = skip) {
    if (!target) return;
    if (!hasEntityPersistence(target)) {
      recordsRequestRef.current += 1;
      setRecords([]);
      setRecordCount(null);
      setPageCount(null);
      setIsLoading(false);
      setStatus(schemaPersistenceMessage(target.schema_name));
      setStatusTone("idle");
      return;
    }
    const requestId = recordsRequestRef.current + 1;
    recordsRequestRef.current = requestId;
    const selectedColumns = target.id === entity?.id
      ? tableColumns
      : defaultGridColumnsForEntity(target, model);
    const scoped = activeScope?.entityId === target.id ? activeScope : null;
    const searchTerm = recordSearch.trim();
    if (scoped?.snapshot) {
      const startedAt = performance.now();
      const matchingSearchRows = searchTerm
        ? scoped.snapshot.filter((record) => recordMatchesSearch(record, searchTerm))
        : scoped.snapshot;
      const matchingRows = advancedFilterJson
        ? matchingSearchRows.filter((record) => recordMatchesFilter(record, advancedFilterJson))
        : matchingSearchRows;
      const snapshotRows = sortRecords(matchingRows, gridSort);
      const responseMs = performance.now() - startedAt;
      if (requestId !== recordsRequestRef.current) return;
      setRecords(snapshotRows);
      setRecordCount(snapshotRows.length);
      setPageCount(1);
      setIsLoading(false);
      setStatus(
        queryStatus(
          snapshotRows.length,
          responseMs,
          queryDetail(searchTerm, Boolean(advancedFilterJson), "local snapshot")
        )
      );
      setStatusTone("ok");
      return;
    }

    if (!hasMethod(target, "Query")) {
      setRecords([]);
      setRecordCount(null);
      setPageCount(null);
      setStatus("Query is not enabled for this entity");
      setStatusTone("idle");
      return;
    }

    setIsLoading(true);
    setStatus(`Loading ${target.caption_n}`);
    setStatusTone("idle");
    try {
      const field = pluralMethod("query", target);
      const query = `query($skip: Int!, $limit: Int!, $filter: JSON, $sort: JSON) { ${field}(filter: $filter, sort: $sort, skip: $skip, limit: $limit) { queryCount: query_count pageCount: page_count items { ${selectionForGridColumns(target, selectedColumns)} } } }`;
      const searchFilter = searchFilterForEntity(target, searchTerm);
      const filter = combineFilters(scoped?.filter ?? null, searchFilter, advancedFilterJson);
      const sort = sortInput(gridSort);
      const startedAt = performance.now();
      const data = await runGraphql(`Query ${target.caption_n}`, target.schema_name, query, {
        skip: nextSkip,
        limit: PAGE_SIZE,
        filter,
        sort
      });
      const responseMs = performance.now() - startedAt;
      const result = (data[field] ?? {}) as QueryResult;
      const items = result.items ?? [];
      const resultCount = result.queryCount ?? result.query_count ?? null;
      const resultPageCount = result.pageCount ?? result.page_count ?? null;
      if (requestId !== recordsRequestRef.current) return;
      setRecords(items);
      setRecordCount(resultCount);
      setPageCount(totalPagesFromQueryResult(resultCount, resultPageCount));
      setStatus(
        queryStatus(resultCount, responseMs, queryDetail(searchTerm, Boolean(advancedFilterJson)))
      );
      setStatusTone("ok");
    } catch (error) {
      if (requestId !== recordsRequestRef.current) return;
      setRecords([]);
      setRecordCount(null);
      setPageCount(null);
      setStatus(error instanceof Error ? error.message : "Unable to load records");
      setStatusTone("error");
    } finally {
      if (requestId === recordsRequestRef.current) setIsLoading(false);
    }
  }

  function selectSchema(schemaName: string) {
    if (!model) return;
    const defaultEntity = model.entity_types.find(
      (item) => item.schema_name === schemaName && item.is_table && !item.is_union
    );
    setActiveSchema(schemaName);
    setActiveEntityId(defaultEntity?.id ?? "");
    setActiveScope(null);
    setSkip(0);
    setRecordSearch("");
    setAdvancedFilter(EMPTY_ADVANCED_FILTER);
    setGridSort(null);
    setModelOpen(false);
    setSchemaOpen(false);
    closeDrawer();
  }

  function selectEntity(nextEntity: EntityType) {
    setActiveEntityId(nextEntity.id);
    setActiveScope(null);
    setSkip(0);
    setRecordSearch("");
    setAdvancedFilter(EMPTY_ADVANCED_FILTER);
    setGridSort(null);
    setModelOpen(false);
    setSchemaOpen(false);
    closeDrawer();
  }

  function openCreate() {
    if (!entity) return;
    if (isAuditEntity(entity)) {
      const message = "Audit records are read-only history";
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    formLoadRequestRef.current += 1;
    loadLookupsForEntity(entity);
    setDrawerMode("create");
    setDrawerRecord(null);
    setFormValues(defaultFormValues(entity, true));
    setFormActionStatus(null);
    setCustomMethodStatuses({});
  }

  async function openEdit(record: RecordValue) {
    if (!entity) return;
    const target = entity;
    loadLookupsForEntity(entity);
    setDrawerMode("edit");
    setDrawerRecord(record);
    setFormValues(defaultFormValues(entity, false, record, isAuditEntity(entity)));
    setCustomMethodStatuses({});
    loadAuditTimeline(target, record);
    await loadFormRecord(target, record);
  }

  function closeDrawer() {
    formLoadRequestRef.current += 1;
    auditRequestRef.current += 1;
    setDrawerRecord(null);
    setFormValues({});
    setFormActionStatus(null);
    setCustomMethodStatuses({});
    setAuditTimeline(null);
  }

  function updateFormValue(prop: PropertyType, value: unknown) {
    setFormActionStatus(null);
    setFormValues((current) => ({ ...current, [prop.name]: value }));
  }

  function persistGridView(patch: Partial<SavedGridView>) {
    if (!entity) return;
    saveGridViewForEntity(entity, {
      visibleColumnIds,
      recordSearch,
      sort: gridSort,
      advancedFilter,
      ...patch
    });
  }

  function updateRecordSearch(value: string) {
    setRecordSearch(value);
    persistGridView({ recordSearch: value });
    setSkip(0);
  }

  function applyAdvancedFilter(filter: AdvancedFilterState) {
    const nextFilter = normalizeAdvancedFilter(entity, filter, filterCapabilities);
    setAdvancedFilter(nextFilter);
    persistGridView({ advancedFilter: nextFilter });
    setSkip(0);
  }

  function clearAdvancedFilter() {
    setAdvancedFilter(EMPTY_ADVANCED_FILTER);
    persistGridView({ advancedFilter: EMPTY_ADVANCED_FILTER });
    setSkip(0);
  }

  function updateGridSort(nextSort: GridSort | null) {
    setGridSort(nextSort);
    persistGridView({ sort: nextSort });
    setSkip(0);
  }

  function updateVisibleColumns(nextIds: string[]) {
    if (!entity) return;
    const validIds = new Set(allTableColumns.map((column) => column.id));
    const nextVisibleIds = nextIds.filter((id) => validIds.has(id));
    if (nextVisibleIds.length === 0) return;
    setVisibleColumnIds(nextVisibleIds);
    persistGridView({ visibleColumnIds: nextVisibleIds });
  }

  function saveGridView() {
    if (!entity) return;
    saveGridViewForEntity(entity, {
      visibleColumnIds,
      recordSearch,
      sort: gridSort,
      advancedFilter
    });
    setStatus(`Saved view for ${entity.caption_n}`);
    setStatusTone("ok");
  }

  function resetGridView() {
    if (!entity) return;
    clearSavedGridView(entity);
    setVisibleColumnIds(defaultGridColumnsForEntity(entity, model).map((column) => column.id));
    setRecordSearch("");
    setAdvancedFilter(EMPTY_ADVANCED_FILTER);
    setGridSort(null);
    setSkip(0);
    setStatus(`Reset view for ${entity.caption_n}`);
    setStatusTone("ok");
  }

  async function loadFormRecord(target: EntityType, record: RecordValue) {
    const key = keyProp(target);
    const id = key ? record[key.name] : null;
    if (!key || id === null || id === undefined || id === "" || !hasMethod(target, "FindById")) {
      setFormActionStatus(null);
      return;
    }

    const requestId = formLoadRequestRef.current + 1;
    formLoadRequestRef.current = requestId;
    setFormActionStatus({ action: "load", message: `Loading ${target.caption_1}`, tone: "idle", isBusy: true });
    try {
      const queryPreview = buildFormQueryPreview(target, record, model);
      if (!queryPreview) {
        setFormActionStatus(null);
        return;
      }
      const startedAt = performance.now();
      const data = await runGraphql(`Load ${target.caption_1}`, target.schema_name, queryPreview.query, queryPreview.variables);
      const responseMs = performance.now() - startedAt;
      if (requestId !== formLoadRequestRef.current) return;
      const loadedRecord = data[queryPreview.method] as RecordValue | null | undefined;
      if (loadedRecord && typeof loadedRecord === "object") {
        setDrawerRecord(loadedRecord);
        setFormValues(defaultFormValues(target, false, loadedRecord, isAuditEntity(target)));
      }
      setFormActionStatus({ action: "load", message: `${target.caption_1} loaded`, tone: "ok", responseMs });
    } catch (error) {
      if (requestId !== formLoadRequestRef.current) return;
      setFormActionStatus({
        action: "load",
        message: error instanceof Error ? error.message : "Unable to load record",
        tone: "error"
      });
    }
  }

  async function loadLookupsForEntity(target: EntityType) {
    if (!model) return;
    const lookupTargets = uniqueEntities(
      target.props
        .map((prop) => (prop.foreign_key ? findEntity(model, prop.foreign_key.schema_name, prop.foreign_key.type_name) : null))
        .filter((item): item is EntityType => Boolean(item))
    );

    await Promise.all(
      lookupTargets.map(async (lookupEntity) => {
        const key = entityKey(lookupEntity);
        if (lookupOptions[key] || loadingLookups[key]) return;
        setLoadingLookups((current) => ({ ...current, [key]: true }));
        try {
          if (!hasMethod(lookupEntity, "Query") && !hasMethod(lookupEntity, "GetAll")) {
            setLookupOptions((current) => ({ ...current, [key]: [] }));
            return;
          }
          const selection = optionSelectionFor(lookupEntity);
          const field = hasMethod(lookupEntity, "Query")
            ? pluralMethod("query", lookupEntity)
            : pluralMethod("get", lookupEntity);
          const query = hasMethod(lookupEntity, "Query")
            ? `query($skip: Int!, $limit: Int!) { ${field}(skip: $skip, limit: $limit) { items { ${selection} } } }`
            : `query { ${field} { ${selection} } }`;
          const variables = hasMethod(lookupEntity, "Query") ? { skip: 0, limit: LOOKUP_LIMIT } : {};
          const data = await runGraphql(`Lookup ${lookupEntity.caption_n}`, lookupEntity.schema_name, query, variables);
          const items = hasMethod(lookupEntity, "Query")
            ? ((data[field] ?? {}) as QueryResult).items ?? []
            : ((data[field] ?? []) as RecordValue[]);
          setLookupOptions((current) => ({
            ...current,
            [key]: items
              .map((record) => lookupOptionForRecord(record, lookupEntity))
              .filter((option): option is LookupOption => Boolean(option))
          }));
        } catch (error) {
          console.warn(`Unable to load lookup options for ${lookupEntity.schema_name}.${lookupEntity.pascal_1}`, error);
          setLookupOptions((current) => ({ ...current, [key]: [] }));
        } finally {
          setLoadingLookups((current) => ({ ...current, [key]: false }));
        }
      })
    );
  }

  function navigateRelationship(action: RelationshipAction) {
    if (!model || !action.target || action.disabled) return;
    setActiveSchema(action.target.schema_name);
    setEntitySearch("");
    setActiveEntityId(action.target.id);
    setActiveScope({
      entityId: action.target.id,
      label: action.subtitle,
      filter: action.filter ?? null,
      snapshot: action.snapshot ?? null
    });
    setSkip(0);
    setRecordSearch("");
    setAdvancedFilter(EMPTY_ADVANCED_FILTER);
    setGridSort(null);
    closeDrawer();
  }

  async function saveRecord() {
    if (!entity) return;
    if (isAuditEntity(entity)) {
      const message = "Audit records are read-only history and cannot be saved";
      setFormActionStatus({ message, tone: "idle" });
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    const isCreate = drawerMode === "create";
    const operation = isCreate ? "Create" : "Update";
    if (!hasMethod(entity, operation)) {
      const message = `${operation} is not enabled for this entity`;
      setFormActionStatus({ message, tone: "idle" });
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    if (!isCreate) {
      const missingConcurrency = missingConcurrencyProps(entity, formValues);
      if (missingConcurrency.length) {
        const message = `Cannot update ${entity.caption_1}: missing ${missingConcurrency
            .map((prop) => prop.caption || prop.name)
            .join(", ")}. Refresh the row after applying the version backfill migration.`;
        setFormActionStatus({ message, tone: "error" });
        setStatus(message);
        setStatusTone("error");
        return;
      }
    }
    if (!formIsDirty) {
      const message = isCreate ? "Enter values before saving" : "No changes to save";
      setFormActionStatus({ message, tone: "idle" });
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    let responseMs: number | null = null;
    try {
      const mutationPreview = buildSaveMutationPreview(entity, formValues, isCreate);
      setFormActionStatus({ action: "save", message: `Saving ${entity.caption_1}`, tone: "idle", isBusy: true });
      const startedAt = performance.now();
      const data = await runGraphql(`${operation} ${entity.caption_1}`, entity.schema_name, mutationPreview.query, mutationPreview.variables);
      responseMs = performance.now() - startedAt;
      const savedRecord = data[mutationPreview.method] as RecordValue | undefined;
      if (savedRecord && typeof savedRecord === "object") {
        setDrawerMode("edit");
        setDrawerRecord(savedRecord);
        setFormValues(defaultFormValues(entity, false, savedRecord));
        loadAuditTimeline(entity, savedRecord);
      }
      setFormActionStatus({ action: "save", message: `${entity.caption_1} saved`, tone: "ok", responseMs });
      setStatus(`${entity.caption_1} saved | Response time: ${formatResponseTime(responseMs)}`);
      setStatusTone("ok");
      await loadRecords(entity, skip);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unable to save record";
      setFormActionStatus({ action: "save", message, tone: "error", responseMs });
      setStatus(message);
      setStatusTone("error");
    }
  }

  async function deleteRecord() {
    if (!entity || !drawerRecord) return;
    if (isAuditEntity(entity)) {
      const message = "Audit records are read-only history and cannot be deleted";
      setFormActionStatus({ message, tone: "idle" });
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    if (!hasMethod(entity, "Delete")) {
      const message = "Delete is not enabled for this entity";
      setFormActionStatus({ message, tone: "idle" });
      setStatus(message);
      setStatusTone("idle");
      return;
    }
    let responseMs: number | null = null;
    try {
      const method = singularMethod("delete", entity);
      const input = normalizeInput(entity, drawerRecord, false);
      const query = `mutation($input: Input${entity.pascal_1}!) { ${method}(input: $input) }`;
      setFormActionStatus({ action: "delete", message: `Deleting ${entity.caption_1}`, tone: "idle", isBusy: true });
      const startedAt = performance.now();
      await runGraphql(`Delete ${entity.caption_1}`, entity.schema_name, query, { input });
      responseMs = performance.now() - startedAt;
      setStatus(`${entity.caption_1} deleted | Response time: ${formatResponseTime(responseMs)}`);
      setStatusTone("ok");
      closeDrawer();
      await loadRecords(entity, skip);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unable to delete record";
      setFormActionStatus({ action: "delete", message, tone: "error", responseMs });
      setStatus(message);
      setStatusTone("error");
    }
  }

  async function executeCustomMethod(preview: CustomMethodPreview) {
    if (!entity) return;
    if (preview.operationKind === "mutation") {
      const confirmed = window.confirm(`Execute ${preview.method.name}? This may change data.`);
      if (!confirmed) return;
    }
    setCustomMethodStatuses((current) => ({
      ...current,
      [preview.method.name]: {
        isBusy: true,
        tone: "idle",
        message: `Executing ${preview.method.name}`
      }
    }));
    let responseMs: number | null = null;
    try {
      const startedAt = performance.now();
      const data = await runGraphql(
        `Custom ${preview.method.kind} ${preview.method.name}`,
        entity.schema_name,
        preview.query,
        preview.variables
      );
      responseMs = performance.now() - startedAt;
      const result = data[preview.fieldName];
      setCustomMethodStatuses((current) => ({
        ...current,
        [preview.method.name]: {
          tone: "ok",
          message: `${preview.method.name} completed`,
          responseMs,
          data: result
        }
      }));
      setStatus(`${preview.method.name} completed | Response time: ${formatResponseTime(responseMs)}`);
      setStatusTone("ok");
    } catch (error) {
      const message = error instanceof Error ? error.message : `Unable to execute ${preview.method.name}`;
      setCustomMethodStatuses((current) => ({
        ...current,
        [preview.method.name]: {
          tone: "error",
          message,
          responseMs
        }
      }));
      setStatus(message);
      setStatusTone("error");
    }
  }

  async function loadAuditTimeline(target: EntityType, record: RecordValue) {
    const requestId = auditRequestRef.current + 1;
    auditRequestRef.current = requestId;
    if (!isAuditedEntity(target)) {
      setAuditTimeline(null);
      return;
    }

    const key = keyProp(target);
    const recordId = key ? record[key.name] : null;
    if (!recordId) {
      setAuditTimeline({
        isLoading: false,
        tone: "idle",
        message: "Audit timeline unavailable without a record id",
        audited: true,
        events: []
      });
      return;
    }

    if (!model?.troubleshooting_enabled) {
      setAuditTimeline({
        isLoading: false,
        tone: "idle",
        message: "Audit timeline disabled. Set APP_ADMIN_TROUBLESHOOTING_ENABLED=true to enable server diagnostics.",
        enabled: false,
        audited: true,
        events: []
      });
      return;
    }

    setAuditTimeline({
      isLoading: true,
      tone: "idle",
      message: "Loading audit timeline",
      enabled: true,
      audited: true,
      events: []
    });
    const startedAt = performance.now();
    try {
      const response = await fetch("/admin/troubleshooting/audit", {
        method: "POST",
        headers: {
          ...adminAuthHeaders(),
          "content-type": "application/json",
          "x-timezone": Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC"
        },
        body: JSON.stringify({
          schema_name: target.schema_name,
          type_name: target.pascal_1,
          record_id: String(recordId),
          limit: 25
        })
      });
      const payload = await response.json().catch(() => ({}));
      const responseMs = performance.now() - startedAt;
      if (requestId !== auditRequestRef.current) return;
      if (!response.ok) {
        setAuditTimeline({
          isLoading: false,
          tone: "error",
          message: payload.error ?? "Unable to load audit timeline",
          responseMs,
          enabled: true,
          audited: true,
          events: []
        });
        return;
      }
      setAuditTimeline({
        isLoading: false,
        tone: "ok",
        message: payload.message ?? `${(payload.events ?? []).length} audit event${(payload.events ?? []).length === 1 ? "" : "s"}`,
        responseMs,
        enabled: payload.enabled,
        audited: payload.audited,
        events: payload.events ?? [],
        currentPolicy: payload.current_policy ?? null
      });
    } catch (error) {
      const responseMs = performance.now() - startedAt;
      if (requestId !== auditRequestRef.current) return;
      setAuditTimeline({
        isLoading: false,
        tone: "error",
        message: error instanceof Error ? error.message : "Unable to load audit timeline",
        responseMs,
        enabled: true,
        audited: true,
        events: []
      });
    }
  }

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand-block">
          <PdsLogo />
          <span className="brand-subtitle">Model-driven admin</span>
        </div>

        <label className="schema-select">
          <span>Schema</span>
          <select
            disabled={!schemas.length}
            onChange={(event) => selectSchema(event.target.value)}
            value={schema?.name ?? ""}
          >
            {schemas.map((item) => (
              <option key={item.id} value={item.name}>
                {item.name}
              </option>
            ))}
          </select>
        </label>

        <div className="sidebar-count">
          <ShieldCheck size={16} />
          Entity count: {entities.length.toLocaleString()}
        </div>

        <label className="sidebar-search">
          <Search size={16} />
          <input
            value={entitySearch}
            onChange={(event) => setEntitySearch(event.target.value)}
            placeholder="Find entity"
            type="search"
          />
        </label>

        <nav className="entity-list">
          {entities.map((item) => (
            <button
              className={`entity-row ${item.id === entity?.id ? "active" : ""}`}
              key={item.id}
              onClick={() => selectEntity(item)}
              type="button"
            >
              <span>
                <strong>{item.caption_n}</strong>
                <small>{item.props.length} fields</small>
              </span>
              <em>{item.is_table ? "table" : "type"}</em>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <button className="about-button" onClick={() => setAboutOpen(true)} type="button">
            <Info size={16} />
            About
          </button>
          <span>UI v{__ADMIN_UI_VERSION__}</span>
        </div>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">{schema?.description ?? "Admin console"}</p>
            <h1>{entity?.caption_n ?? "Admin"}</h1>
            {entity ? (
              <div className="entity-meta-groups compact">
                {mainEntityMetaGroups.map((group) => (
                  <MetaChipGroup group={group} key={group.label} />
                ))}
              </div>
            ) : (
              <span>Select an entity</span>
            )}
          </div>
          <div className="toolbar">
            <IconButton
              ariaLabel="Reload model"
              tooltip="Reload model"
              icon={<RefreshCcw size={17} />}
              onClick={() => loadModel()}
            />
            <Button disabled={!schema} onClick={() => setSchemaOpen(true)}>
              <Database size={16} />
              Schema Model
            </Button>
            <Button disabled={!entity} onClick={() => setModelOpen(true)}>
              <SlidersHorizontal size={16} />
              Entity Model
            </Button>
            <Button onClick={() => setConsoleOpen(true)}>
              <Terminal size={16} />
              Console
              {schemaGraphqlTraces.length > 0 && <Badge tone="accent">{schemaGraphqlTraces.length}</Badge>}
            </Button>
            <Button
              disabled={!entity || !schemaHasPersistence}
              onClick={() => entity && loadRecords(entity, skip)}
            >
              <RefreshCcw size={16} />
              Refresh
            </Button>
            <Button
              variant="primary"
              disabled={!entity || !schemaHasPersistence || entityIsReadOnly || !hasMethod(entity, "Create")}
              onClick={openCreate}
              title={entityIsReadOnly ? "Audit records are read-only" : "New"}
            >
              <Plus size={16} />
              New
            </Button>
          </div>
        </header>

        <div className={`status-line ${statusTone}`}>
          {isLoading ? <Loader2 className="spin" size={16} /> : statusTone === "error" ? <Lock size={16} /> : <ShieldCheck size={16} />}
          {status}
          {statusTone === "error" && !model && (
            <form className="auth-inline" onSubmit={applyAuthorization}>
              <input
                aria-label="Authorization header"
                autoComplete="off"
                onChange={(event) => saveAuthorization(event.target.value)}
                placeholder="Bearer token"
                type="password"
                value={authorizationInput}
              />
              <Button type="submit">
                <KeyRound size={16} />
                Apply
              </Button>
            </form>
          )}
        </div>

        <section className="workbench">
          {schemaHasPersistence ? (
            <RecordsPanel
              entity={entity}
              filterCapabilities={filterCapabilities}
              allColumns={allTableColumns}
              tableColumns={tableColumns}
              visibleColumnIds={visibleColumnIds}
              page={Math.floor(skip / PAGE_SIZE) + 1}
              pageSize={PAGE_SIZE}
              pageCount={pageCount}
              totalRows={recordCount}
              activeScope={activeScope}
              isLoading={isLoading}
              records={records}
              recordSearch={recordSearch}
              recordSearchTerm={recordSearchTerm}
              advancedFilter={advancedFilter}
              lookupOptions={lookupOptions}
              sort={gridSort}
              canGoPrevious={skip > 0}
              canGoNext={canGoNext(skip, recordCount, records.length)}
              onSearchChange={updateRecordSearch}
              onAdvancedFilterApply={applyAdvancedFilter}
              onAdvancedFilterClear={clearAdvancedFilter}
              onSortChange={updateGridSort}
              onVisibleColumnsChange={updateVisibleColumns}
              onSaveView={saveGridView}
              onResetView={resetGridView}
              onFirstPage={() => setSkip(0)}
              onPreviousPage={() => setSkip(Math.max(0, skip - PAGE_SIZE))}
              onNextPage={() => setSkip(skip + PAGE_SIZE)}
              onClearScope={() => setActiveScope(null)}
              onOpenRecord={openEdit}
            />
          ) : (
            <SystemSchemaNotice entityCount={schemaEntities.length} schema={schema} />
          )}
        </section>
      </section>

      {modelOpen && entity && (
        <EntityModelDrawer entity={entity} metaGroups={entityMetaGroups} onClose={() => setModelOpen(false)} />
      )}

      {schemaOpen && schema && (
        <SchemaModelDrawer schema={schema} entities={schemaEntities} onClose={() => setSchemaOpen(false)} />
      )}

      {entity && Object.keys(formValues).length > 0 && (
        <RecordDrawer
          entity={entity}
          mode={drawerMode}
          isReadOnly={entityIsReadOnly}
          formValues={formValues}
          actionStatus={formActionStatus}
          auditTimeline={auditTimeline}
          diffs={formDiffs}
          isDirty={formIsDirty}
          queryPreview={formQueryPreview}
          lookupOptions={lookupOptions}
          loadingLookups={loadingLookups}
          mutationPreview={saveMutationPreview}
          customMethodPreviews={customMethodPreviews}
          customMethodStatuses={customMethodStatuses}
          relationshipActions={relationshipActions}
          onClose={closeDrawer}
          onDelete={deleteRecord}
          onSave={saveRecord}
          onExecuteCustomMethod={executeCustomMethod}
          onUpdateField={updateFormValue}
          onNavigateRelationship={navigateRelationship}
        />
      )}

      {aboutOpen && (
        <AboutModal
          model={model}
          entity={entity}
          version={__ADMIN_UI_VERSION__}
          onClose={() => setAboutOpen(false)}
        />
      )}

      {consoleOpen && (
        <DeveloperConsole
          schemaName={activeSchema}
          traces={schemaGraphqlTraces}
          troubleshootingEnabled={Boolean(model?.troubleshooting_enabled)}
          onClear={() => setGraphqlTraces((current) => current.filter((trace) => trace.schemaName !== activeSchema))}
          onClose={() => setConsoleOpen(false)}
        />
      )}
    </main>
  );
}

function SystemSchemaNotice({ entityCount, schema }: { entityCount: number; schema: AdminModel["schemas"][number] | null }) {
  return (
    <article className="system-schema-panel">
      <Database size={30} />
      <div>
        <strong>{schemaPersistenceMessage(schema?.name ?? SYSTEM_SCHEMA)}</strong>
        <span>{entityCount.toLocaleString()} model object{entityCount === 1 ? "" : "s"} available for inspection</span>
      </div>
      <p>
        The system schema is metadata-only and is not backed by a persistable database source.
      </p>
    </article>
  );
}

function hasSchemaPersistence(schema: AdminModel["schemas"][number] | null) {
  return schema?.name !== SYSTEM_SCHEMA;
}

function hasEntityPersistence(entity: EntityType) {
  return entity.schema_name !== SYSTEM_SCHEMA;
}

function schemaPersistenceMessage(schemaName: string) {
  return schemaName === SYSTEM_SCHEMA
    ? "System schema has no persistence layer"
    : "Schema has no persistence layer";
}

function queryStatus(count: number | null, responseMs: number, detail?: string) {
  const suffix = detail ? ` | ${detail}` : "";
  const countLabel = count === null ? "unavailable" : count.toLocaleString();
  return `Query count: ${countLabel} | Response time: ${formatResponseTime(responseMs)}${suffix}`;
}

function queryDetail(searchTerm: string, hasAdvancedFilter: boolean, extra?: string) {
  const parts = [
    searchTerm ? `matching "${searchTerm}"` : "",
    hasAdvancedFilter ? "advanced filter" : "",
    extra ?? ""
  ].filter(Boolean);
  return parts.length ? parts.join(" | ") : undefined;
}

function buildFormQueryPreview(entity: EntityType, record: RecordValue, model: AdminModel | null): FormQueryPreview | null {
  const key = keyProp(entity);
  const id = key ? record[key.name] : null;
  if (!key || id === null || id === undefined || id === "" || !hasMethod(entity, "FindById")) return null;
  const method = singularMethod("find", entity);
  return {
    operation: "FindById",
    method,
    query: `query($id: String!) { ${method}(id: $id) { ${selectionFor(entity, model)} } }`,
    variables: { id: String(id) }
  };
}

function buildSaveMutationPreview(entity: EntityType, values: RecordValue, isCreate: boolean): SaveMutationPreview {
  const method = singularMethod(isCreate ? "create" : "update", entity);
  return {
    operation: isCreate ? "Create" : "Update",
    method,
    query: `mutation($input: Input${entity.pascal_1}!) { ${method}(input: $input) { ${selectionFor(entity)} } }`,
    variables: { input: normalizeInput(entity, values, isCreate) }
  };
}

function buildCustomMethodPreviews(entity: EntityType, record: RecordValue): CustomMethodPreview[] {
  return (entity.custom_methods ?? []).map((method) => {
    const operationKind = method.kind === "Query" ? "query" : "mutation";
    const variables = Object.fromEntries(
      method.args.map((arg) => [
        customMethodFieldName(arg.name),
        customMethodArgValue(entity, record, arg.name, arg.arg_type)
      ])
    );
    const variableDefinitions = method.args
      .map((arg) => `$${customMethodFieldName(arg.name)}: ${graphqlTypeForCustomArg(arg.arg_type)}`)
      .join(", ");
    const fieldArgs = method.args
      .map((arg) => `${customMethodFieldName(arg.name)}: $${customMethodFieldName(arg.name)}`)
      .join(", ");
    const operationName = customMethodOperationName(method.name);
    const fieldName = customMethodFieldName(method.name);
    const operationSignature = variableDefinitions ? `${operationName}(${variableDefinitions})` : operationName;
    const fieldSignature = fieldArgs ? `${fieldName}(${fieldArgs})` : fieldName;

    return {
      method,
      fieldName,
      operationKind,
      operationName,
      query: `${operationKind} ${operationSignature} { ${fieldSignature} }`,
      variables
    };
  });
}

function customMethodArgValue(entity: EntityType, record: RecordValue, argName: string, argType: string) {
  if (Object.prototype.hasOwnProperty.call(record, argName)) return record[argName];
  const key = keyProp(entity);
  if (key && (argName === "id" || argName === `${entity.snake_1}_id` || argName === `${entity.snake_1}_key`)) {
    return record[key.name] ?? "";
  }
  if (isOptionalCustomArg(argType)) return null;
  if (graphqlScalarForCustomArg(argType) === "Boolean") return false;
  if (graphqlScalarForCustomArg(argType) === "Int" || graphqlScalarForCustomArg(argType) === "Float") return 0;
  return "";
}

function customMethodOperationName(methodName: string) {
  const fieldName = customMethodFieldName(methodName);
  return fieldName.charAt(0).toUpperCase() + fieldName.slice(1);
}

function customMethodFieldName(methodName: string) {
  return methodName
    .split("_")
    .filter(Boolean)
    .map((part, index) => index === 0 ? part : part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

function graphqlTypeForCustomArg(argType: string): string {
  const optional = isOptionalCustomArg(argType);
  const innerType = unwrapOptionalCustomArg(argType);
  const listMatch = innerType.match(/^Vec<(.+)>$/);
  const rendered = listMatch
    ? `[${graphqlScalarForCustomArg(listMatch[1])}!]`
    : graphqlScalarForCustomArg(innerType);
  return optional ? rendered : `${rendered}!`;
}

function isOptionalCustomArg(argType: string) {
  return /^Option<.+>$/.test(argType.trim());
}

function unwrapOptionalCustomArg(argType: string) {
  const trimmed = argType.trim();
  const match = trimmed.match(/^Option<(.+)>$/);
  return (match?.[1] ?? trimmed).trim();
}

function graphqlScalarForCustomArg(argType: string) {
  const normalized = argType.trim().replace(/^std::string::String$/, "String");
  if (normalized === "bool" || normalized === "Boolean") return "Boolean";
  if (/^(i|u)(8|16|32|64|size)$/.test(normalized) || /^Int(8|16|32|64)$/.test(normalized)) return "Int";
  if (normalized === "f32" || normalized === "f64" || normalized === "Float32" || normalized === "Float64") return "Float";
  if (normalized === "serde_json::Value" || normalized === "Value" || normalized === "Json") return "JSON";
  return "String";
}

function recordDiffs(entity: EntityType, original: RecordValue, values: RecordValue, isCreate: boolean): RecordDiff[] {
  const before = normalizeInput(entity, defaultFormValues(entity, isCreate, original), isCreate);
  const after = normalizeInput(entity, values, isCreate);
  return formProps(entity, isCreate)
    .filter((prop) => isCreate || (!prop.is_key && !prop.is_concurrency_control && !prop.is_read_only))
    .filter((prop) => stableValue(before[prop.name]) !== stableValue(after[prop.name]))
    .map((prop) => ({
      prop,
      before: before[prop.name],
      after: after[prop.name]
    }));
}

function loadSavedGridView(entity: EntityType): SavedGridView | null {
  try {
    const raw = window.localStorage.getItem(gridViewKey(entity));
    return raw ? (JSON.parse(raw) as SavedGridView) : null;
  } catch {
    return null;
  }
}

function saveGridViewForEntity(entity: EntityType, view: SavedGridView) {
  try {
    window.localStorage.setItem(gridViewKey(entity), JSON.stringify(view));
  } catch {
    // Preferences are best-effort; storage may be unavailable in restricted browser modes.
  }
}

function clearSavedGridView(entity: EntityType) {
  try {
    window.localStorage.removeItem(gridViewKey(entity));
  } catch {
    // Preferences are best-effort; storage may be unavailable in restricted browser modes.
  }
}

function gridViewKey(entity: EntityType) {
  return `appfw.admin.grid.${entity.schema_name}.${entity.pascal_1}`;
}

function stableValue(value: unknown) {
  if (value === undefined) return "null";
  if (value && typeof value === "object") return JSON.stringify(value);
  return String(value ?? "");
}

function totalPagesFromQueryResult(queryCount: number | null, fallbackPageCount: number | null) {
  if (typeof queryCount === "number" && Number.isFinite(queryCount)) {
    return Math.max(1, Math.ceil(queryCount / PAGE_SIZE));
  }
  if (typeof fallbackPageCount === "number" && Number.isFinite(fallbackPageCount)) {
    return Math.max(1, Math.ceil(fallbackPageCount));
  }
  return null;
}

function canGoNext(skip: number, queryCount: number | null, loadedCount: number) {
  if (typeof queryCount === "number" && Number.isFinite(queryCount)) {
    return skip + PAGE_SIZE < queryCount;
  }
  return loadedCount >= PAGE_SIZE;
}

function isAuditedEntity(entity: EntityType) {
  return Boolean(entity.facets?.some((facet) => facet.toLowerCase() === "audited"));
}

function sortInput(sort: GridSort | null) {
  if (!sort) return null;
  return { [snakeToCamel(sort.propName)]: sort.direction.toUpperCase() };
}

function snakeToCamel(value: string) {
  return value.replace(/_([a-z0-9])/g, (_match, letter: string) => letter.toUpperCase());
}

function sortRecords(records: RecordValue[], sort: GridSort | null) {
  if (!sort) return records;
  const direction = sort.direction === "asc" ? 1 : -1;
  return [...records].sort((left, right) => compareRecordValues(left[sort.propName], right[sort.propName]) * direction);
}

function compareRecordValues(left: unknown, right: unknown) {
  if (left === right) return 0;
  if (left === null || left === undefined || left === "") return 1;
  if (right === null || right === undefined || right === "") return -1;

  if (typeof left === "number" && typeof right === "number") return left - right;
  if (typeof left === "boolean" && typeof right === "boolean") return Number(left) - Number(right);

  const leftTime = typeof left === "string" ? Date.parse(left) : NaN;
  const rightTime = typeof right === "string" ? Date.parse(right) : NaN;
  if (Number.isFinite(leftTime) && Number.isFinite(rightTime)) return leftTime - rightTime;

  return String(left).localeCompare(String(right), undefined, { numeric: true, sensitivity: "base" });
}
