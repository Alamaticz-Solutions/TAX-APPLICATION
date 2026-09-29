import type { ChangeEvent, HTMLAttributes, ReactNode } from "react";
import { composeClassNames, type PdsTone } from "./types";

export type AiGroundingState = "grounded" | "partial" | "unverified" | "policy_denied";
export type AiRiskTier = "low" | "medium" | "high";
export type AssistLevel = "suggest" | "copilot" | "autopilot";

export type AssistLevelOption = {
  value: AssistLevel;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
};

const defaultAssistLevelOptions = [
  {
    value: "suggest",
    label: "Suggest",
    description: "Recommend and ask before every action."
  },
  {
    value: "copilot",
    label: "Co-pilot",
    description: "Help with routine steps; ask on important changes."
  },
  {
    value: "autopilot",
    label: "Autopilot",
    description: "Act only when live governed-write evidence permits it.",
    disabled: true
  }
] as const satisfies readonly AssistLevelOption[];

export type AssistLevelControlProps = HTMLAttributes<HTMLFieldSetElement> & {
  label?: ReactNode;
  detail?: ReactNode;
  value?: AssistLevel;
  options?: readonly AssistLevelOption[];
  onValueChange?: (value: AssistLevel) => void;
  name?: string;
};

export function AssistLevelControl({
  label = "Assist level",
  detail = "Default to the most conservative level per task type.",
  value = "suggest",
  options = defaultAssistLevelOptions,
  onValueChange,
  name = "assist-level",
  className,
  ...props
}: AssistLevelControlProps) {
  const descriptionId = `${name}-description`;
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    onValueChange?.(event.currentTarget.value as AssistLevel);
  };

  return (
    <fieldset
      {...props}
      className={composeClassNames("pds-assist-level-control", className)}
      aria-describedby={detail ? descriptionId : undefined}
    >
      <legend className="pds-assist-level-control__legend">{label}</legend>
      {detail ? <p id={descriptionId} className="pds-assist-level-control__detail">{detail}</p> : null}
      <div className="pds-assist-level-control__options">
        {options.map((option) => {
          const id = `${name}-${option.value}`;
          const checked = option.value === value;
          return (
            <label key={option.value} className="pds-assist-level-control__option" data-selected={checked || undefined}>
              <input
                id={id}
                type="radio"
                name={name}
                value={option.value}
                checked={checked}
                disabled={option.disabled}
                onChange={handleChange}
              />
              <span className="pds-assist-level-control__copy">
                <strong>{option.label}</strong>
                {option.description ? <small>{option.description}</small> : null}
              </span>
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}

export type MemoryChipProps = HTMLAttributes<HTMLDivElement> & {
  label?: ReactNode;
  detail?: ReactNode;
  scope?: ReactNode;
  correctionLabel?: ReactNode;
  resetLabel?: ReactNode;
  onCorrect?: () => void;
  onReset?: () => void;
};

export function MemoryChip({
  label = "Personalized for you",
  detail,
  scope,
  correctionLabel = "Correct",
  resetLabel = "Reset",
  onCorrect,
  onReset,
  className,
  ...props
}: MemoryChipProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-memory-chip", className)}
      role="group"
      aria-label={typeof label === "string" ? label : "Personalized memory control"}
    >
      <span className="pds-memory-chip__mark" aria-hidden="true" />
      <span className="pds-memory-chip__copy">
        <strong>{label}</strong>
        {detail ? <small>{detail}</small> : null}
      </span>
      {scope ? <span className="pds-memory-chip__scope">{scope}</span> : null}
      <span className="pds-memory-chip__actions">
        <button type="button" onClick={onCorrect}>{correctionLabel}</button>
        <button type="button" onClick={onReset}>{resetLabel}</button>
      </span>
    </div>
  );
}

export type AiAttributionAffordanceProps = HTMLAttributes<HTMLDivElement> & {
  label?: ReactNode;
  detail?: ReactNode;
  model?: ReactNode;
  generatedAt?: ReactNode;
  sourceCount?: number;
};

export function AiAttributionAffordance({
  label = "AI-assisted",
  detail,
  model,
  generatedAt,
  sourceCount,
  className,
  ...props
}: AiAttributionAffordanceProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-ai-attribution", className)}
      role="status"
      aria-live="polite"
    >
      <span className="pds-ai-attribution__mark" aria-hidden="true">AI</span>
      <span className="pds-ai-attribution__copy">
        <strong>{label}</strong>
        {detail ? <small>{detail}</small> : null}
      </span>
      {model || generatedAt || typeof sourceCount === "number" ? (
        <span className="pds-ai-attribution__meta">
          {model ? <span>{model}</span> : null}
          {generatedAt ? <time>{generatedAt}</time> : null}
          {typeof sourceCount === "number" ? (
            <span>{sourceCount} {sourceCount === 1 ? "source" : "sources"}</span>
          ) : null}
        </span>
      ) : null}
    </div>
  );
}

export type FreshnessIndicatorValue = "current" | "stale" | "unknown";

export type FreshnessIndicatorProps = HTMLAttributes<HTMLDivElement> & {
  value?: FreshnessIndicatorValue;
  label?: ReactNode;
  detail?: ReactNode;
  timestamp?: ReactNode;
};

export function FreshnessIndicator({
  value = "unknown",
  label = "Freshness",
  detail,
  timestamp,
  className,
  ...props
}: FreshnessIndicatorProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-freshness-indicator", className)}
      data-freshness={value}
      role="status"
      aria-live="polite"
    >
      <span className="pds-freshness-indicator__dot" aria-hidden="true" />
      <span className="pds-freshness-indicator__copy">
        <strong>{label}</strong>
        {detail ? <small>{detail}</small> : null}
      </span>
      {timestamp ? <time className="pds-freshness-indicator__timestamp">{timestamp}</time> : null}
    </div>
  );
}

export type AttentionMarkerProps = HTMLAttributes<HTMLDivElement> & {
  label: ReactNode;
  detail?: ReactNode;
  tone?: Extract<PdsTone, "accent" | "warning" | "danger">;
  risk?: AiRiskTier;
};

export function AttentionMarker({
  label,
  detail,
  tone = "accent",
  risk = "low",
  className,
  ...props
}: AttentionMarkerProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-attention-marker", className)}
      data-tone={tone}
      data-risk={risk}
      role={risk === "high" ? "alert" : "status"}
      aria-live={risk === "high" ? "assertive" : "polite"}
    >
      <span className="pds-attention-marker__glyph" aria-hidden="true" />
      <span className="pds-attention-marker__copy">
        <strong>{label}</strong>
        {detail ? <small>{detail}</small> : null}
      </span>
    </div>
  );
}

