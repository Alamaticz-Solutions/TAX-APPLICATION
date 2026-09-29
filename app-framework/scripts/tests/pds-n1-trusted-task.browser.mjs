#!/usr/bin/env node

import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { createServer } from "node:net";
import { platform, release } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const catalogRoot = join(repoRoot, "appfw_ui/pds_health/catalog-app");
const evidenceRoot = join(repoRoot, "target/appfw/pds-n1-trusted-task");
const screenshotsRoot = join(evidenceRoot, "screenshots");
const outputPath = join(evidenceRoot, "evidence.json");
const requireFromCatalog = createRequire(join(catalogRoot, "package.json"));
const baseSha = "92f87be0e29a7a81fac4db46ea8e27042f57fd9a";
const claimBoundary = "local_n0_snapshot_n1_preparation_non_candidate_non_accepted";

const sourcePaths = [
  "appfw_ui/pds_health/catalog-app/pds-n1-trusted-task.html",
  "appfw_ui/pds_health/catalog-app/src/trusted-task/contract.ts",
  "appfw_ui/pds_health/catalog-app/src/trusted-task/fixtures.ts",
  "appfw_ui/pds_health/catalog-app/src/trusted-task/TrustedTaskExperience.tsx",
  "appfw_ui/pds_health/catalog-app/src/trusted-task/main.tsx",
  "appfw_ui/pds_health/catalog-app/src/trusted-task/trusted-task.css",
  "scripts/tests/pds-n1-trusted-task.test.mjs",
  "scripts/tests/pds-n1-trusted-task.browser.mjs"
];

const scenarios = [
  { state: "loading", theme: "light", grammar: "apple-like", width: 1280, height: 900 },
  { state: "empty", theme: "dark", grammar: "material-like", width: 1280, height: 900 },
  { state: "permitted-preview", theme: "light", grammar: "apple-like", width: 1280, height: 900, interactions: true, deepLink: true },
  { id: "permitted-preview-touch-apple-like", state: "permitted-preview", theme: "light", grammar: "apple-like", width: 390, height: 844, touch: true, actionableTouch: true, touchTargetThreshold: 44 },
  { id: "permitted-preview-touch-material-like", state: "permitted-preview", theme: "light", grammar: "material-like", width: 390, height: 844, touch: true, actionableTouch: true, touchTargetThreshold: 48 },
  { id: "permitted-preview-rtl", state: "permitted-preview", theme: "dark", grammar: "material-like", direction: "rtl", width: 1280, height: 900 },
  { id: "permitted-preview-200-percent-reflow", state: "permitted-preview", theme: "light", grammar: "apple-like", width: 640, height: 450, nominalWidth: 1280, nominalHeight: 900, zoomPercent: 200 },
  { state: "denied", theme: "dark", grammar: "material-like", width: 390, height: 844, touch: true },
  { state: "stale", theme: "light", grammar: "material-like", width: 1280, height: 900 },
  { state: "partial", theme: "dark", grammar: "apple-like", width: 1280, height: 900 },
  { state: "offline", theme: "light", grammar: "material-like", width: 390, height: 844, touch: true },
  { state: "unauthorized", theme: "dark", grammar: "apple-like", width: 1280, height: 900, restricted: true },
  { state: "pending", theme: "light", grammar: "material-like", width: 640, height: 700 },
  { state: "success-receipt", theme: "dark", grammar: "material-like", width: 1280, height: 900 },
  { state: "failed", theme: "light", grammar: "apple-like", width: 1280, height: 900, failedRecovery: true },
  { state: "error", theme: "dark", grammar: "material-like", width: 390, height: 844, touch: true },
  { state: "recovery", theme: "light", grammar: "apple-like", width: 1280, height: 900, reducedMotion: true, forcedColors: true }
];

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

async function freePort() {
  return await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForServer(url, child) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`Vite exited before readiness with ${child.exitCode}.`);
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // The bounded local server is still starting.
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 120));
  }
  throw new Error("Timed out waiting for the bounded local Vite server.");
}

