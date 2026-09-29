import type { HTMLAttributes, ReactNode } from "react";
import { composeClassNames, type PdsDensity, type PdsTone } from "./types";

export type SurfaceProps = Omit<HTMLAttributes<HTMLElement>, "title"> & {
  as?: "section" | "article" | "div";
  variant?: "elevated" | "filled" | "outlined";
  overflow?: "clip" | "visible";
  title?: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  density?: PdsDensity;
  children: ReactNode;
};

export function Surface({
  as: Component = "section",
  variant = "elevated",
  overflow = "clip",
  title,
  subtitle,
  actions,
  density = "comfortable",
  className,
  children,
  ...props
}: SurfaceProps) {
  return (
    <Component
      {...props}
      className={composeClassNames("pds-surface", className)}
      data-density={density}
      data-overflow={overflow}
      data-variant={variant}
    >
      {title || subtitle || actions ? (
        <header className="pds-surface__header">
          <div className="pds-surface__copy">
            {title ? <h2 className="pds-surface__title">{title}</h2> : null}
            {subtitle ? <p className="pds-surface__subtitle">{subtitle}</p> : null}
          </div>
          {actions ? <div className="pds-surface__actions">{actions}</div> : null}
        </header>
      ) : null}
      <div className="pds-surface__body">{children}</div>
    </Component>
  );
}

export type MetricTrendTone = "neutral" | "positive" | "negative" | "warning" | "accent";

export type MetricTrendProps = HTMLAttributes<HTMLSpanElement> & {
  value: ReactNode;
  label?: ReactNode;
  tone?: MetricTrendTone;
  direction?: "up" | "down" | "flat";
};

export function MetricTrend({
  value,
  label,
  tone = "neutral",
  direction = "flat",
  className,
  ...props
}: MetricTrendProps) {
  return (
    <span
      {...props}
      className={composeClassNames("pds-metric-trend", className)}
      data-tone={tone}
      data-direction={direction}
    >
      <span className="pds-metric-trend__glyph" aria-hidden="true">{trendGlyph(direction)}</span>
      <strong>{value}</strong>
      {label ? <span>{label}</span> : null}
    </span>
  );
}

export type KpiTileProps = HTMLAttributes<HTMLElement> & {
  as?: "article" | "section" | "div";
  label: ReactNode;
  value: ReactNode;
  detail?: ReactNode;
  icon?: ReactNode;
  trend?: ReactNode;
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
};

export function KpiTile({
  as: Component = "article",
  label,
  value,
  detail,
  icon,
  trend,
  tone = "neutral",
  className,
  ...props
}: KpiTileProps) {
  return (
    <Component
      {...props}
      className={composeClassNames("pds-kpi-tile", className)}
      data-tone={tone}
    >
      <span className="pds-kpi-tile__topline">
        <span className="pds-kpi-tile__label">{label}</span>
        {icon ? <span className="pds-kpi-tile__icon" aria-hidden="true">{icon}</span> : null}
      </span>
      <strong className="pds-kpi-tile__value">{value}</strong>
      <span className="pds-kpi-tile__footer">
        {detail ? <span className="pds-kpi-tile__detail">{detail}</span> : null}
        {trend ? <span className="pds-kpi-tile__trend">{trend}</span> : null}
      </span>
    </Component>
  );
}

export type ChartShellProps = Omit<HTMLAttributes<HTMLElement>, "title"> & {
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
};

export function ChartShell({
  title,
  subtitle,
  actions,
  footer,
  className,
  children,
  ...props
}: ChartShellProps) {
  return (
    <section {...props} className={composeClassNames("pds-chart-shell", className)}>
      <header className="pds-chart-shell__header">
        <span className="pds-chart-shell__copy">
          <strong>{title}</strong>
          {subtitle ? <span>{subtitle}</span> : null}
        </span>
        {actions ? <span className="pds-chart-shell__actions">{actions}</span> : null}
      </header>
      <div className="pds-chart-shell__figure">{children}</div>
      {footer ? <footer className="pds-chart-shell__footer">{footer}</footer> : null}
    </section>
  );
}

export type ChartLegendItem = {
  id: string;
  label: ReactNode;
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
  value?: ReactNode;
};

export type ChartLegendProps = HTMLAttributes<HTMLUListElement> & {
  items: readonly ChartLegendItem[];
};

