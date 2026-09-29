#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import {
  createReadStream,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync
} from "node:fs";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const catalogRoot = join(repoRoot, "appfw_ui/pds_health/catalog-app");
const distRoot = join(catalogRoot, "dist");
const manifestPath = join(repoRoot, "appfw_ui/pds_health/reference/catalog.json");
const targetRoot = join(repoRoot, "target/appfw");
const defaultEvidenceRoot = join(targetRoot, "pds-catalog-evidence");
const args = parseArgs(process.argv.slice(2));
const jsonOutput = args.json === true;
const outputPath = args.output ? resolve(args.output) : join(targetRoot, "pds-catalog-evidence.json");
const evidenceRoot = args.artifactRoot ? resolve(args.artifactRoot) : defaultEvidenceRoot;
const screenshotsDir = join(evidenceRoot, "screenshots");
const port = args.port ? Number(args.port) : 0;

const scenarios = [
  {
    id: "desktop-light-home",
    label: "Desktop light catalog landing",
    theme: "light",
    viewport: { width: 1280, height: 900 },
    view: "components",
    query: ""
  },
  {
    id: "desktop-dark-chart-shell",
    label: "Desktop dark chart variations",
    theme: "dark",
    viewport: { width: 1280, height: 900 },
    view: "data-visualization",
    query: "ChartShell",
    expectComponents: ["ChartShell"]
  },
  {
    id: "mobile-light-process",
    label: "Mobile light process controls",
    theme: "light",
    viewport: { width: 390, height: 844 },
    query: "Process",
    expectComponents: ["ProcessStepper", "ProcessProgress"]
  },
  {
    id: "mobile-light-operation-state",
    label: "Mobile light full operation-state vocabulary",
    theme: "light",
    viewport: { width: 390, height: 844 },
    query: "OperationState",
    expectComponents: ["OperationState"]
  },
  {
    id: "desktop-dark-conversation",
    label: "Desktop dark conversation surfaces",
    theme: "dark",
    viewport: { width: 1280, height: 900 },
    query: "Conversation",
    expectComponents: [
      "MessageThread",
      "Message",
      "MessageComposer",
      "StreamingText",
      "ToolCallStatus",
      "EntityRefCard",
      "CitationList",
      "ConfidenceSignal",
      "AgentTimeline",
      "FlowGraphShell"
    ]
  },
  {
    id: "mobile-dark-ambient-ai",
    label: "Mobile dark ambient AI surfaces",
    theme: "dark",
    viewport: { width: 390, height: 844 },
    query: "Ambient AI",
    expectComponents: [
      "GeneratedViewShell",
      "SuggestedAction",
      "RecommendationCard",
      "EvidenceSummary",
      "InsightSummary",
      "FreshnessIndicator",
      "AttentionMarker",
      "AiAttributionAffordance",
      "AssistLevelControl",
      "MemoryChip"
    ]
  },
  {
    id: "desktop-light-apple-timeline-range",
    label: "Desktop light Apple-like timeline range interaction",
    theme: "light",
    visualTheme: "apple-like",
    reducedMotion: "no-preference",
    viewport: { width: 1280, height: 900 },
    view: "data-visualization",
    query: "TimelineRangeSelector",
    expectComponents: ["TimelineRangeSelector"],
    timelineEvidence: "full"
  },
  {
    id: "desktop-dark-apple-timeline-range",
    label: "Desktop dark Apple-like timeline range",
    theme: "dark",
    visualTheme: "apple-like",
    reducedMotion: "reduce",
    viewport: { width: 1280, height: 900 },
    view: "data-visualization",
    query: "TimelineRangeSelector",
    expectComponents: ["TimelineRangeSelector"],
    timelineEvidence: "visual"
  },
  {
    id: "mobile-light-material-timeline-range",
    label: "Mobile light Material-like timeline range",
    theme: "light",
    visualTheme: "material-like",
    reducedMotion: "no-preference",
    viewport: { width: 390, height: 844 },
    view: "data-visualization",
    query: "TimelineRangeSelector",
    expectComponents: ["TimelineRangeSelector"],
    timelineEvidence: "responsive"
  },
  {
    id: "mobile-dark-material-timeline-range",
    label: "Mobile dark Material-like reduced-motion timeline range",
    theme: "dark",
    visualTheme: "material-like",
    reducedMotion: "reduce",
    viewport: { width: 390, height: 844 },
    view: "data-visualization",
    query: "TimelineRangeSelector",
    expectComponents: ["TimelineRangeSelector"],
    timelineEvidence: "responsive"
  }
];

mkdirSync(dirname(outputPath), { recursive: true });
mkdirSync(screenshotsDir, { recursive: true });

let server;
let browser;
const startedAt = new Date().toISOString();
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const manifestComponents = manifest.families.flatMap((family) => family.components);
const reconciledComponents = [
  "ButtonGroup",
  "DatePicker",
  "FloatingActionButton",
  "List",
  "ListItem",
  "SearchBar",
  "TimePicker",
  "ToggleButton",
  "Toolbar"
];
const manifestComponentCounts = countValues(manifestComponents);
const missingReconciledComponents = reconciledComponents.filter(
  (component) => !manifestComponentCounts.has(component)
);
const duplicateManifestComponents = duplicateValues(manifestComponentCounts);
const evidence = {
  command: "pds-catalog-evidence",
  ok: false,
  generated_at: startedAt,
  repo_root: repoRoot,
  catalog_root: relative(catalogRoot),
  manifest: {
    path: relative(manifestPath),
    family_count: manifest.families.length,
    component_count: manifestComponents.length,
    components: manifestComponents,
    reconciled_components: reconciledComponents,
    missing_reconciled_components: missingReconciledComponents,
    duplicate_components: duplicateManifestComponents
  },
  non_claims: {
    visual_acceptance: false,
    product_acceptance: false,
    poc_readiness: false,
    release_readiness: false
  },
  artifacts: {
    output: relative(outputPath),
    screenshots_dir: relative(screenshotsDir)
  },
  build: null,
  server: null,
  scenarios: [],
  checks: []
};

