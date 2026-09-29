#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const defaultRepoRoot = resolve(dirname(scriptPath), '..');

function usage() {
  process.stdout.write('Usage: node scripts/check-program-flow.mjs [options]\n\n');
  process.stdout.write('Options:\n');
  process.stdout.write('  --json                 Emit machine-readable output\n');
  process.stdout.write('  --record               Atomically retain the current deterministic snapshot\n');
  process.stdout.write('  --strict               Exit 2 when a policy violation exists\n');
  process.stdout.write('  --repo <path>          Repository root (default: script parent)\n');
  process.stdout.write('  --status <path>        Dashboard status JSON\n');
  process.stdout.write('  --state <path>         Retained observer snapshot\n');
  process.stdout.write('  --stall-minutes <n>    Active-WIP staleness threshold (default: 60)\n');
  process.stdout.write('  --help                 Show this help\n');
}

function parseArgs(argv) {
  const options = {
    json: false,
    record: false,
    strict: false,
    repoRoot: defaultRepoRoot,
    statusPath: null,
    statePath: null,
    stallMinutes: 60,
  };

  function takeValue(index, option) {
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) {
      throw new Error(`${option} requires a value`);
    }
    return value;
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--json') options.json = true;
    else if (arg === '--record') options.record = true;
    else if (arg === '--strict') options.strict = true;
    else if (arg === '--help' || arg === '-h') options.help = true;
    else if (arg === '--repo') options.repoRoot = resolve(takeValue(index++, arg));
    else if (arg === '--status') options.statusPath = resolve(takeValue(index++, arg));
    else if (arg === '--state') options.statePath = resolve(takeValue(index++, arg));
    else if (arg === '--stall-minutes') options.stallMinutes = Number(takeValue(index++, arg));
    else throw new Error(`unknown option: ${arg}`);
  }

  if (!Number.isFinite(options.stallMinutes) || options.stallMinutes < 1) {
    throw new Error('--stall-minutes must be a positive number');
  }

  options.statusPath ??= resolve(
    options.repoRoot,
    'target/appfw/nexus-control-dashboard/status.json',
  );
  options.statePath ??= resolve(
    options.repoRoot,
    'target/appfw/program-flow-observer-state.json',
  );
  return options;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function git(repoRoot, args) {
  return execFileSync('git', ['-C', repoRoot, ...args], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

function diskPressure(repoRoot) {
  const rows = execFileSync('df', ['-Pk', repoRoot], { encoding: 'utf8' })
    .trim()
    .split('\n');
  const columns = rows.at(-1).trim().split(/\s+/);
  const availableKiB = Number(columns.at(-3));
  const freeGiB = availableKiB / 1024 / 1024;
  const state = freeGiB < 50 ? 'hard_stop' : freeGiB < 100 ? 'cleanup_first' : 'healthy';
  return { free_gib: Number(freeGiB.toFixed(1)), state };
}

function repositoryFacts(repoRoot) {
  const status = git(repoRoot, ['status', '--porcelain=v1']);
  const worktrees = git(repoRoot, ['worktree', 'list', '--porcelain'])
    .split('\n')
    .filter((line) => line.startsWith('worktree '));
  const branches = git(repoRoot, ['for-each-ref', '--format=%(refname)', 'refs/heads/'])
    .split('\n')
    .filter(Boolean);

  return {
    head: git(repoRoot, ['rev-parse', 'HEAD']),
    branch: git(repoRoot, ['branch', '--show-current']) || '(detached)',
    dirty_path_count: status ? status.split('\n').length : 0,
    dirty_state_sha256: sha256(status),
    worktree_count: worktrees.length,
    local_branch_count: branches.length,
    disk: diskPressure(repoRoot),
  };
}

function trancheIsActive(tranche) {
  if (Number(tranche?.active_producers ?? 0) > 0) return true;
  const label = String(tranche?.status_label ?? '').toLowerCase();
  const state = String(tranche?.state ?? '').toLowerCase();
  return label === 'in progress'
    || label === 'under review'
    || /(^|_)(in_progress|under_review|active)(_|$)/.test(state);
}

function pipelineFacts(status) {
  if (!Array.isArray(status?.pipelines)) return [];
  return status.pipelines.map((pipeline) => ({
    id: pipeline?.id ?? pipeline?.number ?? pipeline?.uuid ?? null,
    state: pipeline?.state ?? pipeline?.status ?? null,
    commit: pipeline?.commit ?? pipeline?.sha ?? null,
  }));
}

function nonNegativeInteger(value) {
  return Number.isInteger(value) && value >= 0;
}

function validRfc3339(value) {
  if (typeof value !== 'string') return false;
  const match = value.match(
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/,
  );
  if (!match) return false;

  const [, yearText, monthText, dayText, hourText, minuteText, secondText,
    offsetSign, offsetHourText, offsetMinuteText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [
    31, leapYear ? 29 : 28, 31, 30, 31, 30,
    31, 31, 30, 31, 30, 31,
  ];

  if (
    month < 1
    || month > 12
    || day < 1
    || day > daysInMonth[month - 1]
    || hour > 23
    || minute > 59
    || second > 59
  ) {
    return false;
  }
  if (offsetSign && (Number(offsetHourText) > 23 || Number(offsetMinuteText) > 59)) {
    return false;
  }
  return Number.isFinite(Date.parse(value));
}

function dashboardContractViolations(status) {
  const violations = [];
  const unavailable = (detail) => violations.push({
    code: 'dashboard_unavailable',
    detail,
  });

  if (!status || typeof status !== 'object' || Array.isArray(status)) {
    unavailable('dashboard status must be a JSON object');
    return violations;
  }
  if (status.schema !== 'nexus_poc_control_projection@1') {
    unavailable('dashboard schema must be nexus_poc_control_projection@1');
  }
  if (
    status.projection_contract
    !== 'human_ratified_pfc_completion_invariant_accepted_main'
  ) {
    unavailable(
      'dashboard projection_contract must be human_ratified_pfc_completion_invariant_accepted_main',
    );
  }
  if (!validRfc3339(status.generated_at)) {
    unavailable('dashboard generated_at must be an RFC3339 timestamp');
  }
  if (
    !status.accepted_main
    || typeof status.accepted_main !== 'object'
    || !/^[0-9a-f]{40}$/.test(status.accepted_main.sha ?? '')
    || typeof status.accepted_main.state !== 'string'
    || status.accepted_main.state.length === 0
  ) {
    unavailable('dashboard accepted_main must contain a full Git SHA and non-empty state');
  }
  if (!Array.isArray(status.delivery_tranches)) {
    unavailable('dashboard delivery_tranches must be an array');
  } else {
    status.delivery_tranches.forEach((tranche, index) => {
      if (
        !tranche
        || typeof tranche !== 'object'
        || typeof tranche.id !== 'string'
        || tranche.id.length === 0
        || !nonNegativeInteger(tranche.active_producers ?? 0)
      ) {
        unavailable(`dashboard delivery_tranches[${index}] is invalid`);
      }
    });
  }

  const activeProducers = status.wip?.active_producers
    ?? status.flow_metrics?.source_wip?.active;
  const producerLimit = status.wip?.producer_limit
    ?? status.flow_metrics?.source_wip?.limit;
  if (!nonNegativeInteger(activeProducers)) {
    unavailable('dashboard active producer count must be a non-negative integer');
  }
  if (!nonNegativeInteger(producerLimit) || producerLimit < 1) {
    unavailable('dashboard producer limit must be a positive integer');
  }

  return violations;
}

export function evaluateProgramFlow(status, repoFacts, options = {}) {
  const maxActiveTranches = options.maxActiveTranches ?? 1;
  const maxProducers = options.maxProducers ?? 2;
  const stallMinutes = options.stallMinutes ?? 60;
  const tranches = Array.isArray(status?.delivery_tranches) ? status.delivery_tranches : [];
  const activeTranches = tranches.filter(trancheIsActive).map((tranche) => tranche.id);
  const activeProducers = Number(
    status?.wip?.active_producers
      ?? status?.flow_metrics?.source_wip?.active
      ?? 0,
  );
  const configuredProducerLimit = Number(
    status?.wip?.producer_limit
      ?? status?.flow_metrics?.source_wip?.limit
      ?? maxProducers,
  );
  const generatedAtMs = Date.parse(status?.generated_at ?? '');
  const statusAgeMinutes = Number.isFinite(generatedAtMs)
    ? Math.max(0, (Date.now() - generatedAtMs) / 60000)
    : null;
  const violations = dashboardContractViolations(status);

  if (activeTranches.length > maxActiveTranches) {
    violations.push({
      code: 'active_tranche_limit',
      detail: `${activeTranches.length} active tranches exceeds ${maxActiveTranches}`,
    });
  }
  if (activeProducers > maxProducers) {
    violations.push({
      code: 'active_producer_limit',
      detail: `${activeProducers} active producers exceeds ${maxProducers}`,
    });
  }
  if (configuredProducerLimit > maxProducers) {
    violations.push({
      code: 'configured_producer_limit',
      detail: `configured producer limit ${configuredProducerLimit} exceeds ${maxProducers}`,
    });
  }
  if (activeProducers > 0 && activeTranches.length === 0) {
    violations.push({
      code: 'producer_without_active_tranche',
      detail: `${activeProducers} producers are not attributed to an active tranche`,
    });
  }
  if (activeProducers > 0 && statusAgeMinutes !== null && statusAgeMinutes > stallMinutes) {
    violations.push({
      code: 'active_flow_stalled',
      detail: `active flow projection is ${Math.floor(statusAgeMinutes)} minutes old`,
    });
  }
  if (repoFacts.worktree_count > 12) {
    violations.push({
      code: 'worktree_budget',
      detail: `${repoFacts.worktree_count} worktrees exceeds 12`,
    });
  }
  if (repoFacts.local_branch_count > 100) {
    violations.push({
      code: 'branch_budget',
      detail: `${repoFacts.local_branch_count} local branches exceeds 100`,
    });
  }
  if (repoFacts.disk.state === 'hard_stop') {
    violations.push({
      code: 'disk_hard_stop',
      detail: `${repoFacts.disk.free_gib} GiB free is below the 50 GiB hard stop`,
    });
  }

  const material = {
    accepted_main: {
      sha: status?.accepted_main?.sha ?? null,
      state: status?.accepted_main?.state ?? null,
    },
    active_tranches: activeTranches,
    active_producers: activeProducers,
    configured_producer_limit: configuredProducerLimit,
    pipelines: pipelineFacts(status),
    repository: {
      head: repoFacts.head,
      branch: repoFacts.branch,
      dirty_state_sha256: repoFacts.dirty_state_sha256,
      worktree_count: repoFacts.worktree_count,
      local_branch_count: repoFacts.local_branch_count,
      disk_state: repoFacts.disk.state,
    },
  };

  return {
    policy: {
      active_tranche_limit: maxActiveTranches,
      producer_limit: maxProducers,
      event_driven: true,
      recurring_model_heartbeat: false,
    },
    observed: {
      active_tranches: activeTranches,
      active_producers: activeProducers,
      configured_producer_limit: configuredProducerLimit,
      dashboard_generated_at: status?.generated_at ?? null,
      dashboard_age_minutes: statusAgeMinutes === null
        ? null
        : Number(statusAgeMinutes.toFixed(1)),
      repository: repoFacts,
    },
    violations,
    material,
    fingerprint: sha256(JSON.stringify(material)),
  };
}

function changedFields(previous, current) {
  if (!previous?.material) return [];
  return Object.keys(current.material).filter(
    (key) => JSON.stringify(previous.material[key]) !== JSON.stringify(current.material[key]),
  );
}

function writeJsonAtomically(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  renameSync(temporary, path);
}

function renderHuman(result) {
  const state = result.needs_agent ? 'ATTENTION' : 'QUIET';
  process.stdout.write(`Program flow observer: ${state}\n`);
  process.stdout.write(`  active tranches  ${result.observed.active_tranches.length}/1`);
  if (result.observed.active_tranches.length > 0) {
    process.stdout.write(` (${result.observed.active_tranches.join(', ')})`);
  }
  process.stdout.write('\n');
  process.stdout.write(`  active producers ${result.observed.active_producers}/2\n`);
  process.stdout.write(`  worktrees        ${result.observed.repository.worktree_count}/12\n`);
  process.stdout.write(`  local branches   ${result.observed.repository.local_branch_count}/100\n`);
  process.stdout.write(`  disk             ${result.observed.repository.disk.free_gib} GiB free\n`);
  if (result.changed_fields.length > 0) {
    process.stdout.write(`  material change  ${result.changed_fields.join(', ')}\n`);
  }
  for (const violation of result.violations) {
    process.stdout.write(`  violation        ${violation.code}: ${violation.detail}\n`);
  }
  process.stdout.write(`  action           ${result.needs_agent ? 'wake Program Flow Controller' : 'do not invoke a model'}\n`);
}

function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
    if (options.help) {
      usage();
      return;
    }
    if (!existsSync(options.statusPath)) {
      throw new Error(`status file not found: ${options.statusPath}`);
    }

    const status = readJson(options.statusPath);
    const repoFacts = repositoryFacts(options.repoRoot);
    const evaluated = evaluateProgramFlow(status, repoFacts, {
      stallMinutes: options.stallMinutes,
    });
    const previous = existsSync(options.statePath) ? readJson(options.statePath) : null;
    const changed = Boolean(previous) && previous.fingerprint !== evaluated.fingerprint;
    const result = {
      schema: 'appfw_program_flow_observer@1',
      ok: evaluated.violations.length === 0,
      needs_agent: evaluated.violations.length > 0 || changed,
      baseline_initialized: !previous,
      material_change: changed,
      changed_fields: changedFields(previous, evaluated),
      ...evaluated,
      state_path: options.statePath,
      recorded: options.record,
    };

    if (options.record) {
      writeJsonAtomically(options.statePath, {
        schema: result.schema,
        recorded_at: new Date().toISOString(),
        fingerprint: result.fingerprint,
        material: result.material,
      });
    }

    if (options.json) process.stdout.write(`${JSON.stringify(result)}\n`);
    else renderHuman(result);

    if (options.strict && result.violations.length > 0) process.exitCode = 2;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (options?.json) {
      process.stdout.write(`${JSON.stringify({
        schema: 'appfw_program_flow_observer@1',
        ok: false,
        needs_agent: true,
        error: message,
      })}\n`);
    } else {
      process.stderr.write(`Program flow observer failed: ${message}\n`);
    }
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) main();
