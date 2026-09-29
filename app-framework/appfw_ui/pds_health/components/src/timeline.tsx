import {
  useId,
  useRef,
  useState,
  type HTMLAttributes,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode
} from "react";
import { composeClassNames } from "./types";

type TimelineRangeDragKind = "move" | "resize-start" | "resize-end";

type TimelineRangeDragSession = {
  kind: TimelineRangeDragKind;
  pointerId: number;
  pointerStartX: number;
  startIndex: number;
  width: number;
};

export type TimelineRangeItem = {
  id: string;
  label: string;
  scaleLabel?: ReactNode;
  scaleEdge?: "start" | "middle" | "end";
  density?: number;
};

export type TimelineRangeSelection = {
  startIndex: number;
  width: number;
};

export type TimelineRangeSelectorProps = Omit<
  HTMLAttributes<HTMLDivElement>,
  "onChange"
> & {
  ariaLabel: string;
  items: readonly TimelineRangeItem[];
  startIndex: number;
  width: number;
  selectionLabel: string;
  onSelectionChange: (selection: TimelineRangeSelection) => void;
  snapWidths?: readonly number[];
  title?: ReactNode;
  instructions?: ReactNode;
  widthLabel?: ReactNode;
  startBoundaryLabel?: ReactNode;
  endBoundaryLabel?: ReactNode;
};

/**
 * A single adjustable time-window control. The center moves the selected
 * window while either edge resizes it to a configured snap width.
 */
