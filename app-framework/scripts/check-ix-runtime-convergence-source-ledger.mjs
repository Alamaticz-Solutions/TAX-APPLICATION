#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const identities = Object.freeze({
  currentMain: "32160570b7d8d5b8e813e95da053918cba878af8",
  integrationBase: "9ea26157a2bed29424e1052a909303985ba466bd",
  reviewedSource: "8e5b33119b84fa23eee1f1d3534481101afed2a9",
  i1: "1e919fafe783b68cef4abc65f99d585be482b4e7",
  i2: "bb9b8423bbd72003c050079c3f27a279c08d0fd5",
  i2Tree: "4bb816efd6112051ae49db317e933a227b5beeb7",
  i2Ref: "refs/heads/integrate/ix-eight-vignette-foundation-r1",
  correctionCommit: "44770f43f61fa52c928b339d0677091f5b593aef",
  reviewSha256: "84d0d9773b5858f0415949568e9353a4cf0be159668ce519b0099f7587aa4d48",
  generatorBlob: "6ee9d06d407a4d6c8039beb382a98f43d7fa19ad"
});

export const overlayPaths = Object.freeze({
  "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json": [
    "9312fb477ec2c5a29b154289838fc0307d04f053",
    "df29219f619638f0ed114956a1afa988b3f6c458"
  ],
  "docs/specs/ix-eight-vignette-shared-foundation-r1.md": [
    "1f835fe77a5033dbb0c7e3144f04272b46ccb420",
    "527add52e45ab0cf72c64786a738bf168ebb2d4d"
  ],
  "scripts/check-pds-ix-runtime-contract.mjs": [
    "1d330cd32b1276134bad1882f7c1e532a2f3a549",
    "7f7813fba98e610ec5144a06efac4e8f0bd920e9"
  ],
  "scripts/check-pds-ix-runtime-contract.test.mjs": [
    "fdd195c3f90e90be71a4dbfc47311d104e9340fa",
    "ec392772ac64e4484c1e394a4139af627f3b8ce5"
  ]
});

export const dependencyPaths = Object.freeze([
  "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.schema.json",
  "appfw_runtime/contracts/pds_health/working-brief.presentation.json",
  "appfw_runtime/src/ix/presentation.rs",
  "appfw_ui/pds_health/catalog-app/ix-reference.html",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/AdaptiveCompositionRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/AdaptiveInformationLensRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/AmbientAgentContinuityRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/AnalyzeWhyRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/AttentionStewardshipRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/ContextualConversationRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/IxReferenceApp.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/SituationToStrategyRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/WorkingGoalPlanRecipe.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/components.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/contract.ts",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/fixtures.ts",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/ix-reference.css",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/main.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/useFixtureRun.ts",
  "appfw_ui/pds_health/components/src/intelligence-presentation-model.ts",
  "appfw_ui/pds_health/ix-presentation-contract/.gitignore",
  "appfw_ui/pds_health/ix-presentation-contract/fixtures/working-brief.presentation.json",
  "appfw_ui/pds_health/ix-presentation-contract/schema/pds.ix.presentation.v1.schema.json",
  "appfw_ui/pds_health/ix-presentation-contract/scripts/contract.test.mjs",
  "appfw_ui/pds_health/native-components/.gitignore",
  "appfw_ui/pds_health/native-components/babel.config.cjs",
  "appfw_ui/pds_health/native-components/scripts/native-components.rntl.test.tsx",
  "appfw_ui/pds_health/native-components/scripts/package-boundary.test.mjs",
  "appfw_ui/pds_health/native-components/src/design-data.ts",
  "appfw_ui/pds_health/native-components/src/generated/pdsNativeDesignData.ts",
  "appfw_ui/pds_health/native-components/tsconfig.build.json",
  "appfw_ui/pds_health/native-components/tsconfig.json",
  "appfw_ui/pds_health/tokens/generate-pds-native-design-data.mjs",
  "appfw_ui/pds_health/tokens/pds-native-design-data.test.mjs",
  "appfw_ui/pds_health/tokens/pdsNativeDesignData.json",
  "appfw_ui/pds_health/tokens/pdsNativeDesignData.ts",
  "Cargo.lock",
  "appfw_runtime/Cargo.toml",
  "appfw_runtime/src/lib.rs",
  "appfw_ui/pds_health/catalog-app/src/App.tsx",
  "appfw_ui/pds_health/catalog-app/src/app.css",
  "appfw_ui/pds_health/catalog-app/vite.config.ts",
  "appfw_ui/pds_health/components/src/styles.css",
  "appfw_ui/pds_health/tokens/tokens.dtcg.json",
  "scripts/check_app_gen_backend_equivalence.sh"
]);

