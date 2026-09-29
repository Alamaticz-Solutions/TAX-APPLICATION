import type { HTMLAttributes, ReactNode } from "react";
import { useId } from "react";
import { composeClassNames, describedBy } from "./types";

export type ExplorationWorkspacePresentation = "bounded" | "flow";

export type ExplorationWorkspaceProps = Omit<
  HTMLAttributes<HTMLElement>,
  "children" | "title"
> & {
  title?: ReactNode;
  description?: ReactNode;
  headerActions?: ReactNode;
  controls?: ReactNode;
  controlsLabel?: string;
  navigator?: ReactNode;
  navigatorLabel?: string;
  focusRegion: ReactNode;
  focusRegionLabel?: string;
  inspector?: ReactNode;
  inspectorLabel?: string;
  evidenceRail?: ReactNode;
  evidenceRailLabel?: string;
  presentation?: ExplorationWorkspacePresentation;
  ariaLabel?: string;
};

/**
 * A product-neutral floor plan for exploring a dense, object-oriented system.
 *
 * PDS owns the responsive named regions, bounded desktop presentation, and
 * focusable scroll surfaces. Products own object meaning, data, access,
 * selection, filtering, relationships, evidence, citations, and URL state.
 */
export function ExplorationWorkspace({
  title,
  description,
  headerActions,
  controls,
  controlsLabel = "Exploration controls",
  navigator,
  navigatorLabel = "Explore objects",
  focusRegion,
  focusRegionLabel = "Exploration focus",
  inspector,
  inspectorLabel = "Selected object details",
  evidenceRail,
  evidenceRailLabel = "Exploration evidence",
  presentation = "bounded",
  ariaLabel,
  id,
  className,
  "aria-label": nativeAriaLabel,
  "aria-labelledby": nativeAriaLabelledBy,
  "aria-describedby": nativeAriaDescribedBy,
  ...props
}: ExplorationWorkspaceProps) {
  const generatedId = useId();
  const rootId = id ?? `pds-exploration-workspace-${generatedId}`;
  const titleId = title ? `${rootId}-title` : undefined;
  const descriptionId = description ? `${rootId}-description` : undefined;
  const resolvedAriaLabel =
    ariaLabel ?? nativeAriaLabel ?? (title ? undefined : "Exploration workspace");
  const resolvedAriaLabelledBy =
    nativeAriaLabelledBy ?? (resolvedAriaLabel ? undefined : titleId);

  return (
    <section
      {...props}
      id={rootId}
      className={composeClassNames("pds-exploration-workspace", className)}
      data-presentation={presentation}
      data-has-controls={Boolean(controls) || undefined}
      data-has-navigator={Boolean(navigator) || undefined}
      data-has-inspector={Boolean(inspector) || undefined}
      data-has-evidence={Boolean(evidenceRail) || undefined}
      aria-label={resolvedAriaLabel}
      aria-labelledby={resolvedAriaLabelledBy}
      aria-describedby={describedBy(nativeAriaDescribedBy, descriptionId)}
    >
      {title || description || headerActions ? (
        <header className="pds-exploration-workspace__header">
          <div className="pds-exploration-workspace__heading">
            {title ? (
              <h2 id={titleId} className="pds-exploration-workspace__title">
                {title}
              </h2>
            ) : null}
            {description ? (
              <p
                id={descriptionId}
                className="pds-exploration-workspace__description"
              >
                {description}
              </p>
            ) : null}
          </div>
          {headerActions ? (
            <div className="pds-exploration-workspace__header-actions">
              {headerActions}
            </div>
          ) : null}
        </header>
      ) : null}

      {controls ? (
        <section
          className="pds-exploration-workspace__controls"
          aria-label={controlsLabel}
        >
          {controls}
        </section>
      ) : null}

      <div className="pds-exploration-workspace__body">
        {navigator ? (
          <nav
            className="pds-exploration-workspace__navigator"
            aria-label={navigatorLabel}
            tabIndex={0}
          >
            {navigator}
          </nav>
        ) : null}

        <section
          className="pds-exploration-workspace__focus"
          aria-label={focusRegionLabel}
          tabIndex={0}
        >
          {focusRegion}
        </section>

        {inspector ? (
          <aside
            className="pds-exploration-workspace__inspector"
            aria-label={inspectorLabel}
            tabIndex={0}
          >
            {inspector}
          </aside>
        ) : null}
      </div>

      {evidenceRail ? (
        <aside
          className="pds-exploration-workspace__evidence"
          aria-label={evidenceRailLabel}
          tabIndex={0}
        >
          {evidenceRail}
        </aside>
      ) : null}
    </section>
  );
}
