#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

function usage() {
  return `Usage:
  node scripts/audit-worktrees.mjs [--assignment <path>] [--base <ref>] [--stale-hours <hours>] [--json]

This command is report-only. It never runs git worktree prune, git worktree
remove, branch deletion, reset, clean, or checkout.`;
}

function parseArgs(argv) {
  const options = {
    assignment: null,
    base: "origin/main",
    json: false,
    staleHours: 72,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") options.json = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else if (arg === "--assignment" || arg === "--base" || arg === "--stale-hours") {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) throw new Error(`${arg} requires a value`);
      if (arg === "--stale-hours") {
        options.staleHours = Number(value);
      } else {
        options[arg.slice(2)] = value;
      }
      index += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!Number.isFinite(options.staleHours) || options.staleHours < 1) {
    throw new Error("--stale-hours must be a positive number");
  }
  return options;
}

function repositoryRoot() {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  return execFileSync("git", ["rev-parse", "--show-toplevel"], {
    cwd: scriptDir,
    encoding: "utf8",
  }).trim();
}

function runGit(args, cwd, allowFailure = false) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0 && !allowFailure) {
    throw new Error(result.stderr.trim() || `git ${args.join(" ")} failed`);
  }
  return result;
}

function parseWorktreeBlocks(raw) {
  return raw
    .trim()
    .split(/\n\n+/)
    .filter(Boolean)
    .map((block) => {
      const lines = block.split("\n");
      const branchLine = lines.find((line) => line.startsWith("branch "));
      return {
        path: lines.find((line) => line.startsWith("worktree "))?.slice(9),
        listedHead: lines.find((line) => line.startsWith("HEAD "))?.slice(5),
        branch: branchLine ? branchLine.slice("branch refs/heads/".length) : "DETACHED",
        bare: lines.includes("bare"),
        detached: lines.includes("detached"),
        locked: lines.some((line) => line.startsWith("locked")),
        prunable: lines.some((line) => line.startsWith("prunable")),
      };
    })
    .filter((record) => record.path);
}

function loadAssignment(root, assignmentPath) {
  if (!assignmentPath) return null;
  const path = resolve(root, assignmentPath);
  if (!existsSync(path)) throw new Error(`assignment does not exist: ${assignmentPath}`);
  const assignment = JSON.parse(readFileSync(path, "utf8"));
  if (
    assignment.schema !== "appfw_workstation_assignment@4" &&
    assignment.schema !== "appfw_workstation_assignment@3"
  ) {
    throw new Error(
      "assignment is not appfw_workstation_assignment@4 or legacy @3",
    );
  }
  return assignment;
}

function inspectWorktree(record, baseRef, staleHours, assignedBranch, primaryPath, nowEpoch) {
  const output = {
    path: record.path,
    branch: record.branch,
    head: record.listedHead ?? null,
    age_hours: null,
    dirty_files: null,
    merged_into_base: false,
    locked: record.locked,
    detached: record.detached,
    prunable_metadata: record.prunable,
    disposition: "owner-review-required",
    reasons: [],
  };

  if (!existsSync(record.path)) {
    output.disposition = "missing-path-review";
    output.reasons.push("worktree path is missing; inspect prunable metadata before any action");
    return output;
  }

  const headResult = runGit(["-C", record.path, "rev-parse", "HEAD"], primaryPath, true);
  if (headResult.status !== 0) {
    output.reasons.push("HEAD could not be resolved");
    return output;
  }
  output.head = headResult.stdout.trim();

  const statusResult = runGit(["-C", record.path, "status", "--porcelain"], primaryPath, true);
  if (statusResult.status === 0) {
    output.dirty_files = statusResult.stdout.trim().split("\n").filter(Boolean).length;
  } else {
    output.reasons.push("working-tree status could not be read");
  }

  const epochResult = runGit(["-C", record.path, "log", "-1", "--format=%ct"], primaryPath, true);
  if (epochResult.status === 0) {
    const commitEpoch = Number(epochResult.stdout.trim());
    if (Number.isFinite(commitEpoch)) output.age_hours = Math.max(0, Math.round((nowEpoch - commitEpoch) / 3600));
  }

  output.merged_into_base =
    runGit(["merge-base", "--is-ancestor", output.head, baseRef], primaryPath, true).status === 0;

  if (record.locked && output.dirty_files > 0) {
    output.disposition = "protected-dirty-locked";
    output.reasons.push("locked worktree contains uncommitted state");
  } else if (record.locked) {
    output.disposition = "protected-locked";
    output.reasons.push("worktree is explicitly locked");
  } else if (output.dirty_files > 0) {
    output.disposition = "protected-dirty";
    output.reasons.push("worktree contains uncommitted state");
  } else if (record.branch === assignedBranch) {
    output.disposition = "protected-assigned";
    output.reasons.push("branch matches the supplied active workstation assignment");
  } else if (record.path === primaryPath) {
    output.disposition = "protected-primary";
    output.reasons.push("primary worktree is never a cleanup candidate in this report");
  } else if (record.detached && !output.merged_into_base) {
    output.disposition = "protected-detached-unmerged";
    output.reasons.push("detached HEAD is not contained in the base ref");
  } else if (output.merged_into_base) {
    output.disposition = "reclaimable-candidate";
    output.reasons.push("clean unlocked HEAD is contained in the base ref");
  } else if (output.age_hours !== null && output.age_hours >= staleHours) {
    output.disposition = "stale-unmerged-owner-review";
    output.reasons.push(`clean unmerged HEAD is at least ${staleHours} hours old`);
  } else {
    output.disposition = "active-or-owner-review";
    output.reasons.push("clean unmerged worktree requires owner and PR-state confirmation");
  }

  return output;
}

