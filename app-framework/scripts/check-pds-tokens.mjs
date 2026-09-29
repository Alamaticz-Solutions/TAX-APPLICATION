#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const reportPath = join(repoRoot, "target/appfw/pds-token-drift.json");

const options = parseArgs(process.argv.slice(2));
const canonicalSource = "appfw_ui/pds_health/tokens/tokens.dtcg.json";
const generatedOutputs = {
  css: "appfw_ui/pds_health/tokens/pdsTokens.css",
  ts: "appfw_ui/pds_health/tokens/pdsTokens.ts"
};

const sourceConsumers = [
  {
    id: "admin-ui",
    label: "Admin UI CSS token import",
    path: "admin_ui/src/main.tsx",
    pattern: "@appfw/pds-health/tokens/pdsTokens.css"
  },
  {
    id: "admin-ui",
    label: "Admin UI CSS token Vite alias",
    path: "admin_ui/vite.config.ts",
    pattern: '"@appfw/pds-health/tokens/pdsTokens.css"'
  },
  {
    id: "admin-ui",
    label: "Admin UI typed token Vite alias",
    path: "admin_ui/vite.config.ts",
    pattern: '"@appfw/pds-health/tokens/pdsTokens.ts"'
  },
  {
    id: "crm-frontend",
    label: "CRM frontend CSS token import",
    path: "examples/products/crm/frontend/src/styles/index.css",
    pattern: "@appfw/pds-health/tokens/pdsTokens.css"
  },
  {
    id: "crm-frontend",
    label: "CRM frontend CSS token Vite alias",
    path: "examples/products/crm/frontend/vite.config.ts",
    pattern: '"@appfw/pds-health/tokens/pdsTokens.css"'
  },
  {
    id: "crm-frontend",
    label: "CRM frontend typed token Vite alias",
    path: "examples/products/crm/frontend/vite.config.ts",
    pattern: '"@appfw/pds-health/tokens/pdsTokens.ts"'
  }
].filter((consumer) => !options.consumer || consumer.id === options.consumer);

const forbiddenCopies = [
  {
    id: "admin-ui",
    label: "Admin UI CSS token copy",
    path: "admin_ui/src/design/pdsTokens.css"
  },
  {
    id: "admin-ui",
    label: "Admin UI typed token copy",
    path: "admin_ui/src/design/pdsTokens.ts"
  },
  {
    id: "crm-frontend",
    label: "CRM frontend CSS token copy",
    path: "examples/products/crm/frontend/src/design/pdsTokens.css"
  }
].filter((copy) => !options.consumer || copy.id === options.consumer);

if (options.help) {
  printHelp();
  process.exit(0);
}

if (options.consumer && sourceConsumers.length === 0 && forbiddenCopies.length === 0) {
  console.error(`Unknown PDS token consumer: ${options.consumer}`);
  process.exit(2);
}

const failures = [];
const canonicalSourceCheck = checkCanonicalSource(canonicalSource);
const generatedOutputChecks = Object.entries(generatedOutputs).map(([kind, path]) =>
  checkGeneratedOutput(kind, path)
);
const generatedSourceSync = checkGeneratedSourceSync();
const checkedConsumers = sourceConsumers.map(checkSourceConsumer);
const checkedForbiddenCopies = forbiddenCopies.map(checkForbiddenCopy);
const report = {
  schema: "appfw_pds_token_drift@2",
  command: "pds-token-drift",
  ok: failures.length === 0,
  generated_at: new Date().toISOString(),
  fix_applied: options.fix,
  artifact: relative(repoRoot, reportPath),
  canonical_source: canonicalSourceCheck,
  generated_outputs: generatedOutputs,
  generated_output_checks: generatedOutputChecks,
  generated_source_sync: generatedSourceSync,
  source_consumers: checkedConsumers,
  forbidden_copies: checkedForbiddenCopies,
  failures
};

mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");

if (options.json) {
  console.log(JSON.stringify(report, null, 2));
} else if (report.ok) {
  console.log("PDS token source consumption check ok");
} else {
  console.error("PDS token source consumption check failed");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  console.error(`Report: ${relative(repoRoot, reportPath)}`);
}

process.exit(report.ok ? 0 : 1);