export function ChartLegend({
  items,
  className,
  ...props
}: ChartLegendProps) {
  return (
    <ul {...props} className={composeClassNames("pds-chart-legend", className)} role="list">
      {items.map((item) => (
        <li key={item.id} className="pds-chart-legend__item" data-tone={item.tone ?? "neutral"}>
          <span className="pds-chart-legend__swatch" aria-hidden="true" />
          <span>{item.label}</span>
          {item.value ? <strong>{item.value}</strong> : null}
        </li>
      ))}
    </ul>
  );
}

export type FormLayoutProps = HTMLAttributes<HTMLDivElement> & {
  columns?: "one" | "two" | "auto";
  footer?: ReactNode;
  children: ReactNode;
};

export function FormLayout({
  columns = "auto",
  footer,
  className,
  children,
  ...props
}: FormLayoutProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-form-layout", className)}
      data-columns={columns}
    >
      <div className="pds-form-layout__fields">{children}</div>
      {footer ? <div className="pds-form-layout__footer">{footer}</div> : null}
    </div>
  );
}

export type FieldGroupProps = HTMLAttributes<HTMLFieldSetElement> & {
  legend: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
};

export function FieldGroup({
  legend,
  description,
  actions,
  className,
  children,
  ...props
}: FieldGroupProps) {
  return (
    <fieldset {...props} className={composeClassNames("pds-field-group", className)}>
      <div className="pds-field-group__header">
        <div>
          <legend className="pds-field-group__legend">{legend}</legend>
          {description ? <p className="pds-field-group__description">{description}</p> : null}
        </div>
        {actions ? <div className="pds-field-group__actions">{actions}</div> : null}
      </div>
      <div className="pds-field-group__body">{children}</div>
    </fieldset>
  );
}

export type ValidationSummaryItem = {
  id: string;
  label?: ReactNode;
  messages: readonly ReactNode[];
};

export type ValidationSummaryProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  title?: ReactNode;
  items?: readonly ValidationSummaryItem[];
  validation?: Record<string, readonly ReactNode[]>;
};

export function ValidationSummary({
  title = "Review the highlighted fields",
  items,
  validation,
  className,
  ...props
}: ValidationSummaryProps) {
  const normalizedItems = items ?? Object.entries(validation ?? {}).map(([id, messages]) => ({
    id,
    label: id,
    messages
  }));

  if (normalizedItems.length === 0) return null;

  return (
    <div
      {...props}
      className={composeClassNames("pds-validation-summary", className)}
      role="alert"
      aria-live="assertive"
    >
      <div className="pds-validation-summary__title">{title}</div>
      <ul className="pds-validation-summary__list">
        {normalizedItems.map((item) => (
          <li key={item.id}>
            {item.label ? <strong>{item.label}</strong> : null}
            {item.label ? ": " : null}
            {item.messages.map((message, index) => (
              <span key={index}>
                {index > 0 ? ", " : null}
                {message}
              </span>
            ))}
          </li>
        ))}
      </ul>
    </div>
  );
}

export type EmptyStateProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  title: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
  tone?: PdsTone;
};

export function EmptyState({
  title,
  detail,
  action,
  tone = "neutral",
  className,
  ...props
}: EmptyStateProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-empty-state", className)}
      data-tone={tone}
      role="status"
    >
      <div className="pds-empty-state__title">{title}</div>
      {detail ? <div className="pds-empty-state__detail">{detail}</div> : null}
      {action ? <div className="pds-empty-state__action">{action}</div> : null}
    </div>
  );
}

export type InlineAlertProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  tone?: PdsTone;
  title: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
};

export function InlineAlert({
  tone = "neutral",
  title,
  detail,
  action,
  className,
  children,
  ...props
}: InlineAlertProps) {
  const blocking = tone === "danger" || tone === "warning";
  return (
    <div
      {...props}
      className={composeClassNames("pds-inline-alert", className)}
      data-tone={tone}
      role={blocking ? "alert" : "status"}
      aria-live={blocking ? "assertive" : "polite"}
    >
      <span className="pds-inline-alert__mark" aria-hidden="true" />
      <span className="pds-inline-alert__copy">
        <strong>{title}</strong>
        {detail ? <span>{detail}</span> : null}
        {children}
      </span>
      {action ? <span className="pds-inline-alert__action">{action}</span> : null}
    </div>
  );
}

export type BannerProps = InlineAlertProps & {
  onDismiss?: () => void;
  dismissLabel?: string;
};

export function Banner({
  onDismiss,
  dismissLabel = "Dismiss notification",
  action,
  className,
  children,
  ...props
}: BannerProps) {
  const resolvedAction = action || onDismiss ? (
    <>
      {action}
      {onDismiss ? (
        <button
          type="button"
          className="pds-banner__dismiss"
          aria-label={dismissLabel}
          onClick={onDismiss}
        >
          <span aria-hidden="true">x</span>
        </button>
      ) : null}
    </>
  ) : undefined;

  return (
    <InlineAlert
      {...props}
      className={composeClassNames("pds-banner", className)}
      action={resolvedAction}
    >
      {children}
    </InlineAlert>
  );
}