export const admittedRoots = Object.freeze([
  "CHANGELOG.md",
  "appfw_cli/Cargo.toml",
  "appfw_provider_mongo/Cargo.toml",
  "appfw_provider_mssql/Cargo.toml",
  "appfw_provider_neo4j/Cargo.toml",
  "appfw_provider_postgres/Cargo.toml",
  "appfw_provider_snowflake/Cargo.toml",
  "appfw_runtime/Cargo.toml",
  "appfw_runtime/contracts/pds_health/**",
  "appfw_runtime/src/auth.rs",
  "appfw_runtime/src/cors.rs",
  "appfw_runtime/src/ingress.rs",
  "appfw_runtime/src/ix/**",
  "appfw_runtime/src/lib.rs",
  "appfw_runtime/src/routing.rs",
  "appfw_runtime/tests/fixtures/ix/**",
  "appfw_runtime/tests/ix_transport_orchestration.rs",
  "appfw_ui/pds_health/CHANGELOG.md",
  "appfw_ui/pds_health/catalog-app/ix-reference.html",
  "appfw_ui/pds_health/catalog-app/scripts/check-ix-reference.mjs",
  "appfw_ui/pds_health/catalog-app/src/App.tsx",
  "appfw_ui/pds_health/catalog-app/src/app.css",
  "appfw_ui/pds_health/catalog-app/src/examples.tsx",
  "appfw_ui/pds_health/catalog-app/src/ix-reference/**",
  "appfw_ui/pds_health/catalog-app/src/lib/snippets.ts",
  "appfw_ui/pds_health/catalog-app/vite.config.ts",
  "appfw_ui/pds_health/components/README.md",
  "appfw_ui/pds_health/components/package.json",
  "appfw_ui/pds_health/components/package-lock.json",
  "appfw_ui/pds_health/components/scripts/intelligence-presentation-contract.test.mjs",
  "appfw_ui/pds_health/components/scripts/ix-recipes.test.mjs",
  "appfw_ui/pds_health/components/src/catalog.ts",
  "appfw_ui/pds_health/components/src/index.ts",
  "appfw_ui/pds_health/components/src/intelligence-presentation-model.ts",
  "appfw_ui/pds_health/components/src/intelligence-presentation.tsx",
  "appfw_ui/pds_health/components/src/ix-recipes.tsx",
  "appfw_ui/pds_health/components/src/styles.css",
  "appfw_ui/pds_health/components/vite.config.mjs",
  "appfw_ui/pds_health/ix-presentation-contract/**",
  "appfw_ui/pds_health/native-components/**",
  "appfw_ui/pds_health/reference/catalog.json",
  "appfw_ui/pds_health/reference/index.html",
  "appfw_ui/pds_health/scripts/compare-ix-package-archives.mjs",
  "appfw_ui/pds_health/scripts/compare-ix-package-archives.test.mjs",
  "appfw_ui/pds_health/scripts/prove-native-packages-from-clean-checkout.mjs",
  "appfw_ui/pds_health/tokens/generate-pds-native-design-data.mjs",
  "appfw_ui/pds_health/tokens/pds-native-design-data.test.mjs",
  "appfw_ui/pds_health/tokens/pdsNativeDesignData.json",
  "appfw_ui/pds_health/tokens/pdsNativeDesignData.ts",
  "appfw_ui/pds_health/tokens/tokens.dtcg.json",
  "docs/specs/ix-eight-vignette-shared-foundation-r1.md",
  "docs/specs/ix-runtime-convergence-source-ledger-r1.json",
  "docs/runtime/ai-chat-search.md",
  "examples/products/crm/Cargo.lock",
  "scripts/check-appfw-proget-collisions.mjs",
  "scripts/check-appfw-proget-collisions.test.mjs",
  "scripts/check-ix-runtime-consumers.mjs",
  "scripts/check-ix-runtime-consumers.test.mjs",
  "scripts/check-ix-runtime-convergence-source-ledger.mjs",
  "scripts/check-ix-runtime-convergence-source-ledger.test.mjs",
  "scripts/check-pds-components.mjs",
  "scripts/check-pds-ix-runtime-contract.mjs",
  "scripts/check-pds-ix-runtime-contract.test.mjs",
  "scripts/check_app_gen_backend_equivalence.sh",
  "appfw_ui/pds_health/catalog-app/tsconfig.json",
  "docs/frontend/pds-health-design-system.md",
  "appfw_ui/pds_health/catalog-app/scripts/ix-reference-spec-source.mjs",
  "appfw_ui/pds_health/catalog-app/scripts/ix-reference-spec-source.test.mjs",
  "appfw_ui/pds_health/components/scripts/check-package.mjs"
]);

