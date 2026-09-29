import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const checker = path.join(repoRoot, 'scripts/check-chat-eval.mjs');
const diagnosticCommit = '1f1292dcc8d80c3f2a5c69a4045b8798bd62cb4e';
const diagnosticBlobs = {
  'app_gen/_config/chat_evals/_schemas/answer-envelope-v1.schema.json': '0caf8450562b0356c0c92a4be27fceb28dc34610',
  'app_gen/_config/chat_evals/_schemas/chat-eval-fixture.schema.json': '7113f5145555d4ed04276a7adb5548112d3882e4',
  'app_gen/_config/chat_evals/_schemas/chat-transcript-jsonl-event-v1.schema.json': '077dd6a905dd7386132a2cc156fc1e1f46df41d6',
  'app_gen/_config/chat_evals/foundation/attention_recommendation.yaml': 'de5aea92bf3e1836b3ab1e1dd8aabea8fefd687c',
  'app_gen/_config/chat_evals/nexus/stalled_tasks_basic.yaml': 'b1444fad18d37239a3b4d1e024bc9258c34ef8f3',
  'app_gen/_config/chat_evals/nexus/write_gate_intent_preview.yaml': '72c4adae09a7152fa42da4c3a03ed42f5e01e3b2',
  'app_gen/_config/chat_evals/red_team/cross_tenant_leak_blocked.yaml': '292840486467aa87b4c9a78447457157810c45cf'
};

let suiteRoot;
let baselineRoot;

function git(...args) {
  return execFileSync('git', args, { cwd: repoRoot });
}

function fixturePath(root, relativePath) {
  return path.join(root, relativePath.replace('app_gen/_config/chat_evals/', ''));
}

function readFixture(root, relativePath = 'foundation/attention_recommendation.yaml') {
  return JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8'));
}

function writeFixture(root, value, relativePath = 'foundation/attention_recommendation.yaml') {
  fs.writeFileSync(path.join(root, relativePath), `${JSON.stringify(value, null, 2)}\n`);
}

function run(root, extraArgs = [], environment = {}) {
  const artifact = `${root}.report.json`;
  const result = spawnSync(process.execPath, [checker, '--fixture-root', root, '--artifact', artifact, '--json', ...extraArgs], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: { ...process.env, ...environment }
  });
  assert.equal(fs.existsSync(artifact), true, result.stderr || result.stdout);
  return { ...result, report: JSON.parse(fs.readFileSync(artifact, 'utf8')) };
}

function mutate(name, callback, expectedCode) {
  const root = path.join(suiteRoot, name);
  fs.cpSync(baselineRoot, root, { recursive: true });
  const fixture = readFixture(root);
  callback(fixture);
  writeFixture(root, fixture);
  const result = run(root);
  assert.equal(result.status, 1, `${name} unexpectedly passed`);
  assert.ok(result.report.findings.some((finding) => finding.code === expectedCode), `${name} lacked ${expectedCode}: ${JSON.stringify(result.report.findings)}`);
}

test.before(() => {
  suiteRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'appfw-chat-eval-contract-'));
  baselineRoot = path.join(suiteRoot, 'diagnostic-fixtures');
  for (const [relativePath, expectedBlob] of Object.entries(diagnosticBlobs)) {
    const actual = git('rev-parse', `${diagnosticCommit}:${relativePath}`).toString().trim();
    assert.equal(actual, expectedBlob, `diagnostic lineage changed for ${relativePath}`);
    const outputPath = fixturePath(baselineRoot, relativePath);
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, git('show', `${diagnosticCommit}:${relativePath}`));
  }
});

test.after(() => {
  fs.rmSync(suiteRoot, { recursive: true, force: true });
});

test('diagnostic Git-object corpus passes and output is byte-identical on replay', () => {
  const first = run(baselineRoot);
  assert.equal(first.status, 0, JSON.stringify(first.report.findings));
  const firstBytes = fs.readFileSync(`${baselineRoot}.report.json`);
  const second = run(baselineRoot);
  assert.equal(second.status, 0, JSON.stringify(second.report.findings));
  assert.deepEqual(fs.readFileSync(`${baselineRoot}.report.json`), firstBytes);
});

