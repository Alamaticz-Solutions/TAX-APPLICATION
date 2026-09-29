import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installCrmApiMock } from "../support/crmMock";
import { expectNoSeriousA11yViolations, signInForFrontendTests } from "../support/flows";

test.beforeEach(async ({ page }) => {
  // Keep Axe on the settled UI instead of sampling a view-reveal opacity frame.
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
});

async function waitForVisualState(page: Page) {
  await page.evaluate(async () => {
    const finiteAnimations = document.getAnimations().filter(
      (animation) => animation.effect?.getTiming().iterations !== Infinity
    );
    await Promise.all(
      finiteAnimations.map((animation) => animation.finished.catch(() => undefined))
    );
  });
}

async function expectReferenceContrastTokens(page: Page, theme: "light" | "dark") {
  const colors = await page.evaluate((currentTheme) => {
    const resolvedColor = (property: string) => {
      const probe = document.createElement("span");
      probe.style.color = `var(${property})`;
      document.body.append(probe);
      const color = getComputedStyle(probe).color;
      probe.remove();
      return color;
    };
    const elementColor = (selector: string, pseudo?: string) => {
      const element = document.querySelector(selector);
      if (!(element instanceof HTMLElement)) throw new Error(`Missing contrast probe: ${selector}`);
      return getComputedStyle(element, pseudo).color;
    };
    const input = document.querySelector('input[aria-label="Search records"]');
    if (!(input instanceof HTMLInputElement)) throw new Error("Missing contrast probe: search input");

    return {
      theme: document.documentElement.dataset.theme,
      expectedBrand: resolvedColor(
        currentTheme === "dark" ? "--pds-color-brand-blue-bright" : "--pds-color-brand-blue-deeper"
      ),
      expectedSublabel: resolvedColor(
        currentTheme === "dark" ? "--pds-color-text-default" : "--pds-color-text-muted"
      ),
      expectedPlaceholder: resolvedColor("--pds-color-text-muted"),
      kicker: elementColor(".crm-kicker"),
      activeLabel: elementColor(".crm-nav-item.is-active .crm-nav-label"),
      activeIcon: elementColor(".crm-nav-item.is-active .crm-nav-icon"),
      activeSublabel: elementColor(".crm-nav-item.is-active .crm-nav-sublabel"),
      placeholder: getComputedStyle(input, "::placeholder").color,
      placeholderOpacity: getComputedStyle(input, "::placeholder").opacity
    };
  }, theme);

  expect(colors.theme).toBe(theme);
  expect(colors.kicker).toBe(colors.expectedBrand);
  expect(colors.activeLabel).toBe(colors.expectedBrand);
  expect(colors.activeIcon).toBe(colors.expectedBrand);
  expect(colors.activeSublabel).toBe(colors.expectedSublabel);
  expect(colors.placeholder).toBe(colors.expectedPlaceholder);
  expect(colors.placeholderOpacity).toBe("1");
}

test("dashboard, grid, and account form have no serious axe violations", async ({ page }) => {
  await installCrmApiMock(page);

  await page.goto("/");
  await signInForFrontendTests(page);
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).analyze()).violations);

  await page.goto("/data/accounts");
  await expect(page.locator(".crm-kicker")).toBeVisible();
  await expectReferenceContrastTokens(page, "light");
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).analyze()).violations);

  await page.goto("/data/accounts/rl_account_001");
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).analyze()).violations);
});

test("dialogs, query builder, lookup selector, and theme mode remain accessible", async ({ page }) => {
  await installCrmApiMock(page);

  await page.goto("/data/activities");
  await signInForFrontendTests(page);

  await page.getByRole("button", { name: /Query builder/ }).click();
  const queryBuilder = page.getByRole("dialog", { name: "Query builder" });
  await queryBuilder.getByRole("button", { name: "Add filter" }).click();
  const rule = queryBuilder.locator(".pds-data-grid-filter-rule").first();
  await rule.getByLabel("Field").selectOption("type_id");
  await rule.getByLabel("Operator").selectOption("_eq");
  await rule.getByLabel("Value").selectOption("atype-call");
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).include(".crm-query-builder-body").analyze()).violations);
  await queryBuilder.getByRole("button", { name: "Close" }).click();

  await page.getByRole("button", { name: "About PDS Health CRM" }).click();
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).include(".crm-about-dialog").analyze()).violations);
  await page.getByRole("button", { name: "Close about dialog" }).click();

  await page.getByRole("button", { name: "Switch to dark mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await waitForVisualState(page);
  await expectReferenceContrastTokens(page, "dark");
  await expectNoSeriousA11yViolations((await new AxeBuilder({ page }).analyze()).violations);
});