function duplicateHeads(worktrees) {
  const byHead = new Map();
  for (const worktree of worktrees) {
    if (!worktree.head) continue;
    if (!byHead.has(worktree.head)) byHead.set(worktree.head, []);
    byHead.get(worktree.head).push({ path: worktree.path, branch: worktree.branch });
  }
  return [...byHead.entries()]
    .filter(([, records]) => records.length > 1)
    .map(([head, records]) => ({ head, worktrees: records }));
}

function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    console.error(usage());
    process.exit(2);
  }
  if (options.help) {
    console.log(usage());
    return;
  }

  const root = repositoryRoot();
  let assignment;
  try {
    assignment = loadAssignment(root, options.assignment);
  } catch (error) {
    console.error(error.message);
    process.exit(2);
  }

  const listResult = runGit(["worktree", "list", "--porcelain"], root);
  const records = parseWorktreeBlocks(listResult.stdout);
  const primaryPath = records[0]?.path ?? root;
  const nowEpoch = Date.now() / 1000;
  const worktrees = records.map((record) =>
    inspectWorktree(
      record,
      options.base,
      options.staleHours,
      assignment?.branch ?? null,
      primaryPath,
      nowEpoch,
    ),
  );

  const dispositionCounts = {};
  for (const worktree of worktrees) {
    dispositionCounts[worktree.disposition] = (dispositionCounts[worktree.disposition] ?? 0) + 1;
  }

  const output = {
    schema: "appfw_worktree_audit@1",
    ok: true,
    report_only: true,
    base_ref: options.base,
    stale_review_hours: options.staleHours,
    assignment_branch: assignment?.branch ?? null,
    summary: {
      total: worktrees.length,
      dispositions: Object.fromEntries(Object.entries(dispositionCounts).sort(([left], [right]) => left.localeCompare(right))),
      duplicate_head_groups: duplicateHeads(worktrees).length,
    },
    duplicate_heads: duplicateHeads(worktrees),
    worktrees,
    limitations: [
      "No remote PR, unpushed-commit, task-owner, or retained-evidence state is queried.",
      "Commit age is a review signal, not proof of workstation inactivity.",
      "A reclaimable candidate still requires owner and Integration Branch Manager confirmation.",
    ],
    prohibited_automatic_actions: [
      "git worktree prune",
      "git worktree remove",
      "git branch -d",
      "git branch -D",
      "git reset",
      "git clean",
    ],
  };

  if (options.json) {
    console.log(JSON.stringify(output, null, 2));
    return;
  }

  console.log(`Worktrees: ${output.summary.total}`);
  for (const [disposition, count] of Object.entries(output.summary.dispositions)) {
    console.log(`${disposition}: ${count}`);
  }
  console.log(`Duplicate HEAD groups: ${output.summary.duplicate_head_groups}`);
  console.log("Report only. No worktree or branch was changed.");
}

main();
