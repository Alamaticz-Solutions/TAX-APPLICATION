import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type HTMLAttributes,
  type KeyboardEvent,
  type ReactNode
} from "react";
import { Button, SegmentedControl } from "./primitives";
import { composeClassNames } from "./types";

export type RelationshipAtlasAccent =
  | "blue"
  | "teal"
  | "violet"
  | "amber"
  | "green"
  | "coral"
  | "neutral";

export type RelationshipAtlasGroup = {
  id: string;
  label: string;
  detail?: string;
};

export type RelationshipAtlasNode = {
  id: string;
  label: string;
  group: string;
  kind?: string;
  detail?: string;
  accent?: RelationshipAtlasAccent;
};

export type RelationshipAtlasEdge = {
  source: string;
  target: string;
  kind?: string;
};

export type RelationshipAtlasValidationIssue = {
  kind:
    | "duplicate-group"
    | "duplicate-node"
    | "unknown-group"
    | "unknown-source"
    | "unknown-target";
  id: string;
  reference?: string;
};

export function relationshipAtlasValidationIssues(
  groups: readonly RelationshipAtlasGroup[],
  nodes: readonly RelationshipAtlasNode[],
  edges: readonly RelationshipAtlasEdge[]
): readonly RelationshipAtlasValidationIssue[] {
  const issues: RelationshipAtlasValidationIssue[] = [];
  const groupIds = new Set<string>();
  for (const group of groups) {
    if (groupIds.has(group.id)) issues.push({ kind: "duplicate-group", id: group.id });
    groupIds.add(group.id);
  }

  const nodeIds = new Set<string>();
  for (const node of nodes) {
    if (nodeIds.has(node.id)) issues.push({ kind: "duplicate-node", id: node.id });
    nodeIds.add(node.id);
    if (!groupIds.has(node.group)) {
      issues.push({ kind: "unknown-group", id: node.id, reference: node.group });
    }
  }

  edges.forEach((edge, index) => {
    if (!nodeIds.has(edge.source)) {
      issues.push({ kind: "unknown-source", id: String(index), reference: edge.source });
    }
    if (!nodeIds.has(edge.target)) {
      issues.push({ kind: "unknown-target", id: String(index), reference: edge.target });
    }
  });
  return issues;
}

export type RelationshipAtlasProps = Omit<
  HTMLAttributes<HTMLDivElement>,
  "onSelect"
> & {
  ariaLabel: string;
  groups: readonly RelationshipAtlasGroup[];
  nodes: readonly RelationshipAtlasNode[];
  edges: readonly RelationshipAtlasEdge[];
  selectedId?: string | null;
  onSelectNode?: (nodeId: string | null) => void;
  rowHeight?: number;
  emptyMessage?: ReactNode;
  invalidDataMessage?: ReactNode;
};

type NodeRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  centerY: number;
};

const ATLAS_PADDING_X = 20;
const ATLAS_PADDING_TOP = 52;
const ATLAS_PADDING_BOTTOM = 20;
const COLUMN_GAP = 72;
const NODE_VERTICAL_GAP = 12;
const DEFAULT_ROW_HEIGHT = 58;
const SAME_COLUMN_ARC = 34;

function neighborIds(
  edges: readonly RelationshipAtlasEdge[],
  nodeId: string | null
): ReadonlySet<string> {
  if (!nodeId) return new Set();
  const neighbors = new Set<string>();
  edges.forEach((edge) => {
    if (edge.source === nodeId) neighbors.add(edge.target);
    if (edge.target === nodeId) neighbors.add(edge.source);
  });
  return neighbors;
}

function edgeTouches(edge: RelationshipAtlasEdge, nodeId: string | null): boolean {
  return nodeId !== null && (edge.source === nodeId || edge.target === nodeId);
}

/**
 * A deterministic, non-physics relationship map. Groups render as ordered
 * columns, nodes as real buttons in authored order, and edges as an SVG
 * underlay. Selecting or hovering a node highlights its direct relationships
 * and dims the rest. Pair it with `RelationshipAtlasTable`, which presents the
 * same nodes and relationships as a screen-reader-first table; the map itself
 * stays fully keyboard-operable (arrow keys move, Enter/Space select, Escape
 * clears).
 */
