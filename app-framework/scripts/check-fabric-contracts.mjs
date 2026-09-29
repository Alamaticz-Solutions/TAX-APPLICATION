#!/usr/bin/env node
// Fabric contract check: validates every fixture case against the nine fabric
// contract schemas (closed-schema structure, business rules, and record
// digests) and retains a JSON evidence artifact.
//
//   node scripts/check-fabric-contracts.mjs --json
//
// Exit code 0 when every case matches its expectation; 1 otherwise.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  listContractIds,
  runCases
} from "./fabric-contracts/fabric-contracts.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const artifactPath = join(repoRoot, "target", "appfw", "fabric-contracts-check.json");

const results = runCases();
const failures = results.filter((entry) => !entry.ok);

const byContract = listContractIds().map((contract) => {
  const forContract = results.filter((entry) => entry.contract === contract);
  return {
    contract,
    cases: forContract.length,
    valid_cases: forContract.filter((entry) => entry.expect === "valid").length,
    invalid_cases: forContract.filter((entry) => entry.expect === "invalid").length,
    ok: forContract.every((entry) => entry.ok)
  };
});

const artifact = {
  command: "check-fabric-contracts",
  schema: "appfw_fabric_contracts_check@1",
  ok: failures.length === 0,
  contract_count: listContractIds().length,
  case_count: results.length,
  contracts: byContract,
  failures: failures.map((entry) => ({
    contract: entry.contract,
    name: entry.name,
    expect: entry.expect,
    outcome: entry.outcome,
    error: entry.error
  })),
  artifact: "target/appfw/fabric-contracts-check.json"
};

mkdirSync(dirname(artifactPath), { recursive: true });
writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`);

if (process.argv.includes("--json")) {
  process.stdout.write(`${JSON.stringify(artifact)}\n`);
} else {
  process.stdout.write(
    `fabric contracts: ${artifact.ok ? "ok" : "FAILED"} (${artifact.case_count} cases across ${artifact.contract_count} contracts)\n`
  );
  for (const failure of artifact.failures) {
    process.stdout.write(
      `  FAIL ${failure.contract} / ${failure.name}: expected ${failure.expect}, got ${failure.outcome}${failure.error ? ` — ${failure.error}` : ""}\n`
    );
  }
}

process.exit(artifact.ok ? 0 : 1);
