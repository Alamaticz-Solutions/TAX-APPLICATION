#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const classifications = new Set([
  "present_same",
  "present_different",
  "present_yanked",
  "absent",
  "unknown"
]);
const shaPattern = /^[0-9a-f]{64}$/u;

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function sparseIndexPath(crateName) {
  const normalized = crateName.toLowerCase();
  if (normalized.length === 1) return `1/${normalized}`;
  if (normalized.length === 2) return `2/${normalized}`;
  if (normalized.length === 3) return `3/${normalized[0]}/${normalized}`;
  return `${normalized.slice(0, 2)}/${normalized.slice(2, 4)}/${normalized}`;
}

export function classifySparseIndex({ status, body, version, localSha256 }) {
  if (!shaPattern.test(localSha256)) throw new Error("local archive SHA-256 is malformed");
  if (status === 404) {
    return { classification: "absent", remote_checksum: null, remote_yanked: null,
      reason: "exact version is absent from the sparse index" };
  }
  if (status === 401 || status === 403) {
    return { classification: "unknown", remote_checksum: null, remote_yanked: null,
      reason: "registry authorization failed" };
  }
  if (status < 200 || status >= 300) {
    return { classification: "unknown", remote_checksum: null, remote_yanked: null,
      reason: `registry returned HTTP ${status}` };
  }
  const records = [];
  for (const line of body.split("\n").filter((value) => value.length > 0)) {
    try {
      const parsed = JSON.parse(line);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw new Error("not an object");
      }
      records.push(parsed);
    } catch {
      return { classification: "unknown", remote_checksum: null, remote_yanked: null,
        reason: "registry sparse-index response contains malformed or partial JSON" };
    }
  }
  const exact = records.filter((record) => record.vers === version);
  if (exact.length === 0) {
    return { classification: "absent", remote_checksum: null, remote_yanked: null,
      reason: "exact version is absent from the sparse index" };
  }
  if (exact.length !== 1) {
    return { classification: "unknown", remote_checksum: null, remote_yanked: null,
      reason: "registry sparse index contains duplicate exact-version records" };
  }
  const record = exact[0];
  if (!shaPattern.test(record.cksum ?? "")) {
    return { classification: "unknown", remote_checksum: null,
      remote_yanked: typeof record.yanked === "boolean" ? record.yanked : null,
      reason: "exact-version checksum is missing or malformed" };
  }
  if (typeof record.yanked !== "boolean") {
    return { classification: "unknown", remote_checksum: record.cksum, remote_yanked: null,
      reason: "exact-version yanked field is missing or non-boolean" };
  }
  if (record.yanked) {
    return { classification: "present_yanked", remote_checksum: record.cksum,
      remote_yanked: true, reason: "exact version exists but is yanked and must never be revived" };
  }
  if (record.cksum === localSha256) {
    return { classification: "present_same", remote_checksum: record.cksum,
      remote_yanked: false, reason: "exact non-yanked version matches the local archive checksum" };
  }
  return { classification: "present_different", remote_checksum: record.cksum,
    remote_yanked: false, reason: "exact non-yanked version has a different immutable checksum" };
}

function parseManifest(contents) {
  const section = contents.match(/(?:^|\n)\[package\]\s*\n([\s\S]*?)(?=\n\[[^\]]+\]|$)/u)?.[1];
  if (!section) throw new Error("manifest has no [package] section");
  const field = (name) => section.match(new RegExp(`^\\s*${name}\\s*=\\s*"([^"]+)"`, "mu"))?.[1];
  const dependencies = contents.split("\n").filter((line) => (
    /^\s*appfw[-_]/u.test(line) && /=/u.test(line)
  )).map((line) => line.trim());
  return { name: field("name"), version: field("version"), dependencies };
}

async function findCrateArchive(root, expectedName) {
  const entries = await readdir(root, { withFileTypes: true });
  for (const entry of entries) {
    const candidate = path.join(root, entry.name);
    if (entry.isDirectory()) {
      const nested = await findCrateArchive(candidate, expectedName).catch(() => null);
      if (nested) return nested;
    } else if (entry.isFile() && entry.name === expectedName) {
      return candidate;
    }
  }
  throw new Error(`cargo package did not produce ${expectedName}`);
}

async function packageCrate(repoRoot, spec) {
  const workRoot = await mkdtemp(path.join(os.tmpdir(), "appfw-proget-collision-"));
  try {
    const targetDir = path.join(workRoot, "target");
    execFileSync("cargo", [
      "package",
      "--manifest-path", spec.manifestPath,
      "--allow-dirty",
      "--no-verify"
    ], {
      cwd: repoRoot,
      env: { ...process.env, CARGO_TARGET_DIR: targetDir },
      stdio: ["ignore", "pipe", "pipe"]
    });
    const archivePath = await findCrateArchive(
      targetDir,
      `${spec.name}-${spec.version}.crate`
    );
    const bytes = await readFile(archivePath);
    return { sha256: sha256(bytes), bytes: bytes.length };
  } finally {
    await rm(workRoot, { recursive: true, force: true });
  }
}

