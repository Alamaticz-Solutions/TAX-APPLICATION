import { readFile } from "node:fs/promises";
import { expect, test, type Locator, type Page } from "@playwright/test";

const decisionPathName = "Decision path from relationship context to local receipt";
const decisionStageLabels = ["Relationship", "Evidence", "Dependency", "Consequence", "Receipt"];

const stateTitles = {
  loading: "Loading activity context",
  empty: "No activities need attention",
  offline: "Local source unavailable",
  unauthorized: "You do not have access to this activity context",
  error: "The activity view could not be prepared"
} as const;

const viewports = [
  { name: "narrow-mobile", width: 320, height: 760 },
  { name: "mobile", width: 390, height: 844 },
  { name: "tablet", width: 834, height: 1112 },
  { name: "desktop", width: 1440, height: 960 }
];

function decisionStage(page: Page, stage: string) {
  return page
    .getByRole("region", { name: decisionPathName })
    .locator(`[data-stage="${stage}"]`);
}

async function expectDecisionPathOrder(page: Page) {
  const region = page.getByRole("region", { name: decisionPathName });
  await expect(region).toHaveCount(1);
  const stages = region.locator("ol > li");
  await expect(stages).toHaveCount(decisionStageLabels.length);
  for (const [index, label] of decisionStageLabels.entries()) {
    await expect(stages.nth(index).locator(".activity-decision-spine__stage-heading")).toHaveText(label);
  }
}

async function expectNoPageOverflow(page: Page) {
  await expect.poll(() => page.evaluate(() => ({
    documentFits: document.documentElement.scrollWidth <= document.documentElement.clientWidth,
    experienceFits: (() => {
      const experience = document.querySelector(".activity-experience");
      return experience instanceof HTMLElement
        ? experience.scrollWidth <= experience.clientWidth
        : false;
    })()
  }))).toEqual({ documentFits: true, experienceFits: true });
}

async function expectStageState(stage: Locator, state: string) {
  await expect(stage.locator(".activity-decision-spine__state")).toContainText(state);
}

async function decisionSpineLayout(page: Page) {
  return page.getByRole("region", { name: decisionPathName }).evaluate((region) => {
    const detail = region.closest(".activity-detail");
    const relationship = region.querySelector('[data-stage="relationship"]');
    const evidence = region.querySelector('[data-stage="evidence"]');
    const tool = region.querySelector(".activity-decision-spine__tool");
    const action = tool?.querySelector(".pds-button");
    const label = action?.querySelector(".pds-button__label");
    const constraint = tool?.querySelector(".activity-decision-spine__constraint");
    if (
      !(detail instanceof HTMLElement)
      || !(relationship instanceof HTMLElement)
      || !(evidence instanceof HTMLElement)
      || !(tool instanceof HTMLElement)
      || !(action instanceof HTMLButtonElement)
      || !(label instanceof HTMLElement)
    ) {
      throw new Error("Decision-spine layout probe is incomplete");
    }
    const range = document.createRange();
    range.selectNodeContents(label);
    const relationshipRect = relationship.getBoundingClientRect();
    const evidenceRect = evidence.getBoundingClientRect();
    return {
      detailContentWidth: Number.parseFloat(getComputedStyle(detail).width),
      orientation: Math.abs(relationshipRect.top - evidenceRect.top) <= 2 ? "horizontal" : "rows",
      toolWidth: tool.getBoundingClientRect().width,
      actionLabelLines: range.getClientRects().length,
      constraintWidth: constraint instanceof HTMLElement
        ? constraint.getBoundingClientRect().width
        : null
    };
  });
}

