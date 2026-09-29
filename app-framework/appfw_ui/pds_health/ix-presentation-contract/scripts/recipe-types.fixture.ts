import type { PdsIxRecipeRegistration } from "../src/recipes.js";

type AnalyzeWhyRegistration = Extract<
  PdsIxRecipeRegistration,
  { recipeId: "analyze-why" }
>;

const analyzeWhy: AnalyzeWhyRegistration = {
  schemaVersion: "pds.ix.recipe_registration@1",
  recipeId: "analyze-why",
  intentKey: "pds.ix.intent.analyze-why@1",
  artifactType: "fixture.product.analysis@1",
  contentSchemaVersion: "pds.ix.presentation@1",
  rendererKey: "pds.ix.recipe.analyze-why@1",
  requiredCapabilities: [
    "pds.ix.capability.focus-context@1",
    "pds.ix.capability.progressive-presentation@1",
    "pds.ix.capability.evidence-disclosure@1",
    "pds.ix.capability.human-control@1",
    "pds.ix.capability.causal-explanation@1"
  ]
};

const wrongIntent: AnalyzeWhyRegistration = {
  ...analyzeWhy,
  // @ts-expect-error Recipe identity and intent are one closed tuple.
  intentKey: "pds.ix.intent.contextual-conversation@1"
};

const wrongRenderer: AnalyzeWhyRegistration = {
  ...analyzeWhy,
  // @ts-expect-error Recipe identity and renderer are one closed tuple.
  rendererKey: "pds.ix.recipe.contextual-conversation@1"
};

const wrongCapability: AnalyzeWhyRegistration = {
  ...analyzeWhy,
  requiredCapabilities: [
    "pds.ix.capability.focus-context@1",
    "pds.ix.capability.progressive-presentation@1",
    "pds.ix.capability.evidence-disclosure@1",
    "pds.ix.capability.human-control@1",
    // @ts-expect-error Recipe identity and fifth capability are one closed tuple.
    "pds.ix.capability.contextual-follow-up@1"
  ]
};

const productOwnedArtifactType: AnalyzeWhyRegistration = {
  ...analyzeWhy,
  artifactType: "another.product.analysis@7"
};

void wrongIntent;
void wrongRenderer;
void wrongCapability;
void productOwnedArtifactType;