const ledgerSelfPath = "docs/specs/ix-runtime-convergence-source-ledger-r1.json";
const integrationOnlyPaths = new Set([
  "Cargo.lock",
  "appfw_runtime/src/chat.rs",
  "docs/specs/ix-eight-vignette-cross-channel-r1.md"
]);
const finalReconcileDependencies = new Set(dependencyPaths.slice(36, 44));
const fullOid = /^[0-9a-f]{40}$/u;

function git(repoRoot, args, { optional = false } = {}) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  if (result.status !== 0) {
    if (optional) return null;
    throw new Error(`git ${args.join(" ")} failed: ${result.stderr.trim()}`);
  }
  return result.stdout.trim();
}

function lines(value) {
  return value ? value.split("\n").filter(Boolean) : [];
}

function blobAt(repoRoot, commit, relativePath) {
  return git(repoRoot, ["rev-parse", `${commit}:${relativePath}`], { optional: true });
}

function workingBlob(repoRoot, relativePath) {
  return git(repoRoot, ["hash-object", "--", relativePath], { optional: true });
}

function rootForPath(relativePath) {
  const matches = admittedRoots.filter((root) => (
    root.endsWith("/**")
      ? relativePath.startsWith(root.slice(0, -2))
      : relativePath === root
  ));
  if (matches.length !== 1) {
    throw new Error(`${relativePath} must match exactly one admitted Leaf A root; got ${matches.length}`);
  }
  return matches[0];
}

function requiredBy(reviewedPaths, dependencyPath) {
  let prefix = "appfw_runtime/";
  if (dependencyPath.includes("catalog-app/")) prefix = "appfw_ui/pds_health/catalog-app/";
  else if (dependencyPath.includes("components/")) prefix = "appfw_ui/pds_health/components/";
  else if (dependencyPath.includes("native-components/")) prefix = "appfw_ui/pds_health/native-components/";
  else if (dependencyPath.includes("tokens/")) prefix = "appfw_ui/pds_health/native-components/";
  else if (dependencyPath.includes("ix-presentation-contract/")) {
    prefix = "appfw_ui/pds_health/ix-presentation-contract/";
  }
  const matches = reviewedPaths.filter((item) => item.startsWith(prefix));
  return (matches.length > 0 ? matches : reviewedPaths.slice(0, 1)).slice(0, 8);
}

function recordBase({ path: relativePath, sourceKind, sourceCommit, sourceBlob,
  currentMainBlob, disposition, targetBlob, custody, rationale }) {
  return {
    path: relativePath,
    source_kind: sourceKind,
    source_commit_oid: sourceCommit,
    source_blob_oid: sourceBlob,
    current_main_blob_oid: currentMainBlob,
    disposition,
    target_path: disposition === "reject_from_candidate" ? null : relativePath,
    target_blob_oid: targetBlob,
    preserved_obligations: [
      "Preserve the reviewed modular IX/PDS contract or prove equivalent-or-stronger reconciliation."
    ],
    superseded_claims: [],
    custody,
    decision_authority: "0fd733c8c156c86a37b6e47922f9ffc0abed3692aee6a6fa79e037d016607881",
    review_evidence: "07a364ee82fdba65d38d66bb3afe0038b305e9b8",
    rationale,
    delivery_credit: "none"
  };
}

export function loadGitContext(repoRoot, { liveRemoteCommit = null, includeLedgerPath = false } = {}) {
  const reviewedSourcePaths = lines(git(repoRoot, [
    "diff", "--name-only", `${identities.integrationBase}..${identities.reviewedSource}`
  ])).sort();
  const aggregatePaths = lines(git(repoRoot, [
    "diff", "--name-only", `${identities.integrationBase}..${identities.i1}`
  ]));
  const reviewedSet = new Set(reviewedSourcePaths);
  const reviewedPlanningPaths = aggregatePaths.filter((item) => !reviewedSet.has(item)).sort();
  const candidatePaths = new Set(lines(git(repoRoot, [
    "diff", "--name-only", identities.currentMain
  ])));
  for (const item of lines(git(repoRoot, ["ls-files", "--others", "--exclude-standard"]))) {
    candidatePaths.add(item);
  }
  if (includeLedgerPath) candidatePaths.add(ledgerSelfPath);

  const i2Paths = lines(git(repoRoot, [
    "diff-tree", "--no-commit-id", "--name-only", "-r", identities.i2
  ]));
  const i2Parents = lines(git(repoRoot, ["show", "-s", "--format=%P", identities.i2]));
  const candidatePathList = [...candidatePaths].sort();
  const candidateBlobs = Object.fromEntries(candidatePathList.map((item) => [
    item,
    item === ledgerSelfPath ? "SELF_REFERENTIAL_LEDGER" : workingBlob(repoRoot, item)
  ]));

  return {
    repoRoot,
    reviewedSourcePaths,
    reviewedPlanningPaths,
    candidatePaths: candidatePathList,
    candidateBlobs,
    liveRemoteCommit,
    i2Paths,
    i2Parents,
    i2Tree: git(repoRoot, ["show", "-s", "--format=%T", identities.i2]),
    mergeBase: git(repoRoot, ["merge-base", identities.currentMain, "HEAD"]),
    i1IsCandidateAncestor: spawnSync(
      "git", ["merge-base", "--is-ancestor", identities.i1, "HEAD"], { cwd: repoRoot }
    ).status === 0
  };
}