export type GeneratedViewShellProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  description?: ReactNode;
  viewId?: string;
  grounding?: AiGroundingState;
  attribution?: ReactNode;
  freshness?: ReactNode;
  confidence?: ReactNode;
  toolbar?: ReactNode;
  fallback?: ReactNode;
  unresolvedCount?: number;
  ariaLabel?: string;
};

export function GeneratedViewShell({
  title,
  description,
  viewId,
  grounding = "grounded",
  attribution,
  freshness,
  confidence,
  toolbar,
  fallback,
  unresolvedCount = 0,
  ariaLabel = "Generated view",
  className,
  children,
  ...props
}: GeneratedViewShellProps) {
  const hasContent = Boolean(children);
  return (
    <section
      {...props}
      className={composeClassNames("pds-generated-view-shell", className)}
      data-grounding={grounding}
      data-view-id={viewId}
      aria-label={ariaLabel}
    >
      <header className="pds-generated-view-shell__header">
        <div className="pds-generated-view-shell__copy">
          <span className="pds-generated-view-shell__eyebrow">Generated view</span>
          <h3 className="pds-generated-view-shell__title">{title}</h3>
          {description ? <p className="pds-generated-view-shell__description">{description}</p> : null}
        </div>
        {toolbar ? <div className="pds-generated-view-shell__toolbar">{toolbar}</div> : null}
      </header>
      <div className="pds-generated-view-shell__signals">
        {attribution ?? <AiAttributionAffordance detail="Composed from resolved product data." />}
        {freshness}
        {confidence}
      </div>
      {unresolvedCount > 0 ? (
        <AttentionMarker
          tone="warning"
          risk="medium"
          label={`${unresolvedCount} unresolved references`}
          detail="The view is partial; unresolved references are not rendered."
        />
      ) : null}
      <div className="pds-generated-view-shell__body">
        {hasContent ? children : fallback ?? (
          <p className="pds-generated-view-shell__fallback">No grounded view is available.</p>
        )}
      </div>
    </section>
  );
}

