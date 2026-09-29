export const pdsIxRecipeRegistrySchema = "pds.ix.recipe_registry@1";
export const pdsIxRecipeRegistrationSchema = "pds.ix.recipe_registration@1";

const presentationSchemaVersion = "pds.ix.presentation@1";
const projections = ["web-dom", "native-ios", "native-android"];
const commonCapabilities = [
  "pds.ix.capability.focus-context@1",
  "pds.ix.capability.progressive-presentation@1",
  "pds.ix.capability.evidence-disclosure@1",
  "pds.ix.capability.human-control@1"
];

const recipeInputs = [
  [
    "analyze-why",
    "Analyze Why",
    "pds.ix.capability.causal-explanation@1"
  ],
  [
    "contextual-conversation",
    "Contextual Conversation",
    "pds.ix.capability.contextual-follow-up@1"
  ],
  [
    "adaptive-composition",
    "Adaptive Composition",
    "pds.ix.capability.composition-change-explanation@1"
  ],
  [
    "working-goal-plan",
    "Working Goal Plan",
    "pds.ix.capability.goal-plan-revision@1"
  ],
  [
    "adaptive-information-lens",
    "Adaptive Information Lens",
    "pds.ix.capability.full-record-fallback@1"
  ],
  [
    "situation-to-strategy",
    "Situation to Strategy",
    "pds.ix.capability.strategy-challenge@1"
  ],
  [
    "attention-stewardship",
    "Attention Stewardship",
    "pds.ix.capability.attention-correction@1"
  ],
  [
    "ambient-agent-continuity",
    "Agents Helping You",
    "pds.ix.capability.ambient-work-continuity@1"
  ]
];

function deepFreeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const nested of Object.values(value)) deepFreeze(nested);
  }
  return value;
}

const recipes = recipeInputs.map(([id, name, capability], index) => ({
  id,
  ordinal: index + 1,
  name,
  intentKey: `pds.ix.intent.${id}@1`,
  rendererKey: `pds.ix.recipe.${id}@1`,
  presentationSchemaVersion,
  requiredCapabilities: [...commonCapabilities, capability],
  projections: {
    "web-dom": { applicability: "supported", readiness: "prototype" },
    "native-ios": { applicability: "supported", readiness: "not-qualified" },
    "native-android": { applicability: "supported", readiness: "not-qualified" }
  }
}));

export const pdsIxRecipeRegistry = deepFreeze({
  schemaVersion: pdsIxRecipeRegistrySchema,
  presentationSchemaVersion,
  projections,
  capabilities: [
    ...commonCapabilities,
    ...recipeInputs.map(([, , capability]) => capability)
  ],
  recipes
});

export const pdsIxRecipeIds = deepFreeze(
  pdsIxRecipeRegistry.recipes.map(({ id }) => id)
);
export const pdsIxRecipeCapabilities = pdsIxRecipeRegistry.capabilities;

const recipesById = new Map(
  pdsIxRecipeRegistry.recipes.map((recipe) => [recipe.id, recipe])
);
const registrationFields = [
  "schemaVersion",
  "recipeId",
  "intentKey",
  "artifactType",
  "contentSchemaVersion",
  "rendererKey",
  "requiredCapabilities"
];

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactFields(value, fields) {
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  return actual.length === expected.length
    && actual.every((field, index) => field === expected[index]);
}

function arraysEqual(left, right) {
  return Array.isArray(left)
    && left.length === right.length
    && left.every((value, index) => value === right[index]);
}

function isStableKey(value) {
  return typeof value === "string"
    && value.length > 0
    && value.length <= 256
    && /^[A-Za-z0-9._:@/-]+$/u.test(value);
}

export function getPdsIxRecipe(recipeId) {
  return typeof recipeId === "string" ? recipesById.get(recipeId) : undefined;
}

export function validatePdsIxRecipeRegistration(value) {
  const errors = [];
  if (!isRecord(value)) {
    errors.push("$ must be an object");
  } else {
    if (!hasExactFields(value, registrationFields)) {
      errors.push(`$ must contain only ${registrationFields.join(", ")}`);
    }
    if (value.schemaVersion !== pdsIxRecipeRegistrationSchema) {
      errors.push(`$.schemaVersion must equal ${pdsIxRecipeRegistrationSchema}`);
    }
    const recipe = getPdsIxRecipe(value.recipeId);
    if (!recipe) {
      errors.push("$.recipeId must name a registered PDS IX recipe");
    } else {
      if (value.intentKey !== recipe.intentKey) {
        errors.push(`$.intentKey must equal ${recipe.intentKey}`);
      }
      if (value.contentSchemaVersion !== recipe.presentationSchemaVersion) {
        errors.push(
          `$.contentSchemaVersion must equal ${recipe.presentationSchemaVersion}`
        );
      }
      if (value.rendererKey !== recipe.rendererKey) {
        errors.push(`$.rendererKey must equal ${recipe.rendererKey}`);
      }
      if (!arraysEqual(value.requiredCapabilities, recipe.requiredCapabilities)) {
        errors.push("$.requiredCapabilities must exactly match the canonical recipe order");
      }
    }
    if (!isStableKey(value.artifactType)) {
      errors.push("$.artifactType must be a stable ASCII key between 1 and 256 characters");
    }
  }
  return errors.length === 0
    ? { ok: true, value, errors: [] }
    : { ok: false, value: null, errors };
}

export function assertPdsIxRecipeRegistration(value) {
  const result = validatePdsIxRecipeRegistration(value);
  if (!result.ok) {
    throw new TypeError(
      `Invalid ${pdsIxRecipeRegistrationSchema}: ${result.errors.join("; ")}`
    );
  }
  return result.value;
}

export function projectPdsIxRecipeRegistration(value, projection) {
  const registration = assertPdsIxRecipeRegistration(value);
  if (!pdsIxRecipeRegistry.projections.includes(projection)) {
    throw new TypeError(`Unsupported PDS IX renderer projection: ${String(projection)}`);
  }
  const recipe = getPdsIxRecipe(registration.recipeId);
  const projectionState = recipe.projections[projection];
  return Object.freeze({
    registration,
    recipe,
    projection,
    applicability: projectionState.applicability,
    readiness: projectionState.readiness
  });
}
