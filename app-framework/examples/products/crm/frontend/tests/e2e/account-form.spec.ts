import { expect, test } from "@playwright/test";
import { installCrmApiMock } from "../support/crmMock";
import { signInForFrontendTests } from "../support/flows";

test("account form loads by locator, guards dirty state, shows account health, and confirms delete", async ({ page }) => {
  const mock = await installCrmApiMock(page);

  await page.goto("/data/accounts/rl_account_001");
  await signInForFrontendTests(page);

  await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
  await expect(page.getByLabel("Name")).toHaveValue("Acme Analytics");
  await expect(page.getByLabel("Annual Revenue")).toHaveValue("$12,500,000.00");
  await expect(page.getByRole("button", { name: "Save changes" })).toBeDisabled();

  await page.getByLabel("Name").fill("Acme Analytics Enterprise");
  await expect(page.getByText("Unsaved changes")).toBeVisible();
  await expect(page.getByRole("button", { name: "Save changes" })).toBeEnabled();

  page.once("dialog", async (dialog) => {
    expect(dialog.message()).toContain("Save or cancel your changes");
    await dialog.dismiss();
  });
  await page.getByRole("row", { name: /Discovery Call - Acme Analytics/ }).click();

  await page.reload();
  await signInForFrontendTests(page);
  await expect(page.getByLabel("Name")).toHaveValue("Acme Analytics");

  await page.getByRole("button", { name: "Account dashboard" }).click();
  const dashboard = page.getByRole("region", { name: "Account health dashboard" });
  await expect(dashboard).toBeVisible();
  await expect(dashboard.locator(".pds-surface.crm-account-health-hero")).toHaveCount(1);
  await expect(dashboard.getByText("Health score", { exact: true })).toBeVisible();
  await expect(dashboard.locator(".pds-kpi-tile")).toHaveCount(4);
  await expect(dashboard.locator(".pds-chart-shell")).toHaveCount(6);
  await expect(page.getByText("Schedule proposal review")).toBeVisible();
  await page.getByRole("button", { name: "Refresh stored snapshot" }).click();
  await expect(dashboard.getByText("Stored procedure", { exact: true })).toBeVisible();
  await expect.poll(() => mock.requestsFor("RefreshAccountHealthStoredProcedure").length).toBe(1);
  await page.getByRole("button", { name: "Account form" }).click();
  await expect(page.getByLabel("Name")).toHaveValue("Acme Analytics");

  await page.getByRole("button", { name: "Delete" }).click();
  const confirmation = page.getByRole("alertdialog", { name: "Delete Account" });
  await expect(confirmation).toBeVisible();
  await expect(confirmation.getByText("This action cannot be undone.")).toBeVisible();
  await confirmation.getByRole("button", { name: "Cancel" }).click();
  await expect(confirmation).toBeHidden();
});