export function TimelineRangeSelector({
  ariaLabel,
  items,
  startIndex,
  width,
  selectionLabel,
  onSelectionChange,
  snapWidths = [1, 3, 6, 12],
  title = "Adjustable date range",
  instructions = "Drag the selected window to move it. Drag either edge to resize it.",
  widthLabel,
  startBoundaryLabel,
  endBoundaryLabel,
  className,
  ...props
}: TimelineRangeSelectorProps) {
  const instructionsId = useId();
  const trackRef = useRef<HTMLDivElement>(null);
  const dragSessionRef = useRef<TimelineRangeDragSession | null>(null);
  const [dragging, setDragging] = useState<TimelineRangeDragKind | null>(null);
  const totalItems = items.length;
  const normalizedWidth = clamp(width, 1, Math.max(1, totalItems));
  const maxStartIndex = Math.max(0, totalItems - normalizedWidth);
  const normalizedStartIndex = clamp(startIndex, 0, maxStartIndex);
  const endIndex = normalizedStartIndex + normalizedWidth - 1;
  const availableWidths = normalizeSnapWidths(snapWidths, totalItems);
  const maximumDensity = Math.max(
    1,
    ...items.map((item) => Math.max(0, item.density ?? 0))
  );
  const left =
    totalItems === 0 ? 0 : (normalizedStartIndex / totalItems) * 100;
  const selectionWidth =
    totalItems === 0 ? 0 : (normalizedWidth / totalItems) * 100;

  function emitSelection(nextStartIndex: number, nextWidth: number) {
    const resolvedWidth = clamp(nextWidth, 1, Math.max(1, totalItems));
    onSelectionChange({
      startIndex: clamp(
        nextStartIndex,
        0,
        Math.max(0, totalItems - resolvedWidth)
      ),
      width: resolvedWidth
    });
  }

  function beginPointerDrag(
    kind: TimelineRangeDragKind,
    event: ReactPointerEvent<HTMLElement>
  ) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragSessionRef.current = {
      kind,
      pointerId: event.pointerId,
      pointerStartX: event.clientX,
      startIndex: normalizedStartIndex,
      width: normalizedWidth
    };
    setDragging(kind);
  }

  function handlePointerMove(event: ReactPointerEvent<HTMLElement>) {
    const session = dragSessionRef.current;
    const track = trackRef.current;
    if (
      !session ||
      session.pointerId !== event.pointerId ||
      !track ||
      totalItems === 0
    ) {
      return;
    }

    const trackWidth = track.getBoundingClientRect().width;
    if (trackWidth <= 0) return;
    const deltaItems = Math.round(
      ((event.clientX - session.pointerStartX) / trackWidth) * totalItems
    );

    if (session.kind === "move") {
      emitSelection(session.startIndex + deltaItems, session.width);
      return;
    }

    if (session.kind === "resize-start") {
      const fixedEnd = session.startIndex + session.width;
      const widths = availableWidths.filter((option) => option <= fixedEnd);
      const nextWidth = nearestWidth(session.width - deltaItems, widths);
      emitSelection(fixedEnd - nextWidth, nextWidth);
      return;
    }

    const widths = availableWidths.filter(
      (option) => option <= totalItems - session.startIndex
    );
    const nextWidth = nearestWidth(session.width + deltaItems, widths);
    emitSelection(session.startIndex, nextWidth);
  }

  function finishPointerDrag(event: ReactPointerEvent<HTMLElement>) {
    if (dragSessionRef.current?.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    dragSessionRef.current = null;
    setDragging(null);
  }

  function repositionFromTrack(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || totalItems === 0) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const pointerRatio = clamp(
      (event.clientX - bounds.left) / bounds.width,
      0,
      1
    );
    const centerBoundary = Math.round(pointerRatio * totalItems);
    emitSelection(
      centerBoundary - Math.floor(normalizedWidth / 2),
      normalizedWidth
    );
  }

  function handleWindowKeyDown(event: KeyboardEvent<HTMLElement>) {
    let nextStart = normalizedStartIndex;
    if (event.key === "ArrowLeft") nextStart -= 1;
    else if (event.key === "ArrowRight") nextStart += 1;
    else if (event.key === "PageDown") nextStart -= 3;
    else if (event.key === "PageUp") nextStart += 3;
    else if (event.key === "Home") nextStart = 0;
    else if (event.key === "End") nextStart = maxStartIndex;
    else return;

    event.preventDefault();
    emitSelection(nextStart, normalizedWidth);
  }

  function handleEdgeKeyDown(
    edge: "start" | "end",
    event: KeyboardEvent<HTMLElement>
  ) {
    const fixedEnd = normalizedStartIndex + normalizedWidth;
    const maximumWidth =
      edge === "start" ? fixedEnd : totalItems - normalizedStartIndex;
    const widths = availableWidths.filter(
      (option) => option <= maximumWidth
    );
    const currentOptionIndex = Math.max(
      0,
      widths.indexOf(nearestWidth(normalizedWidth, widths))
    );
    let nextOptionIndex = currentOptionIndex;

    if (edge === "start") {
      if (event.key === "ArrowLeft") nextOptionIndex += 1;
      else if (event.key === "ArrowRight") nextOptionIndex -= 1;
      else if (event.key === "Home") nextOptionIndex = widths.length - 1;
      else if (event.key === "End") nextOptionIndex = 0;
      else return;
    } else {
      if (event.key === "ArrowLeft") nextOptionIndex -= 1;
      else if (event.key === "ArrowRight") nextOptionIndex += 1;
      else if (event.key === "Home") nextOptionIndex = 0;
      else if (event.key === "End") nextOptionIndex = widths.length - 1;
      else return;
    }

    event.preventDefault();
    const nextWidth =
      widths[clamp(nextOptionIndex, 0, widths.length - 1)] ?? normalizedWidth;
    emitSelection(
      edge === "start" ? fixedEnd - nextWidth : normalizedStartIndex,
      nextWidth
    );
  }

  if (totalItems === 0) return null;

  const sharedPointerHandlers = {
    onPointerMove: handlePointerMove,
    onPointerUp: finishPointerDrag,
    onPointerCancel: finishPointerDrag
  };
  const startItem = items[normalizedStartIndex];
  const endItem = items[endIndex];

  return (
    <div
      {...props}
      className={composeClassNames("pds-timeline-range", className)}
      role="group"
      aria-label={ariaLabel}
      data-dragging={dragging ?? undefined}
    >
      <div className="pds-timeline-range__summary">
        <strong>{title}</strong>
        <output aria-live="polite" aria-atomic="true">
          {selectionLabel}
          {widthLabel ? <span>{widthLabel}</span> : null}
        </output>
      </div>
      <p id={instructionsId} className="pds-timeline-range__instructions">
        {instructions}
      </p>

      <div className="pds-timeline-range__viewport">
        <div className="pds-timeline-range__scale" aria-hidden="true">
          {items.map((item, index) =>
            item.scaleLabel ? (
              <span
                key={item.id}
                data-edge={item.scaleEdge ?? "middle"}
                style={{
                  left: `${(index / Math.max(1, totalItems - 1)) * 100}%`
                }}
              >
                {item.scaleLabel}
              </span>
            ) : null
          )}
        </div>

        <div
          ref={trackRef}
          className="pds-timeline-range__track"
          onPointerDown={repositionFromTrack}
        >
          <span className="pds-timeline-range__rail" aria-hidden="true" />
          <span className="pds-timeline-range__density" aria-hidden="true">
            {items.map((item, index) => {
              const density = Math.max(0, item.density ?? 0);
              return (
                <span
                  key={item.id}
                  data-populated={density > 0 || undefined}
                  style={{
                    left: `${((index + 0.5) / totalItems) * 100}%`,
                    height: `${4 + Math.round(
                      (density / maximumDensity) * 12
                    )}px`
                  }}
                />
              );
            })}
          </span>

          <span
            className="pds-timeline-range__selection"
            data-dragging={dragging ?? undefined}
            style={{ left: `${left}%`, width: `${selectionWidth}%` }}
          >
            <span
              className="pds-timeline-range__handle pds-timeline-range__handle--start"
              role="slider"
              tabIndex={0}
              aria-label={`${ariaLabel} start`}
              aria-valuemin={0}
              aria-valuemax={endIndex}
              aria-valuenow={normalizedStartIndex}
              aria-valuetext={`${startItem?.label ?? ""}. ${selectionLabel}.`}
              aria-describedby={instructionsId}
              onKeyDown={(event) => handleEdgeKeyDown("start", event)}
              onPointerDown={(event) => beginPointerDrag("resize-start", event)}
              {...sharedPointerHandlers}
            />
            <span
              className="pds-timeline-range__window"
              role="slider"
              tabIndex={0}
              aria-label={`${ariaLabel} position`}
              aria-valuemin={0}
              aria-valuemax={maxStartIndex}
              aria-valuenow={normalizedStartIndex}
              aria-valuetext={selectionLabel}
              aria-describedby={instructionsId}
              onKeyDown={handleWindowKeyDown}
              onPointerDown={(event) => beginPointerDrag("move", event)}
              {...sharedPointerHandlers}
            />
            <span
              className="pds-timeline-range__handle pds-timeline-range__handle--end"
              role="slider"
              tabIndex={0}
              aria-label={`${ariaLabel} end`}
              aria-valuemin={normalizedStartIndex}
              aria-valuemax={totalItems - 1}
              aria-valuenow={endIndex}
              aria-valuetext={`${endItem?.label ?? ""}. ${selectionLabel}.`}
              aria-describedby={instructionsId}
              onKeyDown={(event) => handleEdgeKeyDown("end", event)}
              onPointerDown={(event) => beginPointerDrag("resize-end", event)}
              {...sharedPointerHandlers}
            />
          </span>
        </div>

        <div className="pds-timeline-range__bounds" aria-hidden="true">
          <span>{startBoundaryLabel ?? items[0]?.label}</span>
          <span>{endBoundaryLabel ?? items[totalItems - 1]?.label}</span>
        </div>
      </div>
    </div>
  );
}

function normalizeSnapWidths(
  snapWidths: readonly number[],
  totalItems: number
): number[] {
  const available = [...new Set(snapWidths)]
    .filter((value) => Number.isInteger(value) && value > 0 && value <= totalItems)
    .sort((left, right) => left - right);
  return available.length > 0 ? available : [Math.max(1, totalItems)];
}

function nearestWidth(candidate: number, widths: readonly number[]): number {
  const [first = 1, ...rest] = widths;
  return rest.reduce(
    (nearest, width) =>
      Math.abs(width - candidate) < Math.abs(nearest - candidate)
        ? width
        : nearest,
    first
  );
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}
