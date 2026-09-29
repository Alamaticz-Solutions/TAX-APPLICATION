import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  MouseEvent as ReactMouseEvent,
  ReactNode
} from "react";
import { useLayoutEffect, useRef, useState } from "react";
import {
  Button as AriaButton,
  Menu as AriaMenu,
  MenuItem as AriaMenuItem,
  MenuTrigger as AriaMenuTrigger,
  Popover as AriaPopover
} from "react-aria-components";
import { composeClassNames, type PdsSize, type PdsTone } from "./types";

export type ButtonVariant =
  | "primary"
  | "secondary"
  | "quiet"
  | "danger"
  | "filled"
  | "tonal"
  | "outlined"
  | "text"
  | "elevated";

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: PdsSize;
  isLoading?: boolean;
  isDenied?: boolean;
};

export function Button({
  variant = "secondary",
  size = "md",
  isLoading = false,
  isDenied = false,
  className,
  children,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      {...props}
      className={composeClassNames("pds-button", className)}
      data-size={size}
      data-variant={variant}
      aria-busy={isLoading || undefined}
      aria-disabled={isDenied || undefined}
      disabled={disabled || isLoading || isDenied}
      type={props.type ?? "button"}
    >
      {isLoading ? <span className="pds-button__spinner" aria-hidden="true" /> : null}
      <span className="pds-button__label">{children}</span>
    </button>
  );
}

export type IconButtonProps = Omit<ButtonProps, "children"> & {
  icon: ReactNode;
  ariaLabel: string;
  tooltip?: string;
};

export function IconButton({
  icon,
  ariaLabel,
  tooltip,
  className,
  ...props
}: IconButtonProps) {
  return (
    <Button
      {...props}
      className={composeClassNames("pds-icon-button", className)}
      aria-label={ariaLabel}
      title={tooltip ?? ariaLabel}
    >
      <span className="pds-icon-button__glyph" aria-hidden="true">
        {icon}
      </span>
    </Button>
  );
}

export type ToggleButtonProps = ButtonProps & {
  selected: boolean;
  onSelectedChange?: (selected: boolean) => void;
};

export function ToggleButton({
  selected,
  onSelectedChange,
  className,
  onClick,
  children,
  ...props
}: ToggleButtonProps) {
  return (
    <Button
      {...props}
      className={composeClassNames("pds-toggle-button", className)}
      data-selected={selected}
      aria-pressed={selected}
      onClick={(event) => {
        onClick?.(event);
        if (!event.defaultPrevented) onSelectedChange?.(!selected);
      }}
    >
      {children}
    </Button>
  );
}

export type FloatingActionButtonProps = Omit<ButtonProps, "variant"> & {
  icon: ReactNode;
  label?: ReactNode;
  ariaLabel?: string;
  tone?: "primary" | "secondary" | "tertiary" | "surface";
};

export function FloatingActionButton({
  icon,
  label,
  ariaLabel,
  tone = "primary",
  size = "md",
  className,
  ...props
}: FloatingActionButtonProps) {
  return (
    <Button
      {...props}
      className={composeClassNames("pds-floating-action-button", className)}
      data-tone={tone}
      data-extended={Boolean(label) || undefined}
      size={size}
      variant="elevated"
      aria-label={ariaLabel ?? (typeof label === "string" ? label : undefined)}
    >
      <span className="pds-floating-action-button__icon" aria-hidden="true">{icon}</span>
      {label ? <span className="pds-floating-action-button__label">{label}</span> : null}
    </Button>
  );
}

export type ButtonGroupProps = HTMLAttributes<HTMLDivElement> & {
  variant?: "standard" | "connected";
  children: ReactNode;
  ariaLabel: string;
};

export function ButtonGroup({
  variant = "standard",
  children,
  ariaLabel,
  className,
  ...props
}: ButtonGroupProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-button-group", className)}
      data-variant={variant}
      role="group"
      aria-label={ariaLabel}
    >
      {children}
    </div>
  );
}

export type MenuButtonItem = {
  id: string;
  label: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  badge?: ReactNode;
  href?: string;
  disabled?: boolean;
  selected?: boolean;
  tone?: Extract<PdsTone, "neutral" | "accent" | "danger">;
  onSelect?: () => void;
};

