#!/usr/bin/env node
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

const args = process.argv.slice(2);
const jsonOutput = args.includes('--json');
const injectLeak = args.includes('--inject-leak');
const pipelinePlan = args.includes('--pipeline-plan');

function optionValue(name, fallback) {
  const index = args.indexOf(name);
  if (index === -1) {
    return fallback;
  }
  const value = args[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`${name} requires a value`);
  }
  return value;
}

const fixtureRoot = path.resolve(repoRoot, optionValue('--fixture-root', 'app_gen/_config/chat_evals'));
const artifactPath = path.resolve(repoRoot, optionValue('--artifact', 'target/appfw/chat-eval.json'));
const pipelineArtifactPath = path.resolve(
  repoRoot,
  optionValue('--pipeline-artifact', 'target/appfw/chat-eval-promptfoo-plan.json')
);
const schemaPath = path.join(fixtureRoot, '_schemas/chat-eval-fixture.schema.json');
const answerEnvelopeSchemaPath = path.join(fixtureRoot, '_schemas/answer-envelope-v1.schema.json');
const transcriptEventSchemaPath = path.join(fixtureRoot, '_schemas/chat-transcript-jsonl-event-v1.schema.json');
const specPath = path.join(repoRoot, 'docs/architecture/concerns/chat-eval-spec.md');
const requiredSchemaArtifacts = [
  {
    filePath: schemaPath,
    code: 'missing_fixture_schema',
    message: 'chat-eval fixture schema artifact is required'
  },
  {
    filePath: answerEnvelopeSchemaPath,
    code: 'missing_answer_envelope_schema',
    message: 'answer-envelope v1 schema artifact is required'
  },
  {
    filePath: transcriptEventSchemaPath,
    code: 'missing_transcript_event_schema',
    message: 'chat transcript JSONL event schema artifact is required'
  }
];
let answerEnvelopeSchema;

const requiredChecks = [
  'chat_transcript_schema_contract',
  'chat_answer_envelope_contract',
  'chat_reference_provenance_contract',
  'chat_envelope_compatibility_contract',
  'chat_recommendation_contract',
  'chat_evaluation_posture_contract',
  'chat_scenario_disposition_contract',
  'chat_tool_registry_binding_contract',
  'chat_citation_resolvability_contract',
  'chat_tenant_isolation_contract',
  'chat_write_gate_contract',
  'chat_replay_determinism_contract'
];

const recordLocatorPattern = /^rl_[0-9a-f]{32}$/;
const negativeDispositionKinds = new Set([
  'unsupported',
  'unavailable',
  'partial',
  'stale',
  'malformed',
  'policy_denied'
]);
const foundationCaseContracts = new Map([
  ['unsupported_request', { eventCode: 'unsupported_request', posture: 'deterministic_fallback' }],
  ['policy_denial', { eventCode: 'policy_denied', posture: 'fail_closed_fallback' }],
  ['stale_evidence', { eventCode: 'stale_evidence', posture: 'unverified_fallback' }],
  ['partial_dependency', { eventCode: 'partial_dependency', posture: 'fail_closed_fallback' }],
  ['unavailable_dependency', { eventCode: 'unavailable_dependency', posture: 'fail_closed_fallback' }],
  ['malformed_provider_result', { eventCode: 'malformed_provider_result', posture: 'reject_and_fallback' }]
]);

const allowedAuthTokens = new Set([
  'pdsh_admin',
  'cc_tenant_user',
  'west_sales_rep',
  'other_tenant_admin'
]);
const allowedProvenance = new Set(['synthetic', 'vendor_export', 'recorded_live']);
const allowedEventTypes = new Set(['message', 'tool_call', 'tool_result', 'card', 'citation', 'answer_envelope', 'error']);
const forbiddenToolArgKeys = new Set([
  'raw_sql',
  'sql',
  'raw_query',
  'query_text',
  'table',
  'url',
  'endpoint',
  'http_url'
]);

function rel(filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join('/');
}

function sha256(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

function readJsonCompatibleFixture(filePath) {
  const text = fs.readFileSync(filePath, 'utf8');
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(
      `${rel(filePath)} must be JSON-compatible YAML for the zero-dependency CH6 runner: ${error.message}`
    );
  }
}

function walkFiles(root) {
  if (!fs.existsSync(root)) {
    return [];
  }
  const entries = fs.readdirSync(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === '_schemas') {
        continue;
      }
      files.push(...walkFiles(fullPath));
    } else if (entry.isFile() && /\.(json|ya?ml)$/i.test(entry.name)) {
      files.push(fullPath);
    }
  }
  return files.sort();
}

function stable(value) {
  if (Array.isArray(value)) {
    const values = value.map(stable);
    if (values.every((item) => ['string', 'number', 'boolean'].includes(typeof item) || item === null)) {
      return [...values].sort();
    }
    return values;
  }
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, stable(value[key])])
    );
  }
  return value;
}

function canonicalJson(value) {
  return JSON.stringify(stable(value));
}

