import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cp, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function run(command, args, cwd, env = {}) {
  return spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, ...env }
  });
}

test("compiled package archive passes the final archive gate", async (t) => {
  const root = await mkdtemp(resolve(tmpdir(), "appfw-pds-package-contract-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(resolve(root, "scripts"), { recursive: true });
  await cp(resolve(componentRoot, "dist"), resolve(root, "dist"), { recursive: true });
  for (const file of ["package.json", "README.md", "scripts/check-package.mjs"]) {
    await cp(resolve(componentRoot, file), resolve(root, file));
  }

  const archiveRoot = resolve(root, "archives");
  const cacheRoot = resolve(root, "npm-cache");
  await mkdir(archiveRoot, { recursive: true });
  await mkdir(cacheRoot, { recursive: true });
  const pack = run(
    "npm",
    ["pack", "--json", "--ignore-scripts", "--pack-destination", archiveRoot],
    root,
    { npm_config_cache: cacheRoot }
  );
  assert.equal(pack.status, 0, pack.stderr || pack.stdout);
  const archive = resolve(archiveRoot, JSON.parse(pack.stdout)[0].filename);
  const check = run(process.execPath, ["scripts/check-package.mjs", "--archive", archive], root, {
    npm_config_cache: cacheRoot
  });
  assert.equal(check.status, 0, check.stderr || check.stdout);
});
