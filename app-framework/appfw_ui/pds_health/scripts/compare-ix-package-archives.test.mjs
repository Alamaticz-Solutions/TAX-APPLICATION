import assert from "node:assert/strict";
import test from "node:test";

import {
  assertFrozenBaseline,
  assertRepeatable,
  classifyInventory,
  frozenBaseline
} from "./compare-ix-package-archives.mjs";

function frozenBuild() {
  return {
    commit: "1e919fafe783b68cef4abc65f99d585be482b4e7",
    tree: "1".repeat(40),
    packages: Object.entries(frozenBaseline).map(([name, value], index) => ({
      name,
      version: value.version,
      raw_bytes: value.bytes,
      sha256: value.sha256,
      sha512: String(index).repeat(128),
      inventory: [{ path: "package.json", bytes: 20 + index, sha256: String(index).repeat(64) }]
    }))
  };
}

test("frozen I1 package identities, bytes, and SHA-256 pass", () => {
  assert.doesNotThrow(() => assertFrozenBaseline(frozenBuild()));
});

test("missing frozen package fails closed", () => {
  const candidate = frozenBuild();
  candidate.packages.pop();
  assert.throws(() => assertFrozenBaseline(candidate), /does not reproduce/u);
});

test("frozen raw byte or checksum drift fails closed", () => {
  const bytes = frozenBuild();
  bytes.packages[0].raw_bytes += 1;
  assert.throws(() => assertFrozenBaseline(bytes), /does not reproduce/u);
  const checksum = frozenBuild();
  checksum.packages[0].sha256 = "0".repeat(64);
  assert.throws(() => assertFrozenBaseline(checksum), /does not reproduce/u);
});

test("two exact builds are byte-repeatable", () => {
  const first = frozenBuild();
  assert.doesNotThrow(() => assertRepeatable(first, structuredClone(first), "baseline"));
});

test("repeat source, raw archive, or inventory drift fails closed", () => {
  const first = frozenBuild();
  const source = structuredClone(first);
  source.tree = "2".repeat(40);
  assert.throws(() => assertRepeatable(first, source, "candidate"), /source identity changed/u);
  const raw = structuredClone(first);
  raw.packages[0].sha256 = "0".repeat(64);
  assert.throws(() => assertRepeatable(first, raw, "candidate"), /not byte-repeatable/u);
  const inventory = structuredClone(first);
  inventory.packages[0].inventory[0].sha256 = "f".repeat(64);
  assert.throws(() => assertRepeatable(first, inventory, "candidate"), /not byte-repeatable/u);
});

test("inventory comparison classifies added, removed, changed, and unchanged", () => {
  const baseline = {
    inventory: [
      { path: "a.js", bytes: 1, sha256: "a".repeat(64) },
      { path: "b.js", bytes: 2, sha256: "b".repeat(64) },
      { path: "d.js", bytes: 4, sha256: "d".repeat(64) }
    ]
  };
  const candidate = {
    inventory: [
      { path: "a.js", bytes: 1, sha256: "a".repeat(64) },
      { path: "b.js", bytes: 3, sha256: "c".repeat(64) },
      { path: "c.js", bytes: 1, sha256: "e".repeat(64) }
    ]
  };
  assert.deepEqual(
    classifyInventory(baseline, candidate).map(({ path, classification }) => ({ path, classification })),
    [
      { path: "a.js", classification: "unchanged" },
      { path: "b.js", classification: "changed" },
      { path: "c.js", classification: "added" },
      { path: "d.js", classification: "removed" }
    ]
  );
});

test("same digest with different recorded byte length is still changed", () => {
  const baseline = { inventory: [{ path: "a", bytes: 1, sha256: "a".repeat(64) }] };
  const candidate = { inventory: [{ path: "a", bytes: 2, sha256: "a".repeat(64) }] };
  assert.equal(classifyInventory(baseline, candidate)[0].classification, "changed");
});

test("comparison preserves exact path bytes without normalization", () => {
  const baseline = { inventory: [{ path: "A/Value.js", bytes: 1, sha256: "a".repeat(64) }] };
  const candidate = { inventory: [{ path: "a/value.js", bytes: 1, sha256: "a".repeat(64) }] };
  assert.deepEqual(
    classifyInventory(baseline, candidate).map(({ classification }) => classification),
    ["removed", "added"]
  );
});
