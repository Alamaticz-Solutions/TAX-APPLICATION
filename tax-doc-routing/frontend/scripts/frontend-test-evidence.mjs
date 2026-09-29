#!/usr/bin/env node

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = parseArgs(process.argv.slice(2));
const output = args.output ? resolve(args.output) : join(frontendRoot, "test-results", "frontend-test-evidence.json");
const status = Number(args.status ?? "0");
const playwrightJson = join(frontendRoot, "test-results", "playwright-results.json");
const playwrightHtml = join(frontendRoot, "playwright-report", "index.html");

const result = existsSync(playwrightJson) ? readPlaywrightResult(playwrightJson) : null;
const evidence = {
  command: "frontend-test",
  ok: status === 0 && Boolean(result?.ok),
  generated_at_utc: new Date().toISOString(),
  frontend_root: frontendRoot,
  status,
  mode: "deterministic-mocked-backend",
  coverage: [
    "crm-dashboard",
    "accounts-grid",
    "account-form",
    "query-builder",
    "lookup-selector",
    "delete-confirmation",
    "dark-light-mode",
    "axe-accessibility"
  ],
  playwright: result,
  artifacts: {
    json_report: existsSync(playwrightJson) ? playwrightJson : null,
    html_report: existsSync(playwrightHtml) ? playwrightHtml : null,
    traces_and_screenshots: join(frontendRoot, "test-results", "artifacts")
  }
};

writeFileSync(output, JSON.stringify(evidence, null, 2) + "\n", "utf8");
console.log(output);

function readPlaywrightResult(path) {
  const payload = JSON.parse(readFileSync(path, "utf8"));
  const stats = countTests(payload.suites ?? []);
  return {
    ok: stats.failed === 0 && stats.flaky === 0 && stats.timedOut === 0 && stats.interrupted === 0,
    config_file: payload.config?.configFile ?? null,
    projects: (payload.config?.projects ?? []).map((project) => project.name),
    tests: stats.total,
    passed: stats.passed,
    failed: stats.failed,
    flaky: stats.flaky,
    skipped: stats.skipped,
    timed_out: stats.timedOut,
    interrupted: stats.interrupted
  };
}

function countTests(suites) {
  const stats = {
    total: 0,
    passed: 0,
    failed: 0,
    flaky: 0,
    skipped: 0,
    timedOut: 0,
    interrupted: 0
  };

  for (const suite of suites) {
    merge(stats, countTests(suite.suites ?? []));
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests ?? []) {
        stats.total += 1;
        if (test.status === "expected") stats.passed += 1;
        if (test.status === "unexpected") stats.failed += 1;
        if (test.status === "flaky") stats.flaky += 1;
        if (test.status === "skipped") stats.skipped += 1;
        for (const result of test.results ?? []) {
          if (result.status === "timedOut") stats.timedOut += 1;
          if (result.status === "interrupted") stats.interrupted += 1;
        }
      }
    }
  }

  return stats;
}

function merge(left, right) {
  for (const key of Object.keys(left)) {
    left[key] += right[key] ?? 0;
  }
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--output") {
      parsed.output = values[++index];
    } else if (value === "--status") {
      parsed.status = values[++index];
    }
  }
  return parsed;
}