export function buildLedger(context) {
  const sourceRecords = context.reviewedSourcePaths.map((relativePath) => {
    const sourceBlob = blobAt(context.repoRoot, identities.reviewedSource, relativePath);
    const overlay = overlayPaths[relativePath];
    const preserveBlob = overlay?.[1] ?? sourceBlob;
    const targetBlob = workingBlob(context.repoRoot, relativePath);
    const disposition = targetBlob === preserveBlob ? "byte_preserve" : "reconcile";
    const record = recordBase({
      path: relativePath,
      sourceKind: "reviewed_source",
      sourceCommit: identities.reviewedSource,
      sourceBlob,
      currentMainBlob: blobAt(context.repoRoot, identities.currentMain, relativePath),
      disposition,
      targetBlob,
      custody: "leaf_a",
      rationale: disposition === "byte_preserve"
        ? "Exact reviewed or I2-overlay bytes are retained."
        : "Current-main convergence strengthens behavior while retaining the reviewed obligation."
    });
    if (overlay) {
      record.i2_overlay = {
        i1_blob_oid: overlay[0],
        i2_blob_oid: overlay[1],
        target_disposition: disposition,
        target_blob_oid: targetBlob,
        review_receipt_sha256: identities.reviewSha256
      };
    }
    if (relativePath === "docs/specs/ix-eight-vignette-shared-foundation-r1.md") {
      record.superseded_claims = ["Historical base, admission, WIP, and in-progress execution guidance."];
    }
    return record;
  });

  const planningRecords = context.reviewedPlanningPaths.map((relativePath) => {
    const crossChannel = relativePath === "docs/specs/ix-eight-vignette-cross-channel-r1.md";
    return recordBase({
      path: relativePath,
      sourceKind: "reviewed_planning",
      sourceCommit: identities.i1,
      sourceBlob: blobAt(context.repoRoot, identities.i1, relativePath),
      currentMainBlob: blobAt(context.repoRoot, identities.currentMain, relativePath),
      disposition: crossChannel ? "reconcile" : "reject_from_candidate",
      targetBlob: crossChannel ? "PENDING_INTEGRATION" : null,
      custody: crossChannel ? "integration_product_owner" : "current_main_governance",
      rationale: crossChannel
        ? "Integration and Product Owner retain the reviewed cross-channel outcome for reconciliation."
        : "Historical planning bytes are immutable evidence, not source-lane authority."
    });
  });

  const dependencyRecords = dependencyPaths.map((relativePath, index) => {
    const generator = relativePath === "scripts/check_app_gen_backend_equivalence.sh";
    const integration = relativePath === "Cargo.lock";
    const sourceCommit = generator ? identities.correctionCommit : identities.i1;
    const sourceBlob = generator
      ? identities.generatorBlob
      : blobAt(context.repoRoot, identities.i1, relativePath);
    const targetBlob = integration ? "PENDING_INTEGRATION" : workingBlob(context.repoRoot, relativePath);
    const disposition = integration || finalReconcileDependencies.has(relativePath)
      ? "reconcile"
      : targetBlob === sourceBlob ? "byte_preserve" : "reconcile";
    const record = recordBase({
      path: relativePath,
      sourceKind: "dependency_closure",
      sourceCommit,
      sourceBlob,
      currentMainBlob: blobAt(context.repoRoot, identities.currentMain, relativePath),
      disposition,
      targetBlob,
      custody: integration ? "integration" : "leaf_a",
      rationale: generator
        ? "The reviewed one-line generator correction carries runtime contracts into backend-equivalence projections."
        : integration
          ? "The root lock is reconciled only after the immutable Leaf A handoff."
          : index < 36
            ? "Reviewed behavior depends on this current-main-absent closure path."
            : "Current-main bytes require an explicit equivalent-or-stronger reconciliation."
    });
    record.required_by_reviewed_paths = requiredBy(context.reviewedSourcePaths, relativePath);
    return record;
  });

  const historical = new Map();
  for (const [collection, records] of [
    ["reviewed_source_paths", sourceRecords],
    ["reviewed_planning_paths", planningRecords],
    ["dependency_closure_paths", dependencyRecords]
  ]) {
    for (const record of records) historical.set(record.path, collection);
  }
  const candidatePaths = new Set(context.candidatePaths);
  candidatePaths.add(ledgerSelfPath);
  const coverage = [...candidatePaths].sort().map((relativePath) => {
    if (integrationOnlyPaths.has(relativePath)) {
      throw new Error(`candidate changes Integration-only path ${relativePath}`);
    }
    const collection = historical.get(relativePath);
    return {
      path: relativePath,
      coverage_kind: collection ? "historical_record" : "lane_only_change",
      historical_collection: collection ?? null,
      historical_path: collection ? relativePath : null,
      admitted_root: rootForPath(relativePath),
      candidate_blob_oid: relativePath === ledgerSelfPath
        ? "SELF_REFERENTIAL_LEDGER"
        : context.candidateBlobs[relativePath] ?? workingBlob(context.repoRoot, relativePath),
      rationale: collection
        ? "Candidate bytes are classified by the named historical record."
        : "New current-main convergence implementation or proof output within an exact admitted root.",
      i2_overlay: Object.hasOwn(overlayPaths, relativePath)
    };
  });

  return {
    schema: "appfw.ix_runtime_convergence_source_ledger@1",
    authority: {
      packet_sha256: "0fd733c8c156c86a37b6e47922f9ffc0abed3692aee6a6fa79e037d016607881",
      plan_commit: "07a364ee82fdba65d38d66bb3afe0038b305e9b8",
      current_main_comparison_commit: identities.currentMain,
      leaf_root_count: 69,
      leaf_root_ordered_sha256: "c606ad184f3b8a7fbfd1e6964189b538cb3a58bb1f60d41446f91d68e27d1a21",
      delivery_credit: "none"
    },
    counts: {
      reviewed_source: 52,
      reviewed_planning: 7,
      dependency_closure: 45,
      historical_total: 104
    },
    i2_overlay_receipt: {
      remote_ref: identities.i2Ref,
      commit_oid: identities.i2,
      sole_parent_oid: identities.i1,
      tree_oid: identities.i2Tree,
      changed_paths: Object.keys(overlayPaths),
      reviewed_source_commit: identities.reviewedSource,
      integration_base_commit: identities.integrationBase,
      integration_relation: "history_preserving_cherry_pick_assembly",
      lineage_status: "reviewed_source_assembled",
      comprehensive_review: {
        range: `${identities.i1}..${identities.i2}`,
        status: "GO WITH CONDITIONS",
        blocker: 0,
        critical: 0,
        important: 0,
        should_address: 0,
        nice_to_address: 0,
        artifact_path: "target/appfw/framework-pr-review.md",
        artifact_sha256: identities.reviewSha256,
        conditions_resolution: "Human authorized the exact one-time non-main I2 push; no broader authority followed."
      },
      authority_nonclaims: [
        "No source admission, Product acceptance, merge, release, publication, deployment, security approval, or accepted risk."
      ]
    },
    reviewed_source_paths: sourceRecords,
    reviewed_planning_paths: planningRecords,
    dependency_closure_paths: dependencyRecords,
    candidate_diff_coverage: coverage,
    nonclaims: [
      "This ledger records a local reviewed working-base candidate only.",
      "It grants no push, PR, merge, Product acceptance, package publication, release, deployment, live-provider, security, SRA/CAB, accepted-risk, native-device, hosted-durability, multi-replica, or production-readiness claim."
    ]
  };
}

