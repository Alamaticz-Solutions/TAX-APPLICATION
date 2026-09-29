import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { expectNoSeriousA11yViolations } from "../support/flows";

const decisionPathName = "Decision path from relationship context to local receipt";
const axeStates = ["ready", "partial", "stale", "loading", "empty", "offline", "unauthorized", "error"] as const;

type Rgba = {
  red: number;
  green: number;
  blue: number;
  alpha: number;
};

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
});

async function setTheme(page: Page, theme: "light" | "dark") {
  const current = await page.evaluate(() => document.documentElement.dataset.theme);
  if (current !== theme) {
    await page.getByLabel(`Switch to ${theme} mode`).click();
  }
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  await page.evaluate(async () => {
    const finiteAnimations = document.getAnimations().filter(
      (animation) => animation.effect?.getTiming().iterations !== Infinity
    );
    await Promise.all(finiteAnimations.map((animation) => animation.finished.catch(() => undefined)));
  });
}

async function expectFocusedAxePass(page: Page, label: string) {
  const analysis = await new AxeBuilder({ page }).include(".activity-experience").analyze();
  await expectNoSeriousA11yViolations(analysis.violations);
  expect(
    analysis.violations.filter((violation) => violation.id === "color-contrast"),
    `${label} must not have an Axe color-contrast violation`
  ).toEqual([]);
}

async function expectAdaptiveContrastContract(page: Page, theme: "light" | "dark") {
  const actionLocator = page.locator(
    '.activity-decision-spine__tool .pds-button[data-variant="primary"]'
  );
  await actionLocator.focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  await expect(actionLocator).toBeFocused();

  const colors = await page.evaluate((currentTheme) => {
    const resolvedColor = (property: string) => {
      const probe = document.createElement("span");
      probe.style.color = `var(${property})`;
      document.body.append(probe);
      const color = getComputedStyle(probe).color;
      probe.remove();
      return color;
    };
    const resolvedBackground = (property: string) => {
      const probe = document.createElement("span");
      probe.style.backgroundColor = `var(${property})`;
      document.body.append(probe);
      const color = getComputedStyle(probe).backgroundColor;
      probe.remove();
      return color;
    };
    const element = (selector: string) => {
      const match = document.querySelector(selector);
      if (!(match instanceof Element)) {
        throw new Error(`Missing activity contrast probe: ${selector}`);
      }
      return match;
    };

    const action = element('.activity-decision-spine__tool .pds-button[data-variant="primary"]');
    if (!(action instanceof HTMLButtonElement)) {
      throw new Error("Activity contrast action is not a button");
    }

    return {
      appliedTheme: document.documentElement.dataset.theme,
      expectedAccent: resolvedColor(
        currentTheme === "dark"
          ? "--pds-color-brand-blue-bright"
          : "--pds-color-brand-blue-deeper"
      ),
      pageSurface: resolvedBackground("--pds-color-surface-page"),
      panelSurface: resolvedBackground("--pds-color-surface-panel"),
      selectedSurface: getComputedStyle(element(".activity-queue-item.is-selected")).backgroundColor,
      eyebrow: getComputedStyle(element(".activity-experience__eyebrow")).color,
      selectedLabel: getComputedStyle(
        element(".activity-queue-item.is-selected .activity-queue-item__topline")
      ).color,
      selectedIcon: getComputedStyle(
        element(".activity-queue-item.is-selected svg")
      ).color,
      stageIcon: getComputedStyle(element(".activity-decision-spine__stage-icon")).color,
      railCopy: getComputedStyle(element(".activity-decision-spine__stage > p")).color,
      actionText: getComputedStyle(action).color,
      actionSurface: getComputedStyle(action).backgroundColor,
      focusOutline: getComputedStyle(action).outlineColor
    };
  }, theme);

  expect(colors.appliedTheme).toBe(theme);
  expect(colors.eyebrow).toBe(colors.expectedAccent);
  expect(colors.selectedLabel).toBe(colors.expectedAccent);
  expect(colors.selectedIcon).toBe(colors.expectedAccent);
  expect(colors.stageIcon).toBe(colors.expectedAccent);
  expect(colors.focusOutline).toBe(colors.expectedAccent);

  const pageSurface = parseCssColor(colors.pageSurface);
  const panelSurface = composite(parseCssColor(colors.panelSurface), pageSurface);
  const selectedSurface = composite(parseCssColor(colors.selectedSurface), panelSurface);
  const accent = parseCssColor(colors.expectedAccent);

  expect(contrastRatio(accent, pageSurface), `${theme} eyebrow contrast`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(accent, selectedSurface), `${theme} selected-row contrast`).toBeGreaterThanOrEqual(4.5);
  expect(
    contrastRatio(parseCssColor(colors.railCopy), panelSurface),
    `${theme} decision-path body contrast`
  ).toBeGreaterThanOrEqual(4.5);
  expect(
    contrastRatio(parseCssColor(colors.actionText), parseCssColor(colors.actionSurface)),
    `${theme} primary action contrast`
  ).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(accent, panelSurface), `${theme} focus indicator contrast`).toBeGreaterThanOrEqual(3);
}

