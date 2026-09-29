import {
  projectPdsIxRecipeRegistration,
  validatePdsIxPresentation,
  type PdsIxPresentationEnvelope,
  type PdsIxRecipeProjection,
  type PdsIxRecipeRegistration,
  type ProgressiveResponseRegionModel
} from "@appfw/pds-ix-presentation-contract";
import type { ReactNode } from "react";
import {
  ProgressiveResponse,
  ResolvedContextDisclosure,
  WorkStatus
} from "./intelligence-presentation";

export function resolvePdsIxWebRecipe(registration: unknown): PdsIxRecipeProjection {
  return projectPdsIxRecipeRegistration(registration, "web-dom");
}

export type PdsIxRecipePresentationProps = {
  registration: unknown;
  presentation: unknown;
  className?: string;
  onContextAction?: (registration: PdsIxRecipeRegistration) => void;
  onStatusAction?: (registration: PdsIxRecipeRegistration) => void;
  onRegionAction?: (
    region: ProgressiveResponseRegionModel,
    registration: PdsIxRecipeRegistration
  ) => void;
  onRegionEdit?: (
    region: ProgressiveResponseRegionModel,
    registration: PdsIxRecipeRegistration
  ) => void;
  renderRegionBody?: (
    region: ProgressiveResponseRegionModel,
    registration: PdsIxRecipeRegistration
  ) => ReactNode;
};

function invalidPresentation() {
  return (
    <section
      aria-label="Intelligence presentation unavailable"
      className="pds-ix-recipe-presentation pds-ix-recipe-presentation--invalid"
      data-pds-ix-recipe-invalid="true"
      role="alert"
    >
      This intelligence presentation is unavailable.
    </section>
  );
}

/**
 * Channel-appropriate Web composition for any registered PDS IX recipe.
 * Product data, copy, actions, policy, and lifecycle remain caller-owned.
 */
export function PdsIxRecipePresentation({
  registration,
  presentation,
  className,
  onContextAction,
  onStatusAction,
  onRegionAction,
  onRegionEdit,
  renderRegionBody
}: PdsIxRecipePresentationProps) {
  let projection: PdsIxRecipeProjection;
  try {
    projection = resolvePdsIxWebRecipe(registration);
  } catch {
    return invalidPresentation();
  }
  const validation = validatePdsIxPresentation(presentation);
  if (!validation.ok) return invalidPresentation();

  const model: PdsIxPresentationEnvelope = validation.value;
  const recipeRegistration = projection.registration;
  return (
    <section
      aria-label={`${projection.recipe.name} intelligence presentation`}
      className={className ?? "pds-ix-recipe-presentation"}
      data-pds-ix-recipe={projection.recipe.id}
      data-pds-ix-readiness={projection.readiness}
      data-pds-ix-renderer={recipeRegistration.rendererKey}
      data-pds-ix-projection="web-dom"
    >
      <p className="pds-assistive-announcement" role="status" aria-live="polite" aria-atomic="true">
        {model.announcement}
      </p>
      {model.context ? (
        <ResolvedContextDisclosure
          model={model.context}
          announce={false}
          action={
            model.context.actionLabel && onContextAction ? (
              <button type="button" onClick={() => onContextAction(recipeRegistration)}>
                {model.context.actionLabel}
              </button>
            ) : undefined
          }
        />
      ) : null}
      {model.workStatus ? (
        <WorkStatus
          model={model.workStatus}
          announce={false}
          action={
            model.workStatus.actionLabel && onStatusAction ? (
              <button type="button" onClick={() => onStatusAction(recipeRegistration)}>
                {model.workStatus.actionLabel}
              </button>
            ) : undefined
          }
        />
      ) : null}
      <ProgressiveResponse
        model={model.response}
        announce={false}
        renderBody={
          renderRegionBody
            ? (region) => renderRegionBody(region, recipeRegistration)
            : undefined
        }
        renderAction={
          onRegionAction
            ? (region) => (
                <button
                  type="button"
                  onClick={() => onRegionAction(region, recipeRegistration)}
                >
                  {region.actionLabel}
                </button>
              )
            : undefined
        }
        renderEditAction={
          onRegionEdit
            ? (region) => (
                <button
                  type="button"
                  onClick={() => onRegionEdit(region, recipeRegistration)}
                >
                  Edit this section
                </button>
              )
            : undefined
        }
      />
    </section>
  );
}