function sameSet(actual, expected, label) {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(`${label} does not match its Git-derived exact set`);
  }
}

function requireRecordShape(record, expectedKind) {
  if (record.source_kind !== expectedKind) throw new Error(`${record.path}: wrong source_kind`);
  if (!fullOid.test(record.source_commit_oid) || !fullOid.test(record.source_blob_oid)) {
    throw new Error(`${record.path}: source commit/blob identity is not a full lowercase OID`);
  }
  if (record.current_main_blob_oid !== null && !fullOid.test(record.current_main_blob_oid)) {
    throw new Error(`${record.path}: invalid current-main blob identity`);
  }
  if (!["byte_preserve", "reconcile", "reject_from_candidate"].includes(record.disposition)) {
    throw new Error(`${record.path}: unknown disposition`);
  }
  if (record.delivery_credit !== "none") throw new Error(`${record.path}: delivery credit must be none`);
  for (const field of ["custody", "decision_authority", "review_evidence", "rationale"]) {
    if (typeof record[field] !== "string" || record[field].length === 0) {
      throw new Error(`${record.path}: ${field} is required`);
    }
  }
  if (!Array.isArray(record.preserved_obligations) || record.preserved_obligations.length === 0) {
    throw new Error(`${record.path}: preserved obligations are required`);
  }
  if (!Array.isArray(record.superseded_claims)) {
    throw new Error(`${record.path}: superseded claims must be an array`);
  }
}

