import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const manifest = JSON.parse(await readFile(new URL("package.json", root), "utf8"));

assert.equal(manifest.name, "@appfw/pds-ix-presentation-contract");
assert.equal(manifest.version, "0.2.0");
assert.deepEqual(manifest.dependencies, undefined);
assert.deepEqual(manifest.peerDependencies, undefined);
for (const target of [
  manifest.exports["."].types,
  manifest.exports["."].import,
  manifest.exports["./recipes"].types,
  manifest.exports["./recipes"].import,
  manifest.exports["./schema"],
  manifest.exports["./recipe-registry"],
  manifest.exports["./recipe-registry-schema"],
  manifest.exports["./recipe-registration-schema"],
  manifest.exports["./fixture"]
]) {
  const bytes = await readFile(new URL(target.replace(/^\.\//, ""), root));
  assert.ok(bytes.length > 0, target);
}
process.stdout.write(`${JSON.stringify({ ok: true, name: manifest.name, version: manifest.version })}\n`);