async function exerciseActionableTouch(page, scenario) {
  if (!scenario.actionableTouch) return null;
  const action = page.getByRole("button", { name: "Restore exact context", exact: true });
  const actionCount = await action.count();
  const box = actionCount === 1 ? await action.boundingBox() : null;
  const finitePositive = Boolean(box && Number.isFinite(box.width) && Number.isFinite(box.height) && box.width > 0 && box.height > 0);
  if (actionCount !== 1 || !finitePositive || box.height < scenario.touchTargetThreshold) {
    throw new Error(`${scenario.id} expected one finite positive touch target at least ${scenario.touchTargetThreshold}px high.`);
  }
  const maxTouchPoints = await page.evaluate(() => navigator.maxTouchPoints);
  if (maxTouchPoints < 1) throw new Error(`${scenario.id} did not run in a touch-capable browser context.`);
  if (await page.getByRole("button", { name: "Open local recovery", exact: true }).count() !== 0) {
    throw new Error(`${scenario.id} exposed a permission-bypassing recovery path.`);
  }

  await action.tap();
  await page.waitForTimeout(50);
  if (new URL(page.url()).searchParams.get("resume") !== "resume-neutral-attention-01-v1") {
    throw new Error(`${scenario.id} did not retain the exact resume token.`);
  }
  if (await page.locator("#trusted-task-receipt").evaluate((node) => node !== document.activeElement)) {
    throw new Error(`${scenario.id} did not move focus to the receipt heading.`);
  }
  for (const value of ["receipt-neutral-0001", "corr-fixture-7f31a2", "operation-fixture-success-0001", "simulated_complete"]) {
    if (!await page.getByText(value, { exact: true }).count()) throw new Error(`${scenario.id} lost ${value}.`);
  }
  const actionCountAfterActivation = await action.count();
  if (actionCountAfterActivation !== 0) throw new Error(`${scenario.id} retained or duplicated the restore action after activation.`);

  return {
    ok: true,
    mode: "playwright_touch_context_tap",
    max_touch_points: maxTouchPoints,
    rendered_action_count: actionCount,
    action_count_after_activation: actionCountAfterActivation,
    measured_box_css_px: box,
    minimum_height_threshold_css_px: scenario.touchTargetThreshold,
    exact_resume_token: true,
    receipt_heading_focused: true,
    receipt_identity_visible: true,
    duplicate_or_permission_bypass_path: false
  };
}

