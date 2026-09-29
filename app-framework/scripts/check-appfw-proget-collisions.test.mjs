import assert from "node:assert/strict";
import test from "node:test";

import {
  classifySparseIndex,
  sparseIndexPath
} from "./check-appfw-proget-collisions.mjs";

const local = "a".repeat(64);
const remote = "b".repeat(64);
const version = "0.2.0";
const line = (overrides = {}) => JSON.stringify({
  name: "appfw-runtime",
  vers: version,
  deps: [],
  cksum: local,
  features: {},
  yanked: false,
  ...overrides
});
const classify = (status, body) => classifySparseIndex({
  status,
  body,
  version,
  localSha256: local
});

test("Cargo sparse-index paths are canonical", () => {
  assert.equal(sparseIndexPath("a"), "1/a");
  assert.equal(sparseIndexPath("ab"), "2/ab");
  assert.equal(sparseIndexPath("abc"), "3/a/abc");
  assert.equal(sparseIndexPath("appfw-runtime"), "ap/pf/appfw-runtime");
});

test("exact checksum plus explicit non-yanked is present_same", () => {
  assert.deepEqual(classify(200, line()), {
    classification: "present_same",
    remote_checksum: local,
    remote_yanked: false,
    reason: "exact non-yanked version matches the local archive checksum"
  });
});

test("non-yanked checksum mismatch is present_different", () => {
  const result = classify(200, line({ cksum: remote }));
  assert.equal(result.classification, "present_different");
  assert.equal(result.remote_yanked, false);
});

test("matching-checksum yanked identity is present_yanked", () => {
  const result = classify(200, line({ yanked: true }));
  assert.equal(result.classification, "present_yanked");
  assert.equal(result.remote_checksum, local);
});

test("different-checksum yanked identity remains present_yanked", () => {
  const result = classify(200, line({ cksum: remote, yanked: true }));
  assert.equal(result.classification, "present_yanked");
  assert.equal(result.remote_checksum, remote);
});

test("404 and valid index without exact version are absent", () => {
  assert.equal(classify(404, "").classification, "absent");
  assert.equal(classify(200, line({ vers: "0.1.2" })).classification, "absent");
});

test("missing yanked fails closed as unknown", () => {
  const record = JSON.parse(line());
  delete record.yanked;
  assert.equal(classify(200, JSON.stringify(record)).classification, "unknown");
});

test("non-boolean yanked fails closed as unknown", () => {
  assert.equal(classify(200, line({ yanked: "false" })).classification, "unknown");
});

test("duplicate exact records fail closed as unknown", () => {
  assert.equal(classify(200, `${line()}\n${line()}`).classification, "unknown");
});

test("malformed or partial sparse-index record fails closed as unknown", () => {
  assert.equal(classify(200, `${line()}\n{"vers":`).classification, "unknown");
  assert.equal(classify(200, "null").classification, "unknown");
});

test("missing or malformed checksum fails closed as unknown", () => {
  const missing = JSON.parse(line());
  delete missing.cksum;
  assert.equal(classify(200, JSON.stringify(missing)).classification, "unknown");
  assert.equal(classify(200, line({ cksum: "ABC".repeat(21) + "A" })).classification, "unknown");
});

test("unauthorized, forbidden, and server failure are unknown", () => {
  assert.equal(classify(401, "").classification, "unknown");
  assert.equal(classify(403, "").classification, "unknown");
  assert.equal(classify(503, "").classification, "unknown");
});

test("malformed local archive checksum is rejected before classification", () => {
  assert.throws(() => classifySparseIndex({
    status: 200,
    body: line(),
    version,
    localSha256: "not-a-checksum"
  }), /local archive SHA-256/u);
});
