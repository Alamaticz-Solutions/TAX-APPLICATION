import {
  useEffect,
  useId,
  useRef,
  type HTMLAttributes,
  type ReactNode
} from "react";
import { composeClassNames } from "./types";

export type NarrativeWorkspaceProps = Omit<
  HTMLAttributes<HTMLDivElement>,
  "children"
> & {
  brand: ReactNode;
  primaryNavigation: ReactNode;
  primaryNavigationLabel?: string;
  utilities?: ReactNode;
  contextBand?: ReactNode;
  contextBandLabel?: string;
  footer?: ReactNode;
  mainId?: string;
  skipLinkLabel?: string;
  children: ReactNode;
};

/**
 * A document-scrolling floor plan for long-form, role-aware knowledge work.
 *
 * PDS owns the sticky banner, floating horizontal navigation, labelled
 * regions, sticky-offset publication, responsive reflow, and skip target.
 * Products own navigation controls, labels, routes or modes, current state,
 * context semantics, content anchors, history, and focus restoration.
 * `primaryNavigation` must contain navigation controls, never another nav.
 */
export function NarrativeWorkspace({
  brand,
  primaryNavigation,
  primaryNavigationLabel = "Primary navigation",
  utilities,
  contextBand,
  contextBandLabel = "Workspace context",
  footer,
  mainId,
  skipLinkLabel = "Skip to main content",
  children,
  className,
  ...props
}: NarrativeWorkspaceProps) {
  const generatedId = useId().replace(/:/g, "");
  const resolvedMainId = mainId ?? `pds-narrative-main-${generatedId}`;
  const rootRef = useRef<HTMLDivElement>(null);
  const stickyHeaderRef = useRef<HTMLElement>(null);
  const contextBandRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const root = rootRef.current;
    const header = stickyHeaderRef.current;
    if (!root || !header) return undefined;

    const updateMeasurements = () => {
      root.style.setProperty(
        "--pds-narrative-header-size",
        `${Math.ceil(header.getBoundingClientRect().height)}px`
      );
      root.style.setProperty(
        "--pds-narrative-context-size",
        `${Math.ceil(contextBandRef.current?.getBoundingClientRect().height ?? 0)}px`
      );
    };

    updateMeasurements();
    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", updateMeasurements);
      return () => window.removeEventListener("resize", updateMeasurements);
    }

    const observer = new ResizeObserver(updateMeasurements);
    observer.observe(header);
    if (contextBandRef.current) observer.observe(contextBandRef.current);
    return () => observer.disconnect();
  }, [contextBand]);

  return (
    <div
      {...props}
      ref={rootRef}
      className={composeClassNames("pds-narrative-workspace", className)}
      data-has-utilities={Boolean(utilities) || undefined}
      data-has-context={Boolean(contextBand) || undefined}
      data-has-footer={Boolean(footer) || undefined}
    >
      <a className="pds-narrative-workspace__skip-link" href={`#${resolvedMainId}`}>
        {skipLinkLabel}
      </a>

      <header
        ref={stickyHeaderRef}
        className="pds-narrative-workspace__sticky-header"
      >
        <div className="pds-narrative-workspace__brand">{brand}</div>
        <div className="pds-narrative-workspace__dock">
          <nav
            className="pds-narrative-workspace__navigation"
            aria-label={primaryNavigationLabel}
          >
            {primaryNavigation}
          </nav>
        </div>
        {utilities ? (
          <div className="pds-narrative-workspace__utilities">{utilities}</div>
        ) : null}
      </header>

      {contextBand ? (
        <section
          ref={contextBandRef}
          className="pds-narrative-workspace__context"
          aria-label={contextBandLabel}
        >
          {contextBand}
        </section>
      ) : null}

      <main
        id={resolvedMainId}
        className="pds-narrative-workspace__main"
        tabIndex={-1}
      >
        {children}
      </main>

      {footer ? (
        <footer className="pds-narrative-workspace__footer">{footer}</footer>
      ) : null}
    </div>
  );
}