function generatedAtUtc() {
  const epoch = process.env.SOURCE_DATE_EPOCH;
  if (epoch !== undefined) {
    const milliseconds = Number(epoch) * 1000;
    if (!Number.isFinite(milliseconds)) {
      throw new Error('SOURCE_DATE_EPOCH must be numeric');
    }
    return new Date(milliseconds).toISOString();
  }
  return '1970-01-01T00:00:00.000Z';
}

function isNonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function isDateTime(value) {
  if (!isNonEmptyString(value)) {
    return false;
  }
  const match = /^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(?:[Zz]|([+-])(\d{2}):(\d{2}))$/.exec(value);
  if (!match) {
    return false;
  }
  const [, yearText, monthText, dayText, hourText, minuteText, secondText, , , offsetHourText, offsetMinuteText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);
  const offsetHour = offsetHourText === undefined ? 0 : Number(offsetHourText);
  const offsetMinute = offsetMinuteText === undefined ? 0 : Number(offsetMinuteText);
  if (year === 0 || month < 1 || month > 12 || hour > 23 || minute > 59 || second > 59 || offsetHour > 23 || offsetMinute > 59) {
    return false;
  }
  const monthLengths = [31, year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  return day >= 1 && day <= monthLengths[month - 1];
}

function resolveLocalRef(rootSchema, ref) {
  if (typeof ref !== 'string' || !ref.startsWith('#/')) {
    return null;
  }
  return ref
    .slice(2)
    .split('/')
    .map((part) => part.replaceAll('~1', '/').replaceAll('~0', '~'))
    .reduce((value, part) => value?.[part], rootSchema);
}

function validateJsonSchema(value, schema, rootSchema, instancePath = '$') {
  const errors = [];
  if (!schema || typeof schema !== 'object') {
    return [{ instance_path: instancePath, keyword: 'schema', message: 'schema node must be an object' }];
  }
  if (schema.$ref) {
    const resolved = resolveLocalRef(rootSchema, schema.$ref);
    if (!resolved) {
      return [{ instance_path: instancePath, keyword: '$ref', message: `unresolved local schema ref ${schema.$ref}` }];
    }
    return validateJsonSchema(value, resolved, rootSchema, instancePath);
  }
  if (Object.hasOwn(schema, 'const') && value !== schema.const) {
    errors.push({ instance_path: instancePath, keyword: 'const', message: `must equal ${JSON.stringify(schema.const)}` });
  }
  const actualType = Array.isArray(value) ? 'array' : value === null ? 'null' : typeof value;
  if (schema.type && actualType !== schema.type) {
    errors.push({ instance_path: instancePath, keyword: 'type', message: `must be ${schema.type}` });
    return errors;
  }
  if (actualType === 'object') {
    for (const key of schema.required || []) {
      if (!Object.hasOwn(value, key)) {
        errors.push({ instance_path: instancePath, keyword: 'required', message: `missing required property ${key}` });
      }
    }
    const properties = schema.properties || {};
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!Object.hasOwn(properties, key)) {
          errors.push({ instance_path: `${instancePath}.${key}`, keyword: 'additionalProperties', message: 'unexpected property' });
        }
      }
    }
    for (const [key, childSchema] of Object.entries(properties)) {
      if (Object.hasOwn(value, key)) {
        errors.push(...validateJsonSchema(value[key], childSchema, rootSchema, `${instancePath}.${key}`));
      }
    }
  }
  if (actualType === 'array' && schema.items) {
    for (const [index, item] of value.entries()) {
      errors.push(...validateJsonSchema(item, schema.items, rootSchema, `${instancePath}[${index}]`));
    }
  }
  if (actualType === 'string') {
    if (schema.minLength !== undefined && value.length < schema.minLength) {
      errors.push({ instance_path: instancePath, keyword: 'minLength', message: `must contain at least ${schema.minLength} characters` });
    }
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) {
      errors.push({ instance_path: instancePath, keyword: 'pattern', message: `must match ${schema.pattern}` });
    }
    if (schema.format === 'date-time' && !isDateTime(value)) {
      errors.push({ instance_path: instancePath, keyword: 'format', message: 'must be a valid date-time' });
    }
  }
  if (actualType === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) {
      errors.push({ instance_path: instancePath, keyword: 'minimum', message: `must be >= ${schema.minimum}` });
    }
    if (schema.maximum !== undefined && value > schema.maximum) {
      errors.push({ instance_path: instancePath, keyword: 'maximum', message: `must be <= ${schema.maximum}` });
    }
  }
  return errors;
}

function envelopeLocators(envelope) {
  return [...new Set(findRecordLocators({ refs: envelope?.refs, citations: envelope?.citations }))].sort();
}

