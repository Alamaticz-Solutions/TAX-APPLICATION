import type {
  AdminModel,
  DataType,
  EntityType,
  GridColumn,
  LookupOption,
  MetaGroup,
  PropertyType,
  RecordValue,
  RelationshipAction,
  StandardMethod
} from "../types";

const NAV_TYPES = new Set<DataType>(["NavToOne", "NavToMany", "ManyToMany"]);
const OBJECT_TYPES = new Set<DataType>(["Object", "ObjectArray"]);
const SEARCHABLE_TYPES = new Set<DataType>(["String", "Enum"]);

const METHOD_DESCRIPTIONS: Record<StandardMethod, string> = {
  FindById: "Single-record lookup by key.",
  GetAll: "Unpaged collection read, usually for reference data.",
  Query: "Paged server-side read with filter and sort.",
  Create: "Create mutation is generated for this entity.",
  Update: "Update mutation is generated for this entity.",
  Delete: "Delete mutation is generated for this entity."
};

const FACET_DESCRIPTIONS: Record<string, string> = {
  concurrency: "Optimistic concurrency/version checks apply to writes."
};

export function singularMethod(prefix: string, entity: EntityType) {
  return `${prefix}${graphqlMethodSuffix(entity.snake_1)}`;
}

export function pluralMethod(prefix: string, entity: EntityType) {
  return `${prefix}${graphqlMethodSuffix(entity.snake_n)}`;
}

