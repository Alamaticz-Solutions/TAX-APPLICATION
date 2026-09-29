import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

import {
  assertPdsIxRecipeRegistration,
  getPdsIxRecipe,
  pdsIxRecipeCapabilities,
  pdsIxRecipeIds,
  pdsIxRecipeRegistrationSchema,
  pdsIxRecipeRegistry,
  pdsIxRecipeRegistrySchema,
  projectPdsIxRecipeRegistration,
  validatePdsIxRecipeRegistration
} from "../src/index.js";

const registryUrl = new URL("../registry/pds.ix.recipe-registry.v1.json", import.meta.url);
const registrySchemaUrl = new URL(
  "../schema/pds.ix.recipe-registry.v1.schema.json",
  import.meta.url
);
const registrationSchemaUrl = new URL(
  "../schema/pds.ix.recipe-registration.v1.schema.json",
  import.meta.url
);

async function readJson(url) {
  return JSON.parse(await readFile(url, "utf8"));
}

function registrationFor(recipe) {
  return {
    schemaVersion: pdsIxRecipeRegistrationSchema,
    recipeId: recipe.id,
    intentKey: recipe.intentKey,
    artifactType: `product.ix.${recipe.id}@1`,
    contentSchemaVersion: recipe.presentationSchemaVersion,
    rendererKey: recipe.rendererKey,
    requiredCapabilities: [...recipe.requiredCapabilities]
  };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

test("canonical registry is an exact frozen 8/8 contract", async () => {
  const source = await readJson(registryUrl);
  const schema = await readJson(registrySchemaUrl);
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);

  assert.equal(
    schema.$id,
    "https://appfw.pdshealth.com/schemas/pds.ix.recipe-registry.v1.schema.json"
  );
  assert.equal(pdsIxRecipeRegistry.schemaVersion, pdsIxRecipeRegistrySchema);
  assert.equal(validate(source), true, JSON.stringify(validate.errors));
  assert.deepEqual(pdsIxRecipeRegistry, source);
  assert.deepEqual(pdsIxRecipeIds, [
    "analyze-why",
    "contextual-conversation",
    "adaptive-composition",
    "working-goal-plan",
    "adaptive-information-lens",
    "situation-to-strategy",
    "attention-stewardship",
    "ambient-agent-continuity"
  ]);
  assert.deepEqual(source.recipes.map(({ ordinal }) => ordinal), [1, 2, 3, 4, 5, 6, 7, 8]);
  assert.equal(new Set(source.recipes.map(({ intentKey }) => intentKey)).size, 8);
  assert.equal(new Set(source.recipes.map(({ rendererKey }) => rendererKey)).size, 8);
  assert.equal(new Set(pdsIxRecipeCapabilities).size, 12);
  assert.ok(Object.isFrozen(pdsIxRecipeRegistry));
  assert.ok(Object.isFrozen(pdsIxRecipeIds));
  assert.ok(Object.isFrozen(pdsIxRecipeRegistry.recipes));
  assert.ok(Object.isFrozen(pdsIxRecipeRegistry.recipes[0].requiredCapabilities));
  assert.ok(Object.isFrozen(pdsIxRecipeRegistry.recipes[0].projections["native-ios"]));
  assert.throws(() => pdsIxRecipeIds.push("substituted"), TypeError);
});

test("registry schema rejects identity, order, capability, projection, and unknown-field drift", async () => {
  const source = await readJson(registryUrl);
  const schema = await readJson(registrySchemaUrl);
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const candidates = [];

  const wrongOrder = clone(source);
  [wrongOrder.recipes[0], wrongOrder.recipes[1]] = [wrongOrder.recipes[1], wrongOrder.recipes[0]];
  candidates.push(wrongOrder);

  const wrongIntent = clone(source);
  wrongIntent.recipes[0].intentKey = "pds.ix.intent.substituted@1";
  candidates.push(wrongIntent);

  const missingCapability = clone(source);
  missingCapability.recipes[0].requiredCapabilities.pop();
  candidates.push(missingCapability);

  const qualifiedNative = clone(source);
  qualifiedNative.recipes[0].projections["native-ios"].readiness = "candidate";
  candidates.push(qualifiedNative);

  const unknownField = clone(source);
  unknownField.recipes[0].productRoute = "/not-allowed";
  candidates.push(unknownField);

  for (const candidate of candidates) {
    assert.equal(validate(candidate), false, JSON.stringify(candidate));
  }
});

