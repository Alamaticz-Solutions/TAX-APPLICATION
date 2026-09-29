#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:net";
import { platform, release } from "node:os";
import { createRequire } from "node:module";
import {
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../..");
const catalogRoot = join(repoRoot, "appfw_ui/pds_health/catalog-app");
const evidenceRoot = join(repoRoot, "target/appfw/pds-n0-neutral-my-work");
const screenshotsRoot = join(evidenceRoot, "screenshots");
const outputPath = join(evidenceRoot, "evidence.json");
const requireFromCatalog = createRequire(join(catalogRoot, "package.json"));
const claimBoundary = "local_frozen_f1_n0_snapshot_non_credit";
const snapshotIdentity = {
  f1_base_sha: "83b24c446a76de4d4abf96e43343f606ff708cc2",
  n0_source_commits: [
    "92f87be0e29a7a81fac4db46ea8e27042f57fd9a",
    "a7da1f01afc9d8e622d8756c10dd800ec6be216c"
  ]
};
const protectedFixtureCopy = /Review intake summary|Prepare follow-up notes|Archive completed context/;

const sourcePaths = [
  "appfw_ui/pds_health/catalog-app/pds-n0-neutral-my-work.html",
  "appfw_ui/pds_health/catalog-app/src/neutral-work/contract.ts",
  "appfw_ui/pds_health/catalog-app/src/neutral-work/fixtures.ts",
  "appfw_ui/pds_health/catalog-app/src/neutral-work/NeutralMyWorkExperience.tsx",
  "appfw_ui/pds_health/catalog-app/src/neutral-work/main.tsx",
  "appfw_ui/pds_health/catalog-app/src/neutral-work/neutral-work.css",
  "scripts/tests/pds-n0-neutral-my-work.test.mjs",
  "scripts/tests/pds-n0-neutral-my-work.browser.mjs"
];

const scenarios = [
  { id: "loading-desktop", state: "loading", theme: "light", grammar: "apple-like", width: 1280, height: 900 },
  { id: "populated-desktop", state: "populated", theme: "light", grammar: "apple-like", width: 1280, height: 900, interactions: true, comprehension: true },
  { id: "populated-rtl", state: "populated", theme: "dark", grammar: "material-like", direction: "rtl", width: 1280, height: 900 },
  { id: "empty-desktop", state: "empty", theme: "dark", grammar: "material-like", width: 1280, height: 900 },
  { id: "error-touch", state: "error", theme: "light", grammar: "material-like", width: 390, height: 844, touch: true, recoveryControl: true },
  { id: "partial-desktop", state: "partial", theme: "dark", grammar: "apple-like", width: 1280, height: 900, recoveryControl: true },
  { id: "stale-desktop", state: "stale", theme: "light", grammar: "material-like", width: 1280, height: 900, recoveryControl: true },
  { id: "offline-touch", state: "offline", theme: "dark", grammar: "material-like", width: 390, height: 844, touch: true, recoveryControl: true },
  { id: "unauthorized-restricted", state: "unauthorized", theme: "light", grammar: "apple-like", width: 1280, height: 900, restricted: true },
  { id: "forbidden-restricted", state: "forbidden", theme: "dark", grammar: "material-like", width: 1280, height: 900, restricted: true },
  { id: "conflict-desktop", state: "conflict", theme: "light", grammar: "apple-like", width: 1280, height: 900, recoveryControl: true },
  { id: "timeout-200-percent-reflow", state: "timeout", theme: "dark", grammar: "material-like", width: 640, height: 450, nominalWidth: 1280, zoomPercent: 200, recoveryControl: true },
  { id: "success-reduced-motion", state: "success", theme: "light", grammar: "material-like", width: 1280, height: 900, reducedMotion: true },
  { id: "recovery-forced-colors", state: "recovery", theme: "light", grammar: "apple-like", width: 1280, height: 900, forcedColors: true, reducedMotion: true }
];

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function run(command, args, cwd = repoRoot) {
  return new Promise((resolveRun) => {
    const child = spawn(command, args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("close", (code) => resolveRun({ command: [command, ...args].join(" "), code, ok: code === 0, stdout, stderr }));
  });
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
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The bounded local server is still starting.
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 120));
  }
  throw new Error("Timed out waiting for the bounded local Vite server.");
}

