import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  getPdsIxRecipe,
  pdsIxRecipeIds,
  pdsIxRecipeRegistrationSchema
} from "@appfw/pds-ix-presentation-contract";
import {
  PdsIxRecipePresentation,
  resolvePdsIxWebRecipe
} from "../dist/ix-recipes.js";
import { pdsComponentLifecycle } from "../dist/catalog.js";

const fixture = JSON.parse(await readFile(
  new URL("../../ix-presentation-contract/fixtures/working-brief.presentation.json", import.meta.url),
  "utf8"
));
const catalog = JSON.parse(await readFile(
  new URL("../../reference/catalog.json", import.meta.url),
  "utf8"
));

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

test("public Web resolver supports all eight canonical recipes", () => {
  assert.equal(pdsIxRecipeIds.length, 8);
  for (const recipeId of pdsIxRecipeIds) {
    const recipe = getPdsIxRecipe(recipeId);
    const projection = resolvePdsIxWebRecipe(registrationFor(recipe));
    assert.equal(projection.recipe.id, recipeId);
    assert.equal(projection.projection, "web-dom");
    assert.equal(projection.readiness, "prototype");
    assert.equal(projection.registration.rendererKey, recipe.rendererKey);
  }
});

test("generic Web recipe composition renders caller presentation without recipe copy", () => {
  const recipe = getPdsIxRecipe("adaptive-composition");
  const html = renderToStaticMarkup(React.createElement(PdsIxRecipePresentation, {
    registration: registrationFor(recipe),
    presentation: fixture
  }));
  assert.match(html, /aria-label="Adaptive Composition intelligence presentation"/);
  assert.match(html, /data-pds-ix-recipe="adaptive-composition"/);
  assert.match(html, /data-pds-ix-projection="web-dom"/);
  assert.match(html, /data-pds-ix-readiness="prototype"/);
  assert.match(html, /Working brief/);
  assert.match(html, /Approval owner is not yet resolved/);
  assert.equal(html.match(/role="status"/g)?.length, 1);
  assert.equal(html.match(/Working brief revision 1 is ready\./g)?.length, 1);
  assert.doesNotMatch(html, /recovery readiness/i);
});

test("invalid registration and presentation produce the same non-actionable alert", () => {
  const recipe = getPdsIxRecipe("analyze-why");
  const valid = registrationFor(recipe);
  let callbackCalls = 0;
  const callback = () => {
    callbackCalls += 1;
  };
  const candidates = [
    [{ ...valid, rendererKey: "pds.ix.recipe.contextual-conversation@1" }, fixture],
    [valid, { unknown: true }]
  ];
  for (const [registration, presentation] of candidates) {
    const html = renderToStaticMarkup(React.createElement(PdsIxRecipePresentation, {
      registration,
      presentation,
      onContextAction: callback,
      onStatusAction: callback,
      onRegionAction: callback,
      onRegionEdit: callback,
      renderRegionBody: callback
    }));
    assert.match(html, /role="alert"/);
    assert.match(html, /data-pds-ix-recipe-invalid="true"/);
    assert.match(html, /This intelligence presentation is unavailable\./);
    assert.doesNotMatch(html, /<button/);
    assert.doesNotMatch(html, /data-pds-ix-renderer/);
  }
  assert.equal(callbackCalls, 0);
});

test("recipe package release preserves historical lifecycle metadata", () => {
  const explicitLifecycleEntries = [
    "PdsHealthLogo",
    "RelationshipAtlas",
    "RelationshipAtlasTable",
    "RelationshipExplorer",
    "ExplorationWorkspace",
    "NarrativeWorkspace",
    "EvidenceDisclosure",
    "ResolvedContextDisclosure",
    "WorkStatus",
    "ProgressiveResponse",
    "PdsIxRecipePresentation"
  ];
  for (const component of explicitLifecycleEntries) {
    assert.deepEqual(
      pdsComponentLifecycle[component],
      catalog.componentLifecycle[component],
      `${component} dynamic lifecycle must match the static catalog`
    );
  }
  assert.deepEqual(pdsComponentLifecycle.PdsIxRecipePresentation, {
    status: "experimental",
    since: "0.12.0"
  });
});

test("editable regions retain an Edit control and route the exact registration without another action label", () => {
  const recipe = getPdsIxRecipe("working-goal-plan");
  const registration = registrationFor(recipe);
  const editablePresentation = {
    ...fixture,
    response: {
      ...fixture.response,
      regions: [{
        ...fixture.response.regions[0],
        editable: true,
        actionLabel: undefined
      }]
    }
  };
  let actionCalls = 0;
  let editCallback = null;
  const composition = PdsIxRecipePresentation({
    registration,
    presentation: editablePresentation,
    onRegionAction() {
      actionCalls += 1;
    },
    onRegionEdit(region, receivedRegistration) {
      editCallback = { region, registration: receivedRegistration };
    }
  });
  const responseElement = React.Children.toArray(composition.props.children)
    .find((child) => child?.props?.model?.eyebrow === fixture.response.eyebrow);
  assert.ok(responseElement);
  const projectedRegion = responseElement.props.model.regions[0];
  assert.equal(responseElement.props.model, editablePresentation.response);
  assert.equal(projectedRegion, editablePresentation.response.regions[0]);
  assert.equal(projectedRegion.actionLabel, undefined);
  const editButton = responseElement.props.renderEditAction(projectedRegion);
  assert.ok(editButton);
  editButton.props.onClick();
  assert.equal(actionCalls, 0);
  assert.equal(editCallback.region, projectedRegion);
  assert.equal(editCallback.registration, registration);

  const html = renderToStaticMarkup(composition);
  assert.match(html, />Edit this section<\/button>/);
  assert.equal(html.match(/<button/g)?.length, 1);
});

test("compiled Web entry consumes only declared public package contracts", async () => {
  const source = await readFile(new URL("../dist/ix-recipes.js", import.meta.url), "utf8");
  const declarations = await readFile(new URL("../dist/ix-recipes.d.ts", import.meta.url), "utf8");
  assert.match(source, /from "@appfw\/pds-ix-presentation-contract"/);
  assert.match(declarations, /from "@appfw\/pds-ix-presentation-contract"/);
  for (const content of [source, declarations]) {
    assert.doesNotMatch(content, /ix-reference|reference_playback|\/src\/|\.\.\/\.\.\/ix-presentation-contract/);
    assert.doesNotMatch(content, /\b(?:servicenow|workday|kafka|grafana|okta|nexus)\b/i);
  }
});
