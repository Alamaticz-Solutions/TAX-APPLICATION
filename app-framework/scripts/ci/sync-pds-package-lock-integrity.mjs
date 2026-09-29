#!/usr/bin/env node
/**
 * Refresh package-lock integrity for the local PDS components tarball.
 *
 * The release gate and wave3 PR SPA build pack @appfw/pds-health-components
 * into target/appfw/packages before `npm ci`. npm pack headers can change the
 * archive digest even when package contents are equivalent, so the consumer
 * lock must be synced to the packed archive before install.
 */
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

function usage() {
  console.error(
    "usage: sync-pds-package-lock-integrity.mjs --lock <package-lock.json> --archive <pkg.tgz>"
  );
  process.exit(2);
}

function argValue(flag) {
  const index = process.argv.indexOf(flag);
  if (index < 0 || index + 1 >= process.argv.length) {
    return null;
  }
  return process.argv[index + 1];
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../..");
const lockPath = resolve(argValue("--lock") || usage());
const archivePath = resolve(argValue("--archive") || usage());

const archiveBytes = await readFile(archivePath);
const integrity = `sha512-${createHash("sha512").update(archiveBytes).digest("base64")}`;

const lock = JSON.parse(await readFile(lockPath, "utf8"));
const archiveName = archivePath.split(/[\\/]/).at(-1);
let updated = 0;

for (const [key, meta] of Object.entries(lock.packages || {})) {
  const resolved = String(meta?.resolved || "");
  const matches =
    key === "node_modules/@appfw/pds-health-components" ||
    resolved.endsWith(archiveName) ||
    resolved.includes("appfw-pds-health-components-");
  if (!matches) {
    continue;
  }
  if (meta.integrity !== integrity) {
    meta.integrity = integrity;
    updated += 1;
  }
}

const legacy = lock.dependencies?.["@appfw/pds-health-components"];
if (legacy && legacy.integrity !== integrity) {
  legacy.integrity = integrity;
  updated += 1;
}

if (updated > 0) {
  await writeFile(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      lock: lockPath.replace(`${repoRoot}/`, ""),
      archive: archivePath.replace(`${repoRoot}/`, ""),
      integrity,
      updated
    },
    null,
    2
  )
);
