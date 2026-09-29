#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:net";
import { createRequire } from "node:module";
import { platform, release } from "node:os";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const catalogRoot = join(repoRoot, "appfw_ui/pds_health/catalog-app");
const targetRoot = join(repoRoot, "target/appfw/pds-f1-representative-interactions");
const screenshotRoot = join(targetRoot, "screenshots");
const historyRoot = join(targetRoot, "history");
const outputPath = join(targetRoot, "browser-evidence.json");
const require = createRequire(join(catalogRoot, "package.json"));
const { chromium } = require("@playwright/test");
const AxeBuilder = require("@axe-core/playwright").default;
const playwrightPackage = require("@playwright/test/package.json");

mkdirSync(screenshotRoot, { recursive: true });

const sha256 = (value) => createHash("sha256").update(value).digest("hex");
const sourceSha = (await command("git", ["rev-parse", "HEAD"], repoRoot)).stdout.trim();
const fixturePath = join(catalogRoot, "src/f1/RepresentativeInteractionsFixture.tsx");
const port = await availablePort();
const url = `http://127.0.0.1:${port}/f1-representative-interactions.html`;
const scenarios = [
  { id: "desktop-light", viewport: { width: 1280, height: 900 }, colorScheme: "light", interactions: true },
  { id: "desktop-dark", viewport: { width: 1280, height: 900 }, colorScheme: "dark" },
  { id: "reduced-motion", viewport: { width: 1280, height: 900 }, colorScheme: "light", reducedMotion: "reduce" },
  { id: "forced-colors", viewport: { width: 1280, height: 900 }, colorScheme: "light", forcedColors: "active" },
  { id: "reflow-200-percent", viewport: { width: 640, height: 900 }, colorScheme: "light", textScale: 2 },
  { id: "touch", viewport: { width: 390, height: 844 }, colorScheme: "light", hasTouch: true, touch: true }
];

const evidence = {
  schema: "pds-f1-representative-interactions/1.0",
  ok: false,
  claim_boundary: "local_snapshot_non_candidate",
  generated_at: new Date().toISOString(),
  source_sha: sourceSha,
  base_sha: "86a7e7bcde933e60071600b7798d578c4c65b1f3",
  fixture: {
    path: relative(repoRoot, fixturePath),
    sha256: sha256(readFileSync(fixturePath))
  },
  environment: {
    os: `${platform()} ${release()}`,
    node: process.version,
    playwright: playwrightPackage.version,
    chromium: null
  },
  failures: [],
  retries: existsSync(historyRoot)
    ? readdirSync(historyRoot).filter((name) => /^attempt-\d+$/.test(name)).length
    : 0,
  scenarios: []
};

