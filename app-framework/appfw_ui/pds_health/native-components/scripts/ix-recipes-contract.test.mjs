import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  getPdsIxRecipe,
  pdsIxRecipeIds,
  pdsIxRecipeRegistrationSchema,
  projectPdsIxRecipeRegistration
} from "@appfw/pds-ix-presentation-contract/recipes";

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

test("canonical contract supports all eight recipes on both native projections", () => {
  assert.equal(pdsIxRecipeIds.length, 8);
  for (const recipeId of pdsIxRecipeIds) {
    const recipe = getPdsIxRecipe(recipeId);
    for (const projection of ["native-ios", "native-android"]) {
      const result = projectPdsIxRecipeRegistration(registrationFor(recipe), projection);
      assert.equal(result.recipe, recipe);
      assert.equal(result.registration.recipeId, recipeId);
      assert.equal(result.projection, projection);
      assert.equal(result.applicability, "supported");
      assert.equal(result.readiness, "not-qualified");
    }
  }
});

test("canonical contract rejects malformed recipe registrations", () => {
  const recipe = getPdsIxRecipe("analyze-why");
  const registration = registrationFor(recipe);
  assert.throws(
    () => projectPdsIxRecipeRegistration({ ...registration, rendererKey: "substituted" }, "native-ios"),
    /Invalid pds\.ix\.recipe_registration@1/
  );
});

test("compiled native entry consumes only declared public package contracts", async () => {
  const source = await readFile(new URL("../dist/index.js", import.meta.url), "utf8");
  const subpath = await readFile(new URL("../dist/ix-recipes.js", import.meta.url), "utf8");
  const projection = await readFile(new URL("../dist/ix-recipe-projection.js", import.meta.url), "utf8");
  const declarations = await readFile(new URL("../dist/index.d.ts", import.meta.url), "utf8");
  const subpathDeclarations = await readFile(new URL("../dist/ix-recipes.d.ts", import.meta.url), "utf8");
  assert.match(source, /from "@appfw\/pds-ix-presentation-contract"/);
  assert.match(projection, /from "@appfw\/pds-ix-presentation-contract"/);
  assert.match(declarations, /from "@appfw\/pds-ix-presentation-contract"/);
  assert.match(source, /from "\.\/ix-recipe-projection\.js"/);
  assert.match(subpath, /PdsIxRecipePresentation/);
  assert.match(subpath, /resolvePdsIxNativeRecipe/);
  assert.match(subpathDeclarations, /PdsIxRecipePresentationProps/);
  assert.match(projection, /function resolvePdsIxNativeRecipe/);
  assert.match(projection, /projection !== "native-ios" && projection !== "native-android"/);
  assert.doesNotMatch(projection, /from "\.\/index\.js"|react(?:-native|\/jsx-runtime)?/);
  for (const content of [source, subpath, projection, declarations, subpathDeclarations]) {
    assert.doesNotMatch(content, /ix-reference|reference_playback|\/src\/|\.\.\/\.\.\/ix-presentation-contract/);
    assert.doesNotMatch(content, /\b(?:expo|servicenow|workday|kafka|grafana|okta|nexus)\b/i);
  }
});
