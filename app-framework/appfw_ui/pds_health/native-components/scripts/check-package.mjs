import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const manifest = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
const lock = JSON.parse(await readFile(new URL("package-lock.json", root), "utf8"));
assert.equal(manifest.name, "@appfw/pds-health-native");
assert.equal(manifest.version, "0.2.0");
assert.equal(manifest.packageManager, "npm@10.8.2");
assert.deepEqual(Object.keys(manifest.peerDependencies).sort(), ["@appfw/pds-ix-presentation-contract", "react", "react-native"]);
assert.equal(manifest.peerDependencies["@appfw/pds-ix-presentation-contract"], "0.2.0");
assert.equal(manifest.peerDependencies.react, ">=19.2.3 <20");
assert.equal(manifest.peerDependencies["react-native"], ">=0.85.3 <0.86");
assert.equal(
  manifest.scripts["proof:detached-consumer"],
  "node ../scripts/prove-native-packages-from-clean-checkout.mjs"
);
assert.doesNotMatch(JSON.stringify(manifest), /file:\.\.\/|jest-expo|\bexpo\b/i);
for (const target of [
  manifest.main,
  manifest.types,
  manifest.exports["./design-data"].import,
  manifest.exports["./design-data"].types,
  manifest.exports["./ix-recipes"].import,
  manifest.exports["./ix-recipes"].types,
  manifest.exports["./ix-recipe-projection"].import,
  manifest.exports["./ix-recipe-projection"].types
]) {
  const bytes = await readFile(new URL(target.replace(/^\.\//, ""), root));
  assert.ok(bytes.length > 0, target);
  assert.doesNotMatch(bytes.toString("utf8"), /\.\.\/(?:tokens|ix-presentation-contract)|\/src\//, target);
}
const registryEntries = [];
const integrityExceptions = [];
for (const [path, metadata] of Object.entries(lock.packages)) {
  if (!path.startsWith("node_modules/")) continue;
  if (metadata.link === true || metadata.resolved?.startsWith("file:")) {
    integrityExceptions.push(path);
    continue;
  }
  registryEntries.push(path);
  assert.equal(typeof metadata.version, "string", `${path} must bind a version`);
  assert.match(metadata.resolved ?? "", /^https:\/\/registry\.npmjs\.org\//, `${path} must bind the registry artifact`);
  assert.match(metadata.integrity ?? "", /^sha512-/, `${path} must bind artifact integrity`);
}
assert.deepEqual(integrityExceptions, []);
assert.ok(registryEntries.length > 0);
process.stdout.write(`${JSON.stringify({
  ok: true,
  name: manifest.name,
  version: manifest.version,
  lock: {
    lockfileVersion: lock.lockfileVersion,
    packageManager: manifest.packageManager,
    registryEntries: registryEntries.length,
    integrityExceptions
  }
})}\n`);