function stringArray(value) {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function collectObjectKeys(value, keys = []) {
  if (Array.isArray(value)) {
    for (const item of value) {
      collectObjectKeys(item, keys);
    }
  } else if (value && typeof value === 'object') {
    for (const [key, nested] of Object.entries(value)) {
      keys.push(key);
      collectObjectKeys(nested, keys);
    }
  }
  return keys;
}

function findRecordLocators(value, locators = []) {
  if (typeof value === 'string' && /^rl_[0-9a-f]{32}$/.test(value)) {
    locators.push(value);
  } else if (Array.isArray(value)) {
    for (const item of value) {
      findRecordLocators(item, locators);
    }
  } else if (value && typeof value === 'object') {
    for (const nested of Object.values(value)) {
      findRecordLocators(nested, locators);
    }
  }
  return locators;
}

function artifactFor(filePath) {
  if (!fs.existsSync(filePath)) {
    return {
      path: rel(filePath),
      present: false,
      sha256: null,
      bytes: 0
    };
  }
  const bytes = fs.readFileSync(filePath);
  return {
    path: rel(filePath),
    present: true,
    sha256: sha256(bytes),
    bytes: bytes.length
  };
}

function hasIntentPreview(transcript, operationName) {
  return transcript.some(
    (event) =>
      event &&
      event.type === 'card' &&
      event.kind === 'IntentPreview' &&
      (!operationName || event.operation === operationName)
  );
}

function promptfooPipelinePlan(report, fixtureFiles) {
  return {
    command: 'chat-eval-promptfoo-plan',
    lane: 'CH6',
    ok: report.ok,
    release_ready: false,
    generated_at_utc: report.generated_at_utc,
    mode: 'pipeline-plan',
    deterministic_artifact: rel(artifactPath),
    artifact: rel(pipelineArtifactPath),
    inputs: {
      fixture_dir: report.inputs.fixture_dir,
      fixture_count: report.inputs.fixture_count,
      fixture_files: fixtureFiles.map(rel),
      provenance_counts: report.inputs.provenance_counts
    },
    promptfoo: {
      dependency_required: false,
      dependency_installation: 'pipeline-managed; not required for local deterministic chat-eval',
      offline_only: true,
      command: 'promptfoo eval -o target/appfw/chat-eval-promptfoo-results.json',
      results_artifact: 'target/appfw/chat-eval-promptfoo-results.json',
      required_assertions: [
        'is-json',
        'contains-json',
        'tool-call-f1'
      ],
      required_contracts: [
        'answer_envelope_schema',
        'tool_registry_binding',
        'citation_resolvability',
        'tenant_isolation',
        'write_gate_intent_preview',
        'replay_determinism'
      ],
      evidence_status: 'planned',
      release_evidence_status: 'not-live-evidence'
    },
    scenarios: report.scenarios.map((scenario) => ({
      name: scenario.name,
      path: scenario.path,
      provenance: scenario.provenance,
      ok: scenario.ok,
      finding_count: scenario.finding_count
    })),
    findings: report.findings,
    next_steps: [
      'Generate a promptfoo configuration from the retained chat-eval fixtures once the pipeline dependency is approved.',
      'Run promptfoo only against offline fixtures or approved recorded fixtures in CI; do not make live LLM calls in PIPELINE tier.',
      'Feed target/appfw/chat-eval-promptfoo-results.json into a future release-evidence validator without changing local fixture release_ready:false posture.'
    ]
  };
}

function checkScenario(scenario, filePath) {
  const findings = [];
  const checkFindings = Object.fromEntries(requiredChecks.map((name) => [name, []]));
  const observableOutput = {
    transcript: scenario.transcript,
    answer_envelope: scenario.answer_envelope
  };

  if (injectLeak && scenario.name === 'stalled_tasks_basic') {
    const seeded = scenario.fixture_store?.cross_tenant_seeded_ids?.[0];
    if (seeded) {
      scenario.transcript = [
        ...(Array.isArray(scenario.transcript) ? scenario.transcript : []),
        {
          type: 'message',
          role: 'assistant',
          content: `Injected leak sentinel ${seeded}`
        }
      ];
      observableOutput.transcript = scenario.transcript;
    }
  }

  function add(contract, code, message, detail = {}) {
    const finding = {
      contract,
      code,
      message,
      path: rel(filePath),
      scenario: scenario.name || rel(filePath),
      ...detail
    };
    findings.push(finding);
    checkFindings[contract].push(finding);
  }

  const requiredFields = [
    'name',
    'description',
    'auth_token',
    'provenance',
    'query',
    'context',
    'registry',
    'fixture_store',
    'transcript',
    'replay_transcript',
    'answer_envelope',
    'expect'
  ];
  for (const field of requiredFields) {
    if (!(field in scenario)) {
      add('chat_transcript_schema_contract', 'missing_field', `${field} is required`);
    }
  }

  if (!allowedAuthTokens.has(scenario.auth_token)) {
    add('chat_transcript_schema_contract', 'invalid_auth_token', 'auth_token is not in the known fixture token vocabulary', {
      auth_token: scenario.auth_token
    });
  }
  if (!allowedProvenance.has(scenario.provenance)) {
    add('chat_transcript_schema_contract', 'invalid_provenance', 'provenance must be synthetic, vendor_export, or recorded_live', {
      provenance: scenario.provenance
    });
  }
  if (!Array.isArray(scenario.transcript) || scenario.transcript.length === 0) {
    add('chat_transcript_schema_contract', 'empty_transcript', 'transcript must contain at least one event');
  }
  if (!Array.isArray(scenario.replay_transcript) || scenario.replay_transcript.length === 0) {
    add('chat_transcript_schema_contract', 'empty_replay_transcript', 'replay_transcript must contain at least one event');
  }

  for (const [index, event] of (Array.isArray(scenario.transcript) ? scenario.transcript : []).entries()) {
    if (!event || typeof event !== 'object') {
      add('chat_transcript_schema_contract', 'invalid_event', `transcript event ${index} must be an object`);
      continue;
    }
    if (!allowedEventTypes.has(event.type)) {
      add('chat_transcript_schema_contract', 'invalid_event_type', `unsupported event type ${event.type}`, {
        event_index: index
      });
    }
    if (event.type === 'answer_envelope') {
      if (event.schema !== 'chat_transcript_jsonl@1') {
        add(
          'chat_transcript_schema_contract',
          'invalid_answer_envelope_event_schema',
          'answer_envelope transcript events must use chat_transcript_jsonl@1',
          { event_index: index, schema: event.schema }
        );
      }
      const payload = event.payload;
      if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
        add(
          'chat_transcript_schema_contract',
          'invalid_answer_envelope_event_payload',
          'answer_envelope transcript event payload must be an object',
          { event_index: index }
        );
      } else {
        if (payload.type !== 'data-answer-envelope') {
          add(
            'chat_transcript_schema_contract',
            'invalid_answer_envelope_data_part_type',
            'answer_envelope transcript event payload.type must be data-answer-envelope',
            { event_index: index, payload_type: payload.type }
          );
        }
        const data = payload.data;
        if (!data || typeof data !== 'object' || Array.isArray(data)) {
          add(
            'chat_transcript_schema_contract',
            'missing_answer_envelope_event_data',
            'answer_envelope transcript event payload.data must be an object',
            { event_index: index }
          );
        } else {
          if (data.version !== 'answer_envelope@1') {
            add(
              'chat_transcript_schema_contract',
              'invalid_answer_envelope_event_version',
              'answer_envelope transcript event data.version must be answer_envelope@1',
              { event_index: index, version: data.version }
            );
          }
          if (!Array.isArray(data.refs)) {
            add(
              'chat_transcript_schema_contract',
              'missing_answer_envelope_event_refs',
              'answer_envelope transcript event data.refs must be an array',
              { event_index: index }
            );
          }
          if (!Array.isArray(data.citations)) {
            add(
              'chat_transcript_schema_contract',
              'missing_answer_envelope_event_citations',
              'answer_envelope transcript event data.citations must be an array',
              { event_index: index }
            );
          }
          if (!data.fallback_view || typeof data.fallback_view !== 'string') {
            add(
              'chat_transcript_schema_contract',
              'missing_answer_envelope_event_fallback_view',
              'answer_envelope transcript event data.fallback_view is required',
              { event_index: index }
            );
          }
        }
      }
    }
  }

  const envelope = scenario.answer_envelope || {};
  if (!envelope || typeof envelope !== 'object') {
    add('chat_transcript_schema_contract', 'missing_envelope', 'answer_envelope must be an object');
  } else {
    if (envelope.version !== 'answer_envelope@1') {
      add('chat_transcript_schema_contract', 'invalid_envelope_version', 'answer_envelope.version must be answer_envelope@1', {
        version: envelope.version
      });
    }
    if (!Array.isArray(envelope.refs)) {
      add('chat_transcript_schema_contract', 'missing_envelope_refs', 'answer_envelope.refs must be an array');
    }
    if (!Array.isArray(envelope.citations)) {
      add('chat_transcript_schema_contract', 'missing_envelope_citations', 'answer_envelope.citations must be an array');
    }
    if (!envelope.fallback_view || typeof envelope.fallback_view !== 'string') {
      add('chat_transcript_schema_contract', 'missing_fallback_view', 'answer_envelope.fallback_view is required');
    }
  }

  const transcriptEnvelopes = (Array.isArray(scenario.transcript) ? scenario.transcript : [])
    .filter((event) => event?.type === 'answer_envelope')
    .map((event) => event?.payload?.data)
    .filter((data) => data && typeof data === 'object' && !Array.isArray(data));
  for (const [index, data] of transcriptEnvelopes.entries()) {
    const schemaErrors = validateJsonSchema(data, answerEnvelopeSchema, answerEnvelopeSchema);
    for (const error of schemaErrors) {
      add('chat_answer_envelope_contract', 'answer_envelope_schema_violation', error.message, {
        event_index: index,
        instance_path: error.instance_path,
        schema_keyword: error.keyword
      });
    }
  }

  const records = Array.isArray(scenario.fixture_store?.records) ? scenario.fixture_store.records : [];
  const resolvableIds = new Set(records.map((record) => record?.id).filter((id) => typeof id === 'string'));
  for (const [envelopeIndex, data] of transcriptEnvelopes.entries()) {
    for (const [kind, values] of [['reference', data.refs], ['citation', data.citations]]) {
      for (const [index, value] of (Array.isArray(values) ? values : []).entries()) {
        const locator = value?.record_locator;
        const detail = { envelope_index: envelopeIndex, item_index: index, item_kind: kind };
        if (!isNonEmptyString(value?.source)) {
          add('chat_reference_provenance_contract', `missing_${kind}_source`, `${kind} source must be non-empty`, detail);
        }
        if (!isNonEmptyString(value?.provenance)) {
          add('chat_reference_provenance_contract', `missing_${kind}_provenance`, `${kind} provenance must be non-empty`, detail);
        }
        if (!isDateTime(value?.freshness_watermark)) {
          add('chat_reference_provenance_contract', `invalid_${kind}_freshness`, `${kind} freshness_watermark must be a valid date-time`, detail);
        }
        if (!recordLocatorPattern.test(locator || '')) {
          add('chat_reference_provenance_contract', `invalid_${kind}_locator`, `${kind} record_locator must be opaque`, detail);
        } else if (!resolvableIds.has(locator)) {
          add('chat_reference_provenance_contract', `unresolved_${kind}_locator`, `${kind} record_locator must resolve in the authorized fixture store`, { ...detail, record_locator: locator });
        }
      }
    }
  }

  if (transcriptEnvelopes.length > 0) {
    const topLocators = envelopeLocators(envelope);
    const transcriptLocators = [...new Set(transcriptEnvelopes.flatMap(envelopeLocators))].sort();
    const missingTopLevelLocators = transcriptLocators.filter((locator) => !topLocators.includes(locator));
    if (missingTopLevelLocators.length > 0) {
      add('chat_envelope_compatibility_contract', 'envelope_locator_mismatch', 'every transcript envelope locator must be represented by top-level addressing', {
        top_level: topLocators,
        transcript: transcriptLocators,
        missing_top_level: missingTopLevelLocators
      });
    }
    for (const field of ['fallback_view', 'view_hint']) {
      const transcriptValues = [...new Set(transcriptEnvelopes.map((item) => item[field]).filter(isNonEmptyString))].sort();
      if (transcriptValues.length > 0 && (!isNonEmptyString(envelope[field]) || !transcriptValues.includes(envelope[field]))) {
        add('chat_envelope_compatibility_contract', `envelope_${field}_mismatch`, `top-level and transcript ${field} must be compatible`, {
          top_level: envelope[field] ?? null,
          transcript: transcriptValues
        });
      }
    }
  }

  if (envelope.recommendation !== undefined) {
    const recommendation = envelope.recommendation;
    if (!recommendation || typeof recommendation !== 'object' || Array.isArray(recommendation)) {
      add('chat_recommendation_contract', 'invalid_recommendation', 'recommendation must be an object');
    } else {
      if (recommendation.version !== 'recommendation@1') add('chat_recommendation_contract', 'invalid_recommendation_version', 'recommendation.version must be recommendation@1');
      if (!isNonEmptyString(recommendation.why_now)) add('chat_recommendation_contract', 'missing_recommendation_why_now', 'recommendation.why_now must be non-empty');
      if (!isNonEmptyString(recommendation.safest_permitted_next_step)) add('chat_recommendation_contract', 'missing_recommendation_next_step', 'recommendation.safest_permitted_next_step must be non-empty');
      if (recommendation.preview_only !== true) add('chat_recommendation_contract', 'recommendation_not_preview_only', 'recommendation.preview_only must be boolean true');
      for (const forbidden of ['execution', 'tool_authority', 'write_result']) {
        if (Object.hasOwn(recommendation, forbidden)) add('chat_recommendation_contract', 'recommendation_authority_violation', `recommendation must not contain ${forbidden}`, { field: forbidden });
      }
    }
  }

  if (scenario.expect?.evaluation !== undefined) {
    const evaluation = scenario.expect.evaluation;
    const expected = {
      version: 'ai_evaluation@1',
      mode: 'local-fixture',
      provenance: 'synthetic',
      release_ready: false,
      live_ready: false
    };
    for (const [field, value] of Object.entries(expected)) {
      if (evaluation?.[field] !== value) add('chat_evaluation_posture_contract', `invalid_evaluation_${field}`, `expect.evaluation.${field} must equal ${JSON.stringify(value)}`);
    }
    if (!Array.isArray(evaluation?.replay_structural_diff) || evaluation.replay_structural_diff.length !== 0) {
      add('chat_evaluation_posture_contract', 'evaluation_replay_diff', 'expect.evaluation.replay_structural_diff must be an empty array');
    }
  }

  const disposition = scenario.expect?.disposition;
  if (disposition !== undefined) {
    if (!disposition || typeof disposition !== 'object' || Array.isArray(disposition) || !isNonEmptyString(disposition.kind)) {
      add('chat_scenario_disposition_contract', 'invalid_disposition', 'expect.disposition must declare a kind');
    } else if (disposition.kind === 'unsupported_view') {
      if (disposition.fallback_used !== true || !isNonEmptyString(disposition.fallback_view) || disposition.fallback_view !== envelope.fallback_view) {
        add('chat_scenario_disposition_contract', 'invalid_unsupported_view_fallback', 'unsupported_view must use the declared envelope fallback_view');
      }
      if (isNonEmptyString(disposition.error)) add('chat_scenario_disposition_contract', 'fallback_fabricated_error', 'successful unsupported_view fallback must not declare an error');
    } else if (negativeDispositionKinds.has(disposition.kind)) {
      if (!isNonEmptyString(disposition.error) || disposition.fallback_used === true) {
        add('chat_scenario_disposition_contract', 'invalid_negative_disposition', `${disposition.kind} must declare an error and must not claim fallback success`);
      }
      if (transcriptEnvelopes.length > 0 && envelope.unverified !== true) {
        add('chat_scenario_disposition_contract', 'negative_fabricated_success', `${disposition.kind} must not emit a verified success envelope`);
      }
    } else {
      add('chat_scenario_disposition_contract', 'unknown_disposition', `unsupported disposition kind ${disposition.kind}`);
    }
  }

  if (scenario.expect?.foundation_cases !== undefined) {
    const cases = scenario.expect.foundation_cases;
    if (!cases || typeof cases !== 'object' || Array.isArray(cases) || Object.keys(cases).length === 0) {
      add('chat_scenario_disposition_contract', 'invalid_foundation_cases', 'expect.foundation_cases must be a non-empty object');
    } else {
      const declaredNames = Object.keys(cases).sort();
      const requiredNames = [...foundationCaseContracts.keys()].sort();
      const contextNames = stringArray(scenario.context?.cases).sort();
      if (canonicalJson(declaredNames) !== canonicalJson(requiredNames)) {
        add('chat_scenario_disposition_contract', 'foundation_case_declaration_mismatch', 'expect.foundation_cases must equal the frozen six-case declaration set', {
          declared_cases: declaredNames,
          required_cases: requiredNames,
          missing_cases: requiredNames.filter((name) => !declaredNames.includes(name)),
          unexpected_cases: declaredNames.filter((name) => !requiredNames.includes(name))
        });
      }
      if (canonicalJson(contextNames) !== canonicalJson(requiredNames)) {
        add('chat_scenario_disposition_contract', 'foundation_context_case_mismatch', 'context.cases must equal the frozen six-case declaration set', {
          context_cases: contextNames,
          required_cases: requiredNames
        });
      }
      const errorEvents = (Array.isArray(scenario.transcript) ? scenario.transcript : []).filter((event) => event?.type === 'error');
      for (const [name, posture] of Object.entries(cases)) {
        const contract = foundationCaseContracts.get(name);
        if (!contract || posture !== contract.posture) {
          add('chat_scenario_disposition_contract', 'invalid_foundation_case_posture', `${name} has an unsupported expected posture`, { case_name: name, posture });
          continue;
        }
        const matching = errorEvents.filter((event) => event.code === contract.eventCode);
        if (matching.length !== 1) {
          add('chat_scenario_disposition_contract', 'foundation_case_error_mismatch', `${name} must have exactly one matching error event`, { case_name: name, event_code: contract.eventCode, count: matching.length });
          continue;
        }
        const event = matching[0];
        if (event.fallback_view !== envelope.fallback_view || !isNonEmptyString(event.fallback_view)) {
          add('chat_scenario_disposition_contract', 'foundation_case_fallback_mismatch', `${name} must use the declared envelope fallback_view`, { case_name: name });
        }
        if (event.authority !== 'none') {
          add('chat_scenario_disposition_contract', 'foundation_case_authority_violation', `${name} must declare authority none`, { case_name: name });
        }
      }
      if (envelope.unverified !== true || transcriptEnvelopes.length > 0) {
        add('chat_scenario_disposition_contract', 'foundation_case_fabricated_success', 'negative foundation cases must remain unverified and must not emit a success envelope');
      }
    }
  }

  const registryOperations = new Map();
  for (const operation of scenario.registry?.operations || []) {
    if (!operation || typeof operation !== 'object' || typeof operation.name !== 'string') {
      continue;
    }
    registryOperations.set(operation.name, operation);
  }
  if (registryOperations.size === 0) {
    add('chat_tool_registry_binding_contract', 'empty_registry', 'registry.operations must list at least one named operation');
  }

  const toolCalls = (Array.isArray(scenario.transcript) ? scenario.transcript : []).filter(
    (event) => event && event.type === 'tool_call'
  );
  const expectedToolNames = new Set((scenario.expect?.tool_calls || []).map((item) => item?.name).filter(Boolean));
  for (const expected of expectedToolNames) {
    if (!toolCalls.some((event) => event.name === expected)) {
      add('chat_tool_registry_binding_contract', 'missing_expected_tool_call', `expected tool call ${expected} was not present`);
    }
  }
  for (const event of toolCalls) {
    const operation = registryOperations.get(event.name);
    if (!operation) {
      add('chat_tool_registry_binding_contract', 'unknown_tool_call', `tool call ${event.name} is not registered`, {
        tool_name: event.name
      });
      continue;
    }
    if (event.args_schema !== 'registry' || operation.args_schema !== 'registry') {
      add('chat_tool_registry_binding_contract', 'non_registry_args', 'tool calls must use registry args_schema', {
        tool_name: event.name
      });
    }
    const forbiddenKeys = collectObjectKeys(event.args || {}).filter((key) =>
      forbiddenToolArgKeys.has(String(key).toLowerCase())
    );
    if (forbiddenKeys.length > 0) {
      add('chat_tool_registry_binding_contract', 'provider_owned_request_violation', 'tool args contain raw provider request fields', {
        tool_name: event.name,
        forbidden_keys: [...new Set(forbiddenKeys)].sort()
      });
    }
  }

  const referencedLocators = [...new Set(findRecordLocators(observableOutput))].sort();
  const unresolved = referencedLocators.filter((locator) => !resolvableIds.has(locator));
  if (unresolved.length > 0 && envelope.unverified !== true) {
    add('chat_citation_resolvability_contract', 'unresolved_ref', 'all refs must resolve or the envelope must be marked unverified', {
      unresolved
    });
  }
  for (const citation of Array.isArray(envelope.citations) ? envelope.citations : []) {
    const ref = citation?.ref_uri || citation?.ref;
    if (typeof ref === 'string' && /^rl_[0-9a-f]{32}$/.test(ref) && !resolvableIds.has(ref)) {
      add('chat_citation_resolvability_contract', 'unresolved_citation', 'citation ref does not resolve in fixture store', {
        ref
      });
    }
  }

  const seededLeakValues = stringArray(scenario.expect?.policy?.leak_check?.must_not_appear).concat(
    stringArray(scenario.fixture_store?.cross_tenant_seeded_ids)
  );
  const uniqueLeakValues = [...new Set(seededLeakValues)].filter(Boolean);
  const observableText = JSON.stringify(observableOutput);
  const leaks = uniqueLeakValues.filter((value) => observableText.includes(value));
  for (const value of leaks) {
    add('chat_tenant_isolation_contract', 'policy_leak', 'observable chat output contains a seeded forbidden cross-tenant value', {
      leaked_value: value
    });
  }

  const expectedWritePolicy = scenario.expect?.policy?.write_operations;
  for (const event of toolCalls) {
    const operation = registryOperations.get(event.name);
    const isWrite = operation?.kind === 'write' || event.write_gated === true;
    if (!isWrite) {
      continue;
    }
    if (expectedWritePolicy === 'forbidden') {
      add('chat_write_gate_contract', 'write_tool_forbidden', 'write tool call is forbidden for this scenario', {
        tool_name: event.name
      });
    }
    if (operation?.write_gated !== true || event.write_gated !== true) {
      add('chat_write_gate_contract', 'write_not_gated', 'write-shaped tool call must be marked write_gated', {
        tool_name: event.name
      });
    }
    if (event.execution !== 'preview_only' || !hasIntentPreview(scenario.transcript || [], event.name)) {
      add('chat_write_gate_contract', 'write_not_preview_only', 'write-shaped tool call must render IntentPreview only', {
        tool_name: event.name
      });
    }
    const executedResult = (scenario.transcript || []).some(
      (candidate) => candidate && candidate.type === 'tool_result' && candidate.name === event.name
    );
    if (executedResult) {
      add('chat_write_gate_contract', 'write_executed', 'write-shaped operation produced a tool_result');
    }
  }

  if (canonicalJson(scenario.transcript || []) !== canonicalJson(scenario.replay_transcript || [])) {
    add('chat_replay_determinism_contract', 'replay_diff', 'transcript and replay_transcript canonical forms differ');
  }

  const checks = requiredChecks.map((name) => ({
    name,
    ok: checkFindings[name].length === 0,
    finding_count: checkFindings[name].length,
    findings: checkFindings[name]
  }));
  return {
    name: scenario.name || rel(filePath),
    path: rel(filePath),
    provenance: scenario.provenance || 'unknown',
    auth_token: scenario.auth_token || null,
    ok: findings.length === 0,
    finding_count: findings.length,
    checks,
    findings,
    seeded_cross_tenant_refs: uniqueLeakValues.length,
    leaks_found: leaks.length
  };
}

