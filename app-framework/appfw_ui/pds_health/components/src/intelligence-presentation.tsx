import { useId, type HTMLAttributes, type ReactNode } from "react";
import type {
  EvidenceDisclosureModel,
  ProgressiveResponsePresentationModel,
  ProgressiveResponseRegionModel,
  ResolvedContextPresentationModel,
  WorkStatusPresentationModel
} from "./intelligence-presentation-model";
import { composeClassNames } from "./types";

export type EvidenceDisclosureProps = HTMLAttributes<HTMLDetailsElement> & {
  model: EvidenceDisclosureModel;
};

/**
 * Inspectable source and derivation presentation. Products own the evidence
 * meaning and values; PDS owns only the accessible disclosure anatomy.
 */
export function EvidenceDisclosure({
  model,
  className,
  ...props
}: EvidenceDisclosureProps) {
  const disclosureId = useId();

  return (
    <details {...props} className={composeClassNames("pds-evidence-disclosure", className)}>
      <summary>{model.summary}</summary>
      <dl>
        {model.items.map((item, index) => (
          <div
            className="pds-evidence-disclosure__item"
            data-source-ref={item.sourceRef}
            key={item.sourceRef ?? item.id ?? index}
          >
            <dt id={`${disclosureId}-item-${index}`}>{item.label}</dt>
            <dd>{item.value}</dd>
          </div>
        ))}
      </dl>
    </details>
  );
}

export type ResolvedContextDisclosureProps = HTMLAttributes<HTMLElement> & {
  model: ResolvedContextPresentationModel;
  meta?: ReactNode;
  action?: ReactNode;
  ariaLabel?: string;
  announce?: boolean;
};

/**
 * Shows the bounded context behind a result without defining how context is
 * selected, authorized, refreshed, or corrected.
 */
export function ResolvedContextDisclosure({
  model,
  meta,
  action,
  ariaLabel = "Context used for this work",
  announce = true,
  className,
  ...props
}: ResolvedContextDisclosureProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-resolved-context", className)}
      aria-label={ariaLabel}
    >
      {announce ? (
        <p className="pds-assistive-announcement" role="status" aria-live="polite" aria-atomic="true">
          {model.announcement}
        </p>
      ) : null}
      <div className="pds-resolved-context__copy">
        <span className="pds-resolved-context__eyebrow">{model.eyebrow}</span>
        <strong>{model.title}</strong>
        {model.detail ? <span className="pds-resolved-context__detail">{model.detail}</span> : null}
        <EvidenceDisclosure model={model.evidence} />
        {model.gaps.length > 0 ? (
          <div className="pds-resolved-context__gaps">
            <strong>Context gaps</strong>
            <ul>{model.gaps.map((gap) => <li key={gap}>{gap}</li>)}</ul>
          </div>
        ) : null}
      </div>
      {meta || model.metaLabel || action ? (
        <div className="pds-resolved-context__meta">
          {meta ?? (model.metaLabel ? <span>{model.metaLabel}</span> : null)}
          {action}
        </div>
      ) : null}
    </section>
  );
}

export type WorkStatusProps = HTMLAttributes<HTMLElement> & {
  model: WorkStatusPresentationModel;
  action?: ReactNode;
  ariaLabel?: string;
  announce?: boolean;
};

/**
 * A fail-visible status row for progressive work. `active` affects only its
 * presentation; callers retain ownership of lifecycle and cancellation.
 */
export function WorkStatus({
  model,
  action,
  ariaLabel = "Work status",
  announce = true,
  className,
  ...props
}: WorkStatusProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-work-status", className)}
      aria-label={ariaLabel}
      data-active={model.active || undefined}
    >
      <span className="pds-work-status__signal" aria-hidden="true" />
      <span
        className="pds-work-status__copy"
        role={announce ? "status" : undefined}
        aria-live={announce ? "polite" : undefined}
        aria-atomic={announce ? "true" : undefined}
      >
        <strong>{model.label}</strong>
        {model.detail ? <span>{model.detail}</span> : null}
      </span>
      {action ? <span className="pds-work-status__action">{action}</span> : null}
    </section>
  );
}

export type ProgressiveResponseProps = HTMLAttributes<HTMLElement> & {
  model: ProgressiveResponsePresentationModel;
  meta?: ReactNode;
  renderBody?: (region: ProgressiveResponseRegionModel) => ReactNode;
  renderAction?: (region: ProgressiveResponseRegionModel) => ReactNode;
  renderEditAction?: (region: ProgressiveResponseRegionModel) => ReactNode;
  footer?: ReactNode;
  announce?: boolean;
};

/**
 * Domain-neutral, progressively revealed response regions with a mandatory
 * assistive-technology announcement. Callers own the content, revision,
 * status, evidence, editing, and action semantics.
 */
export function ProgressiveResponse({
  model,
  meta,
  renderBody,
  renderAction,
  renderEditAction,
  footer,
  announce = true,
  className,
  ...props
}: ProgressiveResponseProps) {
  const titleId = useId();

  const renderRegionEditAction = (region: ProgressiveResponseRegionModel) => {
    if (!region.editable || !renderEditAction) return null;
    const action = renderEditAction(region);
    return action === null || action === undefined || action === false ? null : (
      <div className="pds-progressive-response__action">{action}</div>
    );
  };

  if (model.regions.length === 0) {
    return (
      <div className={composeClassNames("pds-progressive-response__empty", className)}>
        {announce ? (
          <p className="pds-assistive-announcement" role="status" aria-live="polite" aria-atomic="true">
            {model.announcement}
          </p>
        ) : null}
        <span>{model.emptyState}</span>
      </div>
    );
  }

  return (
    <section
      {...props}
      className={composeClassNames("pds-progressive-response", className)}
      aria-labelledby={titleId}
      data-active={model.active || undefined}
    >
      {announce ? (
        <p className="pds-assistive-announcement" role="status" aria-live="polite" aria-atomic="true">
          {model.announcement}
        </p>
      ) : null}
      <header className="pds-progressive-response__header">
        <span>
          <span className="pds-progressive-response__eyebrow">{model.eyebrow}</span>
          <h3 id={titleId}>{model.title}</h3>
        </span>
        {meta ?? (model.metaLabel ? <span>{model.metaLabel}</span> : null)}
      </header>
      <div className="pds-progressive-response__regions">
        {model.regions.map((region) => (
          <article
            className="pds-progressive-response__region"
            data-changed={region.changed || undefined}
            data-status={region.status}
            key={region.id}
          >
            <div className="pds-progressive-response__region-meta">
              {region.label ? (
                <span className="pds-progressive-response__region-label">{region.label}</span>
              ) : null}
              <span className="pds-progressive-response__region-status">{region.status}</span>
            </div>
            <h4>{region.title}</h4>
            <div className="pds-progressive-response__region-body">
              {renderBody?.(region) ?? <p>{region.body}</p>}
            </div>
            {region.whyItMatters ? (
              <p className="pds-progressive-response__why"><strong>Why this matters:</strong> {region.whyItMatters}</p>
            ) : null}
            {region.evidence ? <EvidenceDisclosure model={region.evidence} /> : null}
            {region.actionLabel && renderAction ? (
              <div className="pds-progressive-response__action">{renderAction(region)}</div>
            ) : null}
            {renderRegionEditAction(region)}
          </article>
        ))}
      </div>
      {footer || model.footerText ? (
        <footer className="pds-progressive-response__footer">{footer ?? model.footerText}</footer>
      ) : null}
    </section>
  );
}