try {
  if (!args.skipBuild) {
    evidence.build = await runCommand("npm", ["run", "build"], catalogRoot);
  } else {
    evidence.build = {
      command: "npm run build",
      skipped: true,
      ok: existsSync(join(distRoot, "index.html"))
    };
  }

  if (missingReconciledComponents.length > 0 || duplicateManifestComponents.length > 0) {
    throw new Error("Catalog manifest has missing or duplicate reconciled components.");
  }

  if (!evidence.build.ok) {
    throw new Error(`Catalog build failed. See build output in ${relative(outputPath)}.`);
  }

  if (!existsSync(join(distRoot, "index.html"))) {
    throw new Error(`Catalog build output is missing: ${relative(join(distRoot, "index.html"))}`);
  }

  server = await startStaticServer(distRoot, port);
  evidence.server = {
    url: server.url,
    dist_root: relative(distRoot)
  };

  const playwrightModule = await importFromKnownRoots("@playwright/test");
  const chromium = playwrightModule.chromium ?? playwrightModule.default?.chromium;
  if (!chromium) {
    throw new Error("Unable to load chromium from @playwright/test.");
  }

  const axeModule = await importFromKnownRoots("@axe-core/playwright");
  const AxeBuilder = axeModule.default ?? axeModule.AxeBuilder;
  if (!AxeBuilder) {
    throw new Error("Unable to load AxeBuilder from @axe-core/playwright.");
  }

  browser = await chromium.launch();

  for (const scenario of scenarios) {
    evidence.scenarios.push(await runScenario({ AxeBuilder, browser, scenario, url: server.url }));
  }

  evidence.checks = summarizeChecks(evidence.scenarios);
  evidence.ok =
    Boolean(evidence.build?.ok)
    && evidence.scenarios.every((scenario) => scenario.ok)
    && evidence.checks.every((check) => check.ok);
} catch (error) {
  evidence.error = error instanceof Error ? error.message : String(error);
} finally {
  if (browser) await browser.close();
  if (server) await new Promise((resolveClose) => server.instance.close(resolveClose));
}

writeFileSync(outputPath, JSON.stringify(evidence, null, 2) + "\n", "utf8");

if (jsonOutput) {
  console.log(JSON.stringify(evidence, null, 2));
} else if (evidence.ok) {
  console.log(`PDS catalog evidence ok: ${relative(outputPath)}`);
} else {
  console.error(`PDS catalog evidence failed: ${relative(outputPath)}`);
  if (evidence.error) console.error(evidence.error);
}

process.exit(evidence.ok ? 0 : 1);

