import type { CSSProperties, HTMLAttributes, ReactNode } from "react";
import { composeClassNames, type PdsDensity } from "./types";

export type ProcessStepStatus = "complete" | "current" | "upcoming" | "warning" | "blocked";
export type ProcessStepperVariant = "default" | "milestone";

export type ProcessStepItem = {
  id: string;
  label: ReactNode;
  description?: ReactNode;
  metadata?: ReactNode;
  status?: ProcessStepStatus;
  optional?: boolean;
  disabled?: boolean;
  controls?: string;
  content?: ReactNode;
};

export type ProcessStepperProps = Omit<HTMLAttributes<HTMLOListElement>, "onSelect"> & {
  ariaLabel: string;
  steps: readonly ProcessStepItem[];
  currentStepId?: string;
  selectedStepId?: string | null;
  orientation?: "horizontal" | "vertical";
  density?: PdsDensity;
  variant?: ProcessStepperVariant;
  compactPresentation?: "segments";
  onStepSelect?: (step: ProcessStepItem, index: number) => void;
};

export function ProcessStepper({
  ariaLabel,
  steps,
  currentStepId,
  selectedStepId,
  orientation = "horizontal",
  density = "comfortable",
  variant = "default",
  compactPresentation,
  onStepSelect,
  className,
  style,
  ...props
}: ProcessStepperProps) {
  const currentIndex = resolveCurrentStepIndex(steps, currentStepId);
  const resolvedSelectedStepId =
    selectedStepId === undefined
      ? (onStepSelect ? steps[currentIndex]?.id : undefined)
      : (selectedStepId ?? undefined);
  const isInteractive = Boolean(onStepSelect);
  const stepperStyle = {
    ...style,
    "--pds-process-step-count": Math.max(steps.length, 1),
    "--pds-process-milestone-min-width": `${Math.max(steps.length, 1) * 112}px`
  } as CSSProperties;

  const stepper = (
    <ol
      {...props}
      className={composeClassNames("pds-process-stepper", className)}
      style={stepperStyle}
      tabIndex={orientation === "horizontal" ? 0 : undefined}
      data-orientation={orientation}
      data-density={density}
      data-variant={variant}
      data-interactive={isInteractive || undefined}
      data-selected-step-id={resolvedSelectedStepId}
      aria-label={ariaLabel}
    >
      {steps.map((step, index) => {
        const status = resolveStepStatus(step, index, currentIndex);
        const isCurrent = status === "current";
        const isSelected = step.id === resolvedSelectedStepId;
        const canSelect = isInteractive && !step.disabled;
        const triggerProps = {
          className: "pds-process-stepper__trigger",
          "aria-current": isCurrent ? "step" as const : undefined,
          "aria-controls": step.controls,
          "aria-disabled": step.disabled || undefined,
          "data-current": isCurrent || undefined,
          "data-selected": isSelected || undefined,
          "data-status": status
        };

        return (
          <li
            key={step.id}
            className="pds-process-stepper__step"
            data-step-id={step.id}
            data-status={status}
            data-current={isCurrent || undefined}
            data-selected={isSelected || undefined}
            data-selectable={canSelect || undefined}
            data-disabled={step.disabled || undefined}
          >
            <span className="pds-process-stepper__connector" aria-hidden="true" />
            {isInteractive ? (
              <button
                {...triggerProps}
                className="pds-process-stepper__trigger pds-process-stepper__selection-surface"
                type="button"
                disabled={step.disabled}
                aria-pressed={isSelected}
                onClick={() => onStepSelect?.(step, index)}
              >
                {stepMarker(index, status, variant)}
                {stepContent(step, status)}
              </button>
            ) : (
              <>
                {stepMarker(index, status, variant)}
                <span {...triggerProps}>{stepContent(step, status)}</span>
              </>
            )}
            {step.content ? (
              <div id={step.controls} className="pds-process-stepper__content">
                {step.content}
              </div>
            ) : null}
          </li>
        );
      })}
    </ol>
  );

  if (compactPresentation !== "segments" || steps.length === 0) {
    return stepper;
  }

  return (
    <div
      className="pds-process-stepper-adaptive"
      data-compact-presentation={compactPresentation}
      data-interactive={isInteractive || undefined}
    >
      <div className="pds-process-stepper-adaptive__full">{stepper}</div>
      <ProcessProgress
        className="pds-process-stepper-adaptive__compact"
        aria-hidden="true"
        ariaLabel={`${ariaLabel} progress`}
        currentStep={currentIndex + 1}
        totalSteps={steps.length}
        variant="segments"
      />
    </div>
  );
}

function stepMarker(
  index: number,
  status: ProcessStepStatus,
  variant: ProcessStepperVariant
) {
  const content =
    variant === "milestone"
      ? milestoneMarkerContent(status, index)
      : index + 1;

  return (
    <span
      className="pds-process-stepper__marker"
      data-symbol={variant === "milestone" ? status : undefined}
      aria-hidden="true"
    >
      <span>{content}</span>
    </span>
  );
}