test('answer-envelope schema and provenance mutations fail closed', async (t) => {
  const cases = [
    ['reference-provenance', (f) => delete f.transcript[3].payload.data.refs[0].provenance, 'answer_envelope_schema_violation'],
    ['reference-freshness', (f) => { f.transcript[3].payload.data.refs[0].freshness_watermark = 'yesterday'; }, 'answer_envelope_schema_violation'],
    ['reference-freshness-no-timezone', (f) => { f.transcript[3].payload.data.refs[0].freshness_watermark = '2026-07-18T00:00:00'; }, 'answer_envelope_schema_violation'],
    ['citation-provenance', (f) => delete f.transcript[3].payload.data.citations[0].provenance, 'answer_envelope_schema_violation'],
    ['citation-freshness', (f) => { f.transcript[3].payload.data.citations[0].freshness_watermark = 'not-a-date'; }, 'answer_envelope_schema_violation'],
    ['locator', (f) => { f.transcript[3].payload.data.refs[0].record_locator = 'provider-id-1'; }, 'answer_envelope_schema_violation'],
    ['confidence', (f) => { f.transcript[3].payload.data.confidence = 1.1; }, 'answer_envelope_schema_violation'],
    ['missing-required', (f) => delete f.transcript[3].payload.data.version, 'answer_envelope_schema_violation'],
    ['wrong-type', (f) => { f.transcript[3].payload.data.refs = {}; }, 'answer_envelope_schema_violation'],
    ['additional-property', (f) => { f.transcript[3].payload.data.provider_payload = {}; }, 'answer_envelope_schema_violation']
  ];
  for (const [name, callback, code] of cases) {
    await t.test(name, () => mutate(name, callback, code));
  }
});

test('RFC 3339 date-time validation is calendar-aware and timezone-independent', async (t) => {
  const valid = [
    '2026-07-18T00:00:00Z',
    '2026-07-18t00:00:00Z',
    '2026-07-18T00:00:00z',
    '2026-07-18t00:00:00z',
    '2026-07-18T05:30:00+05:30',
    '2024-02-29T23:59:59.123456-08:00'
  ];
  for (const [index, value] of valid.entries()) {
    await t.test(`valid-${index}`, () => {
      const root = path.join(suiteRoot, `datetime-valid-${index}`);
      fs.cpSync(baselineRoot, root, { recursive: true });
      const fixture = readFixture(root);
      fixture.transcript[3].payload.data.refs[0].freshness_watermark = value;
      fixture.replay_transcript[3].payload.data.refs[0].freshness_watermark = value;
      fixture.transcript[3].payload.data.citations[0].freshness_watermark = value;
      fixture.replay_transcript[3].payload.data.citations[0].freshness_watermark = value;
      writeFixture(root, fixture);
      assert.equal(run(root).status, 0);
    });
  }
  const invalid = [
    ['timezone-less', '2026-07-18T00:00:00'],
    ['date-only', '2026-07-18'],
    ['calendar', '2026-02-30T00:00:00Z'],
    ['time', '2026-07-18T24:00:00Z'],
    ['offset', '2026-07-18T00:00:00+24:00'],
    ['arbitrary-leap-second', '2026-07-18T12:00:60Z'],
    ['boundary-leap-second', '2026-06-30T23:59:60Z'],
    ['trailing', '2026-07-18T00:00:00Z trailing']
  ];
  for (const [name, value] of invalid) {
    await t.test(`invalid-reference-${name}`, () => mutate(`datetime-reference-${name}`, (fixture) => {
      fixture.transcript[3].payload.data.refs[0].freshness_watermark = value;
    }, 'answer_envelope_schema_violation'));
    await t.test(`invalid-citation-${name}`, () => mutate(`datetime-citation-${name}`, (fixture) => {
      fixture.transcript[3].payload.data.citations[0].freshness_watermark = value;
    }, 'answer_envelope_schema_violation'));
  }
  await t.test('timezone environments produce byte-identical output', () => {
    const utc = run(baselineRoot, [], { TZ: 'UTC' });
    assert.equal(utc.status, 0);
    const utcBytes = fs.readFileSync(`${baselineRoot}.report.json`);
    const pacific = run(baselineRoot, [], { TZ: 'America/Los_Angeles' });
    assert.equal(pacific.status, 0);
    assert.deepEqual(fs.readFileSync(`${baselineRoot}.report.json`), utcBytes);
  });
  await t.test('timezone environments produce byte-identical rejection', () => {
    const root = path.join(suiteRoot, 'datetime-invalid-two-timezones');
    fs.cpSync(baselineRoot, root, { recursive: true });
    const fixture = readFixture(root);
    fixture.transcript[3].payload.data.refs[0].freshness_watermark = '2026-07-18T00:00:00';
    fixture.replay_transcript[3].payload.data.refs[0].freshness_watermark = '2026-07-18T00:00:00';
    writeFixture(root, fixture);
    const utc = run(root, [], { TZ: 'UTC' });
    assert.equal(utc.status, 1);
    const utcFindings = utc.report.findings.filter((finding) => finding.code === 'answer_envelope_schema_violation');
    assert.equal(utcFindings.length, 1);
    assert.equal(utc.report.scenarios.find((scenario) => scenario.name === fixture.name).ok, false);
    const utcBytes = fs.readFileSync(`${root}.report.json`);
    const pacific = run(root, [], { TZ: 'America/Los_Angeles' });
    assert.equal(pacific.status, utc.status);
    const pacificFindings = pacific.report.findings.filter((finding) => finding.code === 'answer_envelope_schema_violation');
    assert.deepEqual(pacificFindings, utcFindings);
    assert.equal(pacific.report.scenarios.find((scenario) => scenario.name === fixture.name).ok, false);
    assert.deepEqual(fs.readFileSync(`${root}.report.json`), utcBytes);
  });
});