async function measureAdaptation(page, scenario) {
  const result = { direction: null, zoom_reflow: null };
  if (scenario.direction === "rtl") {
    const direction = await page.evaluate(() => document.documentElement.dir);
    const textAlign = await page.locator(".trusted-task__summary").evaluate((node) => getComputedStyle(node).textAlign);
    const evidenceBox = await page.locator(".trusted-task__evidence").boundingBox();
    const previewBox = await page.locator(".trusted-task__preview").boundingBox();
    if (direction !== "rtl" || textAlign !== "start" || !evidenceBox || !previewBox || evidenceBox.x <= previewBox.x) {
      throw new Error(`${scenario.id} did not preserve direct RTL direction, logical text alignment, or workspace geometry: ${JSON.stringify({ direction, textAlign, evidenceBox, previewBox })}`);
    }
    result.direction = {
      ok: true,
      mode: "direct_html_dir",
      html_dir: direction,
      text_align: textAlign,
      geometry_css_px: { evidence: evidenceBox, preview: previewBox }
    };
  }

  if (scenario.zoomPercent) {
    const expectedCssWidth = scenario.nominalWidth / (scenario.zoomPercent / 100);
    const expectedCssHeight = scenario.nominalHeight / (scenario.zoomPercent / 100);
    const reflow = await page.evaluate(() => ({
      inner_width: window.innerWidth,
      inner_height: window.innerHeight,
      client_width: document.documentElement.clientWidth,
      device_pixel_ratio: window.devicePixelRatio,
      horizontal_overflow: document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
      clipped_controls: [...document.querySelectorAll("button, input, select")]
        .filter((node) => {
          const rect = node.getBoundingClientRect();
          return rect.width > 0 && (rect.left < -1 || rect.right > window.innerWidth + 1);
        })
        .map((node) => node.getAttribute("aria-label") ?? node.textContent?.trim() ?? node.tagName)
    }));
    const geometry = {
      overview: await boxIfPresent(page.locator(".trusted-task__overview")),
      evidence: await boxIfPresent(page.locator(".trusted-task__evidence")),
      preview: await boxIfPresent(page.locator(".trusted-task__preview")),
      receipt: await boxIfPresent(page.locator(".trusted-task__receipt"))
    };
    if (reflow.inner_width !== expectedCssWidth || reflow.client_width !== expectedCssWidth || reflow.inner_height !== expectedCssHeight) {
      throw new Error(`${scenario.id} expected ${expectedCssWidth}x${expectedCssHeight} CSS-pixel reflow but received ${reflow.inner_width}x${reflow.inner_height}.`);
    }
    if (reflow.device_pixel_ratio !== 1 || reflow.horizontal_overflow || reflow.clipped_controls.length || Object.values(geometry).some((box) => !box)) {
      throw new Error(`${scenario.id} used DPR substitution, overflowed, clipped a control, or lost geometry.`);
    }
    result.zoom_reflow = {
      ok: true,
      mode: "equivalent_css_pixel_browser_reflow",
      nominal_browser_viewport_css_px: { width: scenario.nominalWidth, height: scenario.nominalHeight },
      effective_layout_viewport_css_px: { width: reflow.inner_width, height: reflow.inner_height },
      zoom_percent: scenario.zoomPercent,
      device_pixel_ratio: reflow.device_pixel_ratio,
      dpr_substitution: false,
      horizontal_overflow: reflow.horizontal_overflow,
      clipped_controls: reflow.clipped_controls,
      geometry_css_px: geometry
    };
  }
  return result;
}

