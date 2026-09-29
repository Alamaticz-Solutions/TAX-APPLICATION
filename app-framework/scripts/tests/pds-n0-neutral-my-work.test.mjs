#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const paths = {
  html: "appfw_ui/pds_health/catalog-app/pds-n0-neutral-my-work.html",
  contract: "appfw_ui/pds_health/catalog-app/src/neutral-work/contract.ts",
  fixtures: "appfw_ui/pds_health/catalog-app/src/neutral-work/fixtures.ts",
  experience: "appfw_ui/pds_health/catalog-app/src/neutral-work/NeutralMyWorkExperience.tsx",
  main: "appfw_ui/pds_health/catalog-app/src/neutral-work/main.tsx",
  css: "appfw_ui/pds_health/catalog-app/src/neutral-work/neutral-work.css",
  sourceTest: "scripts/tests/pds-n0-neutral-my-work.test.mjs",
  browserTest: "scripts/tests/pds-n0-neutral-my-work.browser.mjs"
};

const states = [
  "loading",
  "populated",
  "empty",
  "error",
  "partial",
  "stale",
  "offline",
  "unauthorized",
  "forbidden",
  "conflict",
  "timeout",
  "success",
  "recovery"
];

function source(path) {
  return readFileSync(resolve(root, path), "utf8");
}

test("the exact eight-path source boundary exists", () => {
  for (const path of Object.values(paths)) assert.ok(source(path).length > 0, path);
});

test("the contract freezes all thirteen operational states", () => {
  const contract = source(paths.contract);
  for (const state of states) assert.match(contract, new RegExp(`\\| \\"${state}\\"`));
  assert.match(contract, /NEUTRAL_WORK_OPERATIONAL_STATES/);
  assert.match(contract, /isNeutralWorkOperationalState/);
  assert.match(contract, /isKnownNeutralWorkToken/);
});

test("fixtures are deterministic, complete, and fail closed", () => {
  const fixtures = source(paths.fixtures);
  for (const state of states) {
    assert.match(fixtures, new RegExp(`fixture\\(\\"${state}\\"`), state);
  }
  assert.match(fixtures, /NEUTRAL_WORK_FIXTURES/);
  assert.match(fixtures, /sourceLabel: FIXTURE_SOURCE_LABEL/);
  assert.match(fixtures, /unauthorized: fixture\(\"unauthorized\", \{[\s\S]*?permissionResult: \"restricted\"[\s\S]*?items: \[\][\s\S]*?notifications: \[\]/);
  assert.match(fixtures, /forbidden: fixture\(\"forbidden\", \{[\s\S]*?permissionResult: \"restricted\"[\s\S]*?items: \[\][\s\S]*?notifications: \[\]/);
  assert.match(fixtures, /No local recovery is available without a new permission result\./);
});

test("restricted recovery stays fail closed in both rendering and the handler", () => {
  const experience = source(paths.experience);
  assert.match(experience, /const blocked = fixture\.permissionResult === \"restricted\"/);
  assert.match(experience, /if \(fixture\.permissionResult === \"restricted\"\) return;/);
  assert.match(experience, /recoveryAllowed=\{!blocked\}/);
  assert.match(experience, /data-permission-result=\{fixture\.permissionResult\}/);
});

test("the standalone consumer uses only frozen public PDS imports", () => {
  const implementation = [
    source(paths.experience),
    source(paths.main)
  ].join("\n");
  const pdsImports = [...implementation.matchAll(/from ["'](@appfw\/[^"']+)["']/g)]
    .map((match) => match[1]);
  assert.deepEqual([...new Set(pdsImports)], ["@appfw/pds-health-components"]);
  assert.match(source(paths.main), /@appfw\/pds-health-components\/styles\.css/);
  assert.doesNotMatch(implementation, /react-aria-components|components\/src|src\/f1|catalog-app\/src\/(App|main)|fetch\s*\(|XMLHttpRequest|WebSocket|EventSource|serviceWorker/);
});

test("the composition exposes bounded continuity and adverse-state semantics", () => {
  const experience = source(paths.experience);
  for (const marker of [
    "data-n0-state",
    "neutral-work-queue",
    "neutral-work-detail",
    "neutral-work-notifications",
    "restoreFocus",
    "pushState",
    "replaceState",
    "searchParams.delete",
    "history.back",
    "popstate",
    "aria-busy",
    "approval",
    "attention",
    "freshness"
  ]) assert.match(experience, new RegExp(marker), marker);
});

test("the entry accepts only bounded local evidence inputs", () => {
  const main = source(paths.main);
  assert.match(main, /state/);
  assert.match(main, /theme/);
  assert.match(main, /grammar/);
  assert.match(main, /direction/);
  assert.match(main, /document\.documentElement\.dir = direction/);
  assert.match(main, /isNeutralWorkOperationalState/);
  assert.match(main, /isKnownNeutralWorkToken/);
  assert.doesNotMatch(main, /https?:|import\s*\(/);
});

test("browser evidence names the snapshot and directly covers trust and adaptation", () => {
  const browser = source(paths.browserTest);
  assert.match(browser, /local_frozen_f1_n0_snapshot_non_credit/);
  assert.match(browser, /fixed_at_1_no_dpr_zoom_substitution/);
  assert.match(browser, /equivalent_css_pixel_reflow/);
  assert.match(browser, /direction: \"rtl\"/);
  assert.match(browser, /exerciseRecoveryContract/);
  assert.match(browser, /automated_fixture_instrumentation_only/);
  assert.match(browser, /human_design_disposition: \"not_collected\"/);
});

test("composition CSS uses semantic tokens and adaptation queries", () => {
  const css = source(paths.css);
  assert.match(css, /var\(--pds-/);
  assert.match(css, /prefers-reduced-motion: reduce/);
  assert.match(css, /forced-colors: active/);
  assert.match(css, /data-visual-theme=\"material-like\"/);
  assert.match(css, /text-align: start/);
  assert.doesNotMatch(css, /#[0-9a-f]{3,8}|rgba?\s*\(/i);
});