test('recommendation mutations fail closed', async (t) => {
  const cases = [
    ['recommendation-version', (f) => { f.answer_envelope.recommendation.version = 'recommendation@2'; }, 'invalid_recommendation_version'],
    ['recommendation-why', (f) => { f.answer_envelope.recommendation.why_now = ''; }, 'missing_recommendation_why_now'],
    ['recommendation-next-step', (f) => { f.answer_envelope.recommendation.safest_permitted_next_step = ' '; }, 'missing_recommendation_next_step'],
    ['recommendation-preview', (f) => { f.answer_envelope.recommendation.preview_only = 'true'; }, 'recommendation_not_preview_only'],
    ['recommendation-authority', (f) => { f.answer_envelope.recommendation.write_result = { ok: true }; }, 'recommendation_authority_violation']
  ];
  for (const [name, callback, code] of cases) await t.test(name, () => mutate(name, callback, code));
});

test('evaluation posture mutations fail closed without coercion', async (t) => {
  const cases = [
    ['evaluation-version', (f) => { f.expect.evaluation.version = 'ai_evaluation@2'; }, 'invalid_evaluation_version'],
    ['evaluation-mode', (f) => { f.expect.evaluation.mode = 'live'; }, 'invalid_evaluation_mode'],
    ['evaluation-provenance', (f) => { f.expect.evaluation.provenance = 'recorded_live'; }, 'invalid_evaluation_provenance'],
    ['evaluation-release', (f) => { f.expect.evaluation.release_ready = 'false'; }, 'invalid_evaluation_release_ready'],
    ['evaluation-live', (f) => delete f.expect.evaluation.live_ready, 'invalid_evaluation_live_ready'],
    ['evaluation-replay', (f) => { f.expect.evaluation.replay_structural_diff = [{ op: 'add' }]; }, 'evaluation_replay_diff']
  ];
  for (const [name, callback, code] of cases) await t.test(name, () => mutate(name, callback, code));
});

test('fallback and negative dispositions are explicit and fail closed', async (t) => {
  await t.test('unsupported view uses declared fallback', () => {
    const root = path.join(suiteRoot, 'fallback-pass');
    fs.cpSync(baselineRoot, root, { recursive: true });
    const fixture = readFixture(root);
    fixture.expect.disposition = { kind: 'unsupported_view', fallback_used: true, fallback_view: fixture.answer_envelope.fallback_view };
    writeFixture(root, fixture);
    assert.equal(run(root).status, 0);
  });
  await t.test('corrupt fallback fails', () => mutate('fallback-corrupt', (f) => {
    f.expect.disposition = { kind: 'unsupported_view', fallback_used: true, fallback_view: 'provider-ui' };
  }, 'invalid_unsupported_view_fallback'));
  for (const kind of ['unsupported', 'unavailable', 'partial', 'stale', 'malformed', 'policy_denied']) {
    await t.test(kind, () => {
      const root = path.join(suiteRoot, `negative-${kind}`);
      fs.cpSync(baselineRoot, root, { recursive: true });
      const fixture = readFixture(root);
      fixture.transcript = fixture.transcript.filter((event) => event.type !== 'answer_envelope');
      fixture.replay_transcript = fixture.replay_transcript.filter((event) => event.type !== 'answer_envelope');
      fixture.answer_envelope.unverified = true;
      fixture.expect.disposition = { kind, error: `${kind}_fixture_error`, fallback_used: false };
      writeFixture(root, fixture);
      assert.equal(run(root).status, 0);
    });
  }
});

