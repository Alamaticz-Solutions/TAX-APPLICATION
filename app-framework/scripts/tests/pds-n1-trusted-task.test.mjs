#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const root = resolve(import.meta.dirname, "../..");
const paths = {
  html: "appfw_ui/pds_health/catalog-app/pds-n1-trusted-task.html",
  contract: "appfw_ui/pds_health/catalog-app/src/trusted-task/contract.ts",
  fixtures: "appfw_ui/pds_health/catalog-app/src/trusted-task/fixtures.ts",
  experience: "appfw_ui/pds_health/catalog-app/src/trusted-task/TrustedTaskExperience.tsx",
  main: "appfw_ui/pds_health/catalog-app/src/trusted-task/main.tsx",
  css: "appfw_ui/pds_health/catalog-app/src/trusted-task/trusted-task.css",
  sourceTest: "scripts/tests/pds-n1-trusted-task.test.mjs",
  browserTest: "scripts/tests/pds-n1-trusted-task.browser.mjs"
};

const states = [
  "loading", "empty", "permitted-preview", "denied", "stale", "partial",
  "offline", "unauthorized", "pending", "success-receipt", "failed", "error", "recovery"
];

function source(path) {
  return readFileSync(resolve(root, path), "utf8");
}

test("the exact eight-path source boundary exists", () => {
  assert.equal(Object.keys(paths).length, 8);
  for (const path of Object.values(paths)) assert.ok(source(path).length > 0, path);
});

test("the typed fixture contract is versioned and resolves exact receipt resume identity", () => {
  const contract = source(paths.contract);
  for (const state of states) assert.match(contract, new RegExp(`\\| \\"${state}\\"`), state);
  for (const marker of [
    "TRUSTED_TASK_CONTRACT_VERSION", "TRUSTED_TASK_FIXTURE_VERSION", "TRUSTED_TASK_PREVIEW_VERSION",
    "receiptResolvesWork", "workToken", "resumeToken", "fixtureVersion", "previewVersion", "correlationId"
  ]) assert.match(contract, new RegExp(marker), marker);
});

test("fixtures are deterministic, complete, and explicitly preview-only", () => {
  const fixtures = source(paths.fixtures);
  for (const state of states) assert.match(fixtures, new RegExp(`fixture\\(\\"${state}\\"`), state);
  for (const marker of [
    "preview_only", "No action is sent", "fixture_record_retained", "not_supported",
    "manual_recovery_required", "simulated_complete", "simulated_failed", "n0-snapshot/92f87be0"
  ]) assert.match(fixtures, new RegExp(marker), marker);
});

test("fixtures and composition contain no provider, credential, callback, or executable dispatch fields", () => {
  const implementation = [source(paths.contract), source(paths.fixtures), source(paths.experience), source(paths.main)].join("\n");
  assert.doesNotMatch(implementation, /bearer|oauth|access[_-]?token|providerClient|tenantData|workday|servicenow/i);
  assert.doesNotMatch(implementation, /fetch\s*\(|XMLHttpRequest|WebSocket|EventSource|serviceWorker|executeAction|dispatchAction/);
  assert.doesNotMatch(implementation, /react-aria-components|components\/src|src\/f1/);
});

test("the standalone surface imports only the frozen public PDS package", () => {
  const implementation = [source(paths.experience), source(paths.main)].join("\n");
  const imports = [...implementation.matchAll(/from ["'](@appfw\/[^"']+)["']/g)].map((match) => match[1]);
  assert.deepEqual([...new Set(imports)], ["@appfw/pds-health-components"]);
  assert.match(source(paths.main), /@appfw\/pds-health-components\/styles\.css/);
});

test("the composition exposes trusted preview, receipt, resume, denial, and recovery semantics", () => {
  const experience = source(paths.experience);
  for (const marker of [
    "data-n1-state", "trusted-task-overview", "trusted-task-evidence", "trusted-task-preview",
    "trusted-task-receipt", "Preview only", "Restore exact context", "permission", "correlationId",
    "undoPosture", "auditPosture", "aria-busy", "pushState", "popstate"
  ]) assert.match(experience, new RegExp(marker), marker);
});

test("permission transitions remain fail closed until a fixture explicitly permits preview", () => {
  const experience = source(paths.experience);
  assert.match(experience, /fixture\.preview\?\.permission\.result === "permitted"/);
  assert.match(experience, /permittedFixture\.preview\?\.permission\.result !== "permitted"/);
  assert.match(experience, /canEnterPermittedPreview/);
  assert.doesNotMatch(experience, /activeState === "error"[\s\S]{0,240}onClick=\{(?:show|open|enter)Preview/);
  assert.doesNotMatch(experience, /fixture\.preview\?\.permission\.result !== "permitted"[\s\S]{0,160}<Button/);
});

test("receipt restoration is gated by current permission while failed identity remains distinct", () => {
  const experience = source(paths.experience);
  assert.match(experience, /const canRestoreReceipt = fixture\.preview\?\.permission\.result === "permitted"/);
  assert.doesNotMatch(experience, /canRestoreReceipt = activeState === "failed"/);
  assert.match(experience, /if \(!canRestoreReceipt\) return/);
  assert.match(experience, /activeState === "failed" && canRestoreReceipt \? \(/);
  assert.match(experience, /canRestoreReceipt \? \([\s\S]*Restore exact context[\s\S]*\) : \(/);
  assert.match(experience, /activeState === "failed" \? "failed" : "recovery"/);
});

test("failed recovery preserves the originating receipt correlation and operation identity", () => {
  const contract = source(paths.contract);
  const fixtures = source(paths.fixtures);
  const experience = source(paths.experience);
  assert.match(contract, /operationId: string/);
  assert.match(fixtures, /receiptId: "receipt-neutral-failed-0001"[\s\S]*correlationId: "corr-fixture-91c0e4"[\s\S]*operationId: "operation-fixture-failed-0001"[\s\S]*operationState: "simulated_failed"/);
  assert.match(experience, /activeState === "failed" \? "failed" : "recovery"/);
  assert.match(experience, /next\.searchParams\.set\("state", recoveryState\)/);
  assert.match(experience, /next\.searchParams\.set\("resume", recovery\.work\.resumeToken\)/);
});

test("entry inputs are bounded and composition CSS uses semantic adaptation", () => {
  const main = source(paths.main);
  for (const marker of ["state", "theme", "grammar", "work", "resume", "isTrustedTaskState"]) {
    assert.match(main, new RegExp(marker), marker);
  }
  const css = source(paths.css);
  assert.match(css, /var\(--pds-/);
  assert.match(css, /prefers-reduced-motion: reduce/);
  assert.match(css, /forced-colors: active/);
  assert.match(css, /data-visual-theme=\"material-like\"/);
  assert.doesNotMatch(css, /#[0-9a-f]{3,8}|rgba?\s*\(/i);
});