export function RelationshipAtlas({
  ariaLabel,
  groups,
  nodes,
  edges,
  selectedId = null,
  onSelectNode,
  rowHeight = DEFAULT_ROW_HEIGHT,
  emptyMessage = "No relationships to display.",
  invalidDataMessage = "Some relationships could not be rendered because their group or node references are invalid.",
  className,
  style,
  ...rest
}: RelationshipAtlasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState<number>(960);
  const [hoveredId, setHoveredId] = useState<string | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width && width > 0) setContainerWidth(width);
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const validationIssues = useMemo(
    () => relationshipAtlasValidationIssues(groups, nodes, edges),
    [edges, groups, nodes]
  );
  const renderableNodes = useMemo(() => {
    const groupIds = new Set(groups.map((group) => group.id));
    const seen = new Set<string>();
    return nodes.filter((node) => {
      if (!groupIds.has(node.group) || seen.has(node.id)) return false;
      seen.add(node.id);
      return true;
    });
  }, [groups, nodes]);
  const renderableEdges = useMemo(() => {
    const nodeIds = new Set(renderableNodes.map((node) => node.id));
    return edges.filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target));
  }, [edges, renderableNodes]);

  const columns = useMemo(
    () => groups.map((group) => ({
      group,
      nodes: renderableNodes.filter((node) => node.group === group.id)
    })),
    [groups, renderableNodes]
  );

  const layout = useMemo(() => {
    const columnCount = Math.max(1, columns.length);
    const innerWidth = Math.max(
      240,
      containerWidth - ATLAS_PADDING_X * 2 - COLUMN_GAP * (columnCount - 1)
    );
    const columnWidth = innerWidth / columnCount;
    const rects = new Map<string, NodeRect>();
    let deepestColumn = 0;

    columns.forEach((column, columnIndex) => {
      deepestColumn = Math.max(deepestColumn, column.nodes.length);
      const x = ATLAS_PADDING_X + columnIndex * (columnWidth + COLUMN_GAP);
      column.nodes.forEach((node, nodeIndex) => {
        const y = ATLAS_PADDING_TOP + nodeIndex * rowHeight;
        rects.set(node.id, {
          x,
          y,
          width: columnWidth,
          height: rowHeight - NODE_VERTICAL_GAP,
          centerY: y + (rowHeight - NODE_VERTICAL_GAP) / 2
        });
      });
    });

    const height =
      ATLAS_PADDING_TOP + deepestColumn * rowHeight + ATLAS_PADDING_BOTTOM;
    return { rects, columnWidth, height };
  }, [columns, containerWidth, rowHeight]);

  const focusId = hoveredId ?? selectedId;
  const neighbors = useMemo(
    () => neighborIds(renderableEdges, focusId),
    [focusId, renderableEdges]
  );

  function moveFocus(currentId: string, key: string): void {
    const columnIndex = columns.findIndex((column) =>
      column.nodes.some((node) => node.id === currentId)
    );
    if (columnIndex < 0) return;
    const nodeIndex = columns[columnIndex].nodes.findIndex(
      (node) => node.id === currentId
    );
    let nextId: string | undefined;
    if (key === "ArrowDown") nextId = columns[columnIndex].nodes[nodeIndex + 1]?.id;
    if (key === "ArrowUp") nextId = columns[columnIndex].nodes[nodeIndex - 1]?.id;
    if (key === "ArrowRight" || key === "ArrowLeft") {
      const step = key === "ArrowRight" ? 1 : -1;
      for (
        let candidate = columnIndex + step;
        candidate >= 0 && candidate < columns.length;
        candidate += step
      ) {
        const candidateNodes = columns[candidate].nodes;
        if (candidateNodes.length === 0) continue;
        nextId = candidateNodes[Math.min(nodeIndex, candidateNodes.length - 1)].id;
        break;
      }
    }
    if (!nextId) return;
    const nextButton = containerRef.current?.querySelector<HTMLButtonElement>(
      `[data-atlas-node-id="${CSS.escape(nextId)}"]`
    );
    nextButton?.focus();
  }

  function handleNodeKeyDown(event: KeyboardEvent<HTMLButtonElement>, nodeId: string): void {
    if (["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key)) {
      event.preventDefault();
      moveFocus(nodeId, event.key);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onSelectNode?.(null);
    }
  }

  function edgePath(edge: RelationshipAtlasEdge): string | null {
    const source = layout.rects.get(edge.source);
    const target = layout.rects.get(edge.target);
    if (!source || !target) return null;

    if (Math.abs(source.x - target.x) < 1) {
      const x = source.x;
      const bend = x - SAME_COLUMN_ARC;
      return `M ${x} ${source.centerY} C ${bend} ${source.centerY}, ${bend} ${target.centerY}, ${x} ${target.centerY}`;
    }

    const [left, right] = source.x < target.x ? [source, target] : [target, source];
    const startX = left.x + left.width;
    const endX = right.x;
    const midX = (startX + endX) / 2;
    return `M ${startX} ${left.centerY} C ${midX} ${left.centerY}, ${midX} ${right.centerY}, ${endX} ${right.centerY}`;
  }

  if (renderableNodes.length === 0) {
    return (
      <div
        {...rest}
        className={composeClassNames("pds-atlas", "pds-atlas--empty", className)}
        data-validation-issues={validationIssues.length || undefined}
        style={style}
      >
        {validationIssues.length > 0 ? invalidDataMessage : emptyMessage}
      </div>
    );
  }

  return (
    <div
      {...rest}
      ref={containerRef}
      className={composeClassNames("pds-atlas", className)}
      role="group"
      aria-label={ariaLabel}
      data-focused={focusId ? "true" : undefined}
      data-validation-issues={validationIssues.length || undefined}
      style={{ ...style, height: layout.height }}
    >
      {validationIssues.length > 0 ? (
        <span className="pds-atlas__data-warning" role="status">{invalidDataMessage}</span>
      ) : null}
      <svg
        className="pds-atlas__edges"
        width="100%"
        height={layout.height}
        viewBox={`0 0 ${Math.max(containerWidth, 240)} ${layout.height}`}
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        {renderableEdges.map((edge, index) => {
          const path = edgePath(edge);
          if (!path) return null;
          const active = edgeTouches(edge, focusId);
          return (
            <path
              key={`${edge.source}->${edge.target}:${index}`}
              className={composeClassNames(
                "pds-atlas__edge",
                active && "is-active",
                focusId && !active && "is-dim"
              )}
              d={path}
            />
          );
        })}
      </svg>

      {columns.map((column, columnIndex) => {
        const firstNode = column.nodes[0];
        const rect = firstNode ? layout.rects.get(firstNode.id) : undefined;
        const x = rect?.x ?? ATLAS_PADDING_X + columnIndex * (layout.columnWidth + COLUMN_GAP);
        return (
          <div
            key={column.group.id}
            className="pds-atlas__group-label"
            style={{ left: x, width: layout.columnWidth }}
          >
            <span className="pds-atlas__group-title">{column.group.label}</span>
            {column.group.detail ? (
              <span className="pds-atlas__group-detail">{column.group.detail}</span>
            ) : null}
          </div>
        );
      })}

      {renderableNodes.map((node) => {
        const rect = layout.rects.get(node.id);
        if (!rect) return null;
        const isFocus = focusId === node.id;
        const isSelected = selectedId === node.id;
        const isNeighbor = neighbors.has(node.id);
        return (
          <button
            key={node.id}
            type="button"
            data-atlas-node-id={node.id}
            data-accent={node.accent ?? "neutral"}
            className={composeClassNames(
              "pds-atlas__node",
              isFocus && "is-focus",
              isSelected && "is-selected",
              isNeighbor && "is-neighbor",
              focusId && !isFocus && !isNeighbor && "is-dim"
            )}
            style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height }}
            aria-pressed={isSelected}
            onClick={() => onSelectNode?.(isSelected ? null : node.id)}
            onKeyDown={(event) => handleNodeKeyDown(event, node.id)}
            onMouseEnter={() => setHoveredId(node.id)}
            onMouseLeave={() => setHoveredId(null)}
            onFocus={() => setHoveredId(node.id)}
            onBlur={() => setHoveredId(null)}
          >
            <span className="pds-atlas__node-label">{node.label}</span>
            {node.detail ? (
              <span className="pds-atlas__node-detail">{node.detail}</span>
            ) : null}
            {node.kind ? <span className="pds-visually-hidden">{node.kind}</span> : null}
          </button>
        );
      })}
    </div>
  );
}

