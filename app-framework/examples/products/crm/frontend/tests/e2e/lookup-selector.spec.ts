import { expect, test } from "@playwright/test";
import { installCrmApiMock } from "../support/crmMock";
import { signInForFrontendTests } from "../support/flows";

test("query builder uses selector controls for lookup filter values", async ({ page }) => {
  const mock = await installCrmApiMock(page);

  await page.goto("/data/activities");
  await signInForFrontendTests(page);
  await expect(page.getByText("Discovery Call - Acme Analytics").first()).toBeVisible();

  await page.getByRole("button", { name: /Query builder/ }).click();
  const queryBuilder = page.getByRole("dialog", { name: "Query builder" });
  await queryBuilder.getByRole("button", { name: "Add filter" }).click();
  const rule = queryBuilder.locator(".pds-data-grid-filter-rule").first();
  await rule.getByLabel("Field").selectOption("type_id");
  await rule.getByLabel("Operator").selectOption("_eq");

  const value = rule.getByLabel("Value");
  await expect(value).toHaveJSProperty("tagName", "SELECT");
  await value.selectOption("atype-call");
  await queryBuilder.getByRole("button", { name: "Apply query" }).click();

  await expect(page.getByText("Discovery Call - Acme Analytics").first()).toBeVisible();
  expect(mock.latestFor("queryActivities")?.variables.filter).toEqual({ type_id: { _eq: "atype-call" } });
});