export type SuggestedActionProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  reason: ReactNode;
  risk?: AiRiskTier;
  grounding?: AiGroundingState;
  marker?: ReactNode;
  evidence?: ReactNode;
  preview?: ReactNode;
  actions?: ReactNode;
};

export function SuggestedAction({
  title,
  reason,
  risk = "low",
  grounding = "grounded",
  marker,
  evidence,
  preview,
  actions,
  className,
  ...props
}: SuggestedActionProps) {
  return (
    <article
      {...props}
      className={composeClassNames("pds-suggested-action", className)}
      data-risk={risk}
      data-grounding={grounding}
      aria-label={typeof title === "string" ? title : "Suggested action"}
    >
      <header className="pds-suggested-action__header">
        {marker ? <span className="pds-suggested-action__marker">{marker}</span> : null}
        <div className="pds-suggested-action__copy">
          <span className="pds-suggested-action__eyebrow">Suggested next step</span>
          <h3 className="pds-suggested-action__title">{title}</h3>
          <p className="pds-suggested-action__reason">{reason}</p>
        </div>
        <AttentionMarker
          label={`${risk} risk`}
          detail="Preview required before any write."
          risk={risk}
          tone={risk === "high" ? "danger" : risk === "medium" ? "warning" : "accent"}
        />
      </header>
      {evidence ? <div className="pds-suggested-action__evidence">{evidence}</div> : null}
      {preview ? <div className="pds-suggested-action__preview">{preview}</div> : null}
      {actions ? <div className="pds-suggested-action__actions">{actions}</div> : null}
    </article>
  );
}

export type RecommendationCardProps = SuggestedActionProps & {
  rank?: ReactNode;
};

export function RecommendationCard({
  rank,
  className,
  ...props
}: RecommendationCardProps) {
  return (
    <SuggestedAction
      {...props}
      className={composeClassNames("pds-recommendation-card", className)}
      data-rank={rank ? String(rank) : undefined}
      marker={rank ? <span className="pds-recommendation-card__rank">{rank}</span> : undefined}
    />
  );
}

export type EvidenceSummaryClaim = {
  id: string;
  text: ReactNode;
  citation?: ReactNode;
};

export type EvidenceSummaryProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  description?: ReactNode;
  claims?: readonly EvidenceSummaryClaim[];
  attribution?: ReactNode;
  freshness?: ReactNode;
  citations?: ReactNode;
  emptyState?: ReactNode;
};

export function EvidenceSummary({
  title,
  description,
  claims = [],
  attribution,
  freshness,
  citations,
  emptyState = "Summary unavailable until source references resolve.",
  className,
  ...props
}: EvidenceSummaryProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-evidence-summary", className)}
      aria-label={typeof title === "string" ? title : "Evidence summary"}
    >
      <header className="pds-evidence-summary__header">
        <div className="pds-evidence-summary__copy">
          <span className="pds-evidence-summary__eyebrow">Evidence summary</span>
          <h3 className="pds-evidence-summary__title">{title}</h3>
          {description ? <p className="pds-evidence-summary__description">{description}</p> : null}
        </div>
        {freshness}
      </header>
      {attribution ?? <AiAttributionAffordance detail="Every claim must link to resolved records." />}
      {claims.length > 0 ? (
        <ol className="pds-evidence-summary__claims" aria-label="Grounded claims">
          {claims.map((claim) => (
            <li key={claim.id} className="pds-evidence-summary__claim">
              <span>{claim.text}</span>
              {claim.citation ? <small>{claim.citation}</small> : null}
            </li>
          ))}
        </ol>
      ) : (
        <p className="pds-evidence-summary__empty">{emptyState}</p>
      )}
      {citations ? <div className="pds-evidence-summary__citations">{citations}</div> : null}
    </section>
  );
}

export type InsightSummaryProps = EvidenceSummaryProps & {
  insightTone?: Extract<PdsTone, "accent" | "success" | "warning" | "danger">;
};

export function InsightSummary({
  insightTone = "accent",
  className,
  ...props
}: InsightSummaryProps) {
  return (
    <EvidenceSummary
      {...props}
      className={composeClassNames("pds-insight-summary", className)}
      data-tone={insightTone}
    />
  );
}