export type FeedbackStateKind = "loading" | "empty" | "error" | "denied" | "success" | "info";

export type FeedbackStateMetadata = {
  requestId?: ReactNode;
  correlationId?: ReactNode;
  responseMs?: number | null;
  details?: readonly { label: ReactNode; value: ReactNode }[];
};

export type OperationStateValue =
  | "idle"
  | "pending"
  | "success"
  | "error"
  | "denied"
  | "cancelled";

export type OperationStateResult = {
  label?: ReactNode;
  value: ReactNode;
  detail?: ReactNode;
};

export type RequestCorrelation = {
  requestId?: ReactNode;
  correlationId?: ReactNode;
};

export type OperationStateProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  state: OperationStateValue;
  title: ReactNode;
  detail?: ReactNode;
  stateLabel?: ReactNode;
  result?: OperationStateResult;
  request?: RequestCorrelation;
  action?: ReactNode;
};

export type FeedbackStateProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  kind?: FeedbackStateKind;
  title: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
  metadata?: FeedbackStateMetadata;
};

export function FeedbackState({
  kind = "empty",
  title,
  detail,
  action,
  metadata,
  className,
  children,
  ...props
}: FeedbackStateProps) {
  const entries = feedbackMetadataEntries(metadata);
  const isBlocking = kind === "error" || kind === "denied";
  const role = isBlocking ? "alert" : "status";

  return (
    <div
      {...props}
      className={composeClassNames("pds-feedback-state", className)}
      data-kind={kind}
      role={role}
      aria-live={isBlocking ? "assertive" : "polite"}
      aria-busy={kind === "loading" || undefined}
    >
      <div className="pds-feedback-state__mark" aria-hidden="true" />
      <div className="pds-feedback-state__copy">
        <div className="pds-feedback-state__title">{title}</div>
        {detail ? <div className="pds-feedback-state__detail">{detail}</div> : null}
      </div>
      {children ? <div className="pds-feedback-state__body">{children}</div> : null}
      {entries.length ? (
        <dl className="pds-feedback-state__metadata">
          {entries.map((entry, index) => (
            <div key={index} className="pds-feedback-state__metadata-item">
              <dt>{entry.label}</dt>
              <dd>{entry.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {action ? <div className="pds-feedback-state__action">{action}</div> : null}
    </div>
  );
}

const operationStateLabels: Record<OperationStateValue, string> = {
  idle: "Ready",
  pending: "In progress",
  success: "Succeeded",
  error: "Failed",
  denied: "Denied",
  cancelled: "Cancelled"
};

export function OperationState({
  state,
  title,
  detail,
  stateLabel = operationStateLabels[state],
  result,
  request,
  action,
  className,
  ...props
}: OperationStateProps) {
  const isBlocking = state === "error" || state === "denied";
  const requestEntries = feedbackMetadataEntries(request);

  return (
    <div
      {...props}
      className={composeClassNames("pds-operation-state", className)}
      data-state={state}
      role={isBlocking ? "alert" : "status"}
      aria-live={isBlocking ? "assertive" : "polite"}
      aria-atomic="true"
      aria-busy={state === "pending" || undefined}
    >
      <span className="pds-operation-state__indicator" aria-hidden="true" />
      <div className="pds-operation-state__copy">
        <span className="pds-operation-state__label">{stateLabel}</span>
        <strong className="pds-operation-state__title">{title}</strong>
        {detail ? <span className="pds-operation-state__detail">{detail}</span> : null}
      </div>
      {result ? (
        <div className="pds-operation-state__result">
          <span>{result.label ?? "Result"}</span>
          <strong>{result.value}</strong>
          {result.detail ? <small>{result.detail}</small> : null}
        </div>
      ) : null}
      {requestEntries.length ? (
        <dl className="pds-operation-state__request">
          {requestEntries.map((entry, index) => (
            <div key={index} className="pds-operation-state__request-item">
              <dt>{entry.label}</dt>
              <dd>{entry.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {action ? <div className="pds-operation-state__action">{action}</div> : null}
    </div>
  );
}

export type ErrorStateProps = Omit<FeedbackStateProps, "kind">;

export function ErrorState(props: ErrorStateProps) {
  return <FeedbackState {...props} kind="error" />;
}

export type ForbiddenStateProps = Omit<FeedbackStateProps, "kind">;

export function ForbiddenState(props: ForbiddenStateProps) {
  return <FeedbackState {...props} kind="denied" />;
}

export type LoadingStateProps = Omit<FeedbackStateProps, "kind" | "title"> & {
  title?: ReactNode;
  skeletonLines?: number;
};

export function LoadingState({
  title = "Loading",
  detail,
  action,
  metadata,
  skeletonLines = 0,
  children,
  ...props
}: LoadingStateProps) {
  return (
    <FeedbackState
      {...props}
      kind="loading"
      title={title}
      detail={detail}
      action={action}
      metadata={metadata}
    >
      {children}
      {skeletonLines > 0 ? <Skeleton lines={skeletonLines} /> : null}
    </FeedbackState>
  );
}

export type ToastItem = {
  id: string;
  tone?: PdsTone;
  title: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
  metadata?: FeedbackStateMetadata;
  dismissLabel?: string;
};

export type ToastProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & ToastItem & {
  onDismiss?: (id: string) => void;
};

export function Toast({
  id,
  tone = "neutral",
  title,
  detail,
  action,
  metadata,
  dismissLabel = "Dismiss notification",
  onDismiss,
  className,
  ...props
}: ToastProps) {
  const blocking = tone === "danger" || tone === "warning";
  const entries = feedbackMetadataEntries(metadata);

  return (
    <div
      {...props}
      className={composeClassNames("pds-toast", className)}
      data-tone={tone}
      role={blocking ? "alert" : "status"}
      aria-live={blocking ? "assertive" : "polite"}
    >
      <span className="pds-toast__mark" aria-hidden="true" />
      <span className="pds-toast__copy">
        <strong>{title}</strong>
        {detail ? <span>{detail}</span> : null}
      </span>
      {entries.length ? (
        <dl className="pds-toast__metadata">
          {entries.map((entry, index) => (
            <div key={index} className="pds-toast__metadata-item">
              <dt>{entry.label}</dt>
              <dd>{entry.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {action ? <span className="pds-toast__action">{action}</span> : null}
      {onDismiss ? (
        <button
          type="button"
          className="pds-toast__dismiss"
          aria-label={dismissLabel}
          onClick={() => onDismiss(id)}
        >
          <span aria-hidden="true">x</span>
        </button>
      ) : null}
    </div>
  );
}

export type ToastRegionProps = HTMLAttributes<HTMLDivElement> & {
  toasts: readonly ToastItem[];
  onDismiss?: (id: string) => void;
  position?: "top-end" | "bottom-end";
  ariaLabel?: string;
};

export function ToastRegion({
  toasts,
  onDismiss,
  position = "top-end",
  ariaLabel = "Notifications",
  className,
  ...props
}: ToastRegionProps) {
  if (toasts.length === 0) return null;

  return (
    <div
      {...props}
      className={composeClassNames("pds-toast-region", className)}
      data-position={position}
      role="region"
      aria-label={ariaLabel}
      aria-live="polite"
    >
      {toasts.map((toast) => (
        <Toast key={toast.id} {...toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

export type SkeletonProps = HTMLAttributes<HTMLDivElement> & {
  lines?: number;
  label?: string;
  variant?: "text" | "card";
};

export function Skeleton({
  lines = 3,
  label = "Loading content",
  variant = "text",
  className,
  ...props
}: SkeletonProps) {
  const safeLines = Math.max(1, Math.min(lines, 8));
  return (
    <div
      {...props}
      className={composeClassNames("pds-skeleton", className)}
      data-variant={variant}
      role="status"
      aria-label={label}
      aria-busy="true"
    >
      {Array.from({ length: safeLines }, (_, index) => (
        <span key={index} className="pds-skeleton__line" aria-hidden="true" />
      ))}
    </div>
  );
}

function feedbackMetadataEntries(metadata?: FeedbackStateMetadata) {
  const entries: Array<{ label: ReactNode; value: ReactNode }> = [];
  if (!metadata) return entries;
  if (metadata.requestId) entries.push({ label: "Request ID", value: metadata.requestId });
  if (metadata.correlationId) entries.push({ label: "Correlation ID", value: metadata.correlationId });
  if (metadata.responseMs != null) entries.push({ label: "Response", value: `${Math.round(metadata.responseMs)} ms` });
  entries.push(...(metadata.details ?? []));
  return entries;
}

function trendGlyph(direction: MetricTrendProps["direction"]) {
  if (direction === "up") return "+";
  if (direction === "down") return "-";
  return "=";
}