export function validateLedger(ledger, context, expectedCounts = {
  reviewedSource: 52,
  planning: 7,
  dependency: 45
}) {
  if (ledger.schema !== "appfw.ix_runtime_convergence_source_ledger@1") {
    throw new Error("unsupported IX convergence source ledger schema");
  }
  if (ledger.authority?.current_main_comparison_commit !== identities.currentMain
    || ledger.authority?.leaf_root_count !== 69
    || ledger.authority?.leaf_root_ordered_sha256
      !== "c606ad184f3b8a7fbfd1e6964189b538cb3a58bb1f60d41446f91d68e27d1a21"
    || ledger.authority?.delivery_credit !== "none") {
    throw new Error("ledger authority/base/root binding is not exact");
  }
  const collections = [
    ["reviewed_source_paths", "reviewed_source", expectedCounts.reviewedSource],
    ["reviewed_planning_paths", "reviewed_planning", expectedCounts.planning],
    ["dependency_closure_paths", "dependency_closure", expectedCounts.dependency]
  ];
  const allPaths = [];
  for (const [name, kind, count] of collections) {
    if (!Array.isArray(ledger[name]) || ledger[name].length !== count) {
      throw new Error(`${name} must contain exactly ${count} records`);
    }
    for (const record of ledger[name]) {
      requireRecordShape(record, kind);
      allPaths.push(record.path);
    }
  }
  if (new Set(allPaths).size !== allPaths.length || allPaths.length !== 104) {
    throw new Error("historical collections must contain 104 globally unique paths");
  }
  sameSet(ledger.reviewed_source_paths.map(({ path: item }) => item),
    context.reviewedSourcePaths, "reviewed source paths");
  sameSet(ledger.reviewed_planning_paths.map(({ path: item }) => item),
    context.reviewedPlanningPaths, "reviewed planning paths");
  sameSet(ledger.dependency_closure_paths.map(({ path: item }) => item),
    dependencyPaths, "dependency closure paths");

  const reviewedSet = new Set(context.reviewedSourcePaths);
  for (const record of ledger.reviewed_source_paths) {
    if (record.source_commit_oid !== identities.reviewedSource
      || record.source_blob_oid !== blobAt(context.repoRoot, identities.reviewedSource, record.path)
      || record.current_main_blob_oid !== blobAt(context.repoRoot, identities.currentMain, record.path)) {
      throw new Error(`${record.path}: reviewed source identity mismatch`);
    }
    const overlay = overlayPaths[record.path];
    const expectedTarget = record.path === ledgerSelfPath
      ? "SELF_REFERENTIAL_LEDGER"
      : workingBlob(context.repoRoot, record.path);
    if (record.target_blob_oid !== expectedTarget) throw new Error(`${record.path}: stale target blob`);
    if (record.disposition === "byte_preserve"
      && record.target_blob_oid !== (overlay?.[1] ?? record.source_blob_oid)) {
      throw new Error(`${record.path}: byte-preserve target differs from its reviewed authority`);
    }
    if (overlay) {
      if (record.i2_overlay?.i1_blob_oid !== overlay[0]
        || record.i2_overlay?.i2_blob_oid !== overlay[1]
        || record.i2_overlay?.target_blob_oid !== record.target_blob_oid
        || record.i2_overlay?.review_receipt_sha256 !== identities.reviewSha256) {
        throw new Error(`${record.path}: I2 overlay annotation mismatch`);
      }
    } else if (record.i2_overlay !== undefined) {
      throw new Error(`${record.path}: unexpected I2 overlay annotation`);
    }
  }

  for (const record of ledger.reviewed_planning_paths) {
    if (record.source_commit_oid !== identities.i1
      || record.source_blob_oid !== blobAt(context.repoRoot, identities.i1, record.path)
      || record.current_main_blob_oid !== blobAt(context.repoRoot, identities.currentMain, record.path)) {
      throw new Error(`${record.path}: reviewed planning identity mismatch`);
    }
    const crossChannel = record.path === "docs/specs/ix-eight-vignette-cross-channel-r1.md";
    if (crossChannel) {
      if (record.disposition !== "reconcile" || record.custody !== "integration_product_owner"
        || record.target_blob_oid !== "PENDING_INTEGRATION") {
        throw new Error("cross-channel planning record must remain pending Integration/Product reconciliation");
      }
    } else if (record.disposition !== "reject_from_candidate" || record.target_blob_oid !== null) {
      throw new Error(`${record.path}: historical planning bytes must be rejected from candidate`);
    }
  }

  for (const record of ledger.dependency_closure_paths) {
    const generator = record.path === "scripts/check_app_gen_backend_equivalence.sh";
    const expectedCommit = generator ? identities.correctionCommit : identities.i1;
    const expectedSourceBlob = generator
      ? identities.generatorBlob
      : blobAt(context.repoRoot, identities.i1, record.path);
    if (record.source_commit_oid !== expectedCommit || record.source_blob_oid !== expectedSourceBlob
      || record.current_main_blob_oid !== blobAt(context.repoRoot, identities.currentMain, record.path)) {
      throw new Error(`${record.path}: dependency identity mismatch`);
    }
    if (!Array.isArray(record.required_by_reviewed_paths)
      || record.required_by_reviewed_paths.length === 0
      || record.required_by_reviewed_paths.some((item) => !reviewedSet.has(item))) {
      throw new Error(`${record.path}: required_by_reviewed_paths is empty or outside reviewed source`);
    }
    if (record.path === "Cargo.lock") {
      if (record.custody !== "integration" || record.disposition !== "reconcile"
        || record.target_blob_oid !== "PENDING_INTEGRATION") {
        throw new Error("root Cargo.lock must remain pending Integration custody");
      }
      continue;
    }
    const actualTarget = workingBlob(context.repoRoot, record.path);
    if (record.target_blob_oid !== actualTarget) throw new Error(`${record.path}: stale dependency target blob`);
    if (record.disposition === "byte_preserve" && record.target_blob_oid !== record.source_blob_oid) {
      throw new Error(`${record.path}: byte-preserve dependency differs from source`);
    }
    if (generator && record.target_blob_oid !== identities.generatorBlob) {
      throw new Error("generator correction does not match the exact reviewed target blob");
    }
    if (record.source_blob_oid === record.current_main_blob_oid) {
      throw new Error(`${record.path}: byte-identical path is not actionable dependency closure`);
    }
  }

  const receipt = ledger.i2_overlay_receipt;
  if (receipt?.remote_ref !== identities.i2Ref || receipt?.commit_oid !== identities.i2
    || receipt?.sole_parent_oid !== identities.i1 || receipt?.tree_oid !== identities.i2Tree
    || receipt?.reviewed_source_commit !== identities.reviewedSource
    || receipt?.integration_base_commit !== identities.integrationBase
    || receipt?.integration_relation !== "history_preserving_cherry_pick_assembly"
    || receipt?.lineage_status !== "reviewed_source_assembled") {
    throw new Error("I2 overlay receipt identity/lineage mismatch");
  }
  sameSet(receipt.changed_paths, Object.keys(overlayPaths), "I2 overlay paths");
  if (context.liveRemoteCommit !== identities.i2) throw new Error("live I2 remote ref is absent or moved");
  if (context.i2Parents.length !== 1 || context.i2Parents[0] !== identities.i1
    || context.i2Tree !== identities.i2Tree) {
    throw new Error("I2 parent/tree is not exact");
  }
  sameSet(context.i2Paths, Object.keys(overlayPaths), "I2 Git changed paths");
  const review = receipt.comprehensive_review;
  if (review?.range !== `${identities.i1}..${identities.i2}`
    || review?.status !== "GO WITH CONDITIONS"
    || [review.blocker, review.critical, review.important, review.should_address,
      review.nice_to_address].some((value) => value !== 0)
    || review?.artifact_sha256 !== identities.reviewSha256) {
    throw new Error("I2 comprehensive review receipt is stale or nonzero");
  }
  for (const [relativePath, [i1Blob, i2Blob]] of Object.entries(overlayPaths)) {
    if (blobAt(context.repoRoot, identities.i1, relativePath) !== i1Blob
      || blobAt(context.repoRoot, identities.i2, relativePath) !== i2Blob) {
      throw new Error(`${relativePath}: I2 historical blob tuple mismatch`);
    }
  }

  if (!Array.isArray(ledger.candidate_diff_coverage)) {
    throw new Error("candidate_diff_coverage must be an array");
  }
  const coveragePaths = ledger.candidate_diff_coverage.map(({ path: item }) => item);
  if (new Set(coveragePaths).size !== coveragePaths.length) {
    throw new Error("candidate diff coverage contains duplicate paths");
  }
  const expectedCandidatePaths = new Set(context.candidatePaths);
  expectedCandidatePaths.add(ledgerSelfPath);
  sameSet(coveragePaths, expectedCandidatePaths, "candidate diff coverage");
  const historicalSet = new Set(allPaths);
  for (const coverage of ledger.candidate_diff_coverage) {
    if (integrationOnlyPaths.has(coverage.path)) {
      throw new Error(`${coverage.path}: Integration-only path entered Leaf A candidate`);
    }
    if (coverage.admitted_root !== rootForPath(coverage.path)) {
      throw new Error(`${coverage.path}: stale or ambiguous admitted root`);
    }
    const expectedBlob = coverage.path === ledgerSelfPath
      ? "SELF_REFERENTIAL_LEDGER"
      : workingBlob(context.repoRoot, coverage.path);
    if (coverage.candidate_blob_oid !== expectedBlob) throw new Error(`${coverage.path}: stale coverage blob`);
    if (historicalSet.has(coverage.path)) {
      if (coverage.coverage_kind !== "historical_record"
        || coverage.historical_path !== coverage.path
        || typeof coverage.historical_collection !== "string") {
        throw new Error(`${coverage.path}: historical coverage is not bound to its record`);
      }
    } else if (coverage.coverage_kind !== "lane_only_change"
      || coverage.historical_collection !== null || coverage.historical_path !== null) {
      throw new Error(`${coverage.path}: lane-only coverage vocabulary is invalid`);
    }
    if (coverage.i2_overlay !== Object.hasOwn(overlayPaths, coverage.path)) {
      throw new Error(`${coverage.path}: I2 coverage annotation mismatch`);
    }
  }
  if (context.mergeBase !== identities.currentMain || context.i1IsCandidateAncestor) {
    throw new Error("candidate contains unrelated/non-current-main historical ancestry");
  }
  if (ledger.counts?.reviewed_source !== expectedCounts.reviewedSource
    || ledger.counts?.reviewed_planning !== expectedCounts.planning
    || ledger.counts?.dependency_closure !== expectedCounts.dependency
    || ledger.counts?.historical_total !== 104) {
    throw new Error("ledger count receipt is stale");
  }
  return {
    ok: true,
    counts: ledger.counts,
    candidate_diff_paths: coveragePaths.length,
    live_i2_ref: context.liveRemoteCommit,
    leaf_root_count: admittedRoots.length,
    integration_roots_changed: 0,
    delivery_credit: "none"
  };
}