function graphqlMethodSuffix(identifier: string) {
  return identifier
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

export function hasMethod(entity: EntityType | null, method: StandardMethod) {
  return Boolean(entity?.standard_methods?.includes(method));
}

export function isAuditEntity(entity: EntityType | null) {
  if (!entity) return false;
  return (
    entity.meta?.generatedAuditEntity === true ||
    entity.snake_1.endsWith("_audit") ||
    entity.snake_n.endsWith("_audit")
  );
}

export function entityKey(entity: EntityType) {
  return `${entity.schema_name}.${entity.pascal_1}`;
}

export function lookupKeyForProperty(prop: PropertyType) {
  return prop.foreign_key ? `${prop.foreign_key.schema_name}.${prop.foreign_key.type_name}` : "";
}

export function isNative(prop: PropertyType) {
  return !NAV_TYPES.has(prop.data_type) && !OBJECT_TYPES.has(prop.data_type);
}

export function queryProps(entity: EntityType) {
  return entity.props.filter(isNative);
}

export function relationProps(entity: EntityType) {
  return entity.props.filter((prop) => NAV_TYPES.has(prop.data_type));
}

export function tableProps(entity: EntityType) {
  const props = queryProps(entity);
  const keys = props.filter((prop) => prop.is_key);
  const captions = props.filter((prop) => prop.is_caption && !prop.is_key);
  const required = props.filter((prop) => prop.is_required && !prop.is_key && !prop.is_caption);
  const rest = props.filter((prop) => !keys.includes(prop) && !captions.includes(prop) && !required.includes(prop));
  return [...keys, ...captions, ...required, ...rest].slice(0, 8);
}

export function gridColumnsForEntity(entity: EntityType, model?: AdminModel | null): GridColumn[] {
  return [
    ...queryProps(entity).map(nativeGridColumn),
    ...relatedGridColumnsForEntity(entity, model)
  ];
}

export function defaultGridColumnsForEntity(entity: EntityType, model?: AdminModel | null): GridColumn[] {
  const columns = gridColumnsForEntity(entity, model);
  const defaults = tableProps(entity).map((prop) => columns.find((column) => column.id === prop.id)).filter(Boolean);
  return defaults.length ? defaults as GridColumn[] : columns.slice(0, 8);
}

export function searchFilterForEntity(entity: EntityType, searchTerm: string): Record<string, unknown> | null {
  const term = searchTerm.trim();
  if (!term) return null;
  const clauses = searchProps(entity).map((prop) => ({
    [prop.name]: { _contains: term }
  }));
  if (clauses.length === 0) return null;
  return clauses.length === 1 ? clauses[0] : { _or: clauses };
}

export function combineFilters(...filters: Array<Record<string, unknown> | null | undefined>) {
  const active = filters.filter((filter): filter is Record<string, unknown> => Boolean(filter && Object.keys(filter).length));
  if (active.length === 0) return null;
  if (active.length === 1) return active[0];
  return { _and: active };
}

export function selectionFor(entity: EntityType, model?: AdminModel | null) {
  return [
    ...queryProps(entity).map((prop) => prop.name),
    ...relationshipSelectionFor(entity, model)
  ].join(" ");
}

export function selectionForGridColumns(entity: EntityType, columns: GridColumn[]) {
  const operationalProps = queryProps(entity).filter((prop) => prop.is_key || prop.is_concurrency_control);
  const nativeProps = uniqueProps([
    ...operationalProps,
    ...columns.filter((column) => column.kind === "native").map((column) => column.prop)
  ]);
  return [
    ...nativeProps.map((prop) => prop.name),
    ...relationshipSelectionForGridColumns(columns)
  ].join(" ");
}

export function optionSelectionFor(entity: EntityType) {
  return optionProps(entity)
    .map((prop) => prop.name)
    .join(" ");
}

export function uniqueEntities(entities: EntityType[]) {
  const seen = new Set<string>();
  return entities.filter((entity) => {
    const key = entityKey(entity);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export function findEntity(model: AdminModel, schemaName: string, typeName: string) {
  return model.entity_types.find((entity) => entity.schema_name === schemaName && entity.pascal_1 === typeName) ?? null;
}

export function metaGroupsForEntity(entity: EntityType): MetaGroup[] {
  return [
    {
      label: "Identity",
      description: "Runtime type, field count, and storage shape.",
      items: [
        { label: `${entity.schema_name}.${entity.pascal_1}`, description: "Schema-qualified entity type." },
        { label: `${entity.props.length} fields`, description: "Properties exposed by the model." },
        {
          label: entity.is_table ? "table" : "model",
          description: entity.is_table ? "Backed by a database table." : "Generated model type not marked as a table."
        }
      ]
    },
    {
      label: "Allowed Actions",
      description: "Generated GraphQL operations enabled for this entity.",
      items: (entity.standard_methods ?? []).map((method) => ({
        label: method,
        description: METHOD_DESCRIPTIONS[method]
      }))
    },
    {
      label: "Facets",
      description: "Model traits used by the generator and runtime.",
      items: (entity.facets ?? []).map((facet) => ({
        label: facet,
        description: FACET_DESCRIPTIONS[facet] ?? "Configured model facet."
      }))
    }
  ];
}

export function lookupOptionForRecord(record: RecordValue, entity: EntityType) {
  const key = keyProp(entity);
  const value = key ? valueText(record[key.name]) : "";
  if (!value) return null;
  return {
    value,
    label: labelForRecord(record, entity)
  };
}

export function relationshipAction(model: AdminModel, entity: EntityType, prop: PropertyType, record: RecordValue): RelationshipAction {
  const target = relationTarget(model, entity, prop);
  const title = prop.caption || prop.name;
  const targetLabel = target?.caption_n ?? prop.many_to_many_property?.target_type ?? prop.nav_by_fk_property?.resolved.type_name ?? title;
  const baseSubtitle = `${targetLabel} for ${labelForRecord(record, entity)}`;

  if (!target) {
    return {
      prop,
      target,
      title,
      subtitle: "Target entity not found in model",
      disabled: true,
      reason: "Target entity not found in model"
    };
  }

  if (prop.data_type === "ManyToMany") {
    const snapshot = relationSnapshot(record, prop);
    if (snapshot) {
      return { prop, target, title, subtitle: baseSubtitle, disabled: false, snapshot };
    }

    return {
      prop,
      target,
      title,
      subtitle: `${baseSubtitle} (unscoped)`,
      disabled: false,
      filter: null
    };
  }

  const nav = prop.nav_by_fk_property;
  const sourceKey = keyProp(entity);
  const targetKey = keyProp(target);
  if (!nav || !sourceKey || !targetKey) {
    return {
      prop,
      target,
      title,
      subtitle: baseSubtitle,
      disabled: true,
      reason: "Relationship key metadata is incomplete"
    };
  }

  const currentHasFk = nav.type_name === entity.pascal_1 && nav.schema_name === entity.schema_name;
  if (currentHasFk) {
    const fkValue = record[nav.prop_name];
    const snapshot = relationSnapshot(record, prop);
    if (isEmptyValue(fkValue) && snapshot) {
      return { prop, target, title, subtitle: baseSubtitle, disabled: false, snapshot };
    }
    if (isEmptyValue(fkValue)) {
      return {
        prop,
        target,
        title,
        subtitle: baseSubtitle,
        disabled: true,
        reason: `No ${nav.prop_name} value on this record`
      };
    }
    return {
      prop,
      target,
      title,
      subtitle: baseSubtitle,
      disabled: false,
      filter: { [targetKey.name]: { _eq: fkValue } }
    };
  }

  const sourceValue = record[sourceKey.name];
  if (isEmptyValue(sourceValue)) {
    return {
      prop,
      target,
      title,
      subtitle: baseSubtitle,
      disabled: true,
      reason: `No ${sourceKey.name} value on this record`
    };
  }
  return {
    prop,
    target,
    title,
    subtitle: baseSubtitle,
    disabled: false,
    filter: { [nav.prop_name]: { _eq: sourceValue } }
  };
}

export function displayValue(prop: PropertyType, value: unknown, lookups: Record<string, LookupOption[]>) {
  const raw = valueText(value);
  if (!raw || !prop.foreign_key) return raw;
  return lookups[lookupKeyForProperty(prop)]?.find((option) => option.value === raw)?.label ?? raw;
}

export function displayColumnValue(column: GridColumn, record: RecordValue, lookups: Record<string, LookupOption[]>) {
  if (column.kind === "native") return displayValue(column.prop, record[column.prop.name], lookups);
  return valueText(valueAtPath(record, column.path));
}

export function recordMatchesSearch(record: RecordValue, searchTerm: string) {
  return JSON.stringify(record).toLowerCase().includes(searchTerm.toLowerCase());
}

export function valueText(value: unknown) {
  if (value === null || value === undefined) return "";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

export function fieldDescription(prop: PropertyType) {
  const flags = [
    prop.name,
    prop.is_key ? "key" : "",
    prop.is_required ? "required" : "",
    prop.is_read_only ? "read only" : "",
    prop.foreign_key ? `fk ${prop.foreign_key.type_name}` : "",
    prop.nav_by_fk_property ? `nav ${prop.nav_by_fk_property.resolved.type_name}` : "",
    prop.many_to_many_property ? `many ${prop.many_to_many_property.target_type}` : ""
  ].filter(Boolean);
  return flags.join(" / ");
}

function nativeGridColumn(prop: PropertyType): GridColumn {
  return {
    id: prop.id,
    kind: "native",
    label: prop.caption || prop.name,
    pathLabel: prop.name,
    data_type: prop.data_type,
    prop,
    path: [prop.name],
    sortable: true
  };
}

function relatedGridColumnsForEntity(entity: EntityType, model?: AdminModel | null): GridColumn[] {
  if (!model) return [];
  return relationProps(entity)
    .filter((prop) => prop.data_type === "NavToOne")
    .flatMap((relationProp) => {
      const target = relationTarget(model, entity, relationProp);
      if (!target) return [];
      return captionProps(target).map((targetProp) => ({
        id: `rel:${relationProp.name}.${targetProp.name}`,
        kind: "related" as const,
        label: `${relationProp.caption || relationProp.name}: ${targetProp.caption || targetProp.name}`,
        pathLabel: `${relationProp.name}.${targetProp.name}`,
        data_type: targetProp.data_type,
        prop: targetProp,
        relation_prop: relationProp,
        target_prop: targetProp,
        path: [relationProp.name, targetProp.name],
        sortable: false
      }));
    });
}

function searchProps(entity: EntityType) {
  return uniqueProps([...captionProps(entity), ...tableProps(entity), ...queryProps(entity)])
    .filter((prop) => SEARCHABLE_TYPES.has(prop.data_type))
    .slice(0, 12);
}

function relationshipSelectionFor(entity: EntityType, model?: AdminModel | null) {
  if (!model) return [];
  return relationProps(entity)
    .filter((prop) => prop.data_type !== "ManyToMany")
    .map((prop) => {
      const target = relationTarget(model, entity, prop);
      if (!target) return null;
      const selection = relatedRecordSelectionFor(target);
      return selection.length ? `${prop.name} { ${selection.map((item) => item.name).join(" ")} }` : null;
    })
    .filter((selection): selection is string => Boolean(selection));
}

function relationshipSelectionForGridColumns(columns: GridColumn[]) {
  const byRelation = new Map<string, { relation: PropertyType; props: PropertyType[] }>();
  columns.forEach((column) => {
    if (column.kind !== "related" || !column.relation_prop || !column.target_prop) return;
    const current = byRelation.get(column.relation_prop.name) ?? {
      relation: column.relation_prop,
      props: []
    };
    current.props.push(column.target_prop);
    byRelation.set(column.relation_prop.name, current);
  });

  return Array.from(byRelation.values())
    .map(({ relation, props }) => {
      const selection = uniqueProps(props).map((prop) => prop.name);
      return selection.length ? `${relation.name} { ${selection.join(" ")} }` : null;
    })
    .filter((selection): selection is string => Boolean(selection));
}

function relatedRecordSelectionFor(entity: EntityType) {
  return uniqueProps([...tableProps(entity), ...optionProps(entity)]);
}

function optionProps(entity: EntityType) {
  const key = keyProp(entity);
  return uniqueProps([...(key ? [key] : []), ...captionProps(entity)]);
}

function uniqueProps(props: PropertyType[]) {
  const seen = new Set<string>();
  return props.filter((prop) => {
    if (seen.has(prop.name)) return false;
    seen.add(prop.name);
    return true;
  });
}

export function keyProp(entity: EntityType) {
  return entity.props.find((prop) => prop.is_key) ?? queryProps(entity)[0] ?? null;
}

function captionProps(entity: EntityType) {
  const props = queryProps(entity);
  const captions = props.filter((prop) => prop.is_caption && !prop.is_key);
  const firstName = props.find((prop) => prop.name === "first_name");
  const hasLastNameCaption = captions.some((prop) => prop.name === "last_name");
  if (firstName && hasLastNameCaption) return uniqueProps([firstName, ...captions]).slice(0, 2);
  if (captions.length) return captions.slice(0, 2);

  const preferred = ["name", "display_name", "title", "label", "email", "quote_number", "number"];
  const preferredProps = preferred
    .map((name) => props.find((prop) => prop.name === name && prop.data_type === "String"))
    .filter((prop): prop is PropertyType => Boolean(prop));
  if (preferredProps.length) return preferredProps.slice(0, 2);
  return props.filter((prop) => prop.data_type === "String" && !prop.is_key).slice(0, 2);
}

function labelForRecord(record: RecordValue, entity: EntityType) {
  const label = captionProps(entity)
    .map((prop) => valueText(record[prop.name]))
    .filter(Boolean)
    .join(" ");
  if (label) return label;
  const key = keyProp(entity);
  return key ? valueText(record[key.name]) : entity.caption_1;
}

export function relationTarget(model: AdminModel, entity: EntityType, prop: PropertyType) {
  if (prop.nav_by_fk_property) {
    const nav = prop.nav_by_fk_property;
    const currentHasFk = nav.type_name === entity.pascal_1 && nav.schema_name === entity.schema_name;
    const target = currentHasFk
      ? nav.resolved
      : {
          schema_name: nav.schema_name,
          type_name: nav.type_name
        };
    return findEntity(model, target.schema_name, target.type_name);
  }

  if (prop.many_to_many_property) {
    return findEntity(model, prop.many_to_many_property.target_schema, prop.many_to_many_property.target_type);
  }

  return null;
}

function relationSnapshot(record: RecordValue, prop: PropertyType) {
  const value = record[prop.name];
  if (Array.isArray(value)) return value.filter(isRecordValue);
  if (isRecordValue(value)) return [value];
  return null;
}

function isRecordValue(value: unknown): value is RecordValue {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function valueAtPath(record: RecordValue, path: string[]) {
  return path.reduce<unknown>((value, segment) => {
    if (!isRecordValue(value)) return undefined;
    return value[segment];
  }, record);
}

function isEmptyValue(value: unknown) {
  return value === null || value === undefined || value === "";
}
