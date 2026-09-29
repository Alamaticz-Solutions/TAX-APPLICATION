import { execFileSync } from "node:child_process";
import { lstat, mkdir, mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const componentRoot = resolve(scriptDir, "..");
const distRoot = resolve(componentRoot, "dist");
const packageDocument = JSON.parse(await readFile(resolve(componentRoot, "package.json"), "utf8"));
const npmCache = process.env.npm_config_cache ?? resolve(tmpdir(), "appfw-pds-npm-cache");
await mkdir(npmCache, { recursive: true });

for (const subpath of [
  "./intelligence-presentation",
  "./intelligence-presentation-model",
  "./ix-recipes"
]) {
  if (!packageDocument.exports?.[subpath]) {
    throw new Error(`missing explicit IX package subpath: ${subpath}`);
  }
}
if (packageDocument.peerDependencies?.["@appfw/pds-ix-presentation-contract"] !== "0.2.0"
  || packageDocument.peerDependenciesMeta?.["@appfw/pds-ix-presentation-contract"]?.optional !== true) {
  throw new Error("the IX presentation contract must be an exact optional peer for core installation");
}
if (/"(?:dependencies|devDependencies|peerDependencies)"\s*:\s*\{[\s\S]*?file:/u.test(JSON.stringify(packageDocument))) {
  throw new Error("packed Web manifest may not contain a local file dependency");
}
const rootBarrels = await Promise.all([
  readFile(resolve(distRoot, "index.js"), "utf8"),
  readFile(resolve(distRoot, "index.d.ts"), "utf8")
]);
for (const ixModule of [
  "intelligence-presentation",
  "intelligence-presentation-model",
  "ix-recipes"
]) {
  if (rootBarrels.some((rootBarrel) => (
    rootBarrel.includes(`\"./${ixModule}`) || rootBarrel.includes(`'./${ixModule}`)
  ))) {
    throw new Error(`IX module must remain off the core root barrel: ${ixModule}`);
  }
}

const checkoutPathPatterns = [
  /(?:^|[\s"'`(])\/(?:Users|home|workspace|builds|private\/tmp|tmp)\/[^\s"'`)<>]+/m,
  /(?:^|[\s"'`(])\/opt\/atlassian\/pipelines\/agent\/build(?:\/[^\s"'`)<>]+)?/m,
  /[A-Za-z]:\\(?:Users|workspace|builds)\\[^\s"'`)<>]+/i,
  /[A-Za-z]:\/(?:Users|workspace|builds)\/[^\s"'`)<>]+/i
];

function leakedCheckoutPath(content) {
  const normalized = content.replace(/file:\/\//gi, "").replace(/\\\\/g, "\\");
  return checkoutPathPatterns.some((pattern) => pattern.test(normalized));
}

function exportTargets(value) {
  if (typeof value === "string") return [value];
  if (!value || typeof value !== "object") return [];
  return Object.values(value).flatMap(exportTargets);
}

const requiredFiles = [...new Set(
  Object.values(packageDocument.exports ?? {})
    .flatMap(exportTargets)
    .filter((target) => target.startsWith("./dist/"))
    .map((target) => target.slice("./dist/".length))
)].sort();

for (const file of requiredFiles) {
  const details = await lstat(resolve(distRoot, file));
  if (details.isSymbolicLink() || !details.isFile() || details.size === 0) {
    throw new Error(`missing regular package output: ${file}`);
  }
}

async function filesBelow(directory, base = distRoot) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const path = resolve(directory, entry.name);
    const details = await lstat(path);
    if (details.isSymbolicLink()) {
      throw new Error(`non-regular package entry is not allowed: ${relative(base, path)}`);
    }
    if (details.isDirectory()) return filesBelow(path, base);
    if (details.isFile()) return [relative(base, path)];
    throw new Error(`non-regular package entry is not allowed: ${relative(base, path)}`);
  }));
  return nested.flat();
}

function assertSafePayload(files, root) {
  const allowedFontFiles = new Set([
    "fonts/Poppins-Bold-latin.woff2",
    "fonts/Poppins-OFL.txt"
  ]);
  const rawSources = files.filter(
    (file) => file.endsWith(".tsx")
      || (file.endsWith(".ts") && !file.endsWith(".d.ts"))
      || /(^|\/)src\//.test(file)
      || file.endsWith(".map")
      || (file.endsWith(".woff2") && !allowedFontFiles.has(file))
      || (file.startsWith("fonts/") && !allowedFontFiles.has(file))
  );
  if (rawSources.length) {
    throw new Error(`raw framework source escaped into package: ${rawSources.join(", ")}`);
  }

  return Promise.all(files
    .filter((candidate) => /\.(js|css|d\.ts|json|md|txt)$/.test(candidate))
    .map(async (file) => {
      const content = await readFile(resolve(root, file), "utf8");
      if (
        content.includes("appfw_ui/pds_health")
        || content.includes('"sourcesContent"')
        || content.includes("sourceMappingURL=")
        || leakedCheckoutPath(content)
      ) {
        throw new Error(`framework checkout path or source escaped into ${file}`);
      }
    }));
}

const files = await filesBelow(distRoot);
await assertSafePayload(files, distRoot);

const packResult = JSON.parse(execFileSync(
  "npm",
  ["pack", "--json", "--dry-run", "--ignore-scripts"],
  {
    cwd: componentRoot,
    encoding: "utf8",
    env: { ...process.env, npm_config_cache: npmCache }
  }
));
const packedFiles = packResult[0]?.files?.map((entry) => entry.path) ?? [];
if (packResult[0]?.name !== packageDocument.name || packResult[0]?.version !== packageDocument.version) {
  throw new Error("npm pack metadata does not match the package manifest");
}

const missingPackedFiles = requiredFiles
  .map((file) => `dist/${file}`)
  .filter((file) => !packedFiles.includes(file));
if (missingPackedFiles.length > 0) {
  throw new Error(`required file missing from npm archive plan: ${missingPackedFiles.join(", ")}`);
}

const forbiddenPackedFiles = packedFiles.filter((file) => {
  if (file === "package.json" || file === "README.md") return false;
  if (!file.startsWith("dist/")) return true;
  const relativeFile = file.slice("dist/".length);
  const allowedFontFiles = new Set([
    "fonts/Poppins-Bold-latin.woff2",
    "fonts/Poppins-OFL.txt"
  ]);
  return relativeFile.endsWith(".tsx")
    || (relativeFile.endsWith(".ts") && !relativeFile.endsWith(".d.ts"))
    || relativeFile.endsWith(".map")
    || (relativeFile.endsWith(".woff2") && !allowedFontFiles.has(relativeFile))
    || (relativeFile.startsWith("fonts/") && !allowedFontFiles.has(relativeFile))
    || /(^|\/)src\//.test(relativeFile);
});
if (forbiddenPackedFiles.length > 0) {
  throw new Error(`forbidden file escaped into npm archive: ${forbiddenPackedFiles.join(", ")}`);
}

const styles = await readFile(resolve(distRoot, "styles.css"), "utf8");
if (!styles.startsWith('@import "./tokens.css";')) {
  throw new Error("packaged styles do not resolve packaged tokens");
}

function validateArchiveEntry(entry) {
  const normalized = entry.replace(/\/+$/, "");
  if (!normalized) return;
  const segments = normalized.split("/");
  if (
    normalized.includes("\\")
    || normalized.startsWith("/")
    || segments.includes(".")
    || segments.includes("..")
    || (normalized !== "package" && !normalized.startsWith("package/"))
  ) {
    throw new Error(`unsafe final archive entry: ${entry}`);
  }
}

let archiveVerification = null;
const archiveArgumentIndex = process.argv.indexOf("--archive");
if (archiveArgumentIndex >= 0) {
  const archivePath = process.argv[archiveArgumentIndex + 1];
  if (!archivePath) throw new Error("--archive requires a path");
  const archiveEntries = execFileSync("tar", ["-tzf", resolve(archivePath)], { encoding: "utf8" })
    .split(/\r?\n/)
    .filter(Boolean);
  if (archiveEntries.length === 0) throw new Error("final archive has no entries");
  archiveEntries.forEach(validateArchiveEntry);

  const archiveEntryDetails = execFileSync("tar", ["-tvzf", resolve(archivePath)], { encoding: "utf8" })
    .split(/\r?\n/)
    .filter(Boolean);
  for (const entry of archiveEntryDetails) {
    if (entry[0] !== "-" && entry[0] !== "d") {
      throw new Error(`non-regular final archive entry is not allowed: ${entry}`);
    }
  }

  const extractionRoot = await mkdtemp(resolve(tmpdir(), "appfw-pds-package-"));
  try {
    execFileSync("tar", ["-xzf", resolve(archivePath), "-C", extractionRoot]);
    const archiveRoot = resolve(extractionRoot, "package");
    const archiveFiles = await filesBelow(archiveRoot, archiveRoot);
    if (JSON.stringify([...archiveFiles].sort()) !== JSON.stringify([...packedFiles].sort())) {
      throw new Error("final archive contents differ from the verified npm pack plan");
    }
    await assertSafePayload(
      archiveFiles.filter((file) => file.startsWith("dist/")).map((file) => file.slice("dist/".length)),
      resolve(archiveRoot, "dist")
    );
    for (const file of archiveFiles) {
      const [sourceBytes, archiveBytes] = await Promise.all([
        readFile(resolve(componentRoot, file)),
        readFile(resolve(archiveRoot, file))
      ]);
      if (!archiveBytes.equals(sourceBytes)) {
        throw new Error(`final archive file bytes differ from verified package input: ${file}`);
      }
    }
    archiveVerification = { ok: true, path: resolve(archivePath), fileCount: archiveFiles.length };
  } finally {
    await rm(extractionRoot, { recursive: true, force: true });
  }
}

process.stdout.write(`${JSON.stringify({
  ok: true,
  fileCount: files.length,
  packedFileCount: packedFiles.length,
  archiveVerification,
  requiredFiles
}, null, 2)}\n`);
