import { fireEvent, render } from "@testing-library/react-native";
import React from "react";

import {
  getPdsIxRecipe,
  pdsIxRecipeIds,
  pdsIxRecipeRegistrationSchema,
  type PdsIxRecipeDescriptor
} from "@appfw/pds-ix-presentation-contract/recipes";
import fixture from "../../ix-presentation-contract/fixtures/working-brief.presentation.json";
import {
  PdsIxRecipePresentation,
  resolvePdsIxNativeRecipe,
  type PdsIxRecipePresentationProps
} from "../src/ix-recipes";

function registrationFor(recipe: PdsIxRecipeDescriptor) {
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

function recipe(recipeId: Parameters<typeof getPdsIxRecipe>[0]) {
  const value = getPdsIxRecipe(recipeId);
  if (!value) throw new Error(`missing test recipe ${String(recipeId)}`);
  return value;
}

test("public native resolver supports all eight recipes on both native projections", () => {
  expect(pdsIxRecipeIds).toHaveLength(8);
  for (const recipeId of pdsIxRecipeIds) {
    const descriptor = recipe(recipeId);
    const registration = registrationFor(descriptor);
    for (const projection of ["native-ios", "native-android"] as const) {
      const result = resolvePdsIxNativeRecipe(registration, projection);
      expect(result.recipe).toBe(descriptor);
      expect(result.registration).toBe(registration);
      expect(result.projection).toBe(projection);
      expect(result.applicability).toBe("supported");
      expect(result.readiness).toBe("not-qualified");
    }
  }
  expect(() => resolvePdsIxNativeRecipe(
    registrationFor(recipe("analyze-why")),
    "web-dom" as never
  )).toThrow("Unsupported native PDS IX renderer projection");
});

test("native recipe composition routes the exact registration and original editable region", () => {
  const registration = registrationFor(recipe("working-goal-plan"));
  const onRegionAction = jest.fn();
  const onRegionEdit = jest.fn();
  const props: PdsIxRecipePresentationProps = {
    onRegionAction,
    onRegionEdit,
    presentation: fixture,
    projection: "native-ios",
    registration
  };
  const screen = render(
    <PdsIxRecipePresentation {...props} />
  );
  const edit = screen.getByRole("button", { name: "Edit this section" });
  fireEvent.press(edit);
  expect(onRegionAction).not.toHaveBeenCalled();
  expect(onRegionEdit).toHaveBeenCalledWith(fixture.response.regions[0], registration);
});

test("invalid recipe registration and presentation use the non-actionable native fallback", () => {
  const valid = registrationFor(recipe("analyze-why"));
  const callbacks = {
    onContextAction: jest.fn(),
    onStatusAction: jest.fn(),
    onRegionAction: jest.fn(),
    onRegionEdit: jest.fn(),
    onSourcePress: jest.fn()
  };
  const candidates = [
    [{ ...valid, intentKey: "pds.ix.intent.contextual-conversation@1" }, fixture],
    [valid, { unexpected: true }]
  ] as const;
  for (const [registration, presentation] of candidates) {
    const screen = render(
      <PdsIxRecipePresentation
        {...callbacks}
        presentation={presentation}
        projection="native-android"
        registration={registration}
      />
    );
    expect(screen.getByTestId("pds-presentation-invalid")).toBeTruthy();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "This intelligence presentation is unavailable."
    );
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    screen.unmount();
  }
  for (const callback of Object.values(callbacks)) {
    expect(callback).not.toHaveBeenCalled();
  }
});