function milestoneMarkerContent(status: ProcessStepStatus, index: number) {
  switch (status) {
    case "complete":
      return "✓";
    case "current":
      return "•";
    case "warning":
    case "blocked":
      return "!";
    case "upcoming":
    default:
      return index + 1;
  }
}

export type ProcessProgressProps = HTMLAttributes<HTMLDivElement> & {
  ariaLabel?: string;
  label?: ReactNode;
  detail?: ReactNode;
  currentStep: number;
  totalSteps: number;
  variant?: "bar" | "dots" | "segments";
};

export function ProcessProgress({
  ariaLabel = "Process progress",
  label,
  detail,
  currentStep,
  totalSteps,
  variant = "bar",
  className,
  ...props
}: ProcessProgressProps) {
  const normalizedTotal = Math.max(1, totalSteps);
  const normalizedCurrent = Math.min(Math.max(0, currentStep), normalizedTotal);
  const percent = Math.round((normalizedCurrent / normalizedTotal) * 100);
  const ariaValueText =
    variant === "segments"
      ? `Step ${normalizedCurrent} of ${normalizedTotal}`
      : `${normalizedCurrent} of ${normalizedTotal} steps complete`;
  const progressStyle = {
    "--pds-process-progress-value": `${percent}%`,
    "--pds-process-progress-segment-count": normalizedTotal
  } as CSSProperties;

  return (
    <div
      {...props}
      className={composeClassNames("pds-process-progress", className)}
      data-variant={variant}
      style={{ ...progressStyle, ...props.style }}
    >
      {label || detail ? (
        <span className="pds-process-progress__copy">
          {label ? <strong>{label}</strong> : null}
          {detail ? <span>{detail}</span> : null}
        </span>
      ) : null}
      {variant === "segments" ? (
        <span
          className="pds-process-progress__segments"
          role="progressbar"
          aria-label={ariaLabel}
          aria-valuemin={0}
          aria-valuemax={normalizedTotal}
          aria-valuenow={normalizedCurrent}
          aria-valuetext={ariaValueText}
        >
          {Array.from({ length: normalizedTotal }, (_, index) => {
            const status =
              index < normalizedCurrent - 1
                ? "complete"
                : index === normalizedCurrent - 1
                  ? "current"
                  : "upcoming";
            return <span key={index} data-status={status} />;
          })}
        </span>
      ) : (
        <span
          className="pds-process-progress__track"
          role="progressbar"
          aria-label={ariaLabel}
          aria-valuemin={0}
          aria-valuemax={normalizedTotal}
          aria-valuenow={normalizedCurrent}
          aria-valuetext={ariaValueText}
        >
          <span className="pds-process-progress__bar" />
        </span>
      )}
      {variant === "dots" ? (
        <ol className="pds-process-progress__dots" aria-hidden="true">
          {Array.from({ length: normalizedTotal }, (_, index) => (
            <li key={index} data-active={index < normalizedCurrent || undefined} />
          ))}
        </ol>
      ) : null}
    </div>
  );
}

function resolveCurrentStepIndex(steps: readonly ProcessStepItem[], currentStepId?: string) {
  if (currentStepId) {
    const explicit = steps.findIndex((step) => step.id === currentStepId);
    if (explicit >= 0) return explicit;
  }

  const marked = steps.findIndex((step) => step.status === "current");
  return marked >= 0 ? marked : 0;
}

function resolveStepStatus(
  step: ProcessStepItem,
  index: number,
  currentIndex: number
): ProcessStepStatus {
  if (step.status) return step.status;
  if (index < currentIndex) return "complete";
  if (index === currentIndex) return "current";
  return "upcoming";
}

function stepContent(step: ProcessStepItem, status: ProcessStepStatus) {
  return (
    <span className="pds-process-stepper__copy">
      <span className="pds-process-stepper__title-row">
        <strong>{step.label}</strong>
        {step.optional ? <span className="pds-process-stepper__optional">Optional</span> : null}
      </span>
      {step.description ? <span className="pds-process-stepper__description">{step.description}</span> : null}
      <span className="pds-process-stepper__meta">
        <span className="pds-process-stepper__status-label">{processStatusLabel(status)}</span>
        {step.metadata ? (
          <span className="pds-process-stepper__metadata">{step.metadata}</span>
        ) : null}
      </span>
    </span>
  );
}

function processStatusLabel(status: ProcessStepStatus) {
  switch (status) {
    case "complete":
      return "Complete";
    case "current":
      return "Current";
    case "warning":
      return "Needs review";
    case "blocked":
      return "Blocked";
    case "upcoming":
    default:
      return "Upcoming";
  }
}