export type MenuButtonProps = Omit<HTMLAttributes<HTMLElement>, "children"> & {
  label: ReactNode;
  ariaLabel?: string;
  icon?: ReactNode;
  items?: readonly MenuButtonItem[];
  children?: ReactNode;
  align?: "start" | "end";
  size?: PdsSize;
  variant?: ButtonVariant;
  menuLabel?: string;
  menuTone?: "standard" | "vibrant";
};

export function MenuButton({
  label,
  ariaLabel,
  icon,
  items = [],
  children,
  align = "start",
  size = "md",
  variant = "secondary",
  menuLabel,
  menuTone = "standard",
  className,
  ...props
}: MenuButtonProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const legacyDetailsRef = useRef<HTMLDetailsElement>(null);
  const resolvedMenuLabel = menuLabel ?? (typeof label === "string" ? `${label} menu` : "Action menu");

  useLayoutEffect(() => {
    rootRef.current?.toggleAttribute("open", open);
  }, [open]);

  if (children) {
    const selectLegacyItem = (
      event: ReactMouseEvent<HTMLAnchorElement | HTMLButtonElement>,
      item: MenuButtonItem
    ) => {
      if (item.disabled) {
        event.preventDefault();
        return;
      }
      item.onSelect?.();
      legacyDetailsRef.current?.removeAttribute("open");
    };

    return (
      <details
        {...props}
        ref={legacyDetailsRef}
        className={composeClassNames("pds-menu-button", className)}
        data-align={align}
        data-size={size}
        data-menu-tone={menuTone}
      >
        <summary
          className="pds-menu-button__trigger pds-button"
          data-size={size}
          data-variant={variant}
          role="button"
          aria-label={ariaLabel}
          aria-haspopup="menu"
        >
          {icon ? <span className="pds-menu-button__trigger-icon" aria-hidden="true">{icon}</span> : null}
          <span className="pds-button__label">{label}</span>
          <span className="pds-menu-button__indicator" aria-hidden="true" />
        </summary>
        <div className="pds-menu-button__menu" role="menu" aria-label={resolvedMenuLabel}>
          {items.map((item) => {
            const content = (
              <>
                {item.icon ? <span className="pds-menu-button__item-icon" aria-hidden="true">{item.icon}</span> : null}
                <span className="pds-menu-button__item-copy">
                  <strong>{item.label}</strong>
                  {item.description ? <small>{item.description}</small> : null}
                </span>
                {item.badge ? <span className="pds-menu-button__item-badge">{item.badge}</span> : null}
              </>
            );
            return item.href && !item.disabled ? (
              <a
                key={item.id}
                className="pds-menu-button__item"
                data-tone={item.tone ?? undefined}
                data-selected={item.selected || undefined}
                role="menuitem"
                href={item.href}
                onClick={(event) => selectLegacyItem(event, item)}
              >
                {content}
              </a>
            ) : (
              <button
                key={item.id}
                className="pds-menu-button__item"
                data-tone={item.tone ?? undefined}
                data-selected={item.selected || undefined}
                role="menuitem"
                aria-disabled={item.disabled || undefined}
                type="button"
                disabled={item.disabled}
                onClick={(event) => selectLegacyItem(event, item)}
              >
                {content}
              </button>
            );
          })}
          {children}
        </div>
      </details>
    );
  }

  return (
    <div
      {...props}
      ref={rootRef}
      className={composeClassNames("pds-menu-button", className)}
      data-align={align}
      data-size={size}
      data-menu-tone={menuTone}
    >
      <AriaMenuTrigger isOpen={open} onOpenChange={setOpen}>
        <AriaButton
          className="pds-menu-button__trigger pds-button"
          data-size={size}
          data-variant={variant}
          aria-label={ariaLabel}
          aria-haspopup="menu"
        >
          {icon ? <span className="pds-menu-button__trigger-icon" aria-hidden="true">{icon}</span> : null}
          <span className="pds-button__label">{label}</span>
          <span className="pds-menu-button__indicator" aria-hidden="true" />
        </AriaButton>
        <AriaPopover
          className="pds-menu-button__menu"
          data-menu-tone={menuTone}
          placement={align === "end" ? "bottom end" : "bottom start"}
          UNSTABLE_portalContainer={rootRef.current ?? undefined}
        >
          <AriaMenu
            aria-label={resolvedMenuLabel}
            disabledKeys={items.filter((item) => item.disabled).map((item) => item.id)}
          >
            {/* React Aria owns role="menu" and role="menuitem" semantics. */}
            {items.map((item) => (
              <AriaMenuItem
                key={item.id}
                id={item.id}
                href={item.href}
                textValue={typeof item.label === "string" ? item.label : item.id}
                className="pds-menu-button__item"
                data-tone={item.tone ?? undefined}
                data-selected={item.selected || undefined}
                aria-disabled={item.disabled || undefined}
                isDisabled={item.disabled}
                onAction={item.onSelect}
              >
                {item.icon ? <span className="pds-menu-button__item-icon" aria-hidden="true">{item.icon}</span> : null}
                <span className="pds-menu-button__item-copy">
                  <strong>{item.label}</strong>
                  {item.description ? <small>{item.description}</small> : null}
                </span>
                {item.badge ? <span className="pds-menu-button__item-badge">{item.badge}</span> : null}
              </AriaMenuItem>
            ))}
          </AriaMenu>
        </AriaPopover>
      </AriaMenuTrigger>
    </div>
  );
}