async function runScenario({ AxeBuilder, browser, scenario, url }) {
  const context = await browser.newContext({
    viewport: scenario.viewport,
    colorScheme: scenario.theme,
    reducedMotion: scenario.reducedMotion ?? "reduce"
  });
  const page = await context.newPage();
  const screenshotPath = join(screenshotsDir, `${scenario.id}.png`);

  try {
    // Query scenarios exercise the component search, which lives on the
    // components view; the catalog boots to Overview by default.
    const targetView = scenario.view ?? (scenario.query ? "components" : null);
    const targetUrl = targetView ? `${url}?view=${targetView}` : url;
    await page.goto(targetUrl, { waitUntil: "networkidle" });
    await page.evaluate(({ theme, visualTheme }) => {
      localStorage.setItem("pds-catalog-theme", theme);
      localStorage.setItem(
        "pds-catalog-visual-theme",
        visualTheme ?? "apple-like"
      );
    }, scenario);
    await page.reload({ waitUntil: "networkidle" });

    if (scenario.query) {
      await page.locator(".catalog__search input").fill(scenario.query);
    }

    const timelineEvidence = scenario.timelineEvidence
      ? await exerciseTimelineRange(page, scenario)
      : null;
    const axe = await new AxeBuilder({ page }).analyze();
    await page.screenshot({ path: screenshotPath, fullPage: false });
    const dom = await page.evaluate((expected) => {
      const componentNames = Array.from(document.querySelectorAll(".component-card > .component-card__head .component-card__name > h3"))
        .map((node) => node.textContent?.trim())
        .filter(Boolean);
      const componentNameCounts = componentNames.reduce((counts, component) => {
        counts[component] = (counts[component] ?? 0) + 1;
        return counts;
      }, {});
      const missingManifestComponents = expected.filter(
        (component) => !componentNameCounts[component]
      );
      const unexpectedComponents = componentNames.filter(
        (component) => !expected.includes(component)
      );
      const duplicateComponents = Object.entries(componentNameCounts)
        .filter(([, count]) => count > 1)
        .map(([component, count]) => ({ component, count }));
      const duplicateIds = duplicateElementIds();
      const unnamedInteractive = findUnnamedInteractive();
      const previewOverflow = findPreviewOverflow();
      const escapedPopovers = findEscapedPopovers();
      const horizontalOverflow = Math.max(
        0,
        document.documentElement.scrollWidth - document.documentElement.clientWidth
      );
      const adaptiveProcess = document.querySelector(
        ".pds-process-stepper-adaptive[data-compact-presentation='segments']:not([data-interactive='true'])"
      );
      const adaptiveProcessFull = adaptiveProcess?.querySelector(
        ".pds-process-stepper-adaptive__full"
      );
      const adaptiveProcessCompact = adaptiveProcess?.querySelector(
        ".pds-process-stepper-adaptive__compact"
      );
      const adaptiveProcessSegments = Array.from(
        adaptiveProcessCompact?.querySelectorAll(
          ".pds-process-progress__segments > span"
        ) ?? []
      );

      return {
        title: document.title,
        rootTheme: document.documentElement.dataset.theme || "system",
        rootVisualTheme:
          document.documentElement.dataset.visualTheme || "apple-like",
        familyCount: document.querySelectorAll(".family").length,
        componentCount: componentNames.length,
        componentNames,
        missingManifestComponents,
        unexpectedComponents,
        duplicateComponents,
        expectedComponentsPresent: expected.every((component) => componentNames.includes(component)),
        chartShellVariationCount: document.querySelector("#component-ChartShell")?.querySelectorAll(".pds-chart-shell").length ?? 0,
        processStepperCount: document.querySelectorAll(".pds-process-stepper").length,
        processProgressCount: document.querySelectorAll(".pds-process-progress").length,
        adaptiveProcess: {
          count: adaptiveProcess ? 1 : 0,
          fullPosition: adaptiveProcessFull
            ? getComputedStyle(adaptiveProcessFull).position
            : null,
          fullWidth: adaptiveProcessFull
            ? getComputedStyle(adaptiveProcessFull).width
            : null,
          compactDisplay: adaptiveProcessCompact
            ? getComputedStyle(adaptiveProcessCompact).display
            : null,
          segmentCount: adaptiveProcessSegments.length,
          segmentStatuses: adaptiveProcessSegments.map((segment) =>
            segment.getAttribute("data-status")
          ),
          horizontalOverflow
        },
        operation: {
          count: document.querySelectorAll(".pds-operation-state").length,
          states: Array.from(document.querySelectorAll(".pds-operation-state")).map((node) => node.getAttribute("data-state")),
          politeCount: document.querySelectorAll(".pds-operation-state[role='status'][aria-live='polite']").length,
          assertiveCount: document.querySelectorAll(".pds-operation-state[role='alert'][aria-live='assertive']").length,
          busyCount: document.querySelectorAll(".pds-operation-state[aria-busy='true']").length,
          requestMetadataCount: document.querySelectorAll(".pds-operation-state__request-item").length,
          actionCount: document.querySelectorAll(".pds-operation-state__action button").length,
          pendingAnimation: getComputedStyle(document.querySelector(".pds-operation-state[data-state='pending'] .pds-operation-state__indicator") ?? document.body).animationName,
          keyboardActionFocus: false
        },
        conversation: {
          threadRegionCount: document.querySelectorAll(".pds-message-thread[aria-label]").length,
          namedMessageListCount: document.querySelectorAll(".pds-message-thread__list[aria-label='Messages']").length,
          streamingStatusCount: document.querySelectorAll(".pds-streaming-text[role='status'][aria-live='polite']").length,
          toolStatusCount: document.querySelectorAll(".pds-tool-call-status[role='status']").length,
          assertiveToolStatusCount: document.querySelectorAll(".pds-tool-call-status[role='status'][aria-live='assertive']").length,
          citationRegionCount: document.querySelectorAll(".pds-citation-list[aria-label]").length,
          confidenceStatusCount: document.querySelectorAll(".pds-confidence-signal[role='status']").length,
          timelineRegionCount: document.querySelectorAll(".pds-agent-timeline[aria-label]").length,
          flowGraphImageCount: document.querySelectorAll(".pds-flow-graph-shell__viewport[role='img'][aria-label]").length,
          entityRefRegionCount: document.querySelectorAll(".pds-entity-ref-card[aria-label]").length
        },
        ambient: {
          generatedViewCount: document.querySelectorAll(".pds-generated-view-shell[data-grounding]").length,
          partialGeneratedViewCount: document.querySelectorAll(".pds-generated-view-shell[data-grounding='partial']").length,
          suggestedActionCount: document.querySelectorAll(".pds-suggested-action[data-risk][data-grounding]").length,
          previewGatedSuggestionCount: document.querySelectorAll(".pds-suggested-action .pds-intent-preview").length,
          recommendationCount: document.querySelectorAll(".pds-recommendation-card[data-rank]").length,
          evidenceSummaryCount: document.querySelectorAll(".pds-evidence-summary[aria-label]").length,
          freshnessStatusCount: document.querySelectorAll(".pds-freshness-indicator[role='status']").length,
          attributionStatusCount: document.querySelectorAll(".pds-ai-attribution[role='status'][aria-live='polite']").length,
          attentionStatusCount: document.querySelectorAll(".pds-attention-marker[role='status']").length,
          attentionAlertCount: document.querySelectorAll(".pds-attention-marker[role='alert'][aria-live='assertive']").length,
          assistLevelControlCount: document.querySelectorAll(".pds-assist-level-control").length,
          assistLevelRadioCount: document.querySelectorAll(".pds-assist-level-control input[type='radio']").length,
          memoryChipGroupCount: document.querySelectorAll(".pds-memory-chip[role='group']").length,
          memoryChipActionCount: document.querySelectorAll(".pds-memory-chip button").length
        },
        horizontalOverflow,
        duplicateIds,
        unnamedInteractive,
        previewOverflow,
        escapedPopovers
      };

      function duplicateElementIds() {
        const counts = new Map();
        for (const element of Array.from(document.querySelectorAll("[id]"))) {
          const id = element.id;
          counts.set(id, (counts.get(id) ?? 0) + 1);
        }
        return Array.from(counts.entries())
          .filter(([, count]) => count > 1)
          .map(([id, count]) => ({ id, count }));
      }

      function findUnnamedInteractive() {
        return Array.from(document.querySelectorAll("button, a, input, select, textarea, [role='button'], [role='tab']"))
          .filter((element) => isVisible(element) && !accessibleName(element))
          .slice(0, 20)
          .map((element) => describeElement(element));
      }

      function findPreviewOverflow() {
        const offenders = [];
        for (const preview of Array.from(document.querySelectorAll(".component-card__preview"))) {
          const previewRect = preview.getBoundingClientRect();
          for (const element of Array.from(preview.querySelectorAll("*"))) {
            const rect = element.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) continue;
            if (rect.left < previewRect.left - 1 || rect.right > previewRect.right + 1) {
              if (isClippedOrScrollableOverflow(element, preview, rect, previewRect)) continue;
              offenders.push({
                card: preview.closest(".component-card")?.querySelector("h3")?.textContent?.trim() ?? null,
                element: describeElement(element),
                left_delta: Math.round(rect.left - previewRect.left),
                right_delta: Math.round(rect.right - previewRect.right)
              });
              break;
            }
          }
        }
        return offenders.slice(0, 20);
      }

      function isClippedOrScrollableOverflow(element, preview, rect, previewRect) {
        let ancestor = element.parentElement;
        while (ancestor && ancestor !== preview) {
          const style = getComputedStyle(ancestor);
          const clipsInline =
            style.overflowX === "auto"
            || style.overflowX === "scroll"
            || style.overflowX === "hidden"
            || style.overflowX === "clip";

          if (clipsInline) {
            const ancestorRect = ancestor.getBoundingClientRect();
            const ancestorInsidePreview =
              ancestorRect.left >= previewRect.left - 1
              && ancestorRect.right <= previewRect.right + 1;
            const childOutsideAncestor = rect.left < ancestorRect.left - 1 || rect.right > ancestorRect.right + 1;
            if (ancestorInsidePreview && childOutsideAncestor) return true;
          }

          ancestor = ancestor.parentElement;
        }
        return false;
      }

      function findEscapedPopovers() {
        return Array.from(document.querySelectorAll(".pds-data-grid-control-popover")).filter((popover) => {
          const rect = popover.getBoundingClientRect();
          const visibleInViewport =
            rect.bottom > 0 && rect.top < window.innerHeight && rect.right > 0 && rect.left < window.innerWidth;
          const card = popover.closest(".component-card");
          const cardRect = card?.getBoundingClientRect();
          const contained = Boolean(
            cardRect
            && rect.left >= cardRect.left - 1
            && rect.right <= cardRect.right + 1
            && rect.top >= cardRect.top - 1
            && rect.bottom <= cardRect.bottom + 1
          );
          return visibleInViewport && !contained;
        }).map((element) => describeElement(element));
      }

      function accessibleName(element) {
        const ariaLabel = element.getAttribute("aria-label");
        if (ariaLabel?.trim()) return ariaLabel.trim();
        const labelledBy = element.getAttribute("aria-labelledby");
        if (labelledBy) {
          const value = labelledBy
            .split(/\s+/)
            .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
            .filter(Boolean)
            .join(" ")
            .trim();
          if (value) return value;
        }
        const labels = element.labels ? Array.from(element.labels) : [];
        const labelText = labels.map((label) => label.textContent?.trim() ?? "").filter(Boolean).join(" ").trim();
        if (labelText) return labelText;
        const text = element.textContent?.trim();
        if (text) return text;
        const title = element.getAttribute("title");
        if (title?.trim()) return title.trim();
        return "";
      }

      function isVisible(element) {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none";
      }

      function describeElement(element) {
        return {
          tag: element.tagName.toLowerCase(),
          type: element.getAttribute("type"),
          role: element.getAttribute("role"),
          class: element.getAttribute("class"),
          text: element.textContent?.trim().slice(0, 80) ?? ""
        };
      }
    }, scenario.expectComponents ?? manifestComponents);

    if (!scenario.query && scenario.view === "components") {
      // The component inventory spans two views since the catalog split
      // analytics onto the data-visualization view; union both before
      // auditing manifest completeness.
      await page.goto(`${url}?view=data-visualization`, { waitUntil: "networkidle" });
      const extraNames = await page.evaluate(() =>
        Array.from(document.querySelectorAll(".component-card > .component-card__head .component-card__name > h3"))
          .map((node) => node.textContent?.trim())
          .filter(Boolean));
      const counts = {};
      for (const name of [...dom.componentNames, ...extraNames]) counts[name] = (counts[name] ?? 0) + 1;
      dom.componentNames = Object.keys(counts);
      dom.componentCount = dom.componentNames.length;
      dom.missingManifestComponents = manifestComponents.filter((c) => !counts[c]);
      dom.unexpectedComponents = Object.keys(counts).filter((c) => !manifestComponents.includes(c));
      dom.duplicateComponents = Object.entries(counts)
        .filter(([, n]) => n > 1)
        .map(([component, count]) => ({ component, count }));
    }

    if (scenario.id === "mobile-light-operation-state") {
      const action = page.locator("#component-OperationState .pds-operation-state__action button").first();
      await page.locator("body").click({ position: { x: 1, y: 1 } });
      for (let index = 0; index < 40; index += 1) {
        await page.keyboard.press("Tab");
        if (await action.evaluate((element) => document.activeElement === element)) break;
      }
      dom.operation.keyboardActionFocus = await action.evaluate((element) => document.activeElement === element);
    }

    const seriousViolations = axe.violations.filter((violation) =>
      violation.impact === "critical" || violation.impact === "serious"
    );
    const scenarioChecks = [
      {
        id: "axe-no-serious-violations",
        ok: seriousViolations.length === 0,
        count: seriousViolations.length
      },
      {
        id: "no-horizontal-overflow",
        ok: dom.horizontalOverflow <= 1,
        pixels: dom.horizontalOverflow
      },
      {
        id: "no-preview-overflow",
        ok: dom.previewOverflow.length === 0,
        count: dom.previewOverflow.length
      },
      {
        id: "no-escaped-popovers",
        ok: dom.escapedPopovers.length === 0,
        count: dom.escapedPopovers.length
      },
      {
        id: "no-duplicate-ids",
        ok: dom.duplicateIds.length === 0,
        count: dom.duplicateIds.length
      },
      {
        id: "named-interactive-controls",
        ok: dom.unnamedInteractive.length === 0,
        count: dom.unnamedInteractive.length
      },
      {
        id: "expected-components-present",
        ok: scenario.expectComponents
          ? dom.expectedComponentsPresent
          : dom.missingManifestComponents.length === 0
            && dom.unexpectedComponents.length === 0
            && dom.duplicateComponents.length === 0,
        expected: scenario.expectComponents ?? manifestComponents.length,
        actual: scenario.expectComponents ? dom.componentNames : dom.componentCount,
        missing: scenario.expectComponents ? [] : dom.missingManifestComponents,
        unexpected: scenario.expectComponents ? [] : dom.unexpectedComponents,
        duplicate: scenario.expectComponents ? [] : dom.duplicateComponents
      }
    ];

    if (timelineEvidence) {
      scenarioChecks.push(...timelineEvidence.checks);
    }

    if (scenario.id === "desktop-dark-chart-shell") {
      scenarioChecks.push({
        id: "chart-variations-rendered",
        ok: dom.chartShellVariationCount >= 5,
        expected_minimum: 5,
        actual: dom.chartShellVariationCount
      });
    }

    if (scenario.id === "mobile-light-process") {
      scenarioChecks.push(
        {
          id: "process-stepper-rendered",
          ok: dom.processStepperCount >= 1,
          expected_minimum: 1,
          actual: dom.processStepperCount
        },
        {
          id: "process-progress-rendered",
          ok: dom.processProgressCount >= 1,
          expected_minimum: 1,
          actual: dom.processProgressCount
        },
        {
          id: "process-stepper-compact-presentation",
          ok:
            dom.adaptiveProcess.count === 1
            && dom.adaptiveProcess.fullPosition === "absolute"
            && dom.adaptiveProcess.fullWidth === "1px"
            && dom.adaptiveProcess.compactDisplay === "grid"
            && dom.adaptiveProcess.segmentCount === 4,
          expected: {
            count: 1,
            fullPosition: "absolute",
            fullWidth: "1px",
            compactDisplay: "grid",
            segmentCount: 4
          },
          actual: dom.adaptiveProcess
        },
        {
          id: "process-stepper-compact-statuses",
          ok:
            dom.adaptiveProcess.segmentStatuses.filter(
              (status) => status === "current"
            ).length === 1
            && dom.adaptiveProcess.segmentStatuses.includes("complete")
            && dom.adaptiveProcess.segmentStatuses.includes("upcoming"),
          expected: ["complete", "current", "upcoming"],
          actual: dom.adaptiveProcess.segmentStatuses
        },
        {
          id: "process-stepper-no-horizontal-overflow",
          ok: dom.adaptiveProcess.horizontalOverflow === 0,
          expected: 0,
          actual: dom.adaptiveProcess.horizontalOverflow
        }
      );
    }

    if (scenario.id === "mobile-light-operation-state") {
      scenarioChecks.push(
        {
          id: "operation-state-full-vocabulary",
          ok: ["idle", "pending", "success", "error", "denied", "cancelled"].every((state) => dom.operation.states.includes(state)),
          expected: ["idle", "pending", "success", "error", "denied", "cancelled"],
          actual: dom.operation.states
        },
        {
          id: "operation-state-live-regions",
          ok: dom.operation.politeCount >= 4 && dom.operation.assertiveCount >= 2 && dom.operation.busyCount >= 1,
          expected_minimum: { polite: 4, assertive: 2, busy: 1 },
          actual: { polite: dom.operation.politeCount, assertive: dom.operation.assertiveCount, busy: dom.operation.busyCount }
        },
        {
          id: "operation-state-request-correlation",
          ok: dom.operation.requestMetadataCount >= 2,
          expected_minimum: 2,
          actual: dom.operation.requestMetadataCount
        },
        {
          id: "operation-state-keyboard-focus",
          ok: dom.operation.actionCount >= 1 && dom.operation.keyboardActionFocus,
          expected: { action_count_minimum: 1, keyboard_action_focus: true },
          actual: { action_count: dom.operation.actionCount, keyboard_action_focus: dom.operation.keyboardActionFocus }
        },
        {
          id: "operation-state-reduced-motion",
          ok: dom.operation.pendingAnimation === "none",
          expected: "none",
          actual: dom.operation.pendingAnimation
        }
      );
    }

    if (scenario.id === "desktop-dark-conversation") {
      scenarioChecks.push(
        {
          id: "conversation-thread-region",
          ok: dom.conversation.threadRegionCount >= 1,
          expected_minimum: 1,
          actual: dom.conversation.threadRegionCount
        },
        {
          id: "conversation-message-list-named",
          ok: dom.conversation.namedMessageListCount >= 1,
          expected_minimum: 1,
          actual: dom.conversation.namedMessageListCount
        },
        {
          id: "conversation-streaming-status-live-region",
          ok: dom.conversation.streamingStatusCount >= 1,
          expected_minimum: 1,
          actual: dom.conversation.streamingStatusCount
        },
        {
          id: "conversation-tool-status-live-region",
          ok: dom.conversation.toolStatusCount >= 1 && dom.conversation.assertiveToolStatusCount >= 1,
          expected_minimum: { status: 1, assertive: 1 },
          actual: {
            status: dom.conversation.toolStatusCount,
            assertive: dom.conversation.assertiveToolStatusCount
          }
        },
        {
          id: "conversation-grounding-surfaces",
          ok:
            dom.conversation.entityRefRegionCount >= 1
            && dom.conversation.citationRegionCount >= 1
            && dom.conversation.confidenceStatusCount >= 1,
          expected_minimum: { entity_refs: 1, citations: 1, confidence: 1 },
          actual: {
            entity_refs: dom.conversation.entityRefRegionCount,
            citations: dom.conversation.citationRegionCount,
            confidence: dom.conversation.confidenceStatusCount
          }
        },
        {
          id: "conversation-timeline-and-flowgraph",
          ok: dom.conversation.timelineRegionCount >= 1 && dom.conversation.flowGraphImageCount >= 1,
          expected_minimum: { timelines: 1, flow_graphs: 1 },
          actual: {
            timelines: dom.conversation.timelineRegionCount,
            flow_graphs: dom.conversation.flowGraphImageCount
          }
        }
      );
    }

    if (scenario.id === "mobile-dark-ambient-ai") {
      scenarioChecks.push(
        {
          id: "ambient-generated-view-grounding",
          ok: dom.ambient.generatedViewCount >= 1 && dom.ambient.partialGeneratedViewCount >= 1,
          expected_minimum: { generated_views: 1, partial_views: 1 },
          actual: {
            generated_views: dom.ambient.generatedViewCount,
            partial_views: dom.ambient.partialGeneratedViewCount
          }
        },
        {
          id: "ambient-preview-gated-suggestions",
          ok: dom.ambient.suggestedActionCount >= 1 && dom.ambient.previewGatedSuggestionCount >= 1,
          expected_minimum: { suggested_actions: 1, preview_gated: 1 },
          actual: {
            suggested_actions: dom.ambient.suggestedActionCount,
            preview_gated: dom.ambient.previewGatedSuggestionCount
          }
        },
        {
          id: "ambient-evidence-and-attribution",
          ok:
            dom.ambient.evidenceSummaryCount >= 1
            && dom.ambient.freshnessStatusCount >= 1
            && dom.ambient.attributionStatusCount >= 1,
          expected_minimum: { evidence_summaries: 1, freshness: 1, attribution: 1 },
          actual: {
            evidence_summaries: dom.ambient.evidenceSummaryCount,
            freshness: dom.ambient.freshnessStatusCount,
            attribution: dom.ambient.attributionStatusCount
          }
        },
        {
          id: "ambient-attention-semantics",
          ok: dom.ambient.attentionStatusCount >= 1 && dom.ambient.attentionAlertCount >= 1,
          expected_minimum: { status: 1, alert: 1 },
          actual: {
            status: dom.ambient.attentionStatusCount,
            alert: dom.ambient.attentionAlertCount
          }
        },
        {
          id: "ambient-recommendation-ranking",
          ok: dom.ambient.recommendationCount >= 1,
          expected_minimum: 1,
          actual: dom.ambient.recommendationCount
        },
        {
          id: "ambient-autonomy-and-memory-controls",
          ok:
            dom.ambient.assistLevelControlCount >= 1
            && dom.ambient.assistLevelRadioCount >= 3
            && dom.ambient.memoryChipGroupCount >= 1
            && dom.ambient.memoryChipActionCount >= 2,
          expected_minimum: {
            assist_level_controls: 1,
            assist_level_radios: 3,
            memory_chip_groups: 1,
            memory_chip_actions: 2
          },
          actual: {
            assist_level_controls: dom.ambient.assistLevelControlCount,
            assist_level_radios: dom.ambient.assistLevelRadioCount,
            memory_chip_groups: dom.ambient.memoryChipGroupCount,
            memory_chip_actions: dom.ambient.memoryChipActionCount
          }
        }
      );
    }

    return {
      id: scenario.id,
      label: scenario.label,
      ok: scenarioChecks.every((check) => check.ok),
      theme: scenario.theme,
      visual_theme: scenario.visualTheme ?? "apple-like",
      reduced_motion: scenario.reducedMotion ?? "reduce",
      viewport: scenario.viewport,
      query: scenario.query,
      screenshot: relative(screenshotPath),
      checks: scenarioChecks,
      axe: {
        violation_count: axe.violations.length,
        serious_or_critical_count: seriousViolations.length,
        violations: summarizeViolations(seriousViolations)
      },
      timeline: timelineEvidence?.details ?? null,
      dom
    };
  } finally {
    await context.close();
  }
}

