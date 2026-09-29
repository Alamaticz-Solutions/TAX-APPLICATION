import { expect, type Page } from "@playwright/test";

export async function signInForFrontendTests(page: Page) {
  await page.getByLabel("Bearer token").fill("playwright-token");
  await page.getByLabel("Tenant id").fill("tenant-playwright");
  await page.locator(".crm-identity-controls").getByRole("button", { name: "Apply" }).click();
  await expect(page.getByText(/Signed in/)).toBeVisible();
}

export async function expectNoSeriousA11yViolations(violations: { id: string; impact: string | null | undefined; help: string }[]) {
  const blocking = violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
  expect(
    blocking.map((violation) => {
      const nodes = "nodes" in violation && Array.isArray(violation.nodes)
        ? violation.nodes.map((node: { target?: string[] }) => node.target?.join(",")).filter(Boolean).join(" | ")
        : "";
      return `${violation.id}: ${violation.help}${nodes ? ` [${nodes}]` : ""}`;
    })
  ).toEqual([]);
}
