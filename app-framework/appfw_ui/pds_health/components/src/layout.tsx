import {
  cloneElement,
  Fragment,
  forwardRef,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type HTMLAttributes,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactElement,
  type ReactNode,
  type RefObject
} from "react";
import { createPortal } from "react-dom";
import {
  Dialog as AriaDialog,
  Modal as AriaModal,
  ModalOverlay as AriaModalOverlay,
  Tab as AriaTab,
  TabList as AriaTabList,
  TabPanel as AriaTabPanel,
  Tabs as AriaTabs
} from "react-aria-components";
import { Button } from "./primitives";
import { composeClassNames, describedBy } from "./types";

type NativePopoverElement = HTMLDivElement & {
  hidePopover?: () => void;
  showPopover?: () => void;
};

const nativePopoverProps = { popover: "auto" } as Record<string, string>;
const focusableSelector = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex=\"-1\"])"
].join(",");

function focusableElements(container: HTMLElement | null) {
  if (!container) return [];
  return Array.from(container.querySelectorAll<HTMLElement>(focusableSelector))
    .filter((element) => !element.hasAttribute("hidden") && element.tabIndex >= 0);
}

function useOverlayFocus(open: boolean, containerRef: RefObject<HTMLElement | null>) {
  useEffect(() => {
    if (!open) return undefined;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const animation = requestAnimationFrame(() => {
      const target = focusableElements(containerRef.current)[0] ?? containerRef.current;
      target?.focus();
    });
    return () => {
      cancelAnimationFrame(animation);
      if (previousFocus && document.contains(previousFocus)) {
        window.setTimeout(() => {
          if (document.contains(previousFocus)) previousFocus.focus();
        }, 0);
      }
    };
  }, [containerRef, open]);
}