async function exerciseInteractions(page) {
  const search = page.getByRole("searchbox", { name: "Search work" });
  await search.fill("follow");
  if ((await page.locator(".neutral-work__row").count()) !== 1) throw new Error("Local search did not reduce the queue to one row.");
  await search.fill("");
  await page.getByRole("combobox", { name: "Status" }).selectOption("ready");
  if ((await page.locator(".neutral-work__row").count()) !== 1) throw new Error("Status filtering was not deterministic.");
  await page.getByRole("combobox", { name: "Status" }).selectOption("all");

  const row = page.locator("#work-row-work-alpha");
  await row.click();
  if (new URL(page.url()).searchParams.get("item") !== "alpha-review") throw new Error("Selected detail URL was not retained.");
  if (await page.locator("#neutral-work-detail").evaluate((node) => node !== document.activeElement)) {
    throw new Error("Detail focus did not move to the selected heading.");
  }
  await page.getByRole("button", { name: "Back to queue" }).click();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
  await page.waitForTimeout(50);
  if (await row.evaluate((node) => node !== document.activeElement)) throw new Error("Queue-row focus was not restored.");
  if (await page.getByRole("heading", { name: "Review intake summary" }).count() !== 0) throw new Error("Back to queue did not clear visible selection.");

  await page.reload({ waitUntil: "networkidle" });
  if (new URL(page.url()).searchParams.has("item")) throw new Error("Reload restored a cleared item URL.");
  if (await page.getByRole("heading", { name: "Review intake summary" }).count() !== 0) throw new Error("Reload reopened cleared detail.");

  await row.click();
  await page.goBack();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
  await page.waitForTimeout(50);
  if (await row.evaluate((node) => node !== document.activeElement)) throw new Error("Browser-back focus was not restored.");
  await page.goForward();
  await page.waitForFunction(() => new URL(window.location.href).searchParams.get("item") === "alpha-review");
  if (await page.locator("#neutral-work-detail").evaluate((node) => node !== document.activeElement)) throw new Error("Browser-forward detail focus was not restored.");
  await page.getByRole("button", { name: "Back to queue" }).click();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));

  const notification = page.locator("#notification-notice-alpha");
  await notification.click();
  if (new URL(page.url()).searchParams.get("item") !== "alpha-review") throw new Error("Notification detail URL was not retained.");
  await page.getByRole("button", { name: "Back to queue" }).click();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
  await page.waitForTimeout(50);
  if (await notification.evaluate((node) => node !== document.activeElement)) throw new Error("Notification focus was not restored.");

  const direct = new URL(page.url());
  direct.searchParams.set("item", "alpha-review");
  await page.goto(direct.href, { waitUntil: "networkidle" });
  await page.getByRole("heading", { name: "Review intake summary" }).waitFor();
  await page.getByRole("button", { name: "Back to queue" }).click();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
  if (await page.getByRole("heading", { name: "Review intake summary" }).count() !== 0) throw new Error("Direct deep-link exit retained visible selection.");
  await page.reload({ waitUntil: "networkidle" });
  if (await page.getByRole("heading", { name: "Review intake summary" }).count() !== 0) throw new Error("Direct deep-link exit reopened after reload.");

  return {
    selected_url_retained: true,
    back_command_clears_url_and_selection: true,
    cleared_url_reload_stays_in_queue: true,
    browser_back_forward_restores_focus_and_detail: true,
    notification_focus_restored: true,
    direct_deep_link_exit_is_reload_stable: true
  };
}

