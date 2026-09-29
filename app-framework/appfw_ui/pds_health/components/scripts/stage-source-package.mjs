import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const componentRoot = resolve(scriptDir, "..");
const repoRoot = resolve(componentRoot, "../../..");
const sourceDir = resolve(componentRoot, "src");
const tokenRoot = resolve(repoRoot, "appfw_ui/pds_health/tokens");
const tokenPath = resolve(tokenRoot, "pdsTokens.css");
const fontDir = resolve(tokenRoot, "fonts");
const defaultStageDir = resolve(repoRoot, "target/appfw/pds-components-package/stage");

function git(...args) {
  const result = spawnSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"]
  });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(" ")} failed: ${result.stderr.trim()}`);
  }
  return result.stdout.trim();
}

function outputArgument(argv) {
  const index = argv.indexOf("--output");
  if (index === -1) return defaultStageDir;
  if (!argv[index + 1]) throw new Error("--output requires a path");
  return resolve(repoRoot, argv[index + 1]);
}

const stageDir = outputArgument(process.argv.slice(2));
const allowedRoot = resolve(repoRoot, "target/appfw");
const stageRelative = relative(allowedRoot, stageDir);
if (stageRelative.startsWith("..") || stageRelative === "") {
  throw new Error(`stage output must be below ${allowedRoot}`);
}

const inputStatus = git(
  "status",
  "--porcelain=v1",
  "--untracked-files=all",
  "--",
  "appfw_ui/pds_health/components/package.json",
  "appfw_ui/pds_health/components/src",
  "appfw_ui/pds_health/tokens/pdsTokens.css",
  "appfw_ui/pds_health/tokens/fonts"
);
const dirtyWorkingInputs = inputStatus
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => line.startsWith("??") || line[1] !== " ");
if (dirtyWorkingInputs.length > 0) {
  throw new Error(
    `package inputs differ from the staged index:\n${dirtyWorkingInputs.join("\n")}`
  );
}
const indexTree = git("write-tree");
const sourceTree = git(
  "rev-parse",
  `${indexTree}:appfw_ui/pds_health/components/src`
);
const tokenBlob = git(
  "rev-parse",
  ":appfw_ui/pds_health/tokens/pdsTokens.css"
);
const fontTree = git(
  "rev-parse",
  `${indexTree}:appfw_ui/pds_health/tokens/fonts`
);

await rm(stageDir, { recursive: true, force: true });
await mkdir(resolve(stageDir, "components"), { recursive: true });
await mkdir(resolve(stageDir, "tokens"), { recursive: true });
const packageJson = JSON.parse(await readFile(resolve(componentRoot, "package.json"), "utf8"));
packageJson.sideEffects = ["./components/src/styles.css"];
packageJson.exports = {
  ".": "./components/src/index.ts",
  "./styles.css": "./components/src/styles.css"
};
packageJson.files = ["components/src", "tokens"];
delete packageJson.scripts["package:stage"];
await writeFile(resolve(stageDir, "package.json"), `${JSON.stringify(packageJson, null, 2)}\n`);
await cp(sourceDir, resolve(stageDir, "components/src"), { recursive: true });
await cp(tokenPath, resolve(stageDir, "tokens/pdsTokens.css"));
await cp(fontDir, resolve(stageDir, "tokens/fonts"), { recursive: true });

const stagedStyles = await readFile(resolve(stageDir, "components/src/styles.css"), "utf8");
if (!stagedStyles.startsWith('@import "../../tokens/pdsTokens.css";')) {
  throw new Error("staged component stylesheet no longer references the packaged token path");
}
const stagedTokens = await readFile(resolve(stageDir, "tokens/pdsTokens.css"), "utf8");
const packagedFonts = [
  ...new Set(
    [...stagedTokens.matchAll(/url\(["']?\.\/(fonts\/[A-Za-z0-9._-]+)["']?\)/g)]
      .map((match) => match[1])
  )
];
if (packagedFonts.length === 0) {
  throw new Error("packaged token stylesheet no longer references a packaged font");
}
for (const fontPath of packagedFonts) {
  await readFile(resolve(stageDir, "tokens", fontPath));
}

process.stdout.write(`${JSON.stringify({
  ok: true,
  stageDir: relative(repoRoot, stageDir),
  indexTree,
  sourceTree,
  tokenBlob,
  fontTree,
  packagedFonts
}, null, 2)}\n`);
