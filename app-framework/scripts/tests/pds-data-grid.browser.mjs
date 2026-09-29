#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:net";
import { createRequire } from "node:module";
import { platform, release } from "node:os";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const catalogRoot = join(repoRoot, "appfw_ui/pds_health/catalog-app");
const targetRoot = join(repoRoot, "target/appfw/pds-data-grid");
const screenshotRoot = join(targetRoot, "screenshots");
const outputPath = join(targetRoot, "browser-evidence.json");
const require = createRequire(join(catalogRoot, "package.json"));
const { chromium } = require("@playwright/test");
const AxeBuilder = require("@axe-core/playwright").default;
const playwrightPackage = require("@playwright/test/package.json");

mkdirSync(screenshotRoot, { recursive: true });
const sha256 = (value) => createHash("sha256").update(value).digest("hex");
const sourceSha = (await command("git", ["rev-parse", "HEAD"], repoRoot)).stdout.trim();
const adapterPath = join(repoRoot, "appfw_ui/pds_health/components/src/data-grid.tsx");
const fixturePath = join(catalogRoot, "src/examples.tsx");
const port = await availablePort();
const url = `http://127.0.0.1:${port}/`;
const scenarios = [
  { id: "apple-light", theme: "light", visualTheme: "apple-like", viewport: { width: 1280, height: 900 }, interactions: true },
  { id: "apple-dark", theme: "dark", visualTheme: "apple-like", viewport: { width: 1280, height: 900 } },
  { id: "material-light", theme: "light", visualTheme: "material-like", viewport: { width: 1280, height: 900 } },
  { id: "material-dark", theme: "dark", visualTheme: "material-like", viewport: { width: 1280, height: 900 } },
  { id: "material-mobile", theme: "light", visualTheme: "material-like", viewport: { width: 390, height: 844 }, mobile: true }
];
const evidence = {
  schema: "pds-data-grid-browser-evidence/1.0",
  ok: false,
  claim_boundary: "local_snapshot_non_candidate",
  generated_at: new Date().toISOString(),
  source_sha: sourceSha,
  sources: [adapterPath, fixturePath].map((path) => ({
    path: relative(repoRoot, path),
    sha256: sha256(readFileSync(path))
  })),
  environment: {
    os: `${platform()} ${release()}`,
    node: process.version,
    playwright: playwrightPackage.version,
    chromium: null
  },
  failures: [],
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
  evidence.ok = evidence.scenarios.every(({ ok }) => ok);
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.stack ?? error.message : String(error));
} finally {
  await browser?.close();
  child?.kill("SIGTERM");
  writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`);
}

console.log(JSON.stringify({
  ok: evidence.ok,
  output: relative(repoRoot, outputPath),
  scenarios: evidence.scenarios.map(({ id, ok }) => ({ id, ok }))
}));
if (!evidence.ok) process.exitCode = 1;

async function runScenario(activeBrowser, scenario) {
  const context = await activeBrowser.newContext({
    viewport: scenario.viewport,
    colorScheme: scenario.theme,
    hasTouch: scenario.mobile
  });
  const page = await context.newPage();
  page.setDefaultTimeout(10000);
  const consoleErrors = [];
  const pageErrors = [];
  const externalRequests = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("request", (request) => {
    if (new URL(request.url()).hostname !== "127.0.0.1") externalRequests.push(request.url());
  });

  await page.goto(url, { waitUntil: "networkidle" });
  await page.evaluate(({ theme, visualTheme }) => {
    localStorage.setItem("pds-catalog-theme", theme);
    localStorage.setItem("pds-catalog-visual-theme", visualTheme);
  }, scenario);
  await page.reload({ waitUntil: "networkidle" });
  await page.getByPlaceholder("Search components…").fill("DataGrid");
  const card = page.locator("#component-DataGrid");
  await card.getByRole("grid", { name: "Advanced records grid" }).waitFor({ state: "visible" });
  await card.locator("[role='row'][row-index]").first().waitFor({ state: "visible" });

  const checks = [];
  if (scenario.interactions) checks.push(...await exerciseGrid(page, card));
  if (scenario.mobile) checks.push(...await exerciseMobileGrid(page, card));
  const axe = await new AxeBuilder({ page }).include("#component-DataGrid").analyze();
  const severe = axe.violations.filter(({ impact }) => impact === "serious" || impact === "critical");
  const knownAmbientWarnings = consoleErrors.filter((message) => (
    message.includes("both value and defaultValue props")
    && message.includes("TextField")
  ));
  const unexpectedConsoleErrors = consoleErrors.filter((message) => !knownAmbientWarnings.includes(message));
  const dom = await page.evaluate(({ theme, visualTheme }) => {
    const gridRoot = document.querySelector("#component-DataGrid .pds-data-grid-engine");
    const viewport = document.querySelector("#component-DataGrid .ag-body-viewport");
    return {
      root_theme: document.documentElement.dataset.theme,
      root_visual_theme: document.documentElement.dataset.visualTheme,
      grid_visual_theme: gridRoot?.getAttribute("data-visual-theme"),
      page_horizontal_overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      grid_scroll: viewport ? { client_width: viewport.clientWidth, scroll_width: viewport.scrollWidth } : null,
      expected: { theme, visualTheme }
    };
  }, scenario);
  checks.push(
    check("theme-propagation", dom.root_theme === scenario.theme && dom.root_visual_theme === scenario.visualTheme, dom),
    check("component-theme-adaptation", dom.grid_visual_theme === scenario.visualTheme, dom),
    check("serious-critical-axe", severe.length === 0, severe.map(({ id, impact }) => ({ id, impact }))),
    check("console-errors", unexpectedConsoleErrors.length === 0, {
      unexpected: unexpectedConsoleErrors,
      known_ambient_outside_grid: knownAmbientWarnings
    }),
    check("page-errors", pageErrors.length === 0, pageErrors),
    check("external-requests", externalRequests.length === 0, externalRequests),
    check("page-contained", dom.page_horizontal_overflow <= 1, dom)
  );
  const screenshotPath = join(screenshotRoot, `${scenario.id}.png`);
  const screenshot = await page.screenshot({ path: screenshotPath, fullPage: true });
  await context.close();
  return {
    id: scenario.id,
    ok: checks.every(({ ok }) => ok),
    viewport: scenario.viewport,
    theme: scenario.theme,
    visual_theme: scenario.visualTheme,
    axe: { violations: axe.violations.length, serious_critical: severe.length },
    checks,
    screenshot: relative(repoRoot, screenshotPath),
    screenshot_sha256: sha256(screenshot)
  };
}

async function exerciseGrid(page, card) {
  const checks = [];
  const rows = card.locator("[role='row'][row-index]");
  const recordHeader = card.getByRole("columnheader", { name: /Record/ });
  await recordHeader.click();
  await recordHeader.click();
  await page.waitForTimeout(80);
  checks.push(check(
    "descending-sort",
    (await rows.first().locator("[col-id='record']").textContent())?.trim() === "Record Zeta"
      && await recordHeader.getAttribute("aria-sort") === "descending"
  ));

  await recordHeader.locator(".ag-header-cell-filter-button").click({ force: true });
  const columnFilter = page.locator(".ag-popup input[aria-label='Filter Value']").first();
  await columnFilter.fill("Beta");
  await page.waitForFunction(() => document.querySelectorAll("#component-DataGrid [role='row'][row-index]").length === 1);
  checks.push(check(
    "column-filter",
    await rows.count() === 1
      && (await rows.first().locator("[col-id='record']").textContent())?.trim() === "Record Beta"
  ));
  await columnFilter.fill("");
  await page.keyboard.press("Escape");
  await page.waitForFunction(() => document.querySelectorAll("#component-DataGrid [role='row'][row-index]").length === 10);

  const recordWidthBeforeResize = await recordHeader.evaluate((element) => element.getBoundingClientRect().width);
  const resizeBox = await recordHeader.locator(".ag-header-cell-resize").boundingBox();
  if (!resizeBox) throw new Error("DataGrid record-column resize handle is not measurable.");
  const resizeX = resizeBox.x + resizeBox.width / 2;
  const resizeY = resizeBox.y + resizeBox.height / 2;
  await page.mouse.move(resizeX, resizeY);
  await page.mouse.down();
  await page.mouse.move(resizeX + 48, resizeY, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(80);
  const recordWidthAfterResize = await recordHeader.evaluate((element) => element.getBoundingClientRect().width);
  checks.push(check(
    "column-resize",
    recordWidthAfterResize >= recordWidthBeforeResize + 40,
    { recordWidthBeforeResize, recordWidthAfterResize }
  ));

  const quickFilter = card.getByRole("searchbox", { name: "Filter records" });
  await quickFilter.fill("Kappa");
  await page.waitForFunction(() => document.querySelectorAll("#component-DataGrid [role='row'][row-index]").length === 1);
  checks.push(check(
    "quick-filter",
    await rows.count() === 1
      && (await rows.first().locator("[col-id='record']").textContent())?.trim() === "Record Kappa"
  ));
  await quickFilter.fill("");
  await page.waitForFunction(() => document.querySelectorAll("#component-DataGrid [role='row'][row-index]").length === 10);

  const receipt = card.locator("[data-grid-activation-receipt]");
  const receiptBeforeCheckbox = (await receipt.textContent())?.trim();
  await rows.first().getByRole("checkbox").check();
  await page.waitForFunction(() => (
    document.querySelector("#component-DataGrid .pds-data-grid-toolbar__summary")?.textContent?.includes("1 selected")
  ));
  const selectionSummary = (await card.locator(".pds-data-grid-toolbar__summary").textContent())?.trim();
  const receiptAfterCheckbox = (await receipt.textContent())?.trim();
  checks.push(check(
    "checkbox-selects-without-activation",
    selectionSummary?.includes("1 selected") && receiptAfterCheckbox === receiptBeforeCheckbox,
    { receiptBeforeCheckbox, receiptAfterCheckbox, selectionSummary }
  ));
  const firstCell = rows.first().locator("[col-id='record']");
  const pointerRowId = await rows.first().getAttribute("row-id");
  await firstCell.click();
  const pointerReceipt = (await receipt.textContent())?.trim();
  checks.push(check("pointer-row-activation", pointerReceipt === `Activated ${pointerRowId}`, { pointerRowId, pointerReceipt }));
  await firstCell.focus();
  await page.keyboard.press("ArrowDown");
  const keyboardRowId = await card.locator(".ag-cell-focus").evaluate(
    (element) => element.closest("[role='row'][row-id]")?.getAttribute("row-id") ?? null
  );
  await page.keyboard.press("Enter");
  const keyboardReceipt = (await receipt.textContent())?.trim();
  checks.push(check("keyboard-row-activation", keyboardReceipt === `Activated ${keyboardRowId}`, { keyboardRowId, keyboardReceipt }));

  const embedded = rows.first().locator("[data-grid-embedded-actions]");
  const receiptBeforeEmbedded = (await receipt.textContent())?.trim();
  await embedded.getByRole("link").click();
  await embedded.locator("label").click();
  await embedded.getByRole("combobox").selectOption("archive");
  await embedded.getByRole("button").click();
  checks.push(check(
    "embedded-controls-do-not-activate-row",
    (await receipt.textContent())?.trim() === receiptBeforeEmbedded
      && await embedded.getByRole("button").getAttribute("data-activated") === "true"
  ));

  await recordHeader.click();
  await page.waitForFunction(() => (
    document.querySelector("#component-DataGrid [role='row'][row-index] [col-id='record']")?.textContent?.trim() === "Record Alpha"
  ));
  await rows.first().getByRole("checkbox").check();
  await card.locator(".ag-row-selected").waitFor();
  const selectedRowIdBeforeUpdate = await card.locator(".ag-row-selected").getAttribute("row-id");
  await card.getByRole("button", { name: "Replace rows" }).click();
  await card.getByText("Record Alpha v1", { exact: true }).waitFor();
  const selectedRowIdAfterUpdate = await card.locator(".ag-row-selected").getAttribute("row-id");
  checks.push(check(
    "stable-row-identity-across-data-replacement",
    selectedRowIdBeforeUpdate !== null && selectedRowIdAfterUpdate === selectedRowIdBeforeUpdate,
    { selectedRowIdBeforeUpdate, selectedRowIdAfterUpdate }
  ));

  await card.getByRole("button", { name: "Use single selection" }).click();
  await card.getByRole("button", { name: "Use multiple selection" }).waitFor();
  await rows.first().locator("[col-id='record']").click();
  const singleFirstId = await card.locator(".ag-row-selected").getAttribute("row-id");
  await rows.nth(1).locator("[col-id='record']").click();
  const singleRows = card.locator(".ag-row-selected");
  const singleSecondId = await singleRows.getAttribute("row-id");
  checks.push(check(
    "single-selection-replaces-prior-row",
    await singleRows.count() === 1 && singleFirstId !== singleSecondId,
    { singleFirstId, singleSecondId, selectedCount: await singleRows.count() }
  ));

  await card.locator("[aria-label='Next Page']").click();
  await page.waitForTimeout(80);
  checks.push(check(
    "pagination",
    await card.locator("input[aria-label^='Page number']").inputValue() === "2" && await rows.count() === 2
  ));
  await card.locator("[aria-label='Previous Page']").click();
  await card.getByRole("button", { name: "Show loading" }).click();
  await card.locator(".ag-overlay-loading-center").waitFor({ state: "visible" });
  checks.push(check("loading-overlay", await card.locator(".ag-overlay-loading-center").isVisible()));
  await card.getByRole("button", { name: "Show data" }).click();
  await card.getByRole("button", { name: "Show empty" }).click();
  await card.getByText("No records found", { exact: true }).waitFor({ state: "visible" });
  checks.push(check("empty-overlay", await card.getByText("No records found", { exact: true }).isVisible()));
  await card.getByRole("button", { name: "Restore rows" }).click();
  const compactHeight = await rows.first().evaluate((element) => element.getBoundingClientRect().height);
  await card.locator("label:has(input[type='radio'][value='comfortable'])").click();
  await page.waitForTimeout(80);
  const comfortableHeight = await rows.first().evaluate((element) => element.getBoundingClientRect().height);
  checks.push(check("density-changes-row-height", comfortableHeight > compactHeight, { compactHeight, comfortableHeight }));
  return checks;
}

async function exerciseMobileGrid(page, card) {
  const rows = card.locator("[role='row'][row-index]");
  const quickFilter = card.getByRole("searchbox", { name: "Filter records" });
  await quickFilter.fill("Kappa");
  await page.waitForFunction(() => document.querySelectorAll("#component-DataGrid [role='row'][row-index]").length === 1);
  const filteredRecord = (await rows.first().locator("[col-id='record']").textContent())?.trim();
  await quickFilter.fill("");
  const scroll = await card.locator(".ag-grid-viewport").evaluate((element) => {
    element.scrollLeft = Math.min(120, element.scrollWidth - element.clientWidth);
    return { clientWidth: element.clientWidth, scrollWidth: element.scrollWidth, scrollLeft: element.scrollLeft };
  });
  return [
    check("mobile-filter", filteredRecord === "Record Kappa", filteredRecord),
    check("mobile-grid-scroll-contained", scroll.scrollWidth > scroll.clientWidth && scroll.scrollLeft > 0, scroll)
  ];
}

function check(id, ok, actual = null) {
  return { id, ok: Boolean(ok), actual };
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
