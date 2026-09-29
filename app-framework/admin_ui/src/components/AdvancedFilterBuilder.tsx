import { type CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import { Braces, Filter, FolderPlus, Plus, RotateCcw, Trash2 } from "lucide-react";
import {
  Button,
  DataGridControlPopover,
  DataGridFilterEmpty,
  DataGridFilterGroup,
  DataGridFilterPanel,
  DataGridFilterRule,
  DataGridFilterTrigger,
  IconButton,
  LookupSelect,
  SegmentedControl
} from "@appfw/pds-health-components";
import type {
  AdvancedFilterGroup,
  AdvancedFilterJoin,
  AdvancedFilterNode,
  AdvancedFilterOperator,
  AdvancedFilterRule,
  AdvancedFilterState,
  EntityType,
  FilterCapabilities,
  FilterOperatorCapability,
  LookupOption,
  PropertyType
} from "../types";
import { lookupKeyForProperty } from "../lib/entityModel";
import {
  activeFilterRules,
  advancedFilterToJson,
  defaultOperatorForProp,
  filterableProps,
  normalizeAdvancedFilter,
  operatorCapabilitiesForProp
} from "../lib/filters";

type AdvancedFilterBuilderProps = {
  entity: EntityType | null;
  filterCapabilities: FilterCapabilities | null;
  value: AdvancedFilterState;
  lookupOptions: Record<string, LookupOption[]>;
  onApply: (filter: AdvancedFilterState) => void;
  onClear: () => void;
};

const filterJoinOptions = [
  { value: "and", label: "AND" },
  { value: "or", label: "OR" }
] as const;

export function AdvancedFilterBuilder({
  entity,
  filterCapabilities,
  value,
  lookupOptions,
  onApply,
  onClear
}: AdvancedFilterBuilderProps) {
  const [draft, setDraft] = useState<AdvancedFilterState>(value);
  const [isOpen, setIsOpen] = useState(false);
  const [popoverStyle, setPopoverStyle] = useState<CSSProperties>({});
  const detailsRef = useRef<HTMLDetailsElement>(null);
  const props = useMemo(() => (entity ? filterableProps(entity, filterCapabilities) : []), [entity, filterCapabilities]);
  const activeCount = entity ? activeFilterRules(entity, value, filterCapabilities).length : 0;
  const draftJson = useMemo(
    () => advancedFilterToJson(entity, draft, filterCapabilities),
    [draft, entity, filterCapabilities]
  );

  useEffect(() => {
    setDraft(normalizeAdvancedFilter(entity, value, filterCapabilities));
  }, [entity?.id, filterCapabilities, value]);

  useEffect(() => {
    if (!isOpen) return;
    updatePopoverLayout();
    window.addEventListener("resize", updatePopoverLayout);
    return () => window.removeEventListener("resize", updatePopoverLayout);
  }, [isOpen]);

  function updatePopoverLayout() {
    const summary = detailsRef.current?.querySelector("summary");
    if (!summary) return;
    const rect = summary.getBoundingClientRect();
    const contentLeft = document.querySelector(".content")?.getBoundingClientRect().left ?? 0;
    const gutter = 20;
    const minLeft = contentLeft + gutter;
    const maxWidth = Math.max(320, window.innerWidth - minLeft - gutter);
    const width = Math.min(780, maxWidth);
    const centeredLeft = rect.left + rect.width / 2 - width / 2;
    const left = Math.max(minLeft, Math.min(centeredLeft, window.innerWidth - width - gutter));
    const top = rect.bottom + 8;
    setPopoverStyle({
      left,
      top,
      width,
      maxHeight: `calc(100vh - ${Math.round(top + gutter)}px)`,
      transform: "none"
    });
  }

  function addRule(groupId: string) {
    const rule = createRule(props, filterCapabilities);
    if (!rule) return;
    setDraft((current) => updateGroup(current, groupId, (group) => ({ ...group, children: [...group.children, rule] })));
  }

  function addGroup(groupId: string) {
    const group = createGroup(props, filterCapabilities);
    setDraft((current) => updateGroup(current, groupId, (target) => ({ ...target, children: [...target.children, group] })));
  }

  function updateJoin(groupId: string, join: AdvancedFilterJoin) {
    setDraft((current) => updateGroup(current, groupId, (group) => ({ ...group, join })));
  }

  function updateRule(ruleId: string, patch: Partial<AdvancedFilterRule>) {
    setDraft((current) => updateRuleNode(current, ruleId, (rule) => ({ ...rule, ...patch })));
  }

  function selectRuleProp(rule: AdvancedFilterRule, propId: string) {
    const prop = props.find((item) => item.id === propId);
    if (!prop) return;
    updateRule(rule.id, {
      propId,
      operator: defaultOperatorForProp(prop, filterCapabilities),
      value: defaultValueForProp(prop)
    });
  }

  function removeNode(nodeId: string) {
    setDraft((current) => removeChildNode(current, nodeId));
  }

  function clearFilter() {
    const empty: AdvancedFilterState = { kind: "group", id: "root", join: "and", children: [] };
    setDraft(empty);
    onClear();
    closePopover();
  }

  function applyFilter() {
    onApply(draft);
    closePopover();
  }

  function cancelFilter() {
    setDraft(normalizeAdvancedFilter(entity, value, filterCapabilities));
    closePopover();
  }

  function closePopover() {
    if (detailsRef.current) {
      detailsRef.current.open = false;
    }
    setIsOpen(false);
  }

  if (!entity) return null;

  return (
    <details className="advanced-filter" ref={detailsRef} onToggle={(event) => setIsOpen(event.currentTarget.open)}>
      <DataGridFilterTrigger
        as="summary"
        icon={<Filter size={15} />}
        label="Filter"
        activeCount={activeCount}
      />
      <DataGridControlPopover className="filter-popover" style={popoverStyle} ariaLabel="Advanced Filter" size="lg">
        <DataGridFilterPanel
          title="Advanced Filter"
          summary={entity.caption_n}
          actions={(
            <>
              <Button onClick={clearFilter}>
                <RotateCcw size={14} />
                Clear
              </Button>
              <Button onClick={cancelFilter}>
                Cancel
              </Button>
              <Button variant="primary" onClick={applyFilter}>
                Apply
              </Button>
            </>
          )}
        >
          <FilterGroupEditor
            group={draft}
            isRoot
            filterCapabilities={filterCapabilities}
            lookupOptions={lookupOptions}
            props={props}
            onAddGroup={addGroup}
            onAddRule={addRule}
            onJoinChange={updateJoin}
            onRemoveNode={removeNode}
            onRulePropChange={selectRuleProp}
            onRuleUpdate={updateRule}
          />

          {draftJson && (
            <div className="filter-preview">
              <div>
                <Braces size={14} />
                Filter JSON
              </div>
              <pre>{JSON.stringify(draftJson, null, 2)}</pre>
            </div>
          )}
        </DataGridFilterPanel>
      </DataGridControlPopover>
    </details>
  );
}

function FilterGroupEditor({
  group,
  isRoot = false,
  filterCapabilities,
  lookupOptions,
  props,
  onAddGroup,
  onAddRule,
  onJoinChange,
  onRemoveNode,
  onRulePropChange,
  onRuleUpdate
}: {
  group: AdvancedFilterGroup;
  isRoot?: boolean;
  filterCapabilities: FilterCapabilities | null;
  lookupOptions: Record<string, LookupOption[]>;
  props: PropertyType[];
  onAddGroup: (groupId: string) => void;
  onAddRule: (groupId: string) => void;
  onJoinChange: (groupId: string, join: AdvancedFilterJoin) => void;
  onRemoveNode: (nodeId: string) => void;
  onRulePropChange: (rule: AdvancedFilterRule, propId: string) => void;
  onRuleUpdate: (ruleId: string, patch: Partial<AdvancedFilterRule>) => void;
}) {
  return (
    <DataGridFilterGroup
      isRoot={isRoot}
      controls={(
        <SegmentedControl
          ariaLabel={isRoot ? "Root filter join mode" : "Nested filter join mode"}
          value={group.join}
          options={filterJoinOptions}
          onValueChange={(join) => onJoinChange(group.id, join)}
        />
      )}
      actions={(
        <>
          <Button disabled={!props.length} onClick={() => onAddRule(group.id)}>
            <Plus size={14} />
            Add filter
          </Button>
          <Button disabled={!props.length} onClick={() => onAddGroup(group.id)}>
            <FolderPlus size={14} />
            Add group
          </Button>
          {!isRoot && (
            <IconButton
              size="sm"
              ariaLabel="Remove group"
              tooltip="Remove group"
              icon={<Trash2 size={14} />}
              onClick={() => onRemoveNode(group.id)}
            />
          )}
        </>
      )}
    >
      {group.children.length ? (
        group.children.map((node) =>
          node.kind === "group" ? (
            <FilterGroupEditor
              group={node}
              key={node.id}
              filterCapabilities={filterCapabilities}
              lookupOptions={lookupOptions}
              props={props}
              onAddGroup={onAddGroup}
              onAddRule={onAddRule}
              onJoinChange={onJoinChange}
              onRemoveNode={onRemoveNode}
              onRulePropChange={onRulePropChange}
              onRuleUpdate={onRuleUpdate}
            />
          ) : (
            <FilterRuleEditor
              key={node.id}
              filterCapabilities={filterCapabilities}
              lookupOptions={lookupOptions}
              props={props}
              rule={node}
              onPropChange={onRulePropChange}
              onRemove={() => onRemoveNode(node.id)}
              onUpdate={onRuleUpdate}
            />
          )
        )
      ) : (
        <DataGridFilterEmpty icon={<Filter size={19} />}>No advanced filters applied</DataGridFilterEmpty>
      )}
    </DataGridFilterGroup>
  );
}

function FilterRuleEditor({
  rule,
  filterCapabilities,
  lookupOptions,
  props,
  onPropChange,
  onRemove,
  onUpdate
}: {
  rule: AdvancedFilterRule;
  filterCapabilities: FilterCapabilities | null;
  lookupOptions: Record<string, LookupOption[]>;
  props: PropertyType[];
  onPropChange: (rule: AdvancedFilterRule, propId: string) => void;
  onRemove: () => void;
  onUpdate: (ruleId: string, patch: Partial<AdvancedFilterRule>) => void;
}) {
  const prop = props.find((item) => item.id === rule.propId) ?? props[0];
  if (!prop) return null;
  const operators = operatorCapabilitiesForProp(filterCapabilities, prop);
  const supportedOperators = operators.filter((item) => item.supported);
  const operatorCapability =
    operators.find((item) => item.op === rule.operator && item.supported) ?? supportedOperators[0];
  if (!operatorCapability) return null;
  const operator = operatorCapability.op;

  return (
    <DataGridFilterRule>
      <select value={prop.id} onChange={(event) => onPropChange(rule, event.target.value)}>
        {props.map((item) => (
          <option key={item.id} value={item.id}>
            {item.caption || item.name}
          </option>
        ))}
      </select>
      <select value={operator} onChange={(event) => onUpdate(rule.id, { operator: event.target.value as AdvancedFilterOperator })}>
        {operators.map((item) => (
          <option disabled={!item.supported} key={item.op} title={item.unsupported_reason ?? undefined} value={item.op}>
            {item.supported ? item.label : `${item.label} (not supported)`}
          </option>
        ))}
      </select>
      <FilterValueInput
        operatorCapability={operatorCapability}
        lookupOptions={lookupOptions}
        prop={prop}
        value={rule.value}
        onChange={(nextValue) => onUpdate(rule.id, { value: nextValue })}
      />
      <IconButton
        size="sm"
        ariaLabel="Remove filter"
        tooltip="Remove filter"
        icon={<Trash2 size={14} />}
        onClick={onRemove}
      />
    </DataGridFilterRule>
  );
}

function FilterValueInput({
  prop,
  operatorCapability,
  value,
  lookupOptions,
  onChange
}: {
  prop: PropertyType;
  operatorCapability: FilterOperatorCapability;
  value: string;
  lookupOptions: Record<string, LookupOption[]>;
  onChange: (value: string) => void;
}) {
  const lookupKey = lookupKeyForProperty(prop);
  const options = lookupKey ? lookupOptions[lookupKey] ?? [] : [];
  const isList = operatorCapability.value_shape === "list" || operatorCapability.value_shape === "scalar_or_list";
  const isPeriod = operatorCapability.value_shape === "period";
  if (prop.data_type === "Boolean") {
    return (
      <select value={value || "true"} onChange={(event) => onChange(event.target.value)}>
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    );
  }
  if (!isList && !isPeriod && options.length) {
    return (
      <LookupSelect
        value={value}
        options={options}
        placeholder={`Select ${prop.caption || prop.name}`}
        onValueChange={(nextValue) => onChange(Array.isArray(nextValue) ? nextValue[0] ?? "" : nextValue)}
      />
    );
  }
  return (
    <input
      value={value}
      onChange={(event) => onChange(event.target.value)}
      placeholder={valuePlaceholder(prop, operatorCapability)}
      type={isList || isPeriod ? "text" : inputTypeForProp(prop)}
    />
  );
}

function createGroup(props: PropertyType[], filterCapabilities: FilterCapabilities | null): AdvancedFilterGroup {
  const rule = createRule(props, filterCapabilities);
  return {
    kind: "group",
    id: createFilterId(),
    join: "and",
    children: rule ? [rule] : []
  };
}

function createRule(props: PropertyType[], filterCapabilities: FilterCapabilities | null): AdvancedFilterRule | null {
  const prop = props[0];
  if (!prop) return null;
  return {
    kind: "rule",
    id: createFilterId(),
    propId: prop.id,
    operator: defaultOperatorForProp(prop, filterCapabilities),
    value: defaultValueForProp(prop)
  };
}

function updateGroup(group: AdvancedFilterGroup, groupId: string, updater: (group: AdvancedFilterGroup) => AdvancedFilterGroup): AdvancedFilterGroup {
  if (group.id === groupId) return updater(group);
  return {
    ...group,
    children: group.children.map((child) => (child.kind === "group" ? updateGroup(child, groupId, updater) : child))
  };
}

function updateRuleNode(group: AdvancedFilterGroup, ruleId: string, updater: (rule: AdvancedFilterRule) => AdvancedFilterRule): AdvancedFilterGroup {
  return {
    ...group,
    children: group.children.map((child) => {
      if (child.kind === "rule") return child.id === ruleId ? updater(child) : child;
      return updateRuleNode(child, ruleId, updater);
    })
  };
}

function removeChildNode(group: AdvancedFilterGroup, nodeId: string): AdvancedFilterGroup {
  return {
    ...group,
    children: group.children
      .filter((child) => child.id !== nodeId)
      .map((child) => (child.kind === "group" ? removeChildNode(child, nodeId) : child))
  };
}

function defaultValueForProp(prop: PropertyType) {
  return prop.data_type === "Boolean" ? "true" : "";
}

function inputTypeForProp(prop: PropertyType) {
  if (prop.data_type === "Date") return "date";
  if (prop.data_type === "DateTime") return "datetime-local";
  if (prop.data_type === "Time") return "time";
  if (prop.data_type.startsWith("Int") || prop.data_type.startsWith("Float")) return "number";
  return "text";
}

function valuePlaceholder(prop: PropertyType, operatorCapability: FilterOperatorCapability) {
  if (operatorCapability.value_shape === "list") return "Comma-separated values";
  if (operatorCapability.value_shape === "scalar_or_list") return "Value or comma-separated values";
  if (operatorCapability.value_shape === "period") return "_this_mo, _last_qtr, _today";
  if (prop.data_type.startsWith("Int") || prop.data_type.startsWith("Float")) return "Number";
  if (prop.data_type === "Uuid" || prop.data_type === "ObjectId") return prop.data_type;
  return "Value";
}

function createFilterId() {
  return crypto.randomUUID?.() ?? `filter-${Math.random().toString(36).slice(2)}`;
}