test("all eight registrations validate and project with exact Web/iOS/Android parity", async () => {
  const registrationSchema = await readJson(registrationSchemaUrl);
  assert.equal(
    registrationSchema.$id,
    "https://appfw.pdshealth.com/schemas/pds.ix.recipe-registration.v1.schema.json"
  );
  const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(
    registrationSchema
  );

  for (const recipeId of pdsIxRecipeIds) {
    const recipe = getPdsIxRecipe(recipeId);
    assert.ok(recipe);
    const registration = registrationFor(recipe);
    assert.equal(validateSchema(registration), true, JSON.stringify(validateSchema.errors));
    assert.deepEqual(assertPdsIxRecipeRegistration(registration), registration);

    for (const projection of ["web-dom", "native-ios", "native-android"]) {
      const result = projectPdsIxRecipeRegistration(registration, projection);
      assert.equal(result.recipe, recipe);
      assert.equal(result.registration, registration);
      assert.equal(result.projection, projection);
      assert.equal(result.applicability, "supported");
      assert.equal(
        result.readiness,
        projection === "web-dom" ? "prototype" : "not-qualified"
      );
    }
  }
});

test("registration runtime validator and JSON Schema fail closed on the same adversarial tuples", async () => {
  const registrationSchema = await readJson(registrationSchemaUrl);
  const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(
    registrationSchema
  );
  const baseline = registrationFor(getPdsIxRecipe("analyze-why"));
  const candidates = [
    { ...baseline, unknown: true },
    { ...baseline, schemaVersion: "pds.ix.recipe_registration@2" },
    { ...baseline, recipeId: "unknown-recipe" },
    { ...baseline, intentKey: "pds.ix.intent.contextual-conversation@1" },
    { ...baseline, artifactType: "contains whitespace" },
    { ...baseline, artifactType: "x".repeat(257) },
    { ...baseline, contentSchemaVersion: "product.presentation@1" },
    { ...baseline, rendererKey: "pds.ix.recipe.contextual-conversation@1" },
    { ...baseline, requiredCapabilities: baseline.requiredCapabilities.slice(0, -1) },
    { ...baseline, requiredCapabilities: [...baseline.requiredCapabilities, "unknown"] },
    {
      ...baseline,
      requiredCapabilities: [
        baseline.requiredCapabilities[1],
        baseline.requiredCapabilities[0],
        ...baseline.requiredCapabilities.slice(2)
      ]
    },
    {
      ...baseline,
      requiredCapabilities: [
        ...baseline.requiredCapabilities.slice(0, -1),
        baseline.requiredCapabilities[0]
      ]
    }
  ];

  for (const candidate of candidates) {
    assert.equal(validatePdsIxRecipeRegistration(candidate).ok, false, JSON.stringify(candidate));
    assert.equal(validateSchema(candidate), false, JSON.stringify(candidate));
    assert.throws(() => assertPdsIxRecipeRegistration(candidate), TypeError);
  }
  assert.throws(
    () => projectPdsIxRecipeRegistration(baseline, "native-windows"),
    /Unsupported PDS IX renderer projection/
  );
});

test("public recipe contract contains no renderer, provider, product, or fixture authority", async () => {
  const paths = [
    new URL("../src/recipes.js", import.meta.url),
    new URL("../src/recipes.d.ts", import.meta.url),
    registryUrl,
    registrySchemaUrl,
    registrationSchemaUrl
  ];
  for (const path of paths) {
    const source = await readFile(path, "utf8");
    assert.doesNotMatch(
      source,
      /\b(?:react|document|window|servicenow|workday|kafka|grafana|okta|nexus|recovery-readiness|prompt|credential)\b/i,
      path.pathname
    );
  }
});