function checkCanonicalSource(path) {
  const absolutePath = join(repoRoot, path);
  const exists = existsSync(absolutePath);
  const text = exists ? readFileSync(absolutePath, "utf8") : "";
  const hash = exists ? sha256(normalizeEol(text)) : null;

  if (!exists) {
    failures.push(`missing canonical token source: ${path}`);
  }

  return {
    format: "transitional-dtcg-json",
    path,
    exists,
    sha256: hash,
    ok: exists
  };
}

function checkGeneratedOutput(kind, path) {
  const absolutePath = join(repoRoot, path);
  const exists = existsSync(absolutePath);
  const text = exists ? readFileSync(absolutePath, "utf8") : "";
  const hash = exists ? sha256(normalizeEol(text)) : null;

  if (!exists) {
    failures.push(`missing generated ${kind} token output: ${path}`);
  }

  return {
    kind,
    path,
    exists,
    sha256: hash,
    ok: exists
  };
}

function checkGeneratedSourceSync() {
  // tokens.dtcg.json is the canonical token source (W1a); the CSS/TS files are
  // generated from it and must regenerate byte-identically.
  const generator = "scripts/generate-pds-tokens.mjs";
  if (!canonicalSourceCheck.exists) {
    return {
      generator,
      source: canonicalSource,
      ok: false,
      reason: "missing-json-source"
    };
  }
  try {
    const output = execFileSync(process.execPath, [join(repoRoot, generator), "--check", "--json"], {
      cwd: repoRoot,
      encoding: "utf8"
    });
    const result = JSON.parse(output);
    return {
      generator,
      source: canonicalSource,
      ok: true,
      css_in_sync: result.css_in_sync,
      ts_in_sync: result.ts_in_sync
    };
  } catch (error) {
    failures.push(
      `generated token files drift from ${canonicalSource}; run node ${generator} to regenerate`
    );
    let detail = null;
    try {
      detail = JSON.parse(error.stdout ?? "");
    } catch {
      detail = { error: String(error.message ?? error) };
    }
    return { generator, source: canonicalSource, ok: false, ...detail };
  }
}

function checkSourceConsumer(consumer) {
  const absolutePath = join(repoRoot, consumer.path);
  const exists = existsSync(absolutePath);
  const text = exists ? readFileSync(absolutePath, "utf8") : "";
  const present = exists && text.includes(consumer.pattern);

  if (!exists) {
    failures.push(`${consumer.label} file is missing: ${consumer.path}`);
  } else if (!present) {
    failures.push(`${consumer.label} does not consume ${consumer.pattern}: ${consumer.path}`);
  }

  return {
    id: consumer.id,
    label: consumer.label,
    path: consumer.path,
    pattern: consumer.pattern,
    ok: present
  };
}

function checkForbiddenCopy(copy) {
  const absolutePath = join(repoRoot, copy.path);
  const existed = existsSync(absolutePath);

  if (existed && options.fix) {
    rmSync(absolutePath);
  }

  const exists = existsSync(absolutePath);
  if (exists) {
    failures.push(
      `${copy.label} must be removed; consume ${generatedOutputs.css} or ${generatedOutputs.ts} through the framework package: ${copy.path}`
    );
  }

  return {
    id: copy.id,
    label: copy.label,
    path: copy.path,
    existed,
    removed: existed && options.fix && !exists,
    ok: !exists
  };
}

function normalizeEol(text) {
  return text.replace(/\r\n/g, "\n");
}

function sha256(text) {
  return `sha256:${createHash("sha256").update(text).digest("hex")}`;
}

function parseArgs(args) {
  const parsed = {
    consumer: null,
    fix: false,
    help: false,
    json: false
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--consumer") {
      parsed.consumer = args[index + 1] ?? null;
      index += 1;
    } else if (arg === "--fix") {
      parsed.fix = true;
    } else if (arg === "--json") {
      parsed.json = true;
    } else if (arg === "--help" || arg === "-h") {
      parsed.help = true;
    } else {
      console.error(`Unknown PDS token check option: ${arg}`);
      process.exit(2);
    }
  }

  return parsed;
}

function printHelp() {
  console.log(`Usage: scripts/check-pds-tokens.mjs [--json] [--fix] [--consumer <id>]

Checks that admin UI and product frontend code consume PDS Health design tokens
from the canonical source under appfw_ui/pds_health/tokens, that the generated
pdsTokens.css/.ts stay in byte-identical sync with the canonical
tokens.dtcg.json (scripts/generate-pds-tokens.mjs --check), and rejects local
token copies. Use --fix to remove known forbidden copies if they reappear.

Consumers:
  admin-ui
  crm-frontend`);
}