async function exerciseTimelineRange(page, scenario) {
  const card = page.locator("#component-TimelineRangeSelector");
  const root = card.locator(".pds-timeline-range");
  const track = root.locator(".pds-timeline-range__track");
  const selection = root.locator(".pds-timeline-range__selection");
  const start = root.getByRole("slider", {
    name: "Delivery history window start"
  });
  const position = root.getByRole("slider", {
    name: "Delivery history window position"
  });
  const end = root.getByRole("slider", {
    name: "Delivery history window end"
  });

  await root.waitFor({ state: "visible" });
  await root.scrollIntoViewIfNeeded();

  const checks = [];
  const details = {
    mode: scenario.timelineEvidence,
    initial: await timelineState({ end, position, start }),
    pointer: {},
    keyboard: {},
    geometry: null,
    focus: null,
    aria: null,
    motion: null
  };

  const aria = await root.evaluate((element) => {
    const sliders = [...element.querySelectorAll("[role='slider']")];
    const describedByTargets = sliders.map((slider) => {
      const describedBy = slider.getAttribute("aria-describedby");
      return {
        describedBy,
        description:
          describedBy
            ?.split(/\s+/)
            .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
            .filter(Boolean)
            .join(" ") ?? ""
      };
    });
    const liveOutput = element.querySelector("output");
    return {
      groupLabel: element.getAttribute("aria-label"),
      sliderCount: sliders.length,
      sliders: sliders.map((slider) => ({
        label: slider.getAttribute("aria-label"),
        valueMin: slider.getAttribute("aria-valuemin"),
        valueMax: slider.getAttribute("aria-valuemax"),
        valueNow: slider.getAttribute("aria-valuenow"),
        valueText: slider.getAttribute("aria-valuetext")
      })),
      describedByTargets,
      liveOutput: {
        ariaLive: liveOutput?.getAttribute("aria-live"),
        ariaAtomic: liveOutput?.getAttribute("aria-atomic"),
        text: liveOutput?.textContent?.trim()
      }
    };
  });
  details.aria = aria;
  checks.push({
    id: "timeline-range-aria-contract",
    ok:
      aria.groupLabel === "Delivery history window"
      && aria.sliderCount === 3
      && aria.sliders.every(
        (slider) =>
          Boolean(slider.label)
          && slider.valueMin !== null
          && slider.valueMax !== null
          && slider.valueNow !== null
          && Boolean(slider.valueText)
      )
      && aria.describedByTargets.every(
        ({ describedBy, description }) =>
          Boolean(describedBy) && Boolean(description)
      )
      && aria.liveOutput.ariaLive === "polite"
      && aria.liveOutput.ariaAtomic === "true"
      && Boolean(aria.liveOutput.text),
    actual: aria
  });

  if (scenario.timelineEvidence === "full") {
    await position.scrollIntoViewIfNeeded();
    const trackBox = await track.boundingBox();
    const positionBox = await position.boundingBox();
    if (!trackBox || !positionBox) {
      throw new Error("Timeline range pointer targets are not measurable.");
    }

    await dragPointer(
      page,
      positionBox.x + positionBox.width / 2,
      positionBox.y + positionBox.height / 2,
      positionBox.x + positionBox.width / 2 + (trackBox.width * 2) / 12,
      positionBox.y + positionBox.height / 2
    );
    details.pointer.centerMove = await timelineState({ end, position, start });
    checks.push({
      id: "timeline-range-center-pointer-drag",
      ok:
        details.pointer.centerMove.start === 8
        && details.pointer.centerMove.position === 8
        && details.pointer.centerMove.end === 10,
      expected: { start: 8, position: 8, end: 10 },
      actual: details.pointer.centerMove
    });

    await start.scrollIntoViewIfNeeded();
    const startBox = await start.boundingBox();
    if (!startBox) {
      throw new Error("Timeline range start handle is not measurable.");
    }
    await dragPointer(
      page,
      startBox.x + startBox.width / 2,
      startBox.y + startBox.height / 2,
      startBox.x + startBox.width / 2 - (trackBox.width * 3) / 12,
      startBox.y + startBox.height / 2
    );
    details.pointer.startResize = await timelineState({ end, position, start });
    checks.push({
      id: "timeline-range-start-handle-pointer-resize",
      ok:
        details.pointer.startResize.start === 5
        && details.pointer.startResize.position === 5
        && details.pointer.startResize.end === 10,
      expected: { start: 5, position: 5, end: 10, snappedWidth: 6 },
      actual: {
        ...details.pointer.startResize,
        snappedWidth:
          details.pointer.startResize.end
          - details.pointer.startResize.start
          + 1
      }
    });

    await end.scrollIntoViewIfNeeded();
    const endBox = await end.boundingBox();
    if (!endBox) {
      throw new Error("Timeline range end handle is not measurable.");
    }
    await dragPointer(
      page,
      endBox.x + endBox.width / 2,
      endBox.y + endBox.height / 2,
      endBox.x + endBox.width / 2 - (trackBox.width * 3) / 12,
      endBox.y + endBox.height / 2
    );
    details.pointer.endResize = await timelineState({ end, position, start });
    checks.push({
      id: "timeline-range-end-handle-pointer-resize",
      ok:
        details.pointer.endResize.start === 5
        && details.pointer.endResize.position === 5
        && details.pointer.endResize.end === 7,
      expected: { start: 5, position: 5, end: 7, snappedWidth: 3 },
      actual: {
        ...details.pointer.endResize,
        snappedWidth:
          details.pointer.endResize.end
          - details.pointer.endResize.start
          + 1
      }
    });

    await position.focus();
    await position.press("Home");
    const homeState = await timelineState({ end, position, start });
    await position.press("ArrowLeft");
    const startClamp = await timelineState({ end, position, start });
    await position.press("End");
    const endState = await timelineState({ end, position, start });
    await position.press("ArrowRight");
    const endClamp = await timelineState({ end, position, start });
    details.keyboard.position = {
      home: homeState,
      startClamp,
      end: endState,
      endClamp
    };
    checks.push({
      id: "timeline-range-position-keyboard-home-end-clamp",
      ok:
        homeState.position === 0
        && startClamp.position === 0
        && endState.position === 9
        && endClamp.position === 9,
      expected: { home: 0, startClamp: 0, end: 9, endClamp: 9 },
      actual: {
        home: homeState.position,
        startClamp: startClamp.position,
        end: endState.position,
        endClamp: endClamp.position
      }
    });

    await position.press("Home");
    await start.focus();
    await start.press("End");
    const startNarrow = await timelineState({ end, position, start });
    await start.press("Home");
    const startWide = await timelineState({ end, position, start });
    details.keyboard.startHandle = {
      end: startNarrow,
      home: startWide
    };
    checks.push({
      id: "timeline-range-start-handle-keyboard-snap",
      ok:
        startNarrow.start === 2
        && startNarrow.end === 2
        && startWide.start === 0
        && startWide.end === 2,
      expected: {
        endKey: { start: 2, end: 2, snappedWidth: 1 },
        homeKey: { start: 0, end: 2, snappedWidth: 3 }
      },
      actual: {
        endKey: startNarrow,
        homeKey: startWide
      }
    });

    await end.focus();
    await end.press("End");
    const endWide = await timelineState({ end, position, start });
    await end.press("Home");
    const endNarrow = await timelineState({ end, position, start });
    details.keyboard.endHandle = {
      end: endWide,
      home: endNarrow
    };
    checks.push({
      id: "timeline-range-end-handle-keyboard-snap",
      ok:
        endWide.start === 0
        && endWide.end === 11
        && endNarrow.start === 0
        && endNarrow.end === 0,
      expected: {
        endKey: { start: 0, end: 11, snappedWidth: 12 },
        homeKey: { start: 0, end: 0, snappedWidth: 1 }
      },
      actual: {
        endKey: endWide,
        homeKey: endNarrow
      }
    });

    const focus = {};
    for (const [name, control] of [
      ["start", start],
      ["position", position],
      ["end", end]
    ]) {
      await control.focus();
      await control.press("Home");
      focus[name] = await control.evaluate((element) => {
        const style = getComputedStyle(element);
        return {
          active: document.activeElement === element,
          focusVisible: element.matches(":focus-visible"),
          outlineStyle: style.outlineStyle,
          outlineWidth: style.outlineWidth,
          outlineColor: style.outlineColor
        };
      });
    }
    details.focus = focus;
    checks.push({
      id: "timeline-range-visible-focus",
      ok: Object.values(focus).every(
        (value) =>
          value.active
          && value.focusVisible
          && value.outlineStyle !== "none"
          && Number.parseFloat(value.outlineWidth) >= 3
      ),
      actual: focus
    });
  }

  const geometry = await root.evaluate((element) => {
    const previewElement = element.closest(".component-card__preview");
    const viewportElement = element.querySelector(
      ".pds-timeline-range__viewport"
    );
    const trackElement = element.querySelector(".pds-timeline-range__track");
    const selectionElement = element.querySelector(
      ".pds-timeline-range__selection"
    );
    const rootRect = element.getBoundingClientRect();
    const previewRect = previewElement?.getBoundingClientRect();
    const viewportRect = viewportElement?.getBoundingClientRect();
    const trackRect = trackElement?.getBoundingClientRect();
    const selectionRect = selectionElement?.getBoundingClientRect();
    return {
      pageOverflow:
        document.documentElement.scrollWidth
        - document.documentElement.clientWidth,
      root: rect(rootRect),
      preview: previewRect ? rect(previewRect) : null,
      viewport: viewportRect
        ? {
            ...rect(viewportRect),
            clientWidth: viewportElement.clientWidth,
            scrollWidth: viewportElement.scrollWidth,
            overflowX: getComputedStyle(viewportElement).overflowX
          }
        : null,
      track: trackRect ? rect(trackRect) : null,
      selection: selectionRect ? rect(selectionRect) : null
    };

    function rect(bounds) {
      return {
        left: Math.round(bounds.left),
        right: Math.round(bounds.right),
        top: Math.round(bounds.top),
        bottom: Math.round(bounds.bottom),
        width: Math.round(bounds.width),
        height: Math.round(bounds.height)
      };
    }
  });
  details.geometry = geometry;
  checks.push({
    id: "timeline-range-responsive-containment",
    ok:
      geometry.pageOverflow <= 1
      && geometry.preview !== null
      && geometry.root.left >= geometry.preview.left - 1
      && geometry.root.right <= geometry.preview.right + 1
      && geometry.selection !== null
      && geometry.track !== null
      && geometry.selection.left >= geometry.track.left - 1
      && geometry.selection.right <= geometry.track.right + 1
      && (
        scenario.viewport.width > 720
        || (
          geometry.viewport !== null
          && geometry.viewport.overflowX === "auto"
          && geometry.viewport.scrollWidth > geometry.viewport.clientWidth
        )
      ),
    actual: geometry
  });

  const motion = await selection.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      transitionDuration: style.transitionDuration,
      transitionProperty: style.transitionProperty
    };
  });
  details.motion = motion;
  if (scenario.reducedMotion === "reduce") {
    checks.push({
      id: "timeline-range-reduced-motion",
      ok: motion.transitionDuration
        .split(",")
        .every((duration) => Number.parseFloat(duration) === 0),
      expected: "all transition durations equal 0s",
      actual: motion
    });
  }

  checks.push({
    id: "timeline-range-theme-matrix-cell",
    ok: await page.evaluate(
      ({ theme, visualTheme }) =>
        document.documentElement.dataset.theme === theme
        && document.documentElement.dataset.visualTheme
          === (visualTheme ?? "apple-like"),
      scenario
    ),
    expected: {
      theme: scenario.theme,
      visualTheme: scenario.visualTheme ?? "apple-like"
    },
    actual: await page.evaluate(() => ({
      theme: document.documentElement.dataset.theme,
      visualTheme: document.documentElement.dataset.visualTheme
    }))
  });

  return { checks, details };
}