test("one decision path preserves keyboard selection and an item-isolated two-action receipt", async ({ page }) => {
  const interactionRequests: string[] = [];
  page.on("request", (request) => {
    if (
      request.method() !== "GET"
      || request.resourceType() === "fetch"
      || request.resourceType() === "xhr"
    ) {
      interactionRequests.push(`${request.method()} ${request.resourceType()} ${request.url()}`);
    }
  });

  await page.goto("/activities");
  await expect(page).toHaveURL(/\/activities$/);
  await expect(page.getByRole("heading", { name: "Activities that need attention" })).toBeVisible();
  await expect(page.getByText("Local preview only", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("No provider action was sent", { exact: true }).first()).toBeVisible();
  await expectDecisionPathOrder(page);

  await expect(page.getByText("Decision lens", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Readiness at a glance", { exact: true })).toHaveCount(0);
  await expect(page.locator(".activity-intelligence, .activity-readiness, .activity-preview")).toHaveCount(0);
  await expect(page.locator(".activity-detail").getByText(/^\d+\/\d+$/)).toHaveCount(0);

  const stateControl = page.getByLabel("Activity experience fixture state");
  await stateControl.focus();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: /Discovery call follow-up/ })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: /Prepare operating review/ })).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Prepare operating review" })).toBeVisible();
  await expectDecisionPathOrder(page);

  await expectStageState(decisionStage(page, "evidence"), "Refreshing");
  const inspectConsequence = page.getByRole("button", { name: "Inspect consequence" });
  await inspectConsequence.click();
  await expect(page.getByText("Operating-review context stays local")).toBeVisible();
  await expect(page.getByText(/does not update the account health snapshot/)).toBeVisible();
  await expect(page.getByText(/Evidence is refreshing/)).toBeVisible();

  const recordReceipt = page.getByRole("button", { name: "Record local review receipt" });
  await expect(recordReceipt).toBeFocused();
  await recordReceipt.click();
  const receipt = page.locator(".activity-decision-spine__receipt");
  await expect(receipt).toBeFocused();
  await expect(receipt).toContainText("Local review receipt recorded");
  await expect(receipt).toContainText("No provider action was sent");
  await expectStageState(decisionStage(page, "receipt"), "Recorded locally");

  await page.getByRole("button", { name: /Discovery call follow-up/ }).click();
  await expect(page.getByRole("button", { name: "Inspect consequence" })).toBeVisible();
  await expect(page.locator(".activity-decision-spine__receipt")).toHaveCount(0);
  await expectStageState(decisionStage(page, "receipt"), "Not recorded");

  await page.getByRole("button", { name: /Prepare operating review/ }).click();
  await expect(page.locator(".activity-decision-spine__receipt")).toContainText("Local review receipt recorded");
  await expect(page).toHaveURL(/\/activities$/);
  expect(interactionRequests).toEqual([]);
});

test("decision path responds to its detail allocation before preserving the horizontal rail", async ({ page }) => {
  const activityCss = await readFile(
    new URL(
      "../../src/features/activities/activityWorkQueueExperience.css",
      import.meta.url
    ),
    "utf8"
  );
  const enhancementStart = activityCss.indexOf(
    "@container activity-detail (min-width: 1240.01px)"
  );
  const safeRailRule = activityCss.match(
    /\.activity-decision-spine__rail\s*\{([^}]*)\}/
  )?.[1];
  const safeFactRule = activityCss.match(
    /\.activity-detail__facts\s*\{([^}]*)\}/
  )?.[1];
  expect(enhancementStart).toBeGreaterThan(0);
  expect(safeRailRule).toContain("grid-template-columns: 1fr");
  expect(safeFactRule).toContain("repeat(3, minmax(0, 1fr))");
  expect(activityCss.slice(enhancementStart)).toContain(
    "minmax(250px, 1.25fr)"
  );
  expect(activityCss).not.toContain(
    "@container activity-detail (max-width: 1240px)"
  );

  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("/activities");

  await expect.poll(async () => (await decisionSpineLayout(page)).orientation).toBe("rows");
  const readyLayout = await decisionSpineLayout(page);
  expect(readyLayout.detailContentWidth).toBeLessThanOrEqual(1240);
  expect(readyLayout.toolWidth).toBeGreaterThan(280);
  expect(readyLayout.actionLabelLines).toBe(1);

  await page.getByRole("button", { name: /Review renewal dependencies/ }).click();
  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toBeFocused();
  const blockedLayout = await decisionSpineLayout(page);
  expect(blockedLayout.orientation).toBe("rows");
  expect(blockedLayout.toolWidth).toBeGreaterThan(280);
  expect(blockedLayout.constraintWidth).toBeGreaterThan(280);
  expect(blockedLayout.actionLabelLines).toBe(1);

  await page.setViewportSize({ width: 2400, height: 1200 });
  await expect.poll(async () => (await decisionSpineLayout(page)).orientation).toBe("horizontal");
  const wideLayout = await decisionSpineLayout(page);
  expect(wideLayout.detailContentWidth).toBeGreaterThan(1240);
  expect(wideLayout.actionLabelLines).toBe(1);
  await expectNoPageOverflow(page);
});