function sanitizeIndexUrl(raw) {
  const withoutSparse = raw.startsWith("sparse+") ? raw.slice("sparse+".length) : raw;
  const value = new URL(withoutSparse);
  if (value.protocol !== "https:" || value.username || value.password || value.search || value.hash) {
    throw new Error("registry index must be a credential-free HTTPS URL");
  }
  return value;
}

async function observeIndex(indexUrl, spec, localSha256) {
  const url = new URL(sparseIndexPath(spec.name), indexUrl.href.endsWith("/")
    ? indexUrl
    : new URL(`${indexUrl.href}/`));
  const headers = {};
  if (process.env.PROGET_API_KEY) headers["X-ApiKey"] = process.env.PROGET_API_KEY;
  try {
    const response = await fetch(url, {
      method: "GET",
      headers,
      redirect: "error",
      signal: AbortSignal.timeout(15_000)
    });
    const body = await response.text();
    return classifySparseIndex({ status: response.status, body, version: spec.version, localSha256 });
  } catch {
    return { classification: "unknown", remote_checksum: null, remote_yanked: null,
      reason: "registry was unreachable or returned an unreadable response" };
  }
}

function parseCrate(value) {
  const match = value.match(/^([a-z0-9][a-z0-9_-]*)@([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)=(.+)$/u);
  if (!match) throw new Error(`invalid --crate value: ${value}`);
  return { name: match[1], version: match[2], manifestPath: match[3] };
}

function parseArgs(argv) {
  const result = {
    crates: [],
    artifact: "target/appfw/proget/ix-runtime-convergence-collisions.json",
    json: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--crate") result.crates.push(parseCrate(argv[++index]));
    else if (argv[index] === "--artifact") result.artifact = argv[++index];
    else if (argv[index] === "--json") result.json = true;
    else throw new Error(`unknown argument: ${argv[index]}`);
  }
  if (result.crates.length === 0) throw new Error("at least one --crate is required");
  const identities = new Set(result.crates.map(({ name, version }) => `${name}@${version}`));
  if (identities.size !== result.crates.length) throw new Error("duplicate --crate identity");
  return result;
}

export async function runCollisionCheck(repoRoot, specs, { indexUrl, observe = observeIndex,
  packageLocal = packageCrate } = {}) {
  const status = execFileSync("git", ["status", "--porcelain=v1", "--untracked-files=no"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  if (status) throw new Error("collision evidence requires an immutable clean tracked candidate");
  const candidateCommit = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  const candidateTree = execFileSync("git", ["rev-parse", "HEAD^{tree}"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  const packages = [];
  for (const spec of specs) {
    const manifestAbsolute = path.resolve(repoRoot, spec.manifestPath);
    if (!manifestAbsolute.startsWith(`${repoRoot}${path.sep}`)) throw new Error("manifest path escapes repository");
    const manifest = parseManifest(await readFile(manifestAbsolute, "utf8"));
    if (manifest.name !== spec.name || manifest.version !== spec.version) {
      throw new Error(`${spec.manifestPath} does not identify ${spec.name}@${spec.version}`);
    }
    const local = await packageLocal(repoRoot, spec);
    const remote = await observe(indexUrl, spec, local.sha256);
    if (!classifications.has(remote.classification)) throw new Error("observer returned unknown classification");
    packages.push({
      name: spec.name,
      version: spec.version,
      manifest_path: spec.manifestPath,
      manifest_dependency_metadata: manifest.dependencies,
      local_archive_sha256: local.sha256,
      local_archive_bytes: local.bytes,
      remote_checksum: remote.remote_checksum,
      remote_yanked: remote.remote_yanked,
      classification: remote.classification,
      reason: remote.reason,
      observed_at: new Date().toISOString()
    });
  }
  return {
    schema: "appfw.proget_collision_observation@1",
    ok: packages.every(({ classification }) => (
      classification === "absent" || classification === "present_same"
    )),
    candidate_commit: candidateCommit,
    candidate_tree: candidateTree,
    feed: indexUrl.origin + indexUrl.pathname,
    packages,
    publication_authorized: false,
    nonclaims: [
      "GET-only identity evidence does not authorize publishing, overwriting, or reviving a yanked package.",
      "Only absent or exact-checksum non-yanked present_same clears immutable identity collision proof."
    ]
  };
}

async function main() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const options = parseArgs(process.argv.slice(2));
  const indexUrl = sanitizeIndexUrl(
    process.env.APPFW_CARGO_REGISTRY_INDEX
      ?? "sparse+https://proget.pdsconnect.com/cargo/pds-app-framework-crates/"
  );
  const artifactPath = path.resolve(repoRoot, options.artifact);
  if (!artifactPath.startsWith(`${repoRoot}${path.sep}`)) throw new Error("artifact path escapes repository root");
  const result = await runCollisionCheck(repoRoot, options.crates, { indexUrl });
  await mkdir(path.dirname(artifactPath), { recursive: true });
  await writeFile(artifactPath, `${JSON.stringify(result, null, 2)}\n`, "utf8");
  process.stdout.write(`${JSON.stringify(result, null, options.json ? 2 : 0)}\n`);
  if (!result.ok) process.exitCode = 1;
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : "";
if (invokedPath === import.meta.url) {
  main().catch((error) => {
    process.stderr.write(`check-appfw-proget-collisions: ${error.message}\n`);
    process.exitCode = 1;
  });
}