export function resolveLiveRemote(repoRoot) {
  const result = spawnSync("git", ["ls-remote", "--heads", "origin", identities.i2Ref], {
    cwd: repoRoot,
    encoding: "utf8"
  });
  if (result.status !== 0) throw new Error("unable to resolve the live I2 remote ref");
  const records = lines(result.stdout);
  if (records.length !== 1) throw new Error("live I2 remote ref is absent or ambiguous");
  const [oid, ref] = records[0].split(/\s+/u);
  if (ref !== identities.i2Ref || !fullOid.test(oid)) {
    throw new Error("live I2 remote ref response is malformed");
  }
  return oid;
}

function parseArgs(argv) {
  const result = {
    ledger: ledgerSelfPath,
    reviewedSource: 52,
    planning: 7,
    dependency: 45,
    json: false,
    refresh: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === "--ledger") result.ledger = argv[++index];
    else if (value === "--expect-reviewed-source") result.reviewedSource = Number(argv[++index]);
    else if (value === "--expect-planning") result.planning = Number(argv[++index]);
    else if (value === "--expect-dependency") result.dependency = Number(argv[++index]);
    else if (value === "--json") result.json = true;
    else if (value === "--refresh") result.refresh = true;
    else throw new Error(`unknown argument: ${value}`);
  }
  if (![result.reviewedSource, result.planning, result.dependency].every(Number.isSafeInteger)) {
    throw new Error("expected counts must be safe integers");
  }
  return result;
}