async function dragPointer(page, startX, startY, endX, endY) {
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(endX, endY, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(80);
}

async function timelineState({ end, position, start }) {
  const values = await Promise.all(
    [start, position, end].map(async (control) =>
      Number(await control.getAttribute("aria-valuenow"))
    )
  );
  return {
    start: values[0],
    position: values[1],
    end: values[2]
  };
}

function summarizeChecks(scenarios) {
  const checks = [];
  for (const scenario of scenarios) {
    for (const check of scenario.checks) {
      checks.push({
        id: `${scenario.id}:${check.id}`,
        ok: check.ok,
        detail: Object.fromEntries(Object.entries(check).filter(([key]) => key !== "id" && key !== "ok"))
      });
    }
  }
  return checks;
}

function summarizeViolations(violations) {
  return violations.map((violation) => ({
    id: violation.id,
    impact: violation.impact,
    help: violation.help,
    help_url: violation.helpUrl,
    nodes: violation.nodes.slice(0, 5).map((node) => ({
      target: node.target,
      failure_summary: node.failureSummary
    }))
  }));
}

function startStaticServer(root, requestedPort) {
  const server = createServer((request, response) => {
    const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
    const pathname = decodeURIComponent(requestUrl.pathname);
    const resolvedPath = resolve(root, `.${pathname}`);
    const rootWithSeparator = root.endsWith(sep) ? root : `${root}${sep}`;

    if (resolvedPath !== root && !resolvedPath.startsWith(rootWithSeparator)) {
      response.writeHead(403);
      response.end("Forbidden");
      return;
    }

    let filePath = resolvedPath;
    if (!existsSync(filePath)) {
      filePath = join(root, "index.html");
    } else if (statSync(filePath).isDirectory()) {
      filePath = join(filePath, "index.html");
    }

    if (!existsSync(filePath)) {
      response.writeHead(404);
      response.end("Not found");
      return;
    }

    response.writeHead(200, { "content-type": mimeType(filePath) });
    createReadStream(filePath).pipe(response);
  });

  return new Promise((resolveServer, rejectServer) => {
    server.once("error", rejectServer);
    server.listen(requestedPort, "127.0.0.1", () => {
      const address = server.address();
      resolveServer({
        instance: server,
        url: `http://127.0.0.1:${address.port}/`
      });
    });
  });
}

function mimeType(path) {
  switch (extname(path)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
      return "text/javascript; charset=utf-8";
    case ".css":
      return "text/css; charset=utf-8";
    case ".svg":
      return "image/svg+xml";
    case ".png":
      return "image/png";
    case ".jpg":
    case ".jpeg":
      return "image/jpeg";
    default:
      return "application/octet-stream";
  }
}

function runCommand(command, commandArgs, cwd) {
  return new Promise((resolveCommand) => {
    const child = spawn(command, commandArgs, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (status) => {
      resolveCommand({
        command: [command, ...commandArgs].join(" "),
        cwd: relative(cwd),
        ok: status === 0,
        status,
        stdout: stdout.trim(),
        stderr: stderr.trim()
      });
    });
  });
}

function countValues(values) {
  return values.reduce((counts, value) => {
    counts.set(value, (counts.get(value) ?? 0) + 1);
    return counts;
  }, new Map());
}

function duplicateValues(counts) {
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([component, count]) => ({ component, count }));
}

async function importFromKnownRoots(specifier) {
  const packageRoots = [
    catalogRoot,
    join(repoRoot, "examples/products/crm/frontend"),
    join(repoRoot, "admin_ui")
  ];
  const errors = [];

  for (const root of packageRoots) {
    try {
      const require = createRequire(join(root, "package.json"));
      const resolved = require.resolve(specifier);
      return await import(pathToFileURL(resolved).href);
    } catch (error) {
      errors.push(`${relative(root)}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  throw new Error(`Unable to resolve ${specifier}. Install catalog evidence dependencies or CRM frontend dev dependencies.\n${errors.join("\n")}`);
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--json") {
      parsed.json = true;
    } else if (value === "--skip-build") {
      parsed.skipBuild = true;
    } else if (value === "--output") {
      parsed.output = values[++index];
    } else if (value === "--artifact-root") {
      parsed.artifactRoot = values[++index];
    } else if (value === "--port") {
      parsed.port = values[++index];
    }
  }
  return parsed;
}

function relative(path) {
  return path.startsWith(repoRoot) ? path.slice(repoRoot.length + 1).split(sep).join("/") : path;
}
