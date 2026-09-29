import type {
  AnchorHTMLAttributes,
  ButtonHTMLAttributes,
  HTMLAttributes,
  MouseEvent as ReactMouseEvent,
  ReactNode
} from "react";
import { useId } from "react";
import {
  composeClassNames,
  describedBy,
  type PdsDensity,
  type PdsSize,
  type PdsTone
} from "./types";
import type { ButtonVariant } from "./primitives";

export type ButtonLinkProps = Omit<
  AnchorHTMLAttributes<HTMLAnchorElement>,
  "aria-disabled" | "children" | "href"
> & {
  href: string;
  children: ReactNode;
  variant?: ButtonVariant;
  size?: PdsSize;
  isDisabled?: boolean;
};

/**
 * A navigation action with the same visual contract as Button.
 *
 * ButtonLink always renders a native anchor. Use Button for commands that do
 * not change location.
 */
export function ButtonLink({
  variant = "secondary",
  size = "md",
  isDisabled = false,
  className,
  children,
  onClick,
  tabIndex,
  ...props
}: ButtonLinkProps) {
  return (
    <a
      {...props}
      className={composeClassNames("pds-button", "pds-button-link", className)}
      data-size={size}
      data-variant={variant}
      data-disabled={isDisabled || undefined}
      aria-disabled={isDisabled || undefined}
      tabIndex={isDisabled ? -1 : tabIndex}
      onClick={(event) => {
        if (isDisabled) {
          event.preventDefault();
          return;
        }
        onClick?.(event);
      }}
    >
      <span className="pds-button__label">{children}</span>
    </a>
  );
}

export type WorkSurfaceLinkActivation = {
  kind: "link";
  href: string;
  props?: Omit<
    AnchorHTMLAttributes<HTMLAnchorElement>,
    "children" | "className" | "href"
  >;
};

export type WorkSurfaceButtonActivation = {
  kind: "button";
  onActivate: (event: ReactMouseEvent<HTMLButtonElement>) => void;
  props?: Omit<
    ButtonHTMLAttributes<HTMLButtonElement>,
    "children" | "className" | "onClick"
  >;
};

export type WorkSurfaceActivation =
  | WorkSurfaceLinkActivation
  | WorkSurfaceButtonActivation;

type ActivationOverlayProps = {
  activation: WorkSurfaceActivation;
  className: string;
  labelledBy: string;
  describedBy?: string;
  disabled: boolean;
};

function ActivationOverlay({
  activation,
  className,
  labelledBy,
  describedBy: descriptionIds,
  disabled
}: ActivationOverlayProps) {
  if (activation.kind === "link") {
    const {
      "aria-label": ariaLabel,
      "aria-labelledby": ariaLabelledBy,
      "aria-describedby": ariaDescribedBy,
      onClick,
      tabIndex,
      ...linkProps
    } = activation.props ?? {};

    return (
      <a
        {...linkProps}
        className={className}
        href={activation.href}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabel || ariaLabelledBy ? ariaLabelledBy : labelledBy}
        aria-describedby={describedBy(ariaDescribedBy, descriptionIds)}
        aria-disabled={disabled || undefined}
        tabIndex={disabled ? -1 : tabIndex}
        onClick={(event) => {
          if (disabled) {
            event.preventDefault();
            return;
          }
          onClick?.(event);
        }}
      />
    );
  }

  const {
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
    disabled: actionDisabled,
    type,
    ...buttonProps
  } = activation.props ?? {};
  const isDisabled = disabled || actionDisabled;

  return (
    <button
      {...buttonProps}
      className={className}
      type={type ?? "button"}
      aria-label={ariaLabel}
      aria-labelledby={ariaLabel || ariaLabelledBy ? ariaLabelledBy : labelledBy}
      aria-describedby={describedBy(ariaDescribedBy, descriptionIds)}
      disabled={isDisabled}
      onClick={activation.onActivate}
    />
  );
}

export type InteractiveCardBaseProps = Omit<
  HTMLAttributes<HTMLElement>,
  "children" | "onClick" | "title"
> & {
  as?: "article" | "section" | "div";
  eyebrow?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  leading?: ReactNode;
  metadata?: ReactNode;
  footer?: ReactNode;
  trailingAction?: ReactNode;
  variant?: "outlined" | "filled" | "quiet";
  tone?: PdsTone;
  density?: PdsDensity;
  selected?: boolean;
  disabled?: boolean;
};

export type InteractiveCardProps = InteractiveCardBaseProps & {
  activation: WorkSurfaceActivation;
};

/**
 * A whole-surface link or command with a separately layered trailing action.
 *
 * The primary anchor/button is an empty overlay named by the visible title.
 * This keeps secondary actions as siblings rather than nesting them inside an
 * anchor or button.
 */