async function main() {
  const started = Date.now();
  rmSync(evidenceRoot, { recursive: true, force: true });
  mkdirSync(screenshotsRoot, { recursive: true });
  const port = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const pagePath = "/pds-n1-trusted-task.html";
  const server = spawn(process.platform === "win32" ? "npm.cmd" : "npm", ["run", "dev", "--", "--port", String(port), "--strictPort"], {
    cwd: catalogRoot,
    stdio: ["ignore", "pipe", "pipe"]
  });
  let serverStdout = "";
  let serverStderr = "";
  server.stdout.on("data", (chunk) => { serverStdout += chunk; });
  server.stderr.on("data", (chunk) => { serverStderr += chunk; });

  const evidence = {
    command: "node scripts/tests/pds-n1-trusted-task.browser.mjs",
    ok: false,
    claim_boundary: claimBoundary,
    base_sha: baseSha,
    source_sha: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot, encoding: "utf8" }).trim(),
    immutable_snapshot: { identity: "n0-snapshot/92f87be0", superseded: true, reconciliation_required: true },
    generated_at: new Date().toISOString(),
    source_hashes: Object.fromEntries(sourcePaths.map((path) => [path, sha256(readFileSync(join(repoRoot, path)))])),
    environment: { os: `${platform()} ${release()}`, playwright: null, chromium: null, device_scale_factor_policy: "fixed_at_1_no_dpr_zoom_substitution" },
    scenarios: [],
    failures: [],
    retries: 0,
    summary: {}
  };

  let browser;
  try {
    await waitForServer(`${origin}${pagePath}`, server);
    const playwright = requireFromCatalog("@playwright/test");
    const AxeBuilder = requireFromCatalog("@axe-core/playwright").default;
    evidence.environment.playwright = requireFromCatalog("@playwright/test/package.json").version;
    browser = await playwright.chromium.launch();
    evidence.environment.chromium = browser.version();

    for (const scenario of scenarios) {
      const scenarioStarted = Date.now();
      const context = await browser.newContext({
        viewport: { width: scenario.width, height: scenario.height },
        deviceScaleFactor: 1,
        hasTouch: scenario.touch ?? false,
        colorScheme: scenario.theme
      });
      const page = await context.newPage();
      await page.emulateMedia({
        colorScheme: scenario.theme,
        reducedMotion: scenario.reducedMotion ? "reduce" : "no-preference",
        forcedColors: scenario.forcedColors ? "active" : "none"
      });
      const requests = [];
      const consoleErrors = [];
      const pageErrors = [];
      page.on("request", (request) => requests.push({ url: request.url(), method: request.method(), type: request.resourceType() }));
      page.on("console", (message) => { if (message.type() === "error") consoleErrors.push(message.text()); });
      page.on("pageerror", (error) => pageErrors.push(error.message));

      const url = new URL(pagePath, origin);
      url.searchParams.set("state", scenario.state);
      url.searchParams.set("theme", scenario.theme);
      url.searchParams.set("grammar", scenario.grammar);
      if (scenario.deepLink) url.searchParams.set("work", "work-neutral-attention-01");
      if (scenario.state === "recovery") {
        url.searchParams.set("work", "work-neutral-attention-01");
        url.searchParams.set("resume", "resume-neutral-attention-01-v1");
      }
      await page.goto(url.href, { waitUntil: "networkidle" });
      if (scenario.direction === "rtl") {
        await page.evaluate(() => { document.documentElement.dir = "rtl"; });
      }
      await page.getByRole("heading", { name: "Review before the next step" }).waitFor();
      if (await page.locator(`[data-n1-state="${scenario.state}"]`).count() !== 1) throw new Error(`State ${scenario.state} did not mount.`);

      const actionableTouch = await exerciseActionableTouch(page, scenario);
      const adaptation = await measureAdaptation(page, scenario);

      if (scenario.interactions) {
        await page.getByRole("button", { name: "Restore exact context" }).click();
        await page.waitForTimeout(50);
        if (await page.locator("#trusted-task-receipt").evaluate((node) => node !== document.activeElement)) {
          throw new Error("Receipt focus did not follow exact resume.");
        }
        if (!page.url().includes("resume=resume-neutral-attention-01-v1")) throw new Error("Resume URL did not retain the exact token.");
        await page.goBack();
        await page.waitForTimeout(50);
        if (await page.locator("#trusted-task-overview").evaluate((node) => node !== document.activeElement)) {
          throw new Error("History navigation did not restore overview focus.");
        }
      }

      if (["denied", "stale", "partial", "offline", "error"].includes(scenario.state)) {
        if (await page.getByRole("button", { name: "Return to permitted preview" }).count() !== 0) {
          throw new Error(`${scenario.state} exposed a permission-overriding preview transition.`);
        }
        if (await page.getByRole("button", { name: "Open local recovery" }).count() !== 0) {
          throw new Error(`${scenario.state} exposed a recovery control without an explicit permitted result.`);
        }
        if (!page.url().includes(`state=${scenario.state}`)) throw new Error(`${scenario.state} URL posture changed unexpectedly.`);
      }

      if (["denied", "stale", "partial", "offline"].includes(scenario.state)) {
        const failClosedSnapshot = async () => ({
          url: page.url(),
          state: await page.locator(".trusted-task").getAttribute("data-n1-state"),
          permission: await page.locator(".trusted-task__preview-meta dd").first().innerText(),
          receipt: await page.locator(".trusted-task__receipt").innerText(),
          correlations: await page.locator(".trusted-task__receipt-grid dd").allInnerTexts(),
          operationIdentities: await page.getByText(/^operation-fixture-/).allInnerTexts()
        });
        const before = await failClosedSnapshot();
        const restoreControls = page.getByRole("button", { name: "Restore exact context" });
        if (await restoreControls.count() !== 0) throw new Error(`${scenario.state} exposed Restore exact context without permission.`);
        const expectedPermission = scenario.state === "denied" ? "denied" : "unavailable";
        if (!before.permission.toLowerCase().includes(expectedPermission)) throw new Error(`${scenario.state} lost ${expectedPermission} permission posture.`);
        await page.waitForTimeout(50);
        const after = await failClosedSnapshot();
        if (JSON.stringify(after) !== JSON.stringify(before)) {
          throw new Error(`${scenario.state} changed state, URL, permission, receipt, correlation, or operation identity while fail closed.`);
        }
      }

      if (scenario.failedRecovery) {
        await page.getByRole("button", { name: "Restore exact context" }).first().click();
        await page.waitForTimeout(50);
        if (!page.url().includes("state=failed") || !page.url().includes("resume=resume-neutral-attention-01-v1")) {
          throw new Error("Failed recovery did not retain failed URL and resume identity.");
        }
        for (const value of ["receipt-neutral-failed-0001", "corr-fixture-91c0e4", "operation-fixture-failed-0001", "simulated_failed"]) {
          if (!await page.getByText(value, { exact: true }).count()) throw new Error(`Failed recovery lost ${value}.`);
        }
        if (await page.getByText("corr-fixture-7f31a2", { exact: true }).count()) throw new Error("Failed recovery substituted the success correlation.");
        await page.reload({ waitUntil: "networkidle" });
        await page.waitForTimeout(50);
        if (await page.locator("#trusted-task-receipt").evaluate((node) => node !== document.activeElement)) {
          throw new Error("Failed recovery reload did not restore receipt focus.");
        }
        for (const value of ["receipt-neutral-failed-0001", "corr-fixture-91c0e4", "operation-fixture-failed-0001", "simulated_failed"]) {
          if (!await page.getByText(value, { exact: true }).count()) throw new Error(`Failed recovery reload lost ${value}.`);
        }
        await page.goBack();
        await page.waitForTimeout(50);
        if (!page.url().includes("state=failed") || page.url().includes("resume=")) {
          throw new Error("Failed recovery history did not return to the originating failed URL.");
        }
        if (await page.locator("#trusted-task-receipt").evaluate((node) => node !== document.activeElement)) {
          throw new Error("Failed recovery history did not retain receipt focus.");
        }
        for (const value of ["receipt-neutral-failed-0001", "corr-fixture-91c0e4", "operation-fixture-failed-0001", "simulated_failed"]) {
          if (!await page.getByText(value, { exact: true }).count()) throw new Error(`Failed recovery history lost ${value}.`);
        }
        await page.goForward();
        await page.waitForTimeout(50);
        if (!page.url().includes("state=failed") || !page.url().includes("resume=resume-neutral-attention-01-v1")) {
          throw new Error("Failed recovery forward navigation lost exact resume identity.");
        }

        await page.evaluate(async () => {
          const { TRUSTED_TASK_FIXTURES } = await import("/src/trusted-task/fixtures.ts");
          TRUSTED_TASK_FIXTURES.failed.preview.permission.result = "denied";
          window.dispatchEvent(new PopStateEvent("popstate", { state: { state: "permitted-preview" } }));
        });
        await page.locator('[data-n1-state="permitted-preview"]').waitFor();
        await page.evaluate(() => {
          window.dispatchEvent(new PopStateEvent("popstate", { state: { state: "failed" } }));
        });
        await page.locator('[data-n1-state="failed"]').waitFor();
        const deniedFailedSnapshot = async () => ({
          url: page.url(),
          state: await page.locator(".trusted-task").getAttribute("data-n1-state"),
          permission: await page.locator(".trusted-task__preview-meta dd").first().innerText(),
          receipt: await page.locator(".trusted-task__receipt-grid dd").allInnerTexts()
        });
        const beforeDeniedFailed = await deniedFailedSnapshot();
        if (beforeDeniedFailed.permission !== "denied") throw new Error("Failed fixture did not expose the denied test posture.");
        if (await page.getByRole("button", { name: "Restore exact context" }).count() !== 0) {
          throw new Error("Failed state exposed receipt recovery without explicit permission.");
        }
        await page.waitForTimeout(50);
        const afterDeniedFailed = await deniedFailedSnapshot();
        if (JSON.stringify(afterDeniedFailed) !== JSON.stringify(beforeDeniedFailed)) {
          throw new Error("Non-permitted failed state changed URL, permission, receipt, correlation, or operation identity.");
        }
        for (const value of ["receipt-neutral-failed-0001", "corr-fixture-91c0e4", "operation-fixture-failed-0001", "simulated_failed"]) {
          if (!afterDeniedFailed.receipt.includes(value)) throw new Error(`Non-permitted failed state lost ${value}.`);
        }
      }

      const ids = await page.locator("[id]").evaluateAll((nodes) => nodes.map((node) => node.id));
      const duplicateIds = ids.filter((id, index) => ids.indexOf(id) !== index);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1);
      const bodyText = await page.locator("body").innerText();
      if (scenario.restricted && /Fixture evidence register|work-neutral-attention-01/.test(bodyText)) {
        throw new Error("Unauthorized posture exposed fixture work detail.");
      }
      if (scenario.state === "denied" && !/denied/i.test(bodyText)) throw new Error("Denied posture was not visible.");
      if (scenario.state === "stale" && !/last-known|stale/i.test(bodyText)) throw new Error("Stale posture was not visible.");
      if (scenario.state === "error" && !/recovery/i.test(bodyText)) throw new Error("Error recovery was not visible.");
      if (scenario.state === "success-receipt" && !/corr-fixture-7f31a2/.test(bodyText)) throw new Error("Receipt did not resolve correlation evidence.");
      if (scenario.deepLink && !/local notification or deep link/i.test(bodyText)) throw new Error("Deep-link launch context was not visible.");

      const axe = await new AxeBuilder({ page }).analyze();
      const severe = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
      const external = requests.filter((request) => new URL(request.url).origin !== origin);
      const fonts = requests.filter((request) => /\.(woff2?|ttf|otf)(\?|$)/i.test(request.url));
      const unexpectedFonts = fonts.filter((request) => {
        const requestUrl = new URL(request.url);
        return requestUrl.origin !== origin
          || !/(?:InterVariable|GeistMonoVariable)\.woff2$/.test(requestUrl.pathname);
      });
      const mutations = requests.filter((request) => !["GET", "HEAD", "OPTIONS"].includes(request.method));
      const landmarks = {
        overview: await page.locator("#trusted-task-overview").count(),
        evidence: await page.locator("#trusted-task-evidence").count(),
        preview: await page.locator("#trusted-task-preview").count(),
        receipt: await page.locator("#trusted-task-receipt").count()
      };
      const geometry = expectedGeometry(
        await boxIfPresent(page.locator(".trusted-task__overview")),
        await boxIfPresent(page.locator(".trusted-task__evidence")),
        await boxIfPresent(page.locator(".trusted-task__preview")),
        await boxIfPresent(page.locator(".trusted-task__receipt"))
      );
      const expectedLandmarks = scenario.restricted || scenario.state === "loading" || scenario.state === "empty" || scenario.state === "error" ? 0 : 4;
      const scenarioOk = Object.values(landmarks).reduce((sum, count) => sum + count, 0) === expectedLandmarks &&
        duplicateIds.length === 0 && !overflow && severe.length === 0 && consoleErrors.length === 0 && pageErrors.length === 0 &&
        external.length === 0 && unexpectedFonts.length === 0 && mutations.length === 0 && geometry.ok;
      const screenshot = join(screenshotsRoot, `${scenario.id ?? `${scenario.state}-${scenario.theme}-${scenario.grammar}`}.png`);
      await page.screenshot({ path: screenshot, fullPage: true });
      evidence.scenarios.push({
        ...scenario,
        ok: scenarioOk,
        elapsed_ms: Date.now() - scenarioStarted,
        actionable_touch: actionableTouch,
        adaptation,
        duplicate_ids: duplicateIds,
        horizontal_overflow: overflow,
        landmarks,
        geometry,
        axe: { total: axe.violations.length, serious_or_critical: severe },
        console_errors: consoleErrors,
        page_errors: pageErrors,
        external_requests: external,
        custom_font_requests: fonts,
        unexpected_font_requests: unexpectedFonts,
        mutation_requests: mutations,
        screenshot: relative(repoRoot, screenshot)
      });
      await context.close();
    }

    const unknown = await browser.newPage({ viewport: { width: 900, height: 700 } });
    await unknown.goto(`${origin}${pagePath}?state=unknown&theme=unknown&grammar=unknown&work=unknown&resume=unknown`, { waitUntil: "networkidle" });
    if (await unknown.getByText("Unknown local inputs ignored").count() !== 1) throw new Error("Unknown inputs were not surfaced.");
    await unknown.close();

    evidence.ok = evidence.scenarios.every((scenario) => scenario.ok);
    evidence.summary = {
      passed: evidence.scenarios.filter((scenario) => scenario.ok).length,
      failed: evidence.scenarios.filter((scenario) => !scenario.ok).length,
      serious_or_critical: evidence.scenarios.reduce((count, scenario) => count + scenario.axe.serious_or_critical.length, 0),
      external_requests: evidence.scenarios.reduce((count, scenario) => count + scenario.external_requests.length, 0),
      custom_font_requests: evidence.scenarios.reduce((count, scenario) => count + scenario.custom_font_requests.length, 0),
      unexpected_font_requests: evidence.scenarios.reduce((count, scenario) => count + scenario.unexpected_font_requests.length, 0),
      mutation_requests: evidence.scenarios.reduce((count, scenario) => count + scenario.mutation_requests.length, 0),
      actionable_touch_scenarios: evidence.scenarios.filter((scenario) => scenario.actionable_touch?.ok).length,
      actionable_touch_targets: evidence.scenarios.filter((scenario) => scenario.actionable_touch).map((scenario) => ({
        id: scenario.id,
        grammar: scenario.grammar,
        measured_box_css_px: scenario.actionable_touch.measured_box_css_px,
        minimum_height_threshold_css_px: scenario.actionable_touch.minimum_height_threshold_css_px
      })),
      rtl_scenarios: evidence.scenarios.filter((scenario) => scenario.adaptation.direction?.ok).length,
      zoom_reflow_scenarios: evidence.scenarios.filter((scenario) => scenario.adaptation.zoom_reflow?.ok).length,
      elapsed_ms: Date.now() - started
    };
    if (!evidence.ok) throw new Error("One or more trusted-task browser scenarios failed.");
  } catch (error) {
    evidence.failures.push(error instanceof Error ? error.message : String(error));
    throw error;
  } finally {
    await browser?.close();
    server.kill("SIGTERM");
    evidence.server = { stdout: serverStdout, stderr: serverStderr };
    mkdirSync(evidenceRoot, { recursive: true });
    writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`);
  }

  console.log(JSON.stringify({ ok: evidence.ok, output: relative(repoRoot, outputPath), summary: evidence.summary }, null, 2));
}

function expectedGeometry(overview, evidence, preview, receipt) {
  const boxes = { overview, evidence, preview, receipt };
  const visible = Object.values(boxes).filter(Boolean);
  if (visible.length === 0) return { ok: true, boxes };
  const finite = visible.every((box) => Number.isFinite(box.width) && Number.isFinite(box.height) && box.width > 0 && box.height > 0);
  const minimums = Boolean(overview && evidence && preview && receipt) && overview.height >= 190 &&
    evidence.height >= 330 && preview.height >= 330 && receipt.height >= 190;
  return { ok: finite && minimums, boxes, finite, minimums };
}

async function boxIfPresent(locator) {
  return await locator.count() === 1 ? await locator.boundingBox() : null;
}

await main();
