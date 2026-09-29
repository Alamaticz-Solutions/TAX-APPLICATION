import assert from "node:assert/strict";
import test from "node:test";

import {
  CONTRACTS,
  ContractError,
  listContractIds,
  loadCases,
  loadSchema,
  prepareCaseRecord,
  recordDigest,
  runCases,
  validateRecord
} from "./fabric-contracts.mjs";

test("all nine contracts load and declare closed schemas", () => {
  const ids = listContractIds();
  assert.equal(ids.length, 9);
  for (const id of ids) {
    const schema = loadSchema(id);
    assert.equal(schema.additionalProperties, false, `${id} must be closed`);
    assert.equal(schema.properties.schema.const, id, `${id} schema const must match`);
    assert.ok(
      schema.required.includes("record_digest"),
      `${id} must require record_digest`
    );
  }
});

test("every fixture case matches its expectation", () => {
  const results = runCases();
  const failures = results.filter((entry) => !entry.ok);
  assert.deepEqual(
    failures,
    [],
    `fixture cases diverged: ${JSON.stringify(failures, null, 2)}`
  );
});

test("fixture coverage spans every contract with valid and invalid cases", () => {
  const cases = loadCases().cases;
  for (const id of listContractIds()) {
    const forContract = cases.filter((entry) => entry.contract === id);
    assert.ok(
      forContract.some((entry) => entry.expect === "valid"),
      `${id} needs at least one valid fixture`
    );
    assert.ok(
      forContract.some((entry) => entry.expect === "invalid"),
      `${id} needs at least one invalid fixture`
    );
  }
});

test("digest tampering is detected", () => {
  const entry = loadCases().cases.find(
    (candidate) =>
      candidate.contract === "fabric_app_registration@1" &&
      candidate.expect === "valid"
  );
  const record = prepareCaseRecord(entry);
  validateRecord("fabric_app_registration@1", record);
  record.purpose_summary = "Tampered after signing.";
  assert.throws(
    () => validateRecord("fabric_app_registration@1", record),
    (error) => error instanceof ContractError && error.code === "digest_mismatch"
  );
});

test("record digest is stable across key order", () => {
  const a = { schema: "x", alpha: 1, beta: [1, 2, { c: true }] };
  const b = { beta: [1, 2, { c: true }], alpha: 1, schema: "x" };
  assert.equal(recordDigest(a), recordDigest(b));
});

test("schema field must match the contract it is validated against", () => {
  const entry = loadCases().cases.find(
    (candidate) =>
      candidate.contract === "framework_request@1" && candidate.expect === "valid"
  );
  const record = prepareCaseRecord(entry);
  assert.throws(
    () => validateRecord("fabric_command_proposal@1", record),
    ContractError
  );
});

test("contract file names follow the versioned naming convention", () => {
  for (const [id, file] of Object.entries(CONTRACTS)) {
    assert.match(file, /\.v1\.schema\.json$/, `${id} file name must be versioned`);
  }
});
