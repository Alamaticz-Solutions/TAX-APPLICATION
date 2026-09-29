import { lstatSync, readFileSync, realpathSync } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const RESERVED_SEGMENTS = new Set([
  ".agents",
  ".cache",
  ".codex",
  ".git",
  ".idea",
  ".vscode",
  "build",
  "coverage",
  "dist",
  "node_modules",
  "target",
  "temp",
  "tmp",
]);
const REVIEWABLE_PATH_CACHE = new Map();

export const SENSITIVE_AUTHORITY_ROOTS = Object.freeze([
  "AGENTS.md",
  "Cargo.toml",
  "Cargo.lock",
  "bitbucket-pipelines.yml",
  "docs/reference/program-work-system.md",
  "docs/specs/nexus-poc-multi-workstation-delivery.md",
  "docs/specs/nexus-poc-workstream-topology.json",
  "docs/specs/nexus-add-provider-seams.json",
  "docs/specs/product-increment-portfolio.json",
  "docs/evidence/product-increments/**",
  "docs/start/agent-task-map.md",
  "docs/start/agentic-human-operating-model.md",
  "docs/start/agent-role-cards.md",
  "docs/start/branch-integration-model.md",
  "docs/start/pr-review-agent-harness.md",
  "docs/start/product-pr-review-agent-harness.md",
  "docs/start/product-increment-delivery-model.md",
  "docs/start/product-increment-evidence.schema.json",
  "docs/start/product-increment-plan.schema.json",
  "docs/start/product-increment-portfolio.schema.json",
  "docs/start/workstation-assignment.example.json",
  "docs/start/workstation-assignment.schema.json",
  "docs/start/team-harness-replication.md",
  "docs/start/delivery-profiles.json",
  "scripts/ci/**",
  "scripts/git-hooks/**",
  "scripts/appfw",
  "scripts/appfw-delivery-mode.py",
  "scripts/appfw-delivery-mode-test.py",
  "scripts/audit-worktrees.mjs",
  "scripts/check-doc-examples.sh",
  "scripts/check-nexus-workstreams.mjs",
  "scripts/check-nexus-workstreams.test.mjs",
  "scripts/run-nexus-workstream-tests.mjs",
  "scripts/run-nexus-workstream-tests.test.mjs",
  "scripts/check-product-increment-plan.mjs",
  "scripts/check-product-increment-plan.test.mjs",
  "scripts/check-product-increment-portfolio.mjs",
  "scripts/check-product-increment-portfolio.test.mjs",
  "scripts/product-increment-path-safety.mjs",
  "appfw_runtime/src/chat.rs",
  "appfw_runtime/src/security.rs",
]);

function pathBase(pattern) {
  return String(pattern)
    .replace(/\/\*\*$/, "")
    .replace(/\/\*$/, "")
    .replace(/\/$/, "");
}

function pathsOverlap(left, right) {
  const leftBase = pathBase(left);
  const rightBase = pathBase(right);
  return (
    leftBase === rightBase ||
    leftBase.startsWith(`${rightBase}/`) ||
    rightBase.startsWith(`${leftBase}/`)
  );
}

export function isSensitiveAuthorityPath(value) {
  const base = pathBase(value);
  if (/^docs\/specs\/[^/]+\.product-increment\.json$/.test(base)) {
    return true;
  }
  return SENSITIVE_AUTHORITY_ROOTS.some((root) => pathsOverlap(value, root));
}

export function productIncrementDocsSubcheckForPath(value) {
  const base = pathBase(value);
  if (
    /^docs\/specs\/[^/]+\.product-increment\.json$/.test(base) ||
    base === "docs/specs/product-increment-portfolio.json"
  ) {
    return "product-increment-delivery";
  }
  return null;
}

export function docsChangedSurfaceRuleForPath(value) {
  const subcheck = productIncrementDocsSubcheckForPath(value);
  if (subcheck) return `focused:${subcheck}`;
  if (isSensitiveAuthorityPath(value)) {
    return "product-increment-authority-contract";
  }
  return null;
}

export function classifyDocsChangedSurfaceInput(input) {
  if (typeof input !== "string") {
    throw new TypeError("changed-surface input must be a string");
  }
  return input
    .split("\n")
    .map((path) => (path.endsWith("\r") ? path.slice(0, -1) : path))
    .filter((path) => path.trim() !== "")
    .map((path) => ({
      path,
      rule: docsChangedSurfaceRuleForPath(path),
    }));
}

function isInside(root, candidate) {
  const pathFromRoot = relative(root, candidate);
  return (
    pathFromRoot === "" ||
    (!pathFromRoot.startsWith("../") &&
      pathFromRoot !== ".." &&
      !isAbsolute(pathFromRoot))
  );
}