test('Route A foundation-case shape enforces fallback, error, and authority posture', async (t) => {
  const declaredCases = {
    unsupported_request: 'deterministic_fallback',
    policy_denial: 'fail_closed_fallback',
    stale_evidence: 'unverified_fallback',
    partial_dependency: 'fail_closed_fallback',
    unavailable_dependency: 'fail_closed_fallback',
    malformed_provider_result: 'reject_and_fallback'
  };
  const eventCodes = {
    unsupported_request: 'unsupported_request',
    policy_denial: 'policy_denied',
    stale_evidence: 'stale_evidence',
    partial_dependency: 'partial_dependency',
    unavailable_dependency: 'unavailable_dependency',
    malformed_provider_result: 'malformed_provider_result'
  };
  function configure(fixture) {
    fixture.transcript = fixture.transcript.filter((event) => event.type !== 'answer_envelope');
    fixture.replay_transcript = fixture.replay_transcript.filter((event) => event.type !== 'answer_envelope');
    fixture.answer_envelope.unverified = true;
    fixture.expect.foundation_cases = structuredClone(declaredCases);
    fixture.context.cases = Object.keys(declaredCases);
    for (const code of Object.values(eventCodes)) {
      const event = { type: 'error', code, fallback_view: fixture.answer_envelope.fallback_view, authority: 'none' };
      fixture.transcript.push(event);
      fixture.replay_transcript.push(structuredClone(event));
    }
  }
  await t.test('complete declared matrix passes', () => {
    const root = path.join(suiteRoot, 'foundation-cases-pass');
    fs.cpSync(baselineRoot, root, { recursive: true });
    const fixture = readFixture(root);
    configure(fixture);
    writeFixture(root, fixture);
    assert.equal(run(root).status, 0);
  });
  await t.test('missing error fails', () => mutate('foundation-missing-error', (fixture) => {
    configure(fixture);
    fixture.transcript = fixture.transcript.filter((event) => event.code !== 'policy_denied');
    fixture.replay_transcript = fixture.replay_transcript.filter((event) => event.code !== 'policy_denied');
  }, 'foundation_case_error_mismatch'));
  for (const name of Object.keys(declaredCases)) {
    await t.test(`missing declaration ${name} fails`, () => mutate(`foundation-missing-${name}`, (fixture) => {
      configure(fixture);
      delete fixture.expect.foundation_cases[name];
    }, 'foundation_case_declaration_mismatch'));
  }
  await t.test('deleting from both mutable declarations still fails', () => mutate('foundation-missing-both', (fixture) => {
    configure(fixture);
    delete fixture.expect.foundation_cases.policy_denial;
    fixture.context.cases = fixture.context.cases.filter((name) => name !== 'policy_denial');
  }, 'foundation_case_declaration_mismatch'));
  await t.test('unexpected declaration fails', () => mutate('foundation-unexpected', (fixture) => {
    configure(fixture);
    fixture.expect.foundation_cases.provider_specific = 'deterministic_fallback';
  }, 'foundation_case_declaration_mismatch'));
  await t.test('wrong posture fails', () => mutate('foundation-wrong-posture', (fixture) => {
    configure(fixture);
    fixture.expect.foundation_cases.policy_denial = 'deterministic_fallback';
  }, 'invalid_foundation_case_posture'));
  await t.test('wrong fallback fails', () => mutate('foundation-wrong-fallback', (fixture) => {
    configure(fixture);
    fixture.transcript.find((event) => event.code === 'stale_evidence').fallback_view = 'provider-ui';
    fixture.replay_transcript.find((event) => event.code === 'stale_evidence').fallback_view = 'provider-ui';
  }, 'foundation_case_fallback_mismatch'));
  await t.test('authority fails', () => mutate('foundation-authority', (fixture) => {
    configure(fixture);
    fixture.transcript.find((event) => event.code === 'unsupported_request').authority = 'tool';
    fixture.replay_transcript.find((event) => event.code === 'unsupported_request').authority = 'tool';
  }, 'foundation_case_authority_violation'));
});

test('cross-tenant injection remains a named fail-closed result', () => {
  const result = run(baselineRoot, ['--inject-leak']);
  assert.equal(result.status, 1);
  assert.ok(result.report.findings.some((finding) => finding.contract === 'chat_tenant_isolation_contract' && finding.code === 'policy_leak'));
});