export function InteractiveCard({
  as: Component = "article",
  activation,
  eyebrow,
  title,
  description,
  leading,
  metadata,
  footer,
  trailingAction,
  variant = "outlined",
  tone = "neutral",
  density = "comfortable",
  selected = false,
  disabled = false,
  id,
  className,
  ...props
}: InteractiveCardProps) {
  const generatedId = useId();
  const rootId = id ?? `pds-interactive-card-${generatedId}`;
  const titleId = `${rootId}-title`;
  const descriptionId = description ? `${rootId}-description` : undefined;

  return (
    <Component
      {...props}
      id={rootId}
      className={composeClassNames("pds-interactive-card", className)}
      data-activation={activation.kind}
      data-density={density}
      data-disabled={disabled || undefined}
      data-selected={selected || undefined}
      data-tone={tone}
      data-variant={variant}
    >
      <ActivationOverlay
        activation={activation}
        className="pds-interactive-card__activation"
        labelledBy={titleId}
        describedBy={descriptionId}
        disabled={disabled}
      />
      {leading ? (
        <div className="pds-interactive-card__leading" aria-hidden="true">
          {leading}
        </div>
      ) : null}
      <div className="pds-interactive-card__copy">
        {eyebrow ? (
          <span className="pds-interactive-card__eyebrow">{eyebrow}</span>
        ) : null}
        <strong id={titleId} className="pds-interactive-card__title">
          {title}
        </strong>
        {description ? (
          <span id={descriptionId} className="pds-interactive-card__description">
            {description}
          </span>
        ) : null}
        {metadata ? (
          <span className="pds-interactive-card__metadata">{metadata}</span>
        ) : null}
      </div>
      {trailingAction ? (
        <div className="pds-interactive-card__trailing-action">
          {trailingAction}
        </div>
      ) : null}
      {footer ? (
        <div className="pds-interactive-card__footer">{footer}</div>
      ) : null}
    </Component>
  );
}

export type CardLinkProps = InteractiveCardBaseProps & {
  href: string;
  linkProps?: WorkSurfaceLinkActivation["props"];
};

export function CardLink({
  href,
  linkProps,
  className,
  ...props
}: CardLinkProps) {
  return (
    <InteractiveCard
      {...props}
      className={composeClassNames("pds-card-link", className)}
      activation={{ kind: "link", href, props: linkProps }}
    />
  );
}

export type WorkQueueItemProps = Omit<
  HTMLAttributes<HTMLElement>,
  "children" | "onClick" | "title"
> & {
  as?: "li" | "article" | "div";
  activation: WorkSurfaceActivation;
  eyebrow?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  leading?: ReactNode;
  metadata?: ReactNode;
  status?: ReactNode;
  trailing?: ReactNode;
  trailingAction?: ReactNode;
  variant?: "standard" | "segmented";
  tone?: PdsTone;
  density?: PdsDensity;
  selected?: boolean;
  completed?: boolean;
  disabled?: boolean;
};

/**
 * A scan-friendly queue row with whole-row activation.
 *
 * The optional trailing action is a sibling of the primary activation overlay,
 * so consumers can provide a completion or overflow button without invalid
 * nested interactive markup.
 */
export function WorkQueueItem({
  as: Component = "li",
  activation,
  eyebrow,
  title,
  description,
  leading,
  metadata,
  status,
  trailing,
  trailingAction,
  variant = "standard",
  tone = "neutral",
  density = "compact",
  selected = false,
  completed = false,
  disabled = false,
  id,
  className,
  ...props
}: WorkQueueItemProps) {
  const generatedId = useId();
  const rootId = id ?? `pds-work-queue-item-${generatedId}`;
  const titleId = `${rootId}-title`;
  const descriptionId = description ? `${rootId}-description` : undefined;

  return (
    <Component
      {...props}
      id={rootId}
      className={composeClassNames("pds-work-queue-item", className)}
      data-activation={activation.kind}
      data-completed={completed || undefined}
      data-density={density}
      data-disabled={disabled || undefined}
      data-selected={selected || undefined}
      data-tone={tone}
      data-variant={variant}
    >
      <ActivationOverlay
        activation={activation}
        className="pds-work-queue-item__activation"
        labelledBy={titleId}
        describedBy={descriptionId}
        disabled={disabled}
      />
      {leading ? (
        <div className="pds-work-queue-item__leading" aria-hidden="true">
          {leading}
        </div>
      ) : null}
      <div className="pds-work-queue-item__copy">
        {eyebrow ? (
          <span className="pds-work-queue-item__eyebrow">{eyebrow}</span>
        ) : null}
        <strong id={titleId} className="pds-work-queue-item__title">
          {title}
        </strong>
        {description ? (
          <span id={descriptionId} className="pds-work-queue-item__description">
            {description}
          </span>
        ) : null}
        {metadata ? (
          <span className="pds-work-queue-item__metadata">{metadata}</span>
        ) : null}
      </div>
      {status ? (
        <div className="pds-work-queue-item__status">{status}</div>
      ) : null}
      {trailing ? (
        <div className="pds-work-queue-item__trailing">{trailing}</div>
      ) : null}
      {trailingAction ? (
        <div className="pds-work-queue-item__trailing-action">
          {trailingAction}
        </div>
      ) : null}
    </Component>
  );
}