function trapOverlayFocus(event: ReactKeyboardEvent<HTMLElement>, container: HTMLElement | null) {
  if (event.key !== "Tab") return;
  const focusable = focusableElements(container);
  if (!focusable.length) {
    event.preventDefault();
    container?.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

export type AppShellProps = HTMLAttributes<HTMLDivElement> & {
  brand: ReactNode;
  navigation: ReactNode;
  topBar?: ReactNode;
  footer?: ReactNode;
  navigationLabel?: string;
  responsiveCollapse?: boolean;
  viewportBounded?: boolean;
  sidebarResizable?: boolean;
  sidebarWidth?: number;
  defaultSidebarWidth?: number;
  sidebarMinWidth?: number;
  sidebarMaxWidth?: number;
  sidebarMaxViewportRatio?: number;
  sidebarResizeStep?: number;
  sidebarResizeLabel?: string;
  onSidebarWidthChange?: (width: number) => void;
  onSidebarWidthCommit?: (width: number) => void;
  children: ReactNode;
};

function clampNumber(value: number, minimum: number, maximum: number) {
  return Math.min(Math.max(value, minimum), maximum);
}

export function AppShell({
  brand,
  navigation,
  topBar,
  footer,
  navigationLabel = "Primary navigation",
  responsiveCollapse = false,
  viewportBounded = false,
  sidebarResizable = false,
  sidebarWidth,
  defaultSidebarWidth = 288,
  sidebarMinWidth = 240,
  sidebarMaxWidth = 360,
  sidebarMaxViewportRatio = 0.35,
  sidebarResizeStep = 8,
  sidebarResizeLabel = "Resize primary navigation",
  onSidebarWidthChange,
  onSidebarWidthCommit,
  children,
  className,
  style,
  ...props
}: AppShellProps) {
  const [uncontrolledSidebarWidth, setUncontrolledSidebarWidth] = useState(
    defaultSidebarWidth
  );
  const [viewportWidth, setViewportWidth] = useState(() =>
    typeof window === "undefined" ? 1440 : window.innerWidth
  );
  const dragStateRef = useRef<{ startX: number; startWidth: number } | null>(
    null
  );
  const preferredSidebarWidth = sidebarWidth ?? uncontrolledSidebarWidth;
  const effectiveSidebarMaximum = Math.max(
    sidebarMinWidth,
    Math.min(sidebarMaxWidth, viewportWidth * sidebarMaxViewportRatio)
  );
  const effectiveSidebarWidth = clampNumber(
    preferredSidebarWidth,
    sidebarMinWidth,
    effectiveSidebarMaximum
  );
  const effectiveSidebarWidthRef = useRef(effectiveSidebarWidth);
  effectiveSidebarWidthRef.current = effectiveSidebarWidth;

  useEffect(() => {
    if (!sidebarResizable || typeof window === "undefined") return undefined;
    const updateViewportWidth = () => setViewportWidth(window.innerWidth);
    updateViewportWidth();
    window.addEventListener("resize", updateViewportWidth);
    return () => window.removeEventListener("resize", updateViewportWidth);
  }, [sidebarResizable]);

  useEffect(() => {
    if (sidebarWidth === undefined && effectiveSidebarWidth !== uncontrolledSidebarWidth) {
      setUncontrolledSidebarWidth(effectiveSidebarWidth);
    }
  }, [
    effectiveSidebarWidth,
    sidebarWidth,
    uncontrolledSidebarWidth
  ]);

  const updateSidebarWidth = useCallback(
    (nextWidth: number) => {
      const clampedWidth = clampNumber(
        nextWidth,
        sidebarMinWidth,
        effectiveSidebarMaximum
      );
      effectiveSidebarWidthRef.current = clampedWidth;
      if (sidebarWidth === undefined) setUncontrolledSidebarWidth(clampedWidth);
      onSidebarWidthChange?.(clampedWidth);
    },
    [
      effectiveSidebarMaximum,
      onSidebarWidthChange,
      sidebarMinWidth,
      sidebarWidth
    ]
  );

  const finishSidebarResize = useCallback(() => {
    dragStateRef.current = null;
    if (typeof document !== "undefined") {
      document.body.style.removeProperty("cursor");
      document.body.style.removeProperty("user-select");
    }
    onSidebarWidthCommit?.(effectiveSidebarWidthRef.current);
  }, [onSidebarWidthCommit]);

  const handleResizePointerDown = (
    event: ReactPointerEvent<HTMLDivElement>
  ) => {
    event.preventDefault();
    dragStateRef.current = {
      startX: event.clientX,
      startWidth: effectiveSidebarWidthRef.current
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const handleResizePointerMove = (
    event: ReactPointerEvent<HTMLDivElement>
  ) => {
    const dragState = dragStateRef.current;
    if (!dragState) return;
    updateSidebarWidth(dragState.startWidth + event.clientX - dragState.startX);
  };

  const handleResizePointerEnd = (
    event: ReactPointerEvent<HTMLDivElement>
  ) => {
    if (!dragStateRef.current) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    finishSidebarResize();
  };

  const handleResizeKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    let nextWidth: number | undefined;
    if (event.key === "ArrowLeft") {
      nextWidth = effectiveSidebarWidth - sidebarResizeStep;
    } else if (event.key === "ArrowRight") {
      nextWidth = effectiveSidebarWidth + sidebarResizeStep;
    } else if (event.key === "Home") {
      nextWidth = sidebarMinWidth;
    } else if (event.key === "End") {
      nextWidth = effectiveSidebarMaximum;
    }
    if (nextWidth === undefined) return;
    event.preventDefault();
    updateSidebarWidth(nextWidth);
    onSidebarWidthCommit?.(effectiveSidebarWidthRef.current);
  };

  const shellStyle = sidebarResizable
    ? ({
        ...style,
        "--pds-app-shell-sidebar-width": `${effectiveSidebarWidth}px`
      } as CSSProperties)
    : style;

  return (
    <div
      {...props}
      className={composeClassNames("pds-app-shell", className)}
      data-responsive-collapse={responsiveCollapse || undefined}
      data-viewport-bounded={viewportBounded || undefined}
      data-sidebar-resizable={sidebarResizable || undefined}
      style={shellStyle}
    >
      <aside className="pds-app-shell__sidebar" aria-label={navigationLabel}>
        <div className="pds-app-shell__brand">{brand}</div>
        <nav className="pds-app-shell__nav">{navigation}</nav>
        {footer ? <div className="pds-app-shell__footer">{footer}</div> : null}
      </aside>
      {sidebarResizable ? (
        <div
          className="pds-app-shell__sidebar-resizer"
          role="separator"
          aria-label={sidebarResizeLabel}
          aria-orientation="vertical"
          aria-valuemin={Math.round(sidebarMinWidth)}
          aria-valuemax={Math.round(effectiveSidebarMaximum)}
          aria-valuenow={Math.round(effectiveSidebarWidth)}
          aria-valuetext={`${Math.round(effectiveSidebarWidth)} pixels`}
          tabIndex={0}
          onDoubleClick={() => {
            updateSidebarWidth(defaultSidebarWidth);
            onSidebarWidthCommit?.(effectiveSidebarWidthRef.current);
          }}
          onKeyDown={handleResizeKeyDown}
          onPointerDown={handleResizePointerDown}
          onPointerMove={handleResizePointerMove}
          onPointerUp={handleResizePointerEnd}
          onPointerCancel={handleResizePointerEnd}
        />
      ) : null}
      <div className="pds-app-shell__workspace">
        {topBar ? <header className="pds-app-shell__topbar">{topBar}</header> : null}
        <main
          className="pds-app-shell__main"
          tabIndex={viewportBounded ? 0 : undefined}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

export type BreadcrumbItem = {
  id: string;
  label: ReactNode;
  href?: string;
  current?: boolean;
};

export type BreadcrumbsProps = HTMLAttributes<HTMLElement> & {
  items: readonly BreadcrumbItem[];
  ariaLabel?: string;
  separator?: ReactNode;
  onNavigate?: (href: string) => void;
};

export function Breadcrumbs({
  items,
  ariaLabel = "Breadcrumb",
  separator = "›",
  onNavigate,
  className,
  ...props
}: BreadcrumbsProps) {
  return (
    <nav
      {...props}
      className={composeClassNames("pds-breadcrumbs", className)}
      aria-label={ariaLabel}
    >
      <ol className="pds-breadcrumbs__list">
        {items.map((item, index) => {
          const current = item.current ?? index === items.length - 1;
          const href = item.href;
          return (
            <li className="pds-breadcrumbs__item" key={item.id}>
              {index > 0 ? (
                <span className="pds-breadcrumbs__separator" aria-hidden="true">
                  {separator}
                </span>
              ) : null}
              {href && !current ? (
                <a
                  className="pds-breadcrumbs__link"
                  href={href}
                  onClick={(event) => {
                    if (
                      !onNavigate
                      || event.button !== 0
                      || event.metaKey
                      || event.ctrlKey
                      || event.shiftKey
                      || event.altKey
                    ) {
                      return;
                    }
                    event.preventDefault();
                    onNavigate(href);
                  }}
                >
                  {item.label}
                </a>
              ) : (
                <span
                  className="pds-breadcrumbs__current"
                  aria-current={current ? "page" : undefined}
                >
                  {item.label}
                </span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export type PageHeaderProps = HTMLAttributes<HTMLDivElement> & {
  title: string;
  subtitle?: ReactNode;
  eyebrow?: string;
  actions?: ReactNode;
};

export function PageHeader({
  title,
  subtitle,
  eyebrow,
  actions,
  className,
  ...props
}: PageHeaderProps) {
  return (
    <div {...props} className={composeClassNames("pds-page-header", className)}>
      <div className="pds-page-header__copy">
        {eyebrow ? <p className="pds-page-header__eyebrow">{eyebrow}</p> : null}
        <h1 className="pds-page-header__title">{title}</h1>
        {subtitle ? <p className="pds-page-header__subtitle">{subtitle}</p> : null}
      </div>
      {actions ? <div className="pds-page-header__actions">{actions}</div> : null}
    </div>
  );
}

export type CommandBarProps = HTMLAttributes<HTMLDivElement> & {
  primaryAction?: ReactNode;
  secondaryActions?: ReactNode;
  filters?: ReactNode;
  resultSummary?: ReactNode;
};

export function CommandBar({
  primaryAction,
  secondaryActions,
  filters,
  resultSummary,
  className,
  ...props
}: CommandBarProps) {
  return (
    <div {...props} className={composeClassNames("pds-command-bar", className)}>
      <div className="pds-command-bar__filters">{filters}</div>
      <div className="pds-command-bar__summary" aria-live="polite">
        {resultSummary}
      </div>
      <div className="pds-command-bar__actions">
        {secondaryActions}
        {primaryAction}
      </div>
    </div>
  );
}

export type CommandPaletteItem = {
  id: string;
  group: string;
  label: string;
  detail?: ReactNode;
  href?: string;
  icon?: ReactNode;
  disabled?: boolean;
  keywords?: readonly string[];
  onSelect?: () => void;
};

export type CommandPaletteItemRenderProps = {
  className: string;
  "aria-disabled"?: true;
  onClick: (event: MouseEvent<HTMLElement>) => void;
};

export type CommandPaletteProps = HTMLAttributes<HTMLDivElement> & {
  items: readonly CommandPaletteItem[];
  triggerLabel?: ReactNode;
  triggerIcon?: ReactNode;
  searchLabel?: string;
  searchPlaceholder?: string;
  shortcutLabel?: ReactNode;
  emptyMessage?: ReactNode;
  maxResults?: number;
  clearOnSelect?: boolean;
  enableGlobalShortcut?: boolean;
  renderItem?: (
    item: CommandPaletteItem,
    children: ReactNode,
    props: CommandPaletteItemRenderProps
  ) => ReactNode;
};

export function CommandPalette({
  items,
  triggerLabel = "Search workspace",
  triggerIcon,
  searchLabel = "Search workspace commands",
  searchPlaceholder = "Search commands",
  shortcutLabel = "Ctrl K",
  emptyMessage = "No matching commands.",
  maxResults = 12,
  clearOnSelect = true,
  enableGlobalShortcut = true,
  renderItem,
  className,
  ...props
}: CommandPaletteProps) {
  const id = useId();
  const paletteId = `${id}-palette`;
  const searchId = `${id}-search`;
  const popoverRef = useRef<NativePopoverElement | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  const filteredItems = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const matches = needle
      ? items.filter((item) => commandPaletteSearchText(item).includes(needle))
      : items;
    return matches.slice(0, maxResults);
  }, [items, maxResults, query]);

  const closePalette = useCallback(() => {
    const popover = popoverRef.current;
    if (popover?.hidePopover && popover.matches(":popover-open")) {
      popover.hidePopover();
    }
    setOpen(false);
    if (clearOnSelect) setQuery("");
  }, [clearOnSelect]);

  const openPalette = useCallback(() => {
    const popover = popoverRef.current;
    if (popover?.showPopover && !popover.matches(":popover-open")) {
      popover.showPopover();
    }
    setOpen(true);
    requestAnimationFrame(() => searchRef.current?.focus());
  }, []);

  const togglePalette = useCallback(() => {
    if (open) {
      closePalette();
    } else {
      openPalette();
    }
  }, [closePalette, open, openPalette]);

  useEffect(() => {
    const popover = popoverRef.current;
    const syncOpen = () => setOpen(Boolean(popover?.matches(":popover-open")));
    popover?.addEventListener("toggle", syncOpen);
    return () => popover?.removeEventListener("toggle", syncOpen);
  }, []);

  useEffect(() => {
    if (!open) return undefined;
    const animation = requestAnimationFrame(() => searchRef.current?.focus());
    return () => cancelAnimationFrame(animation);
  }, [open]);

  useEffect(() => {
    if (!enableGlobalShortcut) return undefined;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "k") return;
      event.preventDefault();
      openPalette();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [enableGlobalShortcut, openPalette]);

  const renderCommandItem = (item: CommandPaletteItem) => {
    const children = (
      <>
        <span className="pds-command-palette__icon" aria-hidden="true">
          {item.icon}
        </span>
        <span className="pds-command-palette__copy">
          <strong>{item.label}</strong>
          {item.detail ? <small>{item.detail}</small> : null}
        </span>
        <em>{item.group}</em>
      </>
    );
    const itemProps: CommandPaletteItemRenderProps = {
      className: "pds-command-palette__item",
      "aria-disabled": item.disabled ? true : undefined,
      onClick: (event) => {
        if (item.disabled) {
          event.preventDefault();
          return;
        }
        item.onSelect?.();
        closePalette();
      }
    };
    if (renderItem) return renderItem(item, children, itemProps);
    if (item.href) {
      return (
        <a href={item.href} {...itemProps}>
          {children}
        </a>
      );
    }
    return (
      <button type="button" {...itemProps}>
        {children}
      </button>
    );
  };

  return (
    <div {...props} className={composeClassNames("pds-command-palette", className)}>
      <button
        type="button"
        className="pds-command-palette__trigger"
        aria-expanded={open}
        aria-controls={paletteId}
        onClick={togglePalette}
      >
        <span className="pds-command-palette__trigger-label">
          {triggerIcon ? (
            <span className="pds-command-palette__trigger-icon" aria-hidden="true">
              {triggerIcon}
            </span>
          ) : null}
          <span>{triggerLabel}</span>
        </span>
        {shortcutLabel ? <kbd className="pds-command-palette__shortcut">{shortcutLabel}</kbd> : null}
      </button>
      <div
        id={paletteId}
        ref={popoverRef}
        className="pds-command-palette__popover"
        data-open={open ? "true" : undefined}
        role="dialog"
        aria-label={searchLabel}
        {...nativePopoverProps}
        onKeyDown={(event) => {
          if (event.key === "Escape") closePalette();
        }}
      >
        <div className="pds-command-palette__search">
          <input
            id={searchId}
            ref={searchRef}
            type="search"
            aria-label={searchLabel}
            value={query}
            placeholder={searchPlaceholder}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <div className="pds-command-palette__list">
          {filteredItems.map((item) => (
            <Fragment key={item.id}>{renderCommandItem(item)}</Fragment>
          ))}
          {filteredItems.length === 0 ? (
            <div className="pds-command-palette__empty">{emptyMessage}</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function commandPaletteSearchText(item: CommandPaletteItem) {
  const values = [item.group, item.label, ...(item.keywords ?? [])];
  if (typeof item.detail === "string" || typeof item.detail === "number") {
    values.push(String(item.detail));
  }
  return values.join(" ").toLowerCase();
}

export type TabItem = {
  id: string;
  label: string;
  content: ReactNode;
  disabled?: boolean;
};

export type TabsProps = {
  items: readonly TabItem[];
  selectedId: string;
  onChange: (id: string) => void;
  ariaLabel: string;
};

export function Tabs({ items, selectedId, onChange, ariaLabel }: TabsProps) {
  return (
    <AriaTabs
      className="pds-tabs"
      selectedKey={selectedId}
      onSelectionChange={(key) => onChange(String(key))}
    >
      <AriaTabList className="pds-tabs__list" aria-label={ariaLabel}>
        {items.map((item) => (
          <AriaTab
            key={item.id}
            id={item.id}
            className="pds-tabs__tab"
            isDisabled={item.disabled}
          >
            {item.label}
          </AriaTab>
        ))}
      </AriaTabList>
      {items.map((item) => (
        <AriaTabPanel
          key={item.id}
          id={item.id}
          className="pds-tabs__panel"
        >
          {item.content}
        </AriaTabPanel>
      ))}
    </AriaTabs>
  );
}

export type DialogProps = {
  open: boolean;
  title: string;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  closeLabel?: string;
  role?: "dialog" | "alertdialog";
  size?: "sm" | "md" | "lg";
  className?: string;
};

export function Dialog({
  open,
  title,
  description,
  children,
  footer,
  onClose,
  closeLabel = "Close dialog",
  role = "dialog",
  size = "md",
  className
}: DialogProps) {
  const id = useId();
  const titleId = `${id}-title`;
  const descriptionId = description ? `${id}-description` : undefined;
  return (
    <AriaModalOverlay
      className="pds-dialog-backdrop"
      isOpen={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) onClose();
      }}
      isDismissable={false}
    >
      <AriaModal className="pds-dialog-modal" data-size={size}>
        <AriaDialog
          className={composeClassNames("pds-dialog", className)}
          data-size={size}
          role={role}
          aria-labelledby={titleId}
          aria-describedby={descriptionId}
        >
          <header className="pds-dialog__header">
            <div>
              <h2 id={titleId}>{title}</h2>
              {description ? <p id={descriptionId}>{description}</p> : null}
            </div>
            <Button
              variant="quiet"
              aria-label={closeLabel}
              onClick={onClose}
            >
              Close
            </Button>
          </header>
          <div className="pds-dialog__body" tabIndex={0}>{children}</div>
          {footer ? <footer className="pds-dialog__footer">{footer}</footer> : null}
        </AriaDialog>
      </AriaModal>
    </AriaModalOverlay>
  );
}

export type DrawerProps = HTMLAttributes<HTMLElement> & {
  open: boolean;
  title: string;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  closeLabel?: string;
  side?: "left" | "right";
  size?: "sm" | "md" | "lg";
};

export function Drawer({
  open,
  title,
  description,
  children,
  footer,
  onClose,
  closeLabel = "Close drawer",
  side = "right",
  size = "md",
  className,
  ...props
}: DrawerProps) {
  const id = useId();
  const drawerRef = useRef<HTMLElement | null>(null);
  const titleId = `${id}-title`;
  const descriptionId = description ? `${id}-description` : undefined;
  useOverlayFocus(open, drawerRef);
  if (!open) return null;
  if (typeof document === "undefined") return null;

  return createPortal(
    <div
      className="pds-drawer-backdrop"
      data-side={side}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
    >
      <aside
        {...props}
        ref={drawerRef}
        className={composeClassNames("pds-drawer", className)}
        data-side={side}
        data-size={size}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        tabIndex={-1}
        onKeyDown={(event) => trapOverlayFocus(event, drawerRef.current)}
      >
        <header className="pds-drawer__header">
          <div>
            <h2 id={titleId}>{title}</h2>
            {description ? <p id={descriptionId}>{description}</p> : null}
          </div>
          <button
            type="button"
            className="pds-button"
            data-variant="quiet"
            aria-label={closeLabel}
            onPointerUp={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onClose();
            }}
            onKeyUp={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                event.stopPropagation();
                onClose();
              }
            }}
            onClick={onClose}
          >
            <span className="pds-button__label">Close</span>
          </button>
        </header>
        <div className="pds-drawer__body" tabIndex={0}>{children}</div>
        {footer ? <footer className="pds-drawer__footer">{footer}</footer> : null}
      </aside>
    </div>,
    document.body
  );
}

export type PopoverSize = "sm" | "md" | "lg";

export type PopoverProps = HTMLAttributes<HTMLDivElement> & {
  ariaLabel: string;
  size?: PopoverSize;
  nativePopover?: "auto" | "manual";
  open?: boolean;
  onClose?: () => void;
};

export const Popover = forwardRef<HTMLDivElement, PopoverProps>(
  function Popover({
    ariaLabel,
    size = "md",
    nativePopover,
    open,
    onClose,
    className,
    onKeyDown,
    children,
    ...props
  }, ref) {
    const popoverProps = nativePopover ? ({ popover: nativePopover } as Record<string, string>) : {};
    return (
      <div
        {...props}
        {...popoverProps}
        ref={ref}
        className={composeClassNames("pds-popover", className)}
        data-open={open === undefined ? undefined : open ? "true" : "false"}
        data-size={size}
        role="dialog"
        aria-label={ariaLabel}
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose?.();
          onKeyDown?.(event);
        }}
      >
        {children}
      </div>
    );
  }
);

export type PopoverTriggerProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  controls: string;
  open?: boolean;
  badge?: ReactNode;
};

export function PopoverTrigger({
  controls,
  open = false,
  badge,
  className,
  children,
  ...props
}: PopoverTriggerProps) {
  return (
    <button
      {...props}
      className={composeClassNames("pds-popover-trigger", className)}
      type={props.type ?? "button"}
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-controls={controls}
    >
      <span className="pds-popover-trigger__label">{children}</span>
      {badge ? <span className="pds-popover-trigger__badge">{badge}</span> : null}
    </button>
  );
}

export type TooltipProps = HTMLAttributes<HTMLSpanElement> & {
  content: ReactNode;
  position?: "top" | "right" | "bottom" | "left";
  children: ReactNode;
};

export function Tooltip({
  content,
  position = "top",
  children,
  className,
  ...props
}: TooltipProps) {
  const id = useId();
  const trigger = isValidElement<{ "aria-describedby"?: string }>(children)
    ? cloneElement(children as ReactElement<{ "aria-describedby"?: string }>, {
        "aria-describedby": describedBy(children.props["aria-describedby"], id)
      })
    : (
      <span className="pds-tooltip__trigger" aria-describedby={id}>
        {children}
      </span>
    );

  return (
    <span {...props} className={composeClassNames("pds-tooltip", className)} data-position={position}>
      {trigger}
      <span id={id} className="pds-tooltip__content" role="tooltip">
        {content}
      </span>
    </span>
  );
}

export type ConfirmDialogProps = Omit<DialogProps, "children" | "footer" | "onClose"> & {
  children?: ReactNode;
  confirmLabel?: ReactNode;
  cancelLabel?: ReactNode;
  tone?: "neutral" | "danger";
  confirmDisabled?: boolean;
  isConfirming?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

export function ConfirmDialog({
  children,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  tone = "neutral",
  confirmDisabled = false,
  isConfirming = false,
  onConfirm,
  onCancel,
  role = "alertdialog",
  ...props
}: ConfirmDialogProps) {
  return (
    <Dialog
      {...props}
      role={role}
      onClose={onCancel}
      footer={(
        <>
          <Button variant="secondary" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button
            variant={tone === "danger" ? "danger" : "primary"}
            isLoading={isConfirming}
            disabled={confirmDisabled}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </>
      )}
    >
      <div className="pds-confirm-dialog__body">{children}</div>
    </Dialog>
  );
}
