import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  buildLedger,
  identities,
  loadGitContext,
  validateLedger
} from "./check-ix-runtime-convergence-source-ledger.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const context = loadGitContext(repoRoot, {
  liveRemoteCommit: identities.i2,
  includeLedgerPath: true
});
const baseline = buildLedger(context);

function changed(mutator) {
  const value = structuredClone(baseline);
  mutator(value);
  return value;
}

test("Git-derived 52 + 7 + 45 ledger and candidate coverage pass", () => {
  const result = validateLedger(baseline, context);
  assert.equal(result.ok, true);
  assert.deepEqual(result.counts, {
    reviewed_source: 52,
    reviewed_planning: 7,
    dependency_closure: 45,
    historical_total: 104
  });
  assert.equal(result.leaf_root_count, 69);
  assert.equal(result.integration_roots_changed, 0);
});

test("missing reviewed source record fails closed", () => {
  assert.throws(
    () => validateLedger(changed((value) => value.reviewed_source_paths.pop()), context),
    /exactly 52/u
  );
});

test("overlap between historical collections fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.dependency_closure_paths[0].path = value.reviewed_source_paths[0].path;
  }), context), /104 globally unique|dependency closure paths/u);
});

test("wrong reviewed commit identity fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.reviewed_source_paths[0].source_commit_oid = identities.i1;
  }), context), /reviewed source identity mismatch/u);
});

test("wrong source blob identity fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.reviewed_source_paths[0].source_blob_oid = "0".repeat(40);
  }), context), /reviewed source identity mismatch/u);
});

test("unknown disposition vocabulary fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.reviewed_source_paths[0].disposition = "copy_if_easy";
  }), context), /unknown disposition/u);
});

test("delivery credit is forbidden", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.reviewed_source_paths[0].delivery_credit = "implemented";
  }), context), /delivery credit must be none/u);
});

test("dependency required-by set may not be empty", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.dependency_closure_paths[0].required_by_reviewed_paths = [];
  }), context), /required_by_reviewed_paths/u);
});

test("dependency required-by set may not point outside reviewed source", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.dependency_closure_paths[0].required_by_reviewed_paths = ["README.md"];
  }), context), /required_by_reviewed_paths/u);
});

test("byte-preserve record with changed bytes fails closed", () => {
  const candidate = changed((value) => {
    const record = value.reviewed_source_paths.find(({ disposition }) => disposition === "byte_preserve");
    record.target_blob_oid = "0".repeat(40);
  });
  assert.throws(() => validateLedger(candidate, context), /stale target blob|byte-preserve/u);
});

test("missing I2 overlay annotation fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    const record = value.reviewed_source_paths.find(({ i2_overlay: overlay }) => overlay);
    delete record.i2_overlay;
  }), context), /I2 overlay annotation mismatch/u);
});

test("moved live I2 ref fails closed", () => {
  assert.throws(() => validateLedger(baseline, {
    ...context,
    liveRemoteCommit: identities.i1
  }), /live I2 remote ref/u);
});

test("wrong I2 parent or tree fails closed", () => {
  assert.throws(() => validateLedger(baseline, {
    ...context,
    i2Parents: [identities.integrationBase]
  }), /I2 parent\/tree/u);
});

test("stale comprehensive review digest fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.i2_overlay_receipt.comprehensive_review.artifact_sha256 = "0".repeat(64);
  }), context), /review receipt is stale/u);
});

test("missing candidate coverage fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.candidate_diff_coverage.pop();
  }), context), /candidate diff coverage/u);
});

test("duplicate candidate coverage fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.candidate_diff_coverage.push(structuredClone(value.candidate_diff_coverage[0]));
  }), context), /duplicate paths/u);
});

test("stale admitted root or candidate blob fails closed", () => {
  assert.throws(() => validateLedger(changed((value) => {
    value.candidate_diff_coverage[0].admitted_root = "README.md";
  }), context), /admitted root/u);
  assert.throws(() => validateLedger(changed((value) => {
    value.candidate_diff_coverage[0].candidate_blob_oid = "0".repeat(40);
  }), context), /stale coverage blob/u);
});

test("lane-only path cannot claim historical coverage", () => {
  const candidate = changed((value) => {
    const record = value.candidate_diff_coverage.find(({ coverage_kind: kind }) => (
      kind === "lane_only_change"
    ));
    record.coverage_kind = "historical_record";
    record.historical_collection = "reviewed_source_paths";
    record.historical_path = record.path;
  });
  assert.throws(() => validateLedger(candidate, context), /lane-only coverage vocabulary/u);
});

test("generator target and Cargo lock custody are exact", () => {
  assert.throws(() => validateLedger(changed((value) => {
    const generator = value.dependency_closure_paths.find(({ path: item }) => (
      item === "scripts/check_app_gen_backend_equivalence.sh"
    ));
    generator.target_blob_oid = "0".repeat(40);
  }), context), /stale dependency target blob|generator correction/u);
  assert.throws(() => validateLedger(changed((value) => {
    const lock = value.dependency_closure_paths.find(({ path: item }) => item === "Cargo.lock");
    lock.custody = "leaf_a";
  }), context), /Cargo.lock must remain pending Integration/u);
});

test("unrelated I1 ancestry fails closed", () => {
  assert.throws(() => validateLedger(baseline, {
    ...context,
    i1IsCandidateAncestor: true
  }), /unrelated\/non-current-main historical ancestry/u);
});