export type IntentPreviewChange = {
  id: string;
  label: ReactNode;
  before?: ReactNode;
  after?: ReactNode;
  tone?: Extract<PdsTone, "neutral" | "accent" | "success" | "danger" | "warning">;
};

export type IntentPreviewProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  description?: ReactNode;
  actor?: ReactNode;
  policy?: ReactNode;
  confidence?: ReactNode;
  evidence?: ReactNode;
  changes?: readonly IntentPreviewChange[];
  actions?: ReactNode;
  ariaLabel?: string;
};

export function IntentPreview({
  title,
  description,
  actor,
  policy,
  confidence,
  evidence,
  changes = [],
  actions,
  ariaLabel = "Action intent preview",
  className,
  ...props
}: IntentPreviewProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-intent-preview", className)}
      aria-label={ariaLabel}
    >
      <header className="pds-intent-preview__header">
        <div className="pds-intent-preview__copy">
          <h3 className="pds-intent-preview__title">{title}</h3>
          {description ? <p className="pds-intent-preview__description">{description}</p> : null}
        </div>
        {confidence ? <span className="pds-intent-preview__confidence">{confidence}</span> : null}
      </header>
      {actor || policy || evidence ? (
        <dl className="pds-intent-preview__metadata">
          {actor ? (
            <>
              <dt>Actor</dt>
              <dd>{actor}</dd>
            </>
          ) : null}
          {policy ? (
            <>
              <dt>Policy</dt>
              <dd>{policy}</dd>
            </>
          ) : null}
          {evidence ? (
            <>
              <dt>Evidence</dt>
              <dd>{evidence}</dd>
            </>
          ) : null}
        </dl>
      ) : null}
      {changes.length > 0 ? (
        <ul className="pds-intent-preview__changes" aria-label="Proposed changes">
          {changes.map((change) => (
            <li key={change.id} className="pds-intent-preview__change" data-tone={change.tone ?? "neutral"}>
              <span className="pds-intent-preview__change-label">{change.label}</span>
              {change.before || change.after ? (
                <span className="pds-intent-preview__change-values">
                  {change.before ? <span>{change.before}</span> : null}
                  {change.before && change.after ? <span aria-hidden="true">-&gt;</span> : null}
                  {change.after ? <strong>{change.after}</strong> : null}
                </span>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}
      {actions ? <div className="pds-intent-preview__actions">{actions}</div> : null}
    </section>
  );
}

export type ActionAuditStatus = "pending" | "approved" | "denied" | "completed";

export type ActionAuditItem = {
  id: string;
  label: ReactNode;
  value: ReactNode;
};

export type ActionAuditProps = HTMLAttributes<HTMLElement> & {
  title?: ReactNode;
  actor: ReactNode;
  action: ReactNode;
  status?: ActionAuditStatus;
  timestamp?: ReactNode;
  requestId?: ReactNode;
  correlationId?: ReactNode;
  items?: readonly ActionAuditItem[];
  ariaLabel?: string;
};

export function ActionAudit({
  title = "Action audit",
  actor,
  action,
  status = "pending",
  timestamp,
  requestId,
  correlationId,
  items = [],
  ariaLabel = "Action audit",
  className,
  ...props
}: ActionAuditProps) {
  return (
    <aside
      {...props}
      className={composeClassNames("pds-action-audit", className)}
      data-status={status}
      aria-label={ariaLabel}
    >
      <header className="pds-action-audit__header">
        <h3 className="pds-action-audit__title">{title}</h3>
        <span className="pds-action-audit__status">{status}</span>
      </header>
      <dl className="pds-action-audit__list">
        <dt>Actor</dt>
        <dd>{actor}</dd>
        <dt>Action</dt>
        <dd>{action}</dd>
        {timestamp ? (
          <>
            <dt>When</dt>
            <dd>{timestamp}</dd>
          </>
        ) : null}
        {requestId ? (
          <>
            <dt>Request</dt>
            <dd>{requestId}</dd>
          </>
        ) : null}
        {correlationId ? (
          <>
            <dt>Correlation</dt>
            <dd>{correlationId}</dd>
          </>
        ) : null}
        {items.map((item) => (
          <div key={item.id} className="pds-action-audit__item">
            <dt>{item.label}</dt>
            <dd>{item.value}</dd>
          </div>
        ))}
      </dl>
    </aside>
  );
}

export type UndoCompensationStateValue = "available" | "pending" | "completed" | "expired" | "blocked";

export type UndoCompensationStateProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  detail?: ReactNode;
  state?: UndoCompensationStateValue;
  deadline?: ReactNode;
  action?: ReactNode;
  ariaLabel?: string;
};

export function UndoCompensationState({
  title,
  detail,
  state = "available",
  deadline,
  action,
  ariaLabel = "Undo and compensation state",
  className,
  ...props
}: UndoCompensationStateProps) {
  return (
    <aside
      {...props}
      className={composeClassNames("pds-undo-compensation-state", className)}
      data-state={state}
      role="status"
      aria-live="polite"
      aria-label={ariaLabel}
    >
      <div className="pds-undo-compensation-state__copy">
        <span className="pds-undo-compensation-state__state">{state}</span>
        <h3 className="pds-undo-compensation-state__title">{title}</h3>
        {detail ? <p className="pds-undo-compensation-state__detail">{detail}</p> : null}
        {deadline ? <p className="pds-undo-compensation-state__deadline">{deadline}</p> : null}
      </div>
      {action ? <div className="pds-undo-compensation-state__action">{action}</div> : null}
    </aside>
  );
}

export type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: PdsTone;
};

export function Badge({
  tone = "neutral",
  className,
  children,
  ...props
}: BadgeProps) {
  return (
    <span
      {...props}
      className={composeClassNames("pds-badge", className)}
      data-tone={tone}
    >
      {children}
    </span>
  );
}

export type SegmentedControlOption<Value extends string = string> = {
  value: Value;
  label: ReactNode;
  ariaLabel?: string;
  disabled?: boolean;
};

export type SegmentedControlProps<Value extends string = string> = HTMLAttributes<HTMLDivElement> & {
  ariaLabel: string;
  value: Value;
  options: readonly SegmentedControlOption<Value>[];
  onValueChange: (value: Value) => void;
  size?: PdsSize;
  disabled?: boolean;
};

export function SegmentedControl<Value extends string = string>({
  ariaLabel,
  value,
  options,
  onValueChange,
  size = "sm",
  disabled = false,
  className,
  ...props
}: SegmentedControlProps<Value>) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-segmented-control", className)}
      data-size={size}
      role="group"
      aria-label={ariaLabel}
      aria-disabled={disabled || undefined}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            className="pds-segmented-control__option"
            data-selected={selected || undefined}
            aria-label={option.ariaLabel}
            aria-pressed={selected}
            disabled={disabled || option.disabled}
            onClick={() => onValueChange(option.value)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

export type AlertProps = HTMLAttributes<HTMLDivElement> & {
  tone?: PdsTone;
  title: string;
  detail?: ReactNode;
};

export function Alert({
  tone = "neutral",
  title,
  detail,
  className,
  children,
  ...props
}: AlertProps) {
  const role = tone === "danger" ? "alert" : "status";
  return (
    <div
      {...props}
      className={composeClassNames("pds-alert", className)}
      data-tone={tone}
      role={role}
    >
      <div className="pds-alert__title">{title}</div>
      {detail ? <div className="pds-alert__detail">{detail}</div> : null}
      {children}
    </div>
  );
}
