import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { evaluateProgramFlow } from './check-program-flow.mjs';

const scriptPath = fileURLToPath(new URL('./check-program-flow.mjs', import.meta.url));

const repoFacts = {
  head: 'abc123',
  branch: 'feature/example',
  dirty_path_count: 0,
  dirty_state_sha256: 'clean',
  worktree_count: 4,
  local_branch_count: 20,
  disk: { free_gib: 150, state: 'healthy' },
};

function status(overrides = {}) {
  return {
    schema: 'nexus_poc_control_projection@1',
    projection_contract: 'human_ratified_pfc_completion_invariant_accepted_main',
    generated_at: new Date().toISOString(),
    accepted_main: { sha: 'a'.repeat(40), state: 'green' },
    wip: { active_producers: 1, producer_limit: 2 },
    delivery_tranches: [
      { id: 'A0', status_label: 'In progress', state: 'in_progress', active_producers: 1 },
      { id: 'A1', status_label: 'Not started', state: 'not_started', active_producers: 0 },
    ],
    ...overrides,
  };
}

test('accepts one active tranche and two-or-fewer producers', () => {
  const result = evaluateProgramFlow(status(), repoFacts);
  assert.deepEqual(result.observed.active_tranches, ['A0']);
  assert.equal(result.observed.active_producers, 1);
  assert.deepEqual(result.violations, []);
});

test('rejects multiple active tranches and excess producer capacity', () => {
  const result = evaluateProgramFlow(status({
    wip: { active_producers: 3, producer_limit: 3 },
    delivery_tranches: [
      { id: 'A0', status_label: 'In progress', active_producers: 2 },
      { id: 'A1', status_label: 'Under review', active_producers: 1 },
    ],
  }), repoFacts);
  const codes = result.violations.map((violation) => violation.code);
  assert.ok(codes.includes('active_tranche_limit'));
  assert.ok(codes.includes('active_producer_limit'));
  assert.ok(codes.includes('configured_producer_limit'));
});

test('does not treat a ready tranche as active', () => {
  const result = evaluateProgramFlow(status({
    wip: { active_producers: 0, producer_limit: 2 },
    delivery_tranches: [
      { id: 'A0', status_label: 'Ready to start', state: 'ready_to_start', active_producers: 0 },
    ],
  }), repoFacts);
  assert.deepEqual(result.observed.active_tranches, []);
  assert.deepEqual(result.violations, []);
});

test('flags stale dashboard state only when source work is active', () => {
  const stale = new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString();
  const activeResult = evaluateProgramFlow(status({ generated_at: stale }), repoFacts, {
    stallMinutes: 60,
  });
  assert.ok(activeResult.violations.some((violation) => violation.code === 'active_flow_stalled'));

  const idleResult = evaluateProgramFlow(status({
    generated_at: stale,
    wip: { active_producers: 0, producer_limit: 2 },
    delivery_tranches: [],
  }), repoFacts, { stallMinutes: 60 });
  assert.ok(!idleResult.violations.some((violation) => violation.code === 'active_flow_stalled'));
});

test('rejects a command option with no value', () => {
  const result = spawnSync(process.execPath, [scriptPath, '--repo'], { encoding: 'utf8' });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /--repo requires a value/);
});

test('fails closed when required dashboard fields are absent', () => {
  const result = evaluateProgramFlow({}, repoFacts);
  assert.ok(result.violations.some((violation) => violation.code === 'dashboard_unavailable'));
});

test('fails closed for invalid timestamps and accepted main identity', () => {
  const result = evaluateProgramFlow(status({
    generated_at: 'not-a-timestamp',
    accepted_main: { sha: 'short', state: '' },
  }), repoFacts);
  const unavailable = result.violations.filter(
    (violation) => violation.code === 'dashboard_unavailable',
  );
  assert.equal(unavailable.length, 2);
});

test('fails closed for impossible calendar timestamps and unknown projection contracts', () => {
  const impossibleDate = evaluateProgramFlow(status({
    generated_at: '2026-02-31T12:00:00Z',
  }), repoFacts);
  assert.ok(
    impossibleDate.violations.some((violation) => violation.code === 'dashboard_unavailable'),
  );

  const unknownContract = evaluateProgramFlow(status({
    projection_contract: 'unknown_projection_contract@1',
  }), repoFacts);
  assert.ok(
    unknownContract.violations.some((violation) => violation.code === 'dashboard_unavailable'),
  );
});

test('fails closed for missing or invalid tranche collections', () => {
  const missing = evaluateProgramFlow(status({ delivery_tranches: undefined }), repoFacts);
  assert.ok(missing.violations.some((violation) => violation.code === 'dashboard_unavailable'));

  const invalid = evaluateProgramFlow(status({
    delivery_tranches: [{ id: '', active_producers: -1 }],
  }), repoFacts);
  assert.ok(invalid.violations.some((violation) => violation.code === 'dashboard_unavailable'));
});

test('fails closed for invalid WIP values', () => {
  for (const wip of [
    { active_producers: Number.NaN, producer_limit: 2 },
    { active_producers: -1, producer_limit: 2 },
    { active_producers: 0.5, producer_limit: 2 },
    { active_producers: 0, producer_limit: 0 },
  ]) {
    const result = evaluateProgramFlow(status({ wip }), repoFacts);
    assert.ok(
      result.violations.some((violation) => violation.code === 'dashboard_unavailable'),
    );
  }
});

test('CLI reports unavailable dashboard evidence as attention and strict failure', () => {
  const directory = mkdtempSync(join(tmpdir(), 'appfw-program-flow-'));
  try {
    const statusPath = join(directory, 'status.json');
    writeFileSync(statusPath, '{}\n', 'utf8');
    const result = spawnSync(process.execPath, [
      scriptPath,
      '--repo',
      fileURLToPath(new URL('..', import.meta.url)),
      '--status',
      statusPath,
      '--state',
      join(directory, 'state.json'),
      '--json',
      '--strict',
    ], { encoding: 'utf8' });
    assert.equal(result.status, 2);
    const output = JSON.parse(result.stdout);
    assert.equal(output.ok, false);
    assert.equal(output.needs_agent, true);
    assert.ok(
      output.violations.some((violation) => violation.code === 'dashboard_unavailable'),
    );
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