export type RelationshipAtlasTableProps = {
  caption: string;
  groups: readonly RelationshipAtlasGroup[];
  nodes: readonly RelationshipAtlasNode[];
  rows?: readonly RelationshipAtlasNode[];
  edges: readonly RelationshipAtlasEdge[];
  selectedId?: string | null;
  onSelectNode?: (nodeId: string | null) => void;
  className?: string;
};

/**
 * The accessible tabular equivalent of `RelationshipAtlas`: every node with
 * its group, kind, and direct relationships. Offer it wherever the map is
 * rendered so no relationship information is only available spatially.
 */
export function RelationshipAtlasTable({
  caption,
  groups,
  nodes,
  rows = nodes,
  edges,
  selectedId = null,
  onSelectNode,
  className
}: RelationshipAtlasTableProps) {
  const labels = useMemo(
    () => new Map(nodes.map((node) => [node.id, node.label])),
    [nodes]
  );
  const groupLabels = useMemo(
    () => new Map(groups.map((group) => [group.id, group.label])),
    [groups]
  );

  function relationsFor(nodeId: string): string {
    const related = edges
      .filter((edge) => edge.source === nodeId || edge.target === nodeId)
      .map((edge) => {
        const otherId = edge.source === nodeId ? edge.target : edge.source;
        const direction = edge.source === nodeId ? "→" : "←";
        const kind = edge.kind ? ` (${edge.kind})` : "";
        return `${direction} ${labels.get(otherId) ?? otherId}${kind}`;
      });
    return related.length > 0 ? related.join("; ") : "none recorded";
  }

  return (
    <table className={composeClassNames("pds-atlas-table", className)}>
      <caption>{caption}</caption>
      <thead>
        <tr>
          <th scope="col">Node</th>
          <th scope="col">Group</th>
          <th scope="col">Kind</th>
          <th scope="col">Direct relationships</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((node) => (
          <tr key={node.id} data-selected={selectedId === node.id || undefined}>
            <th scope="row">
              {onSelectNode ? (
                <button
                  type="button"
                  className="pds-atlas-table__node-button"
                  aria-pressed={selectedId === node.id}
                  onClick={() => onSelectNode(selectedId === node.id ? null : node.id)}
                >
                  {node.label}
                </button>
              ) : (
                node.label
              )}
            </th>
            <td>{groupLabels.get(node.group) ?? node.group}</td>
            <td>{node.kind ?? "—"}</td>
            <td>{relationsFor(node.id)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export type RelationshipExplorerView = "map" | "table";

export type RelationshipExplorerProps = Omit<
  HTMLAttributes<HTMLDivElement>,
  "onSelect"
> & {
  ariaLabel: string;
  tableCaption: string;
  groups: readonly RelationshipAtlasGroup[];
  nodes: readonly RelationshipAtlasNode[];
  edges: readonly RelationshipAtlasEdge[];
  selectedId?: string | null;
  onSelectNode?: (nodeId: string | null) => void;
  view?: RelationshipExplorerView;
  defaultView?: RelationshipExplorerView;
  onViewChange?: (view: RelationshipExplorerView) => void;
  mapMinWidth?: number;
  rowHeight?: number;
  /** Bounded rows per page when the accessible table presentation is active. */
  tablePageSize?: number;
  emptyMessage?: ReactNode;
};

/**
 * Responsive map/table composition for relationship data. Products retain
 * ownership of node meaning, edge derivation, citations, and selected state;
 * the design system owns presentation parity and narrow-container fallback.
 */
export function RelationshipExplorer({
  ariaLabel,
  tableCaption,
  groups,
  nodes,
  edges,
  selectedId = null,
  onSelectNode,
  view,
  defaultView = "map",
  onViewChange,
  mapMinWidth = 720,
  rowHeight,
  tablePageSize,
  emptyMessage,
  className,
  ...rest
}: RelationshipExplorerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [internalView, setInternalView] = useState<RelationshipExplorerView>(defaultView);
  const [containerWidth, setContainerWidth] = useState<number | null>(null);
  const [tablePage, setTablePage] = useState(1);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (typeof width === "number" && width > 0) setContainerWidth(width);
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const preferredView = view ?? internalView;
  const mapViable = containerWidth === null || containerWidth >= mapMinWidth;
  const renderedView: RelationshipExplorerView = mapViable ? preferredView : "table";
  const boundedTablePageSize = Number.isSafeInteger(tablePageSize) && Number(tablePageSize) > 0
    ? Math.min(Number(tablePageSize), 100)
    : null;
  const tablePageCount = boundedTablePageSize
    ? Math.max(1, Math.ceil(nodes.length / boundedTablePageSize))
    : 1;
  const effectiveTablePage = Math.min(tablePage, tablePageCount);
  const tableRowStart = boundedTablePageSize
    ? (effectiveTablePage - 1) * boundedTablePageSize
    : 0;
  const tableRows = boundedTablePageSize
    ? nodes.slice(tableRowStart, tableRowStart + boundedTablePageSize)
    : nodes;
  const nodeIdentity = nodes.map((node) => node.id).join("\u0000");

  useEffect(() => {
    setTablePage(1);
  }, [nodeIdentity, boundedTablePageSize]);

  useEffect(() => {
    if (!selectedId || !boundedTablePageSize) return;
    const selectedIndex = nodes.findIndex((node) => node.id === selectedId);
    if (selectedIndex >= 0) setTablePage(Math.floor(selectedIndex / boundedTablePageSize) + 1);
  }, [boundedTablePageSize, nodes, selectedId]);

  function changeView(nextView: RelationshipExplorerView): void {
    if (view === undefined) setInternalView(nextView);
    onViewChange?.(nextView);
  }

  return (
    <div
      {...rest}
      ref={containerRef}
      className={composeClassNames("pds-atlas-explorer", className)}
      data-view={renderedView}
      data-map-viable={mapViable || undefined}
    >
      <div className="pds-atlas-explorer__controls">
        <SegmentedControl<RelationshipExplorerView>
          ariaLabel={`${ariaLabel} presentation`}
          value={renderedView}
          options={[
            { value: "map", label: "Map", disabled: !mapViable },
            { value: "table", label: "Table" }
          ]}
          onValueChange={changeView}
        />
        {!mapViable ? (
          <span className="pds-atlas-explorer__notice">Table view protects readability in this narrow workspace.</span>
        ) : null}
      </div>
      {renderedView === "map" ? (
        <RelationshipAtlas
          ariaLabel={ariaLabel}
          groups={groups}
          nodes={nodes}
          edges={edges}
          selectedId={selectedId}
          onSelectNode={onSelectNode}
          rowHeight={rowHeight}
          emptyMessage={emptyMessage}
        />
      ) : (
        <div className="pds-atlas-explorer__table-scroll">
          <RelationshipAtlasTable
            caption={tableCaption}
            groups={groups}
            nodes={nodes}
            rows={tableRows}
            edges={edges}
            selectedId={selectedId}
            onSelectNode={onSelectNode}
          />
        </div>
      )}
      {renderedView === "table" && boundedTablePageSize && tablePageCount > 1 ? (
        <div className="pds-atlas-explorer__pagination" aria-label={`${ariaLabel} table pages`}>
          <span>
            Rows {tableRowStart + 1}–{Math.min(tableRowStart + tableRows.length, nodes.length)} of {nodes.length}
          </span>
          <div>
            <Button
              size="sm"
              variant="quiet"
              disabled={effectiveTablePage <= 1}
              onClick={() => setTablePage((current) => Math.max(1, current - 1))}
            >
              Previous
            </Button>
            <span>Page {effectiveTablePage} of {tablePageCount}</span>
            <Button
              size="sm"
              variant="quiet"
              disabled={effectiveTablePage >= tablePageCount}
              onClick={() => setTablePage((current) => Math.min(tablePageCount, current + 1))}
            >
              Next
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