test("decision path passes light and dark Axe across coherent and adverse truth states", async ({ page }) => {
  for (const theme of ["light", "dark"] as const) {
    await page.goto("/activities");
    await setTheme(page, theme);
    const stateControl = page.getByLabel("Activity experience fixture state");

    for (const state of axeStates) {
      await stateControl.selectOption(state);
      if (state === "ready" || state === "partial" || state === "stale") {
        await expect(page.getByRole("region", { name: decisionPathName })).toBeVisible();
      } else {
        await expect(page.locator(".activity-state-panel")).toBeVisible();
      }
      await expectFocusedAxePass(page, `${theme}/${state}`);
    }

    await stateControl.selectOption("ready");
    await page.getByRole("button", { name: "Inspect consequence" }).click();
    await expect(page.getByRole("button", { name: "Record local review receipt" })).toBeFocused();
    await expectFocusedAxePass(page, `${theme}/inspected`);
    await page.getByRole("button", { name: "Record local review receipt" }).click();
    await expect(page.locator(".activity-decision-spine__receipt")).toBeFocused();
    await expectFocusedAxePass(page, `${theme}/receipted`);
  }
});

test("decision path uses adaptive semantic colors with text, control, and focus contrast", async ({ page }) => {
  for (const theme of ["light", "dark"] as const) {
    await page.goto("/activities");
    await setTheme(page, theme);
    await expectAdaptiveContrastContract(page, theme);
  }
});

function parseCssColor(color: string): Rgba {
  const channels = color.match(/[\d.]+/g)?.map(Number);
  if (!channels || channels.length < 3) {
    throw new Error(`Unsupported CSS color: ${color}`);
  }
  return {
    red: channels[0],
    green: channels[1],
    blue: channels[2],
    alpha: channels[3] ?? 1
  };
}

function composite(foreground: Rgba, background: Rgba): Rgba {
  const alpha = foreground.alpha + background.alpha * (1 - foreground.alpha);
  if (alpha === 0) {
    return { red: 0, green: 0, blue: 0, alpha: 0 };
  }
  return {
    red: (foreground.red * foreground.alpha + background.red * background.alpha * (1 - foreground.alpha)) / alpha,
    green: (foreground.green * foreground.alpha + background.green * background.alpha * (1 - foreground.alpha)) / alpha,
    blue: (foreground.blue * foreground.alpha + background.blue * background.alpha * (1 - foreground.alpha)) / alpha,
    alpha
  };
}

function contrastRatio(first: Rgba, second: Rgba) {
  const firstLuminance = relativeLuminance(first);
  const secondLuminance = relativeLuminance(second);
  const lighter = Math.max(firstLuminance, secondLuminance);
  const darker = Math.min(firstLuminance, secondLuminance);
  return (lighter + 0.05) / (darker + 0.05);
}

function relativeLuminance(color: Rgba) {
  const channel = (value: number) => {
    const normalized = value / 255;
    return normalized <= 0.04045
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  return (
    0.2126 * channel(color.red)
    + 0.7152 * channel(color.green)
    + 0.0722 * channel(color.blue)
  );
}