test("one derived truth model keeps ready, partial, refreshing, stale, and blocked paths coherent", async ({ page }) => {
  await page.goto("/activities");
  const stateControl = page.getByLabel("Activity experience fixture state");

  await expectStageState(decisionStage(page, "relationship"), "Available");
  await expectStageState(decisionStage(page, "evidence"), "Available");
  await expectStageState(decisionStage(page, "dependency"), "Needs review");
  await expectStageState(decisionStage(page, "consequence"), "Ready to inspect");
  await expectStageState(decisionStage(page, "receipt"), "Not recorded");

  await stateControl.selectOption("partial");
  await expect(page.getByRole("status")).toContainText("Partial relationship context retained");
  await expectDecisionPathOrder(page);
  await expectStageState(decisionStage(page, "relationship"), "Partial");
  await expect(decisionStage(page, "relationship")).toContainText("Retained: 3 active contacts");
  await expectStageState(decisionStage(page, "evidence"), "Available");
  await expect(decisionStage(page, "consequence")).toContainText("Relationship context is partial");
  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await expect(page.getByText(/this local review cannot establish readiness/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toBeFocused();
  await page.getByRole("button", { name: "Return to ready state" }).click();
  await expect(stateControl).toBeFocused();
  await expect(stateControl).toHaveValue("ready");
  await expectStageState(decisionStage(page, "relationship"), "Available");

  await stateControl.selectOption("stale");
  await expect(page.getByRole("status")).toContainText("Last-known evidence retained");
  await expectDecisionPathOrder(page);
  await expectStageState(decisionStage(page, "relationship"), "Available");
  await expectStageState(decisionStage(page, "evidence"), "Stale");
  await expect(decisionStage(page, "evidence")).toContainText("Last known:");
  await expect(decisionStage(page, "consequence")).toContainText("Evidence is stale");
  await page.getByRole("button", { name: "Return to ready state" }).click();
  await expect(stateControl).toBeFocused();

  await page.getByRole("button", { name: /Prepare operating review/ }).click();
  await expectStageState(decisionStage(page, "evidence"), "Refreshing");
  await expect(decisionStage(page, "consequence")).toContainText("Evidence is refreshing");

  await page.getByRole("button", { name: /Review renewal dependencies/ }).click();
  await expectStageState(decisionStage(page, "evidence"), "Stale");
  await expectStageState(decisionStage(page, "dependency"), "Blocked");
  await expect(decisionStage(page, "consequence")).toContainText("Evidence is stale");
  await expect(decisionStage(page, "consequence")).toContainText("A dependency is blocked");
  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await expect(page.getByRole("group", { name: "Consequence review" })).toContainText(
    "this local review does not clear the dependency"
  );
});

test("local receipts are bound to item and truth posture before resurfacing", async ({ page }) => {
  await page.goto("/activities");
  const stateControl = page.getByLabel("Activity experience fixture state");

  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await page.getByRole("button", { name: "Record local review receipt" }).click();
  await expectStageState(decisionStage(page, "receipt"), "Recorded locally");

  await stateControl.selectOption("partial");
  await expectStageState(decisionStage(page, "receipt"), "Not recorded");
  await expect(page.getByRole("button", { name: "Inspect consequence" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toHaveCount(0);
  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toBeFocused();

  await stateControl.selectOption("stale");
  await expectStageState(decisionStage(page, "receipt"), "Not recorded");
  await expect(page.getByRole("button", { name: "Inspect consequence" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toHaveCount(0);

  await stateControl.selectOption("ready");
  await expectStageState(decisionStage(page, "receipt"), "Recorded locally");
  await expect(page.locator(".activity-decision-spine__receipt")).toContainText(
    "Local review receipt recorded"
  );
});

test("adverse fixture states remain truthful, mask protected context, and recover without network action", async ({ page }) => {
  const interactionRequests: string[] = [];
  page.on("request", (request) => {
    if (
      request.method() !== "GET"
      || request.resourceType() === "fetch"
      || request.resourceType() === "xhr"
    ) {
      interactionRequests.push(`${request.method()} ${request.resourceType()} ${request.url()}`);
    }
  });

  await page.goto("/activities");
  const stateControl = page.getByLabel("Activity experience fixture state");

  for (const [state, title] of Object.entries(stateTitles)) {
    await stateControl.selectOption(state);
    await expect(page.getByRole("heading", { name: title })).toBeVisible();
    await expect(page.getByRole("region", { name: decisionPathName })).toHaveCount(0);
    await expect(page.getByText("Local preview only", { exact: true })).toBeVisible();
    await expect(page.getByText("No provider action was sent", { exact: true })).toBeVisible();
  }

  await stateControl.selectOption("empty");
  await expect(page.getByText(/contains no work items in this state/)).toBeVisible();
  await expect(page.getByText(/filter/i)).toHaveCount(0);

  await stateControl.selectOption("unauthorized");
  const denied = page.getByRole("alert");
  await expect(denied).toContainText("decision path, and local receipts stay masked");
  await expect(page.getByText("Local review receipt recorded")).toHaveCount(0);

  await stateControl.selectOption("error");
  await expect(page.getByRole("alert")).toContainText("correlation-safe identifier");
  await page.getByRole("button", { name: "Return to ready state" }).click();
  await expect(stateControl).toBeFocused();
  await expect(stateControl).toHaveValue("ready");
  await expectDecisionPathOrder(page);

  await stateControl.selectOption("offline");
  await expect(page.getByRole("status")).toContainText("No network recovery or provider call was attempted");
  await page.getByRole("button", { name: "Return to ready state" }).click();
  await expectDecisionPathOrder(page);

  expect(interactionRequests).toEqual([]);
});

test("activity queue uses canonical PDS fonts and adds no route-local font payload", async ({ page }) => {
  const fontRequests: string[] = [];
  page.on("request", (request) => {
    if (/\.(?:woff2?|ttf|otf)(?:\?|$)/i.test(request.url())) {
      fontRequests.push(request.url());
    }
  });

  await page.goto("/activities");
  await expect(page.getByRole("region", { name: decisionPathName })).toBeVisible();
  const allowedFonts = new Set([
    "InterVariable.woff2",
    "GeistMonoVariable.woff2",
    "Poppins-Bold-latin.woff2"
  ]);
  expect(fontRequests.length).toBeGreaterThan(0);
  expect(
    fontRequests.every((url) => allowedFonts.has(new URL(url).pathname.split("/").at(-1) ?? ""))
  ).toBe(true);
  expect(
    fontRequests.some((url) => /activities/i.test(new URL(url).pathname))
  ).toBe(false);
});

test("decision path keeps one DOM order, reduced-motion focus, and local bounds at target widths", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/activities");

  for (const viewport of viewports) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await expectDecisionPathOrder(page);
    await expectNoPageOverflow(page);
  }

  const stateControl = page.getByLabel("Activity experience fixture state");
  await stateControl.selectOption("loading");
  const loadingSpinner = page.locator(".activity-state-panel .is-spinning");
  await expect(loadingSpinner).toBeVisible();
  await expect.poll(
    () => loadingSpinner.evaluate((node) => getComputedStyle(node).animationName)
  ).toBe("activity-spin");
  await expect.poll(async () => {
    const durations = await loadingSpinner.evaluate((node) => {
      const style = getComputedStyle(node);
      return [style.animationDuration, style.transitionDuration]
        .flatMap((value) => value.split(","))
        .map((value) => Number.parseFloat(value))
        .filter(Number.isFinite);
    });
    return Math.max(0, ...durations);
  }).toBeLessThanOrEqual(0.00001);

  await stateControl.selectOption("ready");
  await page.getByRole("button", { name: "Inspect consequence" }).click();
  await expect(page.getByRole("button", { name: "Record local review receipt" })).toBeFocused();
  await page.getByRole("button", { name: "Record local review receipt" }).click();
  const receipt = page.locator(".activity-decision-spine__receipt");
  await expect(receipt).toBeFocused();
  await expect.poll(async () => {
    const duration = await receipt.evaluate((node) => getComputedStyle(node).animationDuration);
    return Number.parseFloat(duration);
  }).toBeLessThanOrEqual(0.00001);
});

test("activity queue renders a structured 1,000-item deterministic fixture inside local bounds", async ({ page }) => {
  await page.route("**/activityWorkQueueFixture.ts**", async (route) => {
    await route.fulfill({
      contentType: "application/javascript",
      body: 'export const activityWorkQueueContract = "activity-work-queue.v2"; export const activityQueueCorrelationPrefix = "crm-demo-activity"; export const activityWorkQueueFixture = Array.from({ length: 1000 }, (_, index) => ({ id: "activity-large-" + index, subject: "Large activity " + (index + 1), account: "Deterministic account", owner: "Fixture owner", dueLabel: "Due today", priority: index % 3 === 0 ? "High" : "Normal", status: "Open", freshness: "Current", summary: "Large deterministic fixture item.", correlationId: "crm-demo-activity-" + index, decisionPath: { relationship: { detail: "Fixture relationship", state: "ready" }, evidence: { detail: "Fixture evidence", state: "ready" }, dependency: { detail: "Fixture dependency", state: "ready" }, consequence: { title: "Local preview", detail: "No provider action was sent." } } }));'
    });
  });

  await page.goto("/activities");
  await expect(page.locator(".activity-queue-item")).toHaveCount(1000);
  await expectDecisionPathOrder(page);
  await page.setViewportSize({ width: 320, height: 760 });
  await expectNoPageOverflow(page);
});