let child;
let browser;
try {
  child = spawn(
    process.execPath,
    [join(catalogRoot, "node_modules/vite/bin/vite.js"), "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    { cwd: catalogRoot, stdio: ["ignore", "pipe", "pipe"] }
  );
  await waitForServer(url, child);
  browser = await chromium.launch();
  evidence.environment.chromium = browser.version();

  for (const scenario of scenarios) {
    evidence.scenarios.push(await runScenario(browser, scenario));
  }

  evidence.ok = evidence.scenarios.every((scenario) => scenario.ok);
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.stack ?? error.message : String(error));
} finally {
  await browser?.close();
  child?.kill("SIGTERM");
  writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`);
}

console.log(JSON.stringify({ ok: evidence.ok, output: outputPath, scenarios: evidence.scenarios.map(({ id, ok }) => ({ id, ok })) }));
if (!evidence.ok) process.exitCode = 1;

async function runScenario(activeBrowser, scenario) {
  const context = await activeBrowser.newContext({
    viewport: scenario.viewport,
    colorScheme: scenario.colorScheme,
    reducedMotion: scenario.reducedMotion,
    forcedColors: scenario.forcedColors,
    hasTouch: scenario.hasTouch
  });
  const page = await context.newPage();
  page.setDefaultTimeout(8000);
  const consoleErrors = [];
  const pageErrors = [];
  const externalRequests = [];
  const fontRequests = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("request", (request) => {
    const requestUrl = new URL(request.url());
    if (requestUrl.hostname !== "127.0.0.1") externalRequests.push(request.url());
    if (request.resourceType() === "font") fontRequests.push(request.url());
  });

  await page.goto(url, { waitUntil: "networkidle" });
  await page.evaluate((colorScheme) => {
    document.documentElement.dataset.theme = colorScheme;
    document.documentElement.dataset.visualTheme = "apple-like";
  }, scenario.colorScheme);
  if (scenario.textScale) {
    await page.evaluate((scale) => {
      document.documentElement.style.fontSize = `${scale * 100}%`;
    }, scenario.textScale);
  }

  const checks = [];
  if (scenario.interactions) checks.push(...await exerciseKeyboardInteractions(page));
  if (scenario.touch) checks.push(...await exerciseTouchInteractions(page));
  if (scenario.reducedMotion === "reduce") checks.push(...await exerciseReducedMotionLabel(page));
  if (scenario.forcedColors === "active") checks.push(...await exerciseForcedColorFocusAndInvalid(page));

  const axe = await new AxeBuilder({ page }).analyze();
  const severe = axe.violations.filter(({ impact }) => impact === "serious" || impact === "critical");
  const unexpectedFontRequests = fontRequests.filter((requestUrl) => {
    const parsed = new URL(requestUrl);
    return parsed.hostname !== "127.0.0.1"
      || !/(?:InterVariable|GeistMonoVariable)\.woff2$/.test(parsed.pathname);
  });
  checks.push(
    check("serious-critical-axe", severe.length === 0, severe.map(({ id, impact, nodes }) => ({
      id,
      impact,
      targets: nodes.map((node) => node.target)
    }))),
    check("console-errors", consoleErrors.length === 0, consoleErrors),
    check("page-errors", pageErrors.length === 0, pageErrors),
    check("external-requests", externalRequests.length === 0, externalRequests),
    check("approved-self-hosted-font-requests", unexpectedFontRequests.length === 0, {
      observed: fontRequests,
      unexpected: unexpectedFontRequests
    })
  );

  const dom = await page.evaluate(() => ({
    duplicateIds: [...document.querySelectorAll("[id]")]
      .map((element) => element.id)
      .filter((id, index, values) => values.indexOf(id) !== index),
    horizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
    controlHeights: [...document.querySelectorAll("input, button")]
      .map((element) => Math.round(element.getBoundingClientRect().height))
      .filter((height) => height > 0)
  }));
  checks.push(
    check("unique-ids", dom.duplicateIds.length === 0, dom.duplicateIds),
    check("reflow-without-horizontal-overflow", !dom.horizontalOverflow, dom),
    check("stable-positive-control-geometry", dom.controlHeights.every((height) => height >= 30), dom.controlHeights)
  );

  const screenshotPath = join(screenshotRoot, `${scenario.id}.png`);
  const screenshot = await page.screenshot({ path: screenshotPath, fullPage: true });
  await context.close();
  return {
    id: scenario.id,
    ok: checks.every(({ ok }) => ok),
    viewport: scenario.viewport,
    preferences: {
      color_scheme: scenario.colorScheme,
      reduced_motion: scenario.reducedMotion ?? "no-preference",
      forced_colors: scenario.forcedColors ?? "none",
      text_scale: scenario.textScale ?? 1,
      touch: Boolean(scenario.touch)
    },
    axe: { violations: axe.violations.length, serious_critical: severe.length },
    checks,
    screenshot: relative(repoRoot, screenshotPath),
    screenshot_sha256: sha256(screenshot)
  };
}

async function exerciseKeyboardInteractions(page) {
  const checks = [];
  const summary = page.getByLabel("Work summary");
  const summaryHeight = await summary.evaluate((element) => element.getBoundingClientRect().height);
  await summary.fill("Review neutral item");
  checks.push(check("text-invalid-corrected", await page.getByText("Enter a work summary.").count() === 0));
  checks.push(check("text-control-geometry", Math.round(await summary.evaluate((element) => element.getBoundingClientRect().height)) === Math.round(summaryHeight)));

  const combo = page.getByRole("combobox", { name: "Status" });
  await combo.fill("Needs");
  await combo.press("ArrowDown");
  await combo.press("Enter");
  checks.push(check("combobox-keyboard-selection", (await page.getByTestId("event-summary").textContent()) === "Status selected: attention"));
  await page.getByRole("button", { name: "Ready", exact: true }).click();
  await page.getByRole("button", { name: "Show status options" }).click();
  checks.push(check("combobox-disabled-option", await page.getByRole("option", { name: /Protected/ }).getAttribute("aria-disabled") === "true"));
  await page.keyboard.press("Escape");
  if (await page.getByRole("option").count()) await page.keyboard.press("Escape");
  await page.getByRole("option").first().waitFor({ state: "detached" }).catch(() => undefined);

  const comboGroup = page.locator(".pds-field.pds-lookup-select > .pds-input-group");
  const readyGeometry = await roundedBox(comboGroup);
  await page.getByRole("button", { name: "Loading", exact: true }).click();
  const loadingGeometry = await roundedBox(comboGroup);
  checks.push(check("combobox-loading-state", await page.getByText("Loading options.").count() === 1));
  await page.getByRole("button", { name: "Empty", exact: true }).click();
  const emptyGeometry = await roundedBox(comboGroup);
  checks.push(check("combobox-empty-state", await page.getByText("No fixture statuses match.").count() === 1));
  await page.getByRole("button", { name: "Ready", exact: true }).click();
  const restoredGeometry = await roundedBox(comboGroup);
  checks.push(check(
    "combobox-state-stable-geometry",
    [loadingGeometry, emptyGeometry, restoredGeometry].every((geometry) => JSON.stringify(geometry) === JSON.stringify(readyGeometry)),
    { ready: readyGeometry, loading: loadingGeometry, empty: emptyGeometry, restored: restoredGeometry }
  ));

  const menuRoot = page.getByTestId("menu-root");
  const trigger = menuRoot.locator(".pds-menu-button__trigger");
  checks.push(check("menu-root-runtime-contract", await menuRoot.evaluate((element) =>
    element instanceof HTMLDivElement && element.dataset.rootContract === "element-neutral"
  )));
  await menuRoot.dispatchEvent("pointerdown", { pointerType: "mouse", bubbles: true });
  checks.push(check("menu-trigger-semantics", await trigger.evaluate((element) =>
    element instanceof HTMLButtonElement && element.getAttribute("aria-haspopup") === "menu"
  )));
  const triggerBox = await trigger.boundingBox();
  await trigger.click();
  await page.getByRole("menuitem", { name: /Review item/ }).press("Enter");
  checks.push(check("menu-keyboard-selection", (await page.getByTestId("event-summary").textContent()) === "Review selected"));
  await trigger.click();
  checks.push(check("menu-disabled-item", await page.getByRole("menuitem", { name: /Defer item/ }).getAttribute("aria-disabled") === "true"));
  await page.keyboard.press("Escape");
  await page.waitForFunction(() => document.activeElement?.classList.contains("pds-menu-button__trigger"));
  checks.push(check("menu-escape-focus-return", await trigger.evaluate((element) => document.activeElement === element)));
  checks.push(check("menu-root-forwarded-pointer-event", await menuRoot.getAttribute("data-last-pointer-type") === "mouse"));
  await trigger.click();
  await page.mouse.click(8, 8);
  checks.push(check("menu-outside-dismissal", await page.getByRole("menu").count() === 0));
  checks.push(...await exerciseVibrantMenuTone(page, trigger, menuRoot));
  const triggerAfter = await trigger.boundingBox();
  checks.push(check("menu-trigger-stable-geometry", JSON.stringify(triggerBox) === JSON.stringify(triggerAfter), { before: triggerBox, after: triggerAfter }));

  const legacyRoot = page.getByTestId("legacy-menu-root");
  const legacyTrigger = legacyRoot.locator(".pds-menu-button__trigger");
  checks.push(check("legacy-menu-details-root", await legacyRoot.evaluate((element) => element instanceof HTMLDetailsElement)));
  await legacyTrigger.click();
  checks.push(check("legacy-menu-child-inside-menu", await legacyRoot.getByRole("menu").getByRole("menuitem", { name: "Legacy child action" }).count() === 1));
  checks.push(check("legacy-menu-disabled-item", await legacyRoot.getByRole("menuitem", { name: "Unavailable legacy action" }).isDisabled()));
  await legacyRoot.getByRole("menuitem", { name: "Run legacy action" }).press("Enter");
  checks.push(check("legacy-menu-item-callback", (await page.getByTestId("event-summary").textContent()) === "Legacy item selected"));
  checks.push(check("legacy-menu-closes-after-item", !(await legacyRoot.evaluate((element) => element.open))));
  await legacyTrigger.click();
  await legacyRoot.getByRole("menuitem", { name: "Legacy child action" }).press("Enter");
  checks.push(check("legacy-menu-child-keyboard", (await page.getByTestId("event-summary").textContent()) === "Legacy child selected"));
  await page.keyboard.press("Escape");

  await page.getByLabel("Review date").fill("2026-08-14");
  await page.getByLabel("Review time").fill("09:30");
  checks.push(check("bounded-date-time-corrected", await page.getByText(/Choose a review (date|time)/).count() === 0));
  return checks;
}

async function exerciseTouchInteractions(page) {
  const checks = [];
  const combo = page.getByRole("combobox", { name: "Status" });
  await combo.tap();
  await combo.fill("Scheduled");
  await page.getByRole("option", { name: /Scheduled/ }).tap();
  checks.push(check("combobox-touch-selection", (await page.getByTestId("event-summary").textContent()) === "Status selected: scheduled"));

  const trigger = page.getByTestId("menu-root").locator(".pds-menu-button__trigger");
  await trigger.tap();
  await page.getByRole("menuitem", { name: /Remove from view/ }).tap();
  checks.push(check("menu-touch-selection", (await page.getByTestId("event-summary").textContent()) === "Remove selected"));
  return checks;
}

async function exerciseVibrantMenuTone(page, trigger, menuRoot) {
  const checks = [];
  await page.evaluate(() => { document.documentElement.dataset.visualTheme = "apple-like"; });
  await trigger.click();
  const appleTone = await openedMenuTone(page, menuRoot);
  checks.push(check(
    "menu-vibrant-tone-apple-like",
    appleTone.marker === "vibrant" && appleTone.insideRoot &&
      (appleTone.background !== "rgba(0, 0, 0, 0)" || appleTone.backgroundImage !== "none"),
    appleTone
  ));
  await page.keyboard.press("Escape");

  await page.evaluate(() => { document.documentElement.dataset.visualTheme = "material-like"; });
  await trigger.click();
  const materialTone = await openedMenuTone(page, menuRoot);
  checks.push(check(
    "menu-vibrant-tone-material-like",
    materialTone.marker === "vibrant" && materialTone.insideRoot &&
      materialTone.background === materialTone.expectedBackground &&
      materialTone.itemColor === materialTone.expectedItemColor,
    materialTone
  ));
  await page.keyboard.press("Escape");
  await page.evaluate(() => { document.documentElement.dataset.visualTheme = "apple-like"; });
  return checks;
}

async function openedMenuTone(page, menuRoot) {
  const menu = page.getByRole("menu");
  await menu.waitFor();
  return menu.evaluate((menuElement, rootElement) => {
    const overlay = menuElement.closest(".pds-menu-button__menu");
    const item = menuElement.querySelector('[role="menuitem"]');
    const resolveColor = (property, value) => {
      const probe = document.createElement("span");
      probe.style[property] = value;
      document.body.append(probe);
      const resolved = getComputedStyle(probe)[property];
      probe.remove();
      return resolved;
    };
    return {
      marker: overlay?.getAttribute("data-menu-tone"),
      insideRoot: Boolean(overlay && rootElement.contains(overlay)),
      background: overlay ? getComputedStyle(overlay).backgroundColor : null,
      backgroundImage: overlay ? getComputedStyle(overlay).backgroundImage : null,
      itemColor: item ? getComputedStyle(item).color : null,
      expectedBackground: resolveColor("backgroundColor", "var(--pds-m3-color-tertiary-container)"),
      expectedItemColor: resolveColor("color", "var(--pds-m3-color-on-tertiary-container)")
    };
  }, await menuRoot.elementHandle());
}

async function exerciseReducedMotionLabel(page) {
  const summary = page.getByLabel("Work summary");
  const wrapper = page.locator(".pds-text-field__label-wrapper").first();
  const before = await roundedBox(wrapper);
  await summary.focus();
  await summary.fill("Reduced motion proof");
  const after = await roundedBox(wrapper);
  const motion = await wrapper.evaluate((element) => ({
    activeAnimations: element.getAnimations({ subtree: true }).filter(({ playState }) => playState === "running").length,
    transitionDurations: [...element.querySelectorAll(".pds-text-field__label")]
      .map((label) => getComputedStyle(label).transitionDuration)
  }));
  return [
    check("reduced-motion-label-no-animation", motion.activeAnimations === 0 && motion.transitionDurations.every((value) => value === "0s"), motion),
    check("reduced-motion-label-stable-geometry", JSON.stringify(before) === JSON.stringify(after), { before, after })
  ];
}

async function exerciseForcedColorFocusAndInvalid(page) {
  const summary = page.getByLabel("Work summary");
  await summary.focus();
  const result = await summary.evaluate((element) => {
    const parseRgb = (value) => (value.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
    const luminance = (rgb) => {
      const values = rgb.map((channel) => {
        const value = channel / 255;
        return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * values[0] + 0.7152 * values[1] + 0.0722 * values[2];
    };
    const contrast = (foreground, background) => {
      const lighter = Math.max(luminance(parseRgb(foreground)), luminance(parseRgb(background)));
      const darker = Math.min(luminance(parseRgb(foreground)), luminance(parseRgb(background)));
      return (lighter + 0.05) / (darker + 0.05);
    };
    const inputStyle = getComputedStyle(element);
    const fieldSurface = element.closest(".pds-text-field");
    const fieldSurfaceStyle = fieldSurface ? getComputedStyle(fieldSurface) : null;
    const error = document.getElementById("work-summary-error");
    const errorStyle = error ? getComputedStyle(error) : null;
    const background = getComputedStyle(document.body).backgroundColor;
    return {
      invalid: element.getAttribute("aria-invalid"),
      focused: document.activeElement === element,
      outlineStyle: inputStyle.outlineStyle,
      outlineWidth: Number.parseFloat(inputStyle.outlineWidth),
      focusContrast: contrast(inputStyle.outlineColor, background),
      borderStyle: inputStyle.borderStyle,
      borderWidth: Number.parseFloat(inputStyle.borderWidth),
      borderContrast: contrast(inputStyle.borderColor, background),
      surfaceBorderStyle: fieldSurfaceStyle?.borderStyle ?? "none",
      surfaceBorderWidth: Number.parseFloat(fieldSurfaceStyle?.borderWidth ?? "0"),
      surfaceBorderContrast: fieldSurfaceStyle ? contrast(fieldSurfaceStyle.borderColor, background) : 0,
      errorVisible: Boolean(error && error.getBoundingClientRect().height > 0),
      errorContrast: errorStyle ? contrast(errorStyle.color, background) : 0
    };
  });
  return [
    check(
      "forced-colors-focus-contrast",
      result.focused && (
        (result.outlineStyle !== "none" && result.outlineWidth >= 2 && result.focusContrast >= 3) ||
        (result.borderStyle !== "none" && result.borderWidth >= 1 && result.borderContrast >= 3) ||
        (result.surfaceBorderStyle !== "none" && result.surfaceBorderWidth >= 1 && result.surfaceBorderContrast >= 3)
      ),
      result
    ),
    check("forced-colors-invalid-contrast", result.invalid === "true" && result.errorVisible && result.errorContrast >= 4.5, result)
  ];
}

async function roundedBox(locator) {
  const box = await locator.boundingBox();
  return box && Object.fromEntries(Object.entries(box).map(([key, value]) => [key, Math.round(value)]));
}

function check(id, ok, actual = null) {
  return { id, ok, actual };
}

async function availablePort() {
  const server = createServer();
  await new Promise((resolveReady, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolveReady);
  });
  const address = server.address();
  const selected = typeof address === "object" && address ? address.port : 0;
  await new Promise((resolveClosed) => server.close(resolveClosed));
  return selected;
}

async function waitForServer(target, serverProcess) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (serverProcess.exitCode !== null) throw new Error(`Vite exited with ${serverProcess.exitCode}.`);
    try {
      const response = await fetch(target);
      if (response.ok) return;
    } catch {
      // The bounded retry loop retains the eventual failure in the evidence artifact.
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  throw new Error(`Timed out waiting for ${target}.`);
}

function command(binary, args, cwd) {
  return new Promise((resolveCommand, reject) => {
    const processHandle = spawn(binary, args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    processHandle.stdout.on("data", (chunk) => { stdout += chunk; });
    processHandle.stderr.on("data", (chunk) => { stderr += chunk; });
    processHandle.once("error", reject);
    processHandle.once("close", (code) => resolveCommand({ code, stdout, stderr }));
  });
}
