import { expect, test } from "@playwright/test";
import { installCrmApiMock } from "../support/crmMock";
import { signInForFrontendTests } from "../support/flows";

test("CRM dashboard loads analytics, charts, and request telemetry", async ({ page }) => {
  const mock = await installCrmApiMock(page);

  await page.goto("/");
  await signInForFrontendTests(page);

  await expect(page.getByRole("heading", { name: "CRM Command Center" })).toBeVisible();
  await expect(page.getByLabel("CRM key performance indicators").getByText("Open pipeline")).toBeVisible();
  await expect(page.getByLabel("CRM key performance indicators").getByText("Weighted pipeline")).toBeVisible();
  await expect(page.getByLabel("CRM key performance indicators").getByText("Lead conversion")).toBeVisible();
  const revenuePanel = page.locator(".pds-chart-shell", {
    has: page.locator(".pds-chart-shell__copy strong", { hasText: "Revenue by state" })
  });
  await expect(revenuePanel).toHaveCount(1);
  await expect(revenuePanel).toBeVisible();
  const revenueCaptionLink = revenuePanel.getByRole("link", { exact: true, name: "Accounts" });
  await expect(revenueCaptionLink).toBeVisible();
  await expect(revenueCaptionLink).toHaveAttribute("href", "/data/accounts");
  await expect(revenuePanel.locator("canvas")).toBeVisible();
  const revenueChartBounds = await revenuePanel.evaluate((panel: HTMLElement) => {
    const canvas = panel.querySelector("canvas");
    const chartWrap = panel.querySelector(".crm-chart-wrap");
    if (!canvas || !chartWrap) return { contained: false };
    const panelBox = panel.getBoundingClientRect();
    const canvasBox = canvas.getBoundingClientRect();
    const chartBox = chartWrap.getBoundingClientRect();
    const tolerance = 1;
    return {
      contained:
        canvasBox.left >= panelBox.left - tolerance &&
        canvasBox.right <= panelBox.right + tolerance &&
        chartBox.left >= panelBox.left - tolerance &&
        chartBox.right <= panelBox.right + tolerance
    };
  });
  expect(revenueChartBounds.contained).toBe(true);
  await expect(page.getByText(/requests$/)).toBeVisible();

  expect(mock.requests.some((request) => request.operation === "DashboardAggregateAccounts")).toBe(true);
  expect(mock.requests.some((request) => request.operation === "DashboardTopOpportunities")).toBe(true);
});