async function main() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const options = parseArgs(process.argv.slice(2));
  const ledgerPath = path.resolve(repoRoot, options.ledger);
  if (!ledgerPath.startsWith(`${repoRoot}${path.sep}`)) throw new Error("ledger path escapes repository root");
  if (options.refresh) {
    const context = loadGitContext(repoRoot, {
      liveRemoteCommit: identities.i2,
      includeLedgerPath: true
    });
    const ledger = buildLedger(context);
    await writeFile(ledgerPath, `${JSON.stringify(ledger, null, 2)}\n`, "utf8");
  }
  const liveRemoteCommit = resolveLiveRemote(repoRoot);
  const context = loadGitContext(repoRoot, { liveRemoteCommit, includeLedgerPath: true });
  const ledger = JSON.parse(await readFile(ledgerPath, "utf8"));
  const result = validateLedger(ledger, context, {
    reviewedSource: options.reviewedSource,
    planning: options.planning,
    dependency: options.dependency
  });
  process.stdout.write(`${JSON.stringify(result, null, options.json ? 2 : 0)}\n`);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : "";
if (invokedPath === import.meta.url) {
  main().catch((error) => {
    process.stderr.write(`check-ix-runtime-convergence-source-ledger: ${error.message}\n`);
    process.exitCode = 1;
  });
}
