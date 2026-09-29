export const pdsIxRecipeRegistrySchema: "pds.ix.recipe_registry@1";
export const pdsIxRecipeRegistrationSchema: "pds.ix.recipe_registration@1";

export type PdsIxRecipeId =
  | "analyze-why"
  | "contextual-conversation"
  | "adaptive-composition"
  | "working-goal-plan"
  | "adaptive-information-lens"
  | "situation-to-strategy"
  | "attention-stewardship"
  | "ambient-agent-continuity";

export type PdsIxRecipeCapability =
  | "pds.ix.capability.focus-context@1"
  | "pds.ix.capability.progressive-presentation@1"
  | "pds.ix.capability.evidence-disclosure@1"
  | "pds.ix.capability.human-control@1"
  | "pds.ix.capability.causal-explanation@1"
  | "pds.ix.capability.contextual-follow-up@1"
  | "pds.ix.capability.composition-change-explanation@1"
  | "pds.ix.capability.goal-plan-revision@1"
  | "pds.ix.capability.full-record-fallback@1"
  | "pds.ix.capability.strategy-challenge@1"
  | "pds.ix.capability.attention-correction@1"
  | "pds.ix.capability.ambient-work-continuity@1";

export type PdsIxRendererProjection = "web-dom" | "native-ios" | "native-android";
export type PdsIxRecipeReadiness = "prototype" | "not-qualified";

export type PdsIxRecipeProjectionState = Readonly<{
  applicability: "supported";
  readiness: PdsIxRecipeReadiness;
}>;

export type PdsIxRecipeDescriptor = Readonly<{
  id: PdsIxRecipeId;
  ordinal: number;
  name: string;
  intentKey: `pds.ix.intent.${PdsIxRecipeId}@1`;
  rendererKey: `pds.ix.recipe.${PdsIxRecipeId}@1`;
  presentationSchemaVersion: "pds.ix.presentation@1";
  requiredCapabilities: readonly PdsIxRecipeCapability[];
  projections: Readonly<Record<PdsIxRendererProjection, PdsIxRecipeProjectionState>>;
}>;

export type PdsIxRecipeRegistry = Readonly<{
  schemaVersion: typeof pdsIxRecipeRegistrySchema;
  presentationSchemaVersion: "pds.ix.presentation@1";
  projections: readonly PdsIxRendererProjection[];
  capabilities: readonly PdsIxRecipeCapability[];
  recipes: readonly PdsIxRecipeDescriptor[];
}>;

type PdsIxRecipeRegistrationFor<
  TRecipeId extends PdsIxRecipeId,
  TRecipeCapability extends PdsIxRecipeCapability
> = {
  schemaVersion: typeof pdsIxRecipeRegistrationSchema;
  recipeId: TRecipeId;
  intentKey: `pds.ix.intent.${TRecipeId}@1`;
  artifactType: string;
  contentSchemaVersion: "pds.ix.presentation@1";
  rendererKey: `pds.ix.recipe.${TRecipeId}@1`;
  requiredCapabilities: readonly [
    "pds.ix.capability.focus-context@1",
    "pds.ix.capability.progressive-presentation@1",
    "pds.ix.capability.evidence-disclosure@1",
    "pds.ix.capability.human-control@1",
    TRecipeCapability
  ];
};

export type PdsIxRecipeRegistration =
  | PdsIxRecipeRegistrationFor<
      "analyze-why",
      "pds.ix.capability.causal-explanation@1"
    >
  | PdsIxRecipeRegistrationFor<
      "contextual-conversation",
      "pds.ix.capability.contextual-follow-up@1"
    >
  | PdsIxRecipeRegistrationFor<
      "adaptive-composition",
      "pds.ix.capability.composition-change-explanation@1"
    >
  | PdsIxRecipeRegistrationFor<
      "working-goal-plan",
      "pds.ix.capability.goal-plan-revision@1"
    >
  | PdsIxRecipeRegistrationFor<
      "adaptive-information-lens",
      "pds.ix.capability.full-record-fallback@1"
    >
  | PdsIxRecipeRegistrationFor<
      "situation-to-strategy",
      "pds.ix.capability.strategy-challenge@1"
    >
  | PdsIxRecipeRegistrationFor<
      "attention-stewardship",
      "pds.ix.capability.attention-correction@1"
    >
  | PdsIxRecipeRegistrationFor<
      "ambient-agent-continuity",
      "pds.ix.capability.ambient-work-continuity@1"
    >;

export type PdsIxRecipeRegistrationValidation =
  | { ok: true; value: PdsIxRecipeRegistration; errors: readonly [] }
  | { ok: false; value: null; errors: readonly string[] };

export type PdsIxRecipeProjection = Readonly<{
  registration: PdsIxRecipeRegistration;
  recipe: PdsIxRecipeDescriptor;
  projection: PdsIxRendererProjection;
  applicability: "supported";
  readiness: PdsIxRecipeReadiness;
}>;

export const pdsIxRecipeIds: readonly PdsIxRecipeId[];
export const pdsIxRecipeCapabilities: readonly PdsIxRecipeCapability[];
export const pdsIxRecipeRegistry: PdsIxRecipeRegistry;

export function getPdsIxRecipe(recipeId: unknown): PdsIxRecipeDescriptor | undefined;
export function validatePdsIxRecipeRegistration(
  value: unknown
): PdsIxRecipeRegistrationValidation;
export function assertPdsIxRecipeRegistration(value: unknown): PdsIxRecipeRegistration;
export function projectPdsIxRecipeRegistration(
  value: unknown,
  projection: PdsIxRendererProjection
): PdsIxRecipeProjection;