function main() {
  if (!fs.existsSync(fixtureRoot)) {
    throw new Error(`fixture root does not exist: ${rel(fixtureRoot)}`);
  }
  const fixtureFiles = walkFiles(fixtureRoot);
  answerEnvelopeSchema = readJsonCompatibleFixture(answerEnvelopeSchemaPath);
  const schemaArtifacts = requiredSchemaArtifacts.map((artifact) => artifactFor(artifact.filePath));
  const schemaFindings = requiredSchemaArtifacts
    .map((artifact, index) => ({ ...artifact, artifact: schemaArtifacts[index] }))
    .filter((artifact) => !artifact.artifact.present)
    .map((artifact) => ({
      contract: 'chat_transcript_schema_contract',
      code: artifact.code,
      message: artifact.message,
      path: rel(artifact.filePath)
    }));
  const scenarios = [];
  const scenarioReports = [];
  const loadFindings = [];
  const provenanceCounts = {
    synthetic: 0,
    vendor_export: 0,
    recorded_live: 0
  };

  for (const file of fixtureFiles) {
    try {
      const scenario = readJsonCompatibleFixture(file);
      scenarios.push({ file, scenario });
      if (Object.hasOwn(provenanceCounts, scenario.provenance)) {
        provenanceCounts[scenario.provenance] += 1;
      }
    } catch (error) {
      loadFindings.push({
        contract: 'chat_transcript_schema_contract',
        code: 'fixture_parse_error',
        message: error.message,
        path: rel(file)
      });
    }
  }

  for (const { file, scenario } of scenarios) {
    scenarioReports.push(checkScenario(scenario, file));
  }

  const allScenarioFindings = scenarioReports.flatMap((scenario) => scenario.findings);
  const allFindings = [...schemaFindings, ...loadFindings, ...allScenarioFindings];
  const checks = requiredChecks.map((name) => {
    const findings = allFindings.filter((finding) => finding.contract === name);
    return {
      name,
      ok: findings.length === 0,
      finding_count: findings.length
    };
  });

  const seededCrossTenantRefs = scenarioReports.reduce(
    (total, scenario) => total + scenario.seeded_cross_tenant_refs,
    0
  );
  const leaksFound = scenarioReports.reduce((total, scenario) => total + scenario.leaks_found, 0);
  const ok = allFindings.length === 0 && scenarioReports.length > 0;
  const inputArtifacts = [artifactFor(specPath), ...schemaArtifacts, ...fixtureFiles.map(artifactFor)];
  const report = {
    command: 'chat-eval',
    lane: 'CH',
    ok,
    release_ready: false,
    generated_at_utc: generatedAtUtc(),
    mode: 'local-fixture',
    inputs: {
      fixture_dir: rel(fixtureRoot),
      fixture_count: scenarioReports.length,
      provenance_counts: provenanceCounts
    },
    checks,
    scenarios: scenarioReports,
    red_team: {
      seeded_cross_tenant_refs: seededCrossTenantRefs,
      leaks_found: leaksFound,
      ok: leaksFound === 0
    },
    judge: {
      enabled: false,
      disposition: 'local-fixture'
    },
    risk_acceptance: null,
    artifacts: inputArtifacts,
    findings: allFindings
  };

  if (pipelinePlan) {
    const pipelineReport = promptfooPipelinePlan(report, fixtureFiles);
    report.pipeline = {
      enabled: true,
      mode: 'pipeline-plan',
      artifact: rel(pipelineArtifactPath),
      release_ready: false
    };
    fs.mkdirSync(path.dirname(pipelineArtifactPath), { recursive: true });
    fs.writeFileSync(pipelineArtifactPath, `${JSON.stringify(pipelineReport, null, 2)}\n`, 'utf8');
  }

  fs.mkdirSync(path.dirname(artifactPath), { recursive: true });
  fs.writeFileSync(artifactPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');

  if (jsonOutput) {
    console.log(JSON.stringify(report, null, 2));
  } else if (ok) {
    console.log(`chat-eval ok: ${rel(artifactPath)}`);
  } else {
    console.error(`chat-eval failed: ${rel(artifactPath)}`);
    for (const finding of allFindings.slice(0, 10)) {
      console.error(`  - ${finding.scenario || finding.path}: ${finding.code}: ${finding.message}`);
    }
  }
  process.exit(ok ? 0 : 1);
}

try {
  main();
} catch (error) {
  const report = {
    command: 'chat-eval',
    lane: 'CH',
    ok: false,
    release_ready: false,
    generated_at_utc: generatedAtUtc(),
    mode: 'local-fixture',
    inputs: {
      fixture_dir: rel(fixtureRoot),
      fixture_count: 0,
      provenance_counts: {
        synthetic: 0,
        vendor_export: 0,
        recorded_live: 0
      }
    },
    checks: requiredChecks.map((name) => ({
      name,
      ok: false,
      finding_count: name === 'chat_transcript_schema_contract' ? 1 : 0
    })),
    scenarios: [],
    red_team: {
      seeded_cross_tenant_refs: 0,
      leaks_found: 0,
      ok: false
    },
    judge: {
      enabled: false,
      disposition: 'local-fixture'
    },
    risk_acceptance: null,
    artifacts: [
      artifactFor(specPath),
      artifactFor(schemaPath),
      artifactFor(answerEnvelopeSchemaPath),
      artifactFor(transcriptEventSchemaPath)
    ],
    findings: [
      {
        contract: 'chat_transcript_schema_contract',
        code: 'runner_error',
        message: error.message
      }
    ]
  };
  fs.mkdirSync(path.dirname(artifactPath), { recursive: true });
  fs.writeFileSync(artifactPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  if (jsonOutput) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.error(`chat-eval failed: ${error.message}`);
  }
  process.exit(1);
}