export function isSafeRepositoryPath(
  value,
  { allowTrailingGlob = false } = {},
) {
  if (
    typeof value !== "string" ||
    value.trim() === "" ||
    isAbsolute(value) ||
    value.includes("\\") ||
    value.includes("\0")
  ) {
    return false;
  }
  const hasTrailingGlob = value.endsWith("/**");
  if (hasTrailingGlob && !allowTrailingGlob) return false;
  const base = hasTrailingGlob ? value.slice(0, -3) : value;
  if (base.includes("*") || base.includes("?") || base.includes("[")) {
    return false;
  }
  const segments = base.split("/");
  return (
    segments.length > 0 &&
    segments.every(
      (segment) =>
        segment !== "" &&
        segment !== "." &&
        segment !== ".." &&
        !RESERVED_SEGMENTS.has(segment) &&
        !/^\.env(?:\.|$)/.test(segment) &&
        /^[A-Za-z0-9._-]+$/.test(segment),
    )
  );
}

export function repositoryPathIsReviewable(
  repositoryRoot,
  value,
  { allowTrailingGlob = false } = {},
) {
  if (!isSafeRepositoryPath(value, { allowTrailingGlob })) return false;

  const base = pathBase(value);
  const cacheKey = `${repositoryRoot}\0${base}\0${allowTrailingGlob}`;
  if (REVIEWABLE_PATH_CACHE.has(cacheKey)) {
    return REVIEWABLE_PATH_CACHE.get(cacheKey);
  }
  const finish = (result) => {
    REVIEWABLE_PATH_CACHE.set(cacheKey, result);
    return result;
  };
  const ignored = spawnSync(
    "git",
    ["check-ignore", "--no-index", "--quiet", "--", base],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
  if (ignored.status === 0 || ![0, 1].includes(ignored.status)) {
    return finish(false);
  }

  const canonicalRoot = realpathSync(repositoryRoot);
  const segments = base.split("/");
  let current = canonicalRoot;
  for (const segment of segments) {
    current = resolve(current, segment);
    let stat;
    try {
      stat = lstatSync(current);
    } catch (error) {
      if (error?.code === "ENOENT") break;
      return finish(false);
    }
    if (stat.isSymbolicLink()) return finish(false);
    let canonicalCurrent;
    try {
      canonicalCurrent = realpathSync(current);
    } catch {
      return finish(false);
    }
    if (!isInside(canonicalRoot, canonicalCurrent)) return finish(false);
  }

  return finish(isInside(canonicalRoot, resolve(canonicalRoot, base)));
}

const scriptPath = fileURLToPath(import.meta.url);
const PATH_SAFETY_USAGE =
  "Usage: node scripts/product-increment-path-safety.mjs --classify-sensitive|--classify-docs-subcheck <repository-path>\n" +
  "   or: node scripts/product-increment-path-safety.mjs --classify-docs-surface-batch < changed-paths.txt";

function isMainModule() {
  if (!process.argv[1]) return false;
  try {
    return realpathSync(process.argv[1]) === realpathSync(scriptPath);
  } catch {
    return resolve(process.argv[1]) === resolve(scriptPath);
  }
}

export function evaluatePathSafetyCommand(args, input = "") {
  const [command, value, ...extra] = args;
  const singlePathCommands = new Set([
    "--classify-sensitive",
    "--classify-docs-subcheck",
  ]);
  if (
    command === "--classify-docs-surface-batch" &&
    value === undefined &&
    extra.length === 0
  ) {
    const classifications = classifyDocsChangedSurfaceInput(input);
    return {
      status: 0,
      stdout:
        classifications.length > 0
          ? `${classifications
              .map(({ path, rule }) => `${path}|${rule ?? ""}`)
              .join("\n")}\n`
          : "",
      stderr: "",
    };
  } else if (
    !value ||
    extra.length > 0 ||
    !singlePathCommands.has(command)
  ) {
    return { status: 2, stdout: "", stderr: `${PATH_SAFETY_USAGE}\n` };
  } else if (command === "--classify-sensitive" && isSensitiveAuthorityPath(value)) {
    return {
      status: 0,
      stdout: "product-increment-authority-contract\n",
      stderr: "",
    };
  } else if (command === "--classify-docs-subcheck") {
    const subcheck = productIncrementDocsSubcheckForPath(value);
    if (subcheck) {
      return { status: 0, stdout: `${subcheck}\n`, stderr: "" };
    }
  }
  return { status: 1, stdout: "", stderr: "" };
}

if (isMainModule()) {
  const args = process.argv.slice(2);
  const input =
    args[0] === "--classify-docs-surface-batch"
      ? readFileSync(0, "utf8")
      : "";
  const result = evaluatePathSafetyCommand(args, input);
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exitCode = result.status;
}