async function instrumentComprehension(page) {
  const fiveSecondStarted = Date.now();
  const row = page.locator("#work-row-work-alpha");
  await page.getByRole("heading", { name: "Current work" }).waitFor({ timeout: 5_000 });
  await row.getByText("Review intake summary", { exact: true }).waitFor({ timeout: 5_000 });
  await row.getByText("Confirm the bounded context and evidence posture.", { exact: true }).waitFor({ timeout: 5_000 });
  await row.getByText("needed", { exact: true }).waitFor({ timeout: 5_000 });
  const fiveSecondElapsed = Date.now() - fiveSecondStarted;
  if (fiveSecondElapsed > 5_000) throw new Error("Five-second orientation instrumentation exceeded its bound.");

  const thirtySecondStarted = Date.now();
  await page.getByRole("searchbox", { name: "Search work" }).fill("review");
  await row.click();
  await page.getByRole("heading", { name: "Review intake summary" }).waitFor();
  await page.getByText("Evidence summary", { exact: true }).waitFor();
  const posture = page.getByLabel("Work posture");
  await posture.getByText("pending", { exact: true }).waitFor();
  await posture.getByText("needed", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Back to queue" }).click();
  await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
  await page.getByRole("searchbox", { name: "Search work" }).fill("");
  const thirtySecondElapsed = Date.now() - thirtySecondStarted;
  if (thirtySecondElapsed > 30_000) throw new Error("Thirty-second task instrumentation exceeded its bound.");

  return {
    posture: "automated_fixture_instrumentation_only",
    acceptance_credit: false,
    human_design_disposition: "not_collected",
    five_second_orientation: {
      ok: true,
      elapsed_ms: fiveSecondElapsed,
      cues: ["current-work-heading", "work-title", "why-summary", "attention-needed"]
    },
    thirty_second_task: {
      ok: true,
      elapsed_ms: thirtySecondElapsed,
      steps: ["search", "select", "read-evidence", "inspect-approval-and-attention", "return-to-queue"]
    }
  };
}

async function exerciseRecoveryContract(context, url, scenario, origin) {
  const probe = await context.newPage();
  const requests = [];
  probe.on("request", (request) => requests.push({ url: request.url(), method: request.method() }));
  await probe.goto(url.href, { waitUntil: "networkidle" });
  await probe.getByRole("heading", { name: "My Work", exact: true }).waitFor();
  const control = probe.getByRole("button", { name: "Open local recovery" });
  const renderedCount = await control.count();
  const expectedCount = scenario.recoveryControl ? 1 : 0;
  if (renderedCount !== expectedCount) {
    throw new Error(`${scenario.id} rendered ${renderedCount} recovery controls; expected ${expectedCount}.`);
  }

  const initialUrl = probe.url();
  if (scenario.restricted) {
    if (await probe.locator('[data-permission-result="restricted"]').count() !== 1) {
      throw new Error(`${scenario.id} did not retain its fixture-owned restricted permission result.`);
    }
    if (protectedFixtureCopy.test(await probe.locator("body").innerText())) {
      throw new Error(`${scenario.id} exposed protected fixture copy during recovery-control proof.`);
    }
    if (probe.url() !== initialUrl) throw new Error(`${scenario.id} changed URL while recovery remained unavailable.`);
  } else if (scenario.recoveryControl) {
    await control.click();
    await probe.locator('[data-n0-state="recovery"][data-permission-result="permitted"]').waitFor();
    await probe.getByRole("heading", { name: "Review intake summary" }).waitFor();
  }

  const externalRequests = requests.filter((request) => new URL(request.url).origin !== origin);
  const mutationRequests = requests.filter((request) => !["GET", "HEAD"].includes(request.method));
  if (externalRequests.length || mutationRequests.length) {
    throw new Error(`${scenario.id} recovery proof emitted an external or mutation request.`);
  }
  await probe.close();
  return {
    expected_control_count: expectedCount,
    rendered_control_count: renderedCount,
    executed: scenario.recoveryControl === true,
    restricted_fail_closed: scenario.restricted === true,
    external_requests: externalRequests.length,
    mutation_requests: mutationRequests.length
  };
}

async function measureAdaptation(page, scenario) {
  const result = { direction: null, zoom_reflow: null, focus_geometry: null };
  if (scenario.direction === "rtl") {
    const direction = await page.evaluate(() => document.documentElement.dir);
    const rowTextAlign = await page.locator("#work-row-work-alpha").evaluate((node) => getComputedStyle(node).textAlign);
    const queue = await page.locator(".neutral-work__queue").boundingBox();
    const detail = await page.locator(".neutral-work__detail").boundingBox();
    if (direction !== "rtl" || rowTextAlign !== "start" || !queue || !detail || queue.x <= detail.x) {
      throw new Error("RTL direction, logical text alignment, or queue/detail geometry was not preserved.");
    }
    result.direction = {
      ok: true,
      html_dir: direction,
      row_text_align: rowTextAlign,
      queue_x: queue.x,
      detail_x: detail.x
    };
  }

  if (scenario.zoomPercent) {
    const expectedCssWidth = scenario.nominalWidth / (scenario.zoomPercent / 100);
    const reflow = await page.evaluate(() => ({
      inner_width: window.innerWidth,
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
    if (reflow.inner_width !== expectedCssWidth || reflow.client_width !== expectedCssWidth) {
      throw new Error(`200% CSS-pixel reflow expected ${expectedCssWidth}px but received ${reflow.inner_width}/${reflow.client_width}px.`);
    }
    if (reflow.device_pixel_ratio !== 1 || reflow.horizontal_overflow || reflow.clipped_controls.length) {
      throw new Error("200% CSS-pixel reflow used DPR substitution, overflowed, or clipped a control.");
    }
    result.zoom_reflow = {
      ok: true,
      method: "equivalent_css_pixel_reflow",
      nominal_browser_width_css_px: scenario.nominalWidth,
      effective_layout_width_css_px: reflow.inner_width,
      zoom_percent: scenario.zoomPercent,
      device_pixel_ratio: reflow.device_pixel_ratio,
      dpr_substitution: false,
      horizontal_overflow: reflow.horizontal_overflow,
      clipped_controls: reflow.clipped_controls
    };
  }

  if (scenario.direction === "rtl" || scenario.zoomPercent) {
    const row = page.locator("#work-row-work-alpha");
    await row.click();
    const detail = page.locator("#neutral-work-detail");
    await detail.waitFor();
    if (await detail.evaluate((node) => node !== document.activeElement)) {
      throw new Error(`${scenario.id} did not preserve detail focus through its adaptation geometry.`);
    }
    const detailRect = await page.locator(".neutral-work__detail").boundingBox();
    if (!detailRect || detailRect.x < -1 || detailRect.x + detailRect.width > scenario.width + 1) {
      throw new Error(`${scenario.id} detail geometry exceeded its CSS viewport.`);
    }
    await page.getByRole("button", { name: "Back to queue" }).click();
    await page.waitForFunction(() => !new URL(window.location.href).searchParams.has("item"));
    await page.waitForTimeout(50);
    if (await row.evaluate((node) => node !== document.activeElement)) {
      throw new Error(`${scenario.id} did not restore queue focus through its adaptation geometry.`);
    }
    result.focus_geometry = { ok: true, detail_within_css_viewport: true, queue_focus_restored: true };
  }
  return result;
}

async function main() {
  const started = Date.now();
  rmSync(evidenceRoot, { recursive: true, force: true });
  mkdirSync(screenshotsRoot, { recursive: true });
  const port = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const pagePath = "/pds-n0-neutral-my-work.html";
  const server = spawn(
    process.platform === "win32" ? "npm.cmd" : "npm",
    ["run", "dev", "--", "--port", String(port), "--strictPort"],
    { cwd: catalogRoot, stdio: ["ignore", "pipe", "pipe"] }
  );
  let serverStdout = "";
  let serverStderr = "";
  server.stdout.on("data", (chunk) => { serverStdout += chunk; });
  server.stderr.on("data", (chunk) => { serverStderr += chunk; });

  const git = await run("git", ["rev-parse", "HEAD"]);
  const evidence = {
    command: "node scripts/tests/pds-n0-neutral-my-work.browser.mjs",
    ok: false,
    claim_boundary: claimBoundary,
    snapshot_identity: snapshotIdentity,
    generated_at: new Date().toISOString(),
    source_sha: git.stdout.trim(),
    source_hashes: Object.fromEntries(sourcePaths.map((path) => [path, sha256(readFileSync(join(repoRoot, path)))])),
    environment: {
      os: `${platform()} ${release()}`,
      playwright: null,
      chromium: null,
      device_scale_factor_policy: "fixed_at_1_no_dpr_zoom_substitution"
    },
    server: { origin, stdout: "", stderr: "" },
    scenario_count: scenarios.length,
    scenarios: [],
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
      page.on("request", (request) => requests.push({ url: request.url(), type: request.resourceType(), method: request.method() }));
      page.on("console", (message) => { if (message.type() === "error") consoleErrors.push(message.text()); });
      page.on("pageerror", (error) => pageErrors.push(error.message));

      const url = new URL(pagePath, origin);
      url.searchParams.set("state", scenario.state);
      url.searchParams.set("theme", scenario.theme);
      url.searchParams.set("grammar", scenario.grammar);
      if (scenario.direction) url.searchParams.set("direction", scenario.direction);
      if (scenario.state === "success") url.searchParams.set("item", "alpha-review");
      await page.goto(url.href, { waitUntil: "networkidle" });
      await page.getByRole("heading", { name: "My Work", exact: true }).waitFor();
      if (await page.locator(`[data-n0-state="${scenario.state}"]`).count() !== 1) throw new Error(`State ${scenario.state} did not mount.`);
      const comprehensionInstrumentation = scenario.comprehension ? await instrumentComprehension(page) : null;
      const interactionChecks = scenario.interactions ? await exerciseInteractions(page) : null;
      const adaptationChecks = await measureAdaptation(page, scenario);
      const recoveryChecks = await exerciseRecoveryContract(context, url, scenario, origin);

      const ids = await page.locator("[id]").evaluateAll((nodes) => nodes.map((node) => node.id));
      const duplicateIds = ids.filter((id, index) => ids.indexOf(id) !== index);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1);
      const bodyText = await page.locator("body").innerText();
      if (scenario.restricted && protectedFixtureCopy.test(bodyText)) {
        throw new Error(`${scenario.state} exposed protected fixture copy.`);
      }
      const axe = await new AxeBuilder({ page }).analyze();
      const seriousOrCritical = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
      const externalRequests = requests.filter((request) => {
        const requestUrl = new URL(request.url);
        return requestUrl.origin !== origin;
      });
      const customFontRequests = requests.filter((request) => /\.(woff2?|ttf|otf)(\?|$)/i.test(request.url));
      const unexpectedFontRequests = customFontRequests.filter((request) => {
        const requestUrl = new URL(request.url);
        return requestUrl.origin !== origin
          || !/(?:InterVariable|GeistMonoVariable)\.woff2$/.test(requestUrl.pathname);
      });
      const mutationRequests = requests.filter((request) => !["GET", "HEAD"].includes(request.method));
      const landmarks = {
        queue: await page.locator("#neutral-work-queue").count(),
        detail: await page.locator("#neutral-work-detail").count(),
        notifications: await page.locator("#neutral-work-notifications").count()
      };
      const scenarioOk = Object.values(landmarks).every((count) => count === 1) &&
        duplicateIds.length === 0 && !overflow && seriousOrCritical.length === 0 &&
        consoleErrors.length === 0 && pageErrors.length === 0 && externalRequests.length === 0 &&
        unexpectedFontRequests.length === 0 && mutationRequests.length === 0;
      const screenshot = join(screenshotsRoot, `${scenario.id}.png`);
      await page.screenshot({ path: screenshot, fullPage: true });
      evidence.scenarios.push({
        ...scenario,
        ok: scenarioOk,
        elapsed_ms: Date.now() - scenarioStarted,
        retries: 0,
        comprehension_instrumentation: comprehensionInstrumentation,
        interaction_checks: interactionChecks,
        adaptation_checks: adaptationChecks,
        recovery_checks: recoveryChecks,
        landmarks,
        duplicate_ids: duplicateIds,
        horizontal_overflow: overflow,
        axe: { total: axe.violations.length, serious_or_critical: seriousOrCritical },
        console_errors: consoleErrors,
        page_errors: pageErrors,
        requests,
        external_requests: externalRequests,
        custom_font_requests: customFontRequests,
        unexpected_font_requests: unexpectedFontRequests,
        mutation_requests: mutationRequests,
        screenshot: relative(repoRoot, screenshot)
      });
      await context.close();
    }

    const unknown = await browser.newPage({ viewport: { width: 900, height: 700 } });
    await unknown.goto(`${origin}${pagePath}?state=unknown&theme=unknown&grammar=unknown&direction=unknown&item=unknown`, { waitUntil: "networkidle" });
    if (await unknown.getByText("Unknown local inputs ignored").count() !== 1) throw new Error("Unknown inputs were not surfaced.");
    if (await unknown.getByRole("heading", { name: "Review intake summary" }).count() !== 0) throw new Error("Unknown deep link selected work.");
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
      restricted_states_fail_closed: evidence.scenarios.filter((scenario) => scenario.restricted).every((scenario) => scenario.recovery_checks.restricted_fail_closed),
      recovery_controls_exercised: evidence.scenarios.filter((scenario) => scenario.recoveryControl).length,
      rtl_scenarios: evidence.scenarios.filter((scenario) => scenario.direction === "rtl").length,
      zoom_reflow_scenarios: evidence.scenarios.filter((scenario) => scenario.zoomPercent === 200).length,
      comprehension_instrumentation: evidence.scenarios.find((scenario) => scenario.comprehension_instrumentation)?.comprehension_instrumentation ?? null,
      elapsed_ms: Date.now() - started
    };
    if (!evidence.ok) throw new Error("One or more browser scenarios failed.");
  } finally {
    await browser?.close();
    server.kill("SIGTERM");
    evidence.server.stdout = serverStdout;
    evidence.server.stderr = serverStderr;
    writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`);
  }

  process.stdout.write(`${JSON.stringify({ ok: evidence.ok, output: relative(repoRoot, outputPath), summary: evidence.summary })}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error.message}\n`);
  process.exitCode = 1;
});
