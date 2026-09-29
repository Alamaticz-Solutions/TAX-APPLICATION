import { expect, test } from "@playwright/test";
import { installCrmApiMock } from "../support/crmMock";
import { signInForFrontendTests } from "../support/flows";

async function observeLoadingPreviewMounts(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const previewWindow = window as Window & { __pdsLoadingPreviewMounts?: string[] };
    previewWindow.__pdsLoadingPreviewMounts = [];

    const recordPreview = (node: Node) => {
      if (!(node instanceof Element)) return;
      for (const selector of [".pds-data-grid-loading-preview", ".pds-form-loading-preview"]) {
        if (node.matches(selector)) previewWindow.__pdsLoadingPreviewMounts?.push(selector);
        node.querySelectorAll(selector).forEach(() => previewWindow.__pdsLoadingPreviewMounts?.push(selector));
      }
    };

    new MutationObserver((mutations) => {
      mutations.forEach((mutation) => mutation.addedNodes.forEach(recordPreview));
    }).observe(document.documentElement, { childList: true, subtree: true });
  });
}

async function loadingPreviewMounts(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    return ((window as Window & { __pdsLoadingPreviewMounts?: string[] }).__pdsLoadingPreviewMounts ?? []).slice();
  });
}

test("accounts grid supports server search, query builder, columns, pagination, and theme switching", async ({ page }) => {
  const mock = await installCrmApiMock(page);

  await page.goto("/data/accounts");
  await signInForFrontendTests(page);

  await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
  await expect(page.getByText("Acme Analytics").first()).toBeVisible();
  await expect(page.getByText("1-2 of 2")).toBeVisible();
  await expect(page.locator(".pds-data-grid[data-density]")).toHaveAttribute("data-density", "compact");

  await page.getByRole("searchbox", { name: "Search records" }).fill("North");
  await expect(page.getByText("Northstar Manufacturing").first()).toBeVisible();
  expect(mock.latestFor("queryAccounts")?.variables.filter).toBeTruthy();
  await page.getByRole("searchbox", { name: "Search records" }).fill("");
  await expect(page.getByText("Acme Analytics").first()).toBeVisible();

  await page.getByRole("button", { name: /Columns/ }).click();
  const columns = page.getByRole("dialog", { name: "Column chooser" });
  await expect(columns).toBeVisible();
  await expect(columns.getByText("Annual Revenue")).toBeVisible();
  await expect(columns.getByText("Currency")).toBeVisible();
  await columns.getByRole("checkbox", { name: /Annual Revenue/ }).check();

  await page.getByRole("button", { name: /Query builder/ }).click();
  const queryBuilder = page.getByRole("dialog", { name: "Query builder" });
  await expect(queryBuilder).toBeVisible();
  await expect(queryBuilder.getByRole("button", { name: "Apply query" })).toBeDisabled();
  await queryBuilder.getByRole("button", { name: "Add filter" }).click();
  const rule = queryBuilder.locator(".pds-data-grid-filter-rule").first();
  await rule.getByLabel("Field").selectOption("billing_state");
  await rule.getByLabel("Value").fill("CA");
  await expect(queryBuilder.getByText("Modified")).toBeVisible();
  await expect(queryBuilder.getByRole("button", { name: "Apply query" })).toBeEnabled();
  await queryBuilder.getByRole("button", { name: "Apply query" }).click();
  await expect(page.getByText("Acme Analytics").first()).toBeVisible();
  expect(mock.latestFor("queryAccounts")?.variables.filter).toEqual({ billing_state: { _contains: "CA" } });

  const beforeTheme = await page.evaluate(() => document.documentElement.dataset.theme);
  await page.getByRole("button", { name: /Switch to (dark|light) mode/ }).click();
  await expect
    .poll(() => page.evaluate(() => document.documentElement.dataset.theme))
    .not.toBe(beforeTheme);
  await expect(page.getByText("Display preference saved")).toBeVisible();
});

test("fast list and record loads avoid heavy loading preview flashes", async ({ page }) => {
  await observeLoadingPreviewMounts(page);
  await installCrmApiMock(page);

  await page.goto("/data/accounts");
  await signInForFrontendTests(page);

  await expect(page.getByText("Acme Analytics").first()).toBeVisible();
  expect(await loadingPreviewMounts(page)).toEqual([]);

  await page.getByText("Acme Analytics").first().click();
  await expect(page.getByRole("button", { name: "Save changes" })).toBeVisible();
  expect(await loadingPreviewMounts(page)).toEqual([]);
});

test("grid and record form loading previews preserve enterprise layout rhythm", async ({ page }) => {
  await installCrmApiMock(page, { delayMs: 2000 });

  await page.goto("/data/accounts");
  await signInForFrontendTests(page);

  const gridSkeleton = page.locator(".pds-data-grid-loading-preview");
  await expect(gridSkeleton).toBeVisible();
  const gridMetrics = await gridSkeleton.evaluate((node: HTMLElement) => {
    const rows = Array.from(node.querySelectorAll<HTMLElement>(".pds-data-grid-loading-preview__row"));
    const cells = Array.from(node.querySelectorAll<HTMLElement>(".pds-data-grid-loading-preview__cell"));
    return {
      height: node.getBoundingClientRect().height,
      rowCount: rows.length,
      minRowHeight: Math.min(...rows.map((row) => row.getBoundingClientRect().height)),
      minCellHeight: Math.min(...cells.map((cell) => cell.getBoundingClientRect().height))
    };
  });
  expect(gridMetrics.height).toBeGreaterThanOrEqual(430);
  expect(gridMetrics.rowCount).toBe(7);
  expect(gridMetrics.minRowHeight).toBeGreaterThanOrEqual(48);
  expect(gridMetrics.minCellHeight).toBeGreaterThanOrEqual(14);

  await expect(page.getByText("Acme Analytics").first()).toBeVisible();
  await page.getByText("Acme Analytics").first().click();

  const formSkeleton = page.locator(".crm-form-card--skeleton");
  await expect(formSkeleton).toBeVisible();
  const formMetrics = await formSkeleton.evaluate((node: HTMLElement) => {
    const fields = Array.from(node.querySelectorAll<HTMLElement>(".pds-form-loading-preview__field"));
    const inputs = Array.from(node.querySelectorAll<HTMLElement>(".pds-form-loading-preview__control"));
    const body = node.querySelector<HTMLElement>(".pds-surface__body");
    return {
      height: node.getBoundingClientRect().height,
      bodyHeight: body?.getBoundingClientRect().height ?? 0,
      fieldCount: fields.length,
      minFieldHeight: Math.min(...fields.map((field) => field.getBoundingClientRect().height)),
      minInputHeight: Math.min(...inputs.map((input) => input.getBoundingClientRect().height))
    };
  });
  expect(formMetrics.height).toBeGreaterThanOrEqual(520);
  expect(formMetrics.bodyHeight).toBeGreaterThanOrEqual(520);
  expect(formMetrics.fieldCount).toBeGreaterThanOrEqual(4);
  expect(formMetrics.minFieldHeight).toBeGreaterThanOrEqual(108);
  expect(formMetrics.minInputHeight).toBeGreaterThanOrEqual(40);
  await expect(page.getByRole("button", { name: "Save changes" })).toBeVisible();
});
