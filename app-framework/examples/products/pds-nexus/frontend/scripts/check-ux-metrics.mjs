#!/usr/bin/env node

import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync
} from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const frontendRoot = resolve(scriptDir, '..');
const productRoot = resolve(frontendRoot, '..');
const repoRoot = resolve(productRoot, '../../..');
const targetRoot = join(frontendRoot, 'target/appfw');
const outputPath = join(targetRoot, 'nexus-ux-metrics.json');
const visualEvidencePath = join(targetRoot, 'nexus-visual-evidence.json');
const sourceEvidencePath = join(productRoot, '.appfw/source-evidence/nexus-denovo-synthetic.json');
const pilotContractPath = join(productRoot, '.appfw/source-evidence/nexus-pilot-contract.yaml');
const frontendSourcePath = join(frontendRoot, 'src/main.tsx');
const args = parseArgs(process.argv.slice(2));
const jsonOutput = args.json === true;

mkdirSync(targetRoot, { recursive: true });

const evidence = {
  command: 'pds-nexus-ux-metrics',
  ok: false,
  generated_at_utc: new Date().toISOString(),
  scenario: 'stalled_tasks_basic',
  mode: 'local-synthetic-baseline',
  release_ready: false,
  live_ready: false,
  disposition: 'baseline-only; not live user analytics or live AI/search certification',
  product_root: rel(productRoot),
  frontend_root: rel(frontendRoot),
  inputs: {},
  baselines: {},
  checks: []
};

try {
  const visualEvidence = readJson(visualEvidencePath);
  const sourceEvidence = readJson(sourceEvidencePath);
  const pilotContract = readText(pilotContractPath);
  const frontendSource = readText(frontendSourcePath);

  evidence.inputs = {
    visual_evidence: artifactFor(visualEvidencePath),
    source_evidence: artifactFor(sourceEvidencePath),
    pilot_contract: artifactFor(pilotContractPath),
    frontend_source: artifactFor(frontendSourcePath)
  };

  const scenarioSummary = summarizeScenarios(visualEvidence);
  const sourceShape = sourceEvidence.shape ?? {};
  const sampleRecords = Array.isArray(sourceEvidence.sample_records) ? sourceEvidence.sample_records : [];
  const tenantBSentinel = sampleRecords.find((record) => record?.tenant === 'tenant_b');

  const plansPresented = countOccurrences(frontendSource, '<RecommendationCard');
  const openedPreviewScenarioCount = scenarioSummary.scenarios.filter((scenario) => scenario.opened_preview).length;
  const disabledConfirmCount = scenarioSummary.max.disabled_confirm_count;
  const executableWrites = disabledConfirmCount > 0 ? 0 : null;
  const tenantSentinelExposed = tenantBSentinel?.task_name
    ? frontendSource.includes(String(tenantBSentinel.task_name))
    : false;

  evidence.baselines = {
    ux_scope: {
      question: 'What tasks are stalled?',
      source_provenance: sourceEvidence.provenance ?? null,
      fixture_task_count: sourceShape.task_count ?? null,
      fixture_team_count: sourceShape.team_count ?? null,
      primary_tenant: sourceShape.primary_tenant ?? null,
      stalled_threshold_days: sourceShape.stalled_threshold_days ?? null,
      rendered_record_refs: scenarioSummary.max.entity_refs,
      rendered_citations: scenarioSummary.max.citations
    },
    plan_acceptance: {
      plans_presented: plansPresented,
      plans_opened_in_evidence: openedPreviewScenarioCount,
      plans_accepted: 0,
      acceptance_rate: 0,
      executable_write_count: executableWrites,
      reason: 'The pilot is preview-only; no live user cohort or ServiceNow write path exists.'
    },
    calibration: {
      confidence_signals_rendered: scenarioSummary.min.confidence_signals,
      current_label: 'high',
      calibrated_against_live_outcomes: false,
      status: 'static confidence label over synthetic resolved refs; live calibration remains future work'
    },
    grounding: {
      record_locators_resolved: scenarioSummary.max.entity_refs,
      evidence_summaries_rendered: scenarioSummary.min.evidence_summaries,
      tenant_b_sentinel_exposed: tenantSentinelExposed,
      tenant_b_sentinel_record_locator: tenantBSentinel?.id ?? null,
      source_system: 'Synthetic De Novo projection'
    },
    guardrails: {
      preview_only_visible: scenarioSummary.all.preview_only_visible,
      service_now_blocked_visible: scenarioSummary.all.service_now_blocked_visible,
      intent_preview_dialog_confirm_disabled: disabledConfirmCount > 0,
      live_chat_enabled: false,
      live_service_now_write_enabled: false
    },
    visual_accessibility: {
      scenario_count: scenarioSummary.scenarios.length,
      serious_or_critical_axe_violations: scenarioSummary.total.serious_axe_violations,
      max_horizontal_overflow_px: scenarioSummary.max.horizontal_overflow,
      unnamed_interactive_controls: scenarioSummary.total.unnamed_interactive
    }
  };

  addCheck('visual-evidence-ok', visualEvidence.ok === true, 'Nexus visual/a11y evidence must pass before UX metrics can be retained.', {
    path: rel(visualEvidencePath)
  });
  addCheck('visual-scenario-coverage', scenarioSummary.scenarios.length >= 3, 'UX baseline needs desktop light, desktop dark preview, and mobile dark evidence.', {
    scenario_count: scenarioSummary.scenarios.length
  });
  addCheck('source-evidence-synthetic', sourceEvidence.provenance === 'synthetic', 'The baseline must honestly remain synthetic until live source evidence exists.', {
    provenance: sourceEvidence.provenance ?? null
  });
  addCheck('pilot-contract-preview-only', pilotContract.includes('write_mode: intent_preview_only'), 'Pilot contract must retain preview-only write posture.');
  addCheck('plans-presented', plansPresented >= 1, 'At least one recommendation plan must render.');
  addCheck('preview-opened', openedPreviewScenarioCount >= 1, 'Evidence must open the Intent Preview flow at least once.', {
    opened_preview_scenarios: openedPreviewScenarioCount
  });
  addCheck('no-plans-accepted', evidence.baselines.plan_acceptance.plans_accepted === 0, 'Synthetic preview-only baseline must not claim accepted/executed plans.');
  addCheck('disabled-confirm-visible', disabledConfirmCount >= 1, 'Intent Preview must expose disabled confirmation in evidence.', {
    disabled_confirm_count: disabledConfirmCount
  });
  addCheck('record-locators-resolved', scenarioSummary.max.entity_refs >= 3, 'The stalled-task answer must render resolved record-locator refs.', {
    rendered_record_refs: scenarioSummary.max.entity_refs
  });
  addCheck('citations-rendered', scenarioSummary.max.citations >= 1, 'The stalled-task answer must render citations.');
  addCheck('tenant-sentinel-not-rendered', tenantSentinelExposed === false, 'The tenant_b sentinel must not render in the tenant_a baseline.', {
    tenant_b_sentinel_record_locator: tenantBSentinel?.id ?? null
  });
  addCheck('no-serious-axe-violations', scenarioSummary.total.serious_axe_violations === 0, 'No serious/critical axe violations are allowed.', {
    serious_or_critical_axe_violations: scenarioSummary.total.serious_axe_violations
  });
  addCheck('no-horizontal-overflow', scenarioSummary.max.horizontal_overflow === 0, 'No horizontal overflow is allowed in evidence scenarios.', {
    max_horizontal_overflow_px: scenarioSummary.max.horizontal_overflow
  });
  addCheck('not-release-ready', evidence.release_ready === false && evidence.live_ready === false, 'Local synthetic UX metrics must not claim release/live readiness.');

  evidence.ok = evidence.checks.every((check) => check.ok);
} catch (error) {
  evidence.error = error instanceof Error ? error.message : String(error);
  addCheck('metrics-script-completed', false, evidence.error);
}

writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, 'utf8');

if (jsonOutput) {
  console.log(JSON.stringify(evidence, null, 2));
} else if (evidence.ok) {
  console.log(`PDS Nexus UX metric baseline OK: ${outputPath}`);
} else {
  console.error(`PDS Nexus UX metric baseline failed: ${outputPath}`);
  if (evidence.error) console.error(evidence.error);
}

process.exit(evidence.ok ? 0 : 1);

function addCheck(id, ok, message, detail = {}) {
  evidence.checks.push({ id, ok, message, ...detail });
}

function parseArgs(argv) {
  return Object.fromEntries(
    argv.map((arg) => {
      if (arg === '--json') return ['json', true];
      return [arg.replace(/^--/, ''), true];
    })
  );
}

function rel(filePath) {
  return relative(repoRoot, filePath).split(sep).join('/');
}

function readJson(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`${rel(filePath)} is missing; run npm run appfw:evidence first.`);
  }
  return JSON.parse(readFileSync(filePath, 'utf8'));
}

function readText(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`${rel(filePath)} is missing.`);
  }
  return readFileSync(filePath, 'utf8');
}

function artifactFor(filePath) {
  const bytes = readFileSync(filePath);
  return {
    path: rel(filePath),
    sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
    bytes: bytes.length
  };
}

function summarizeScenarios(visualEvidence) {
  const scenarios = Array.isArray(visualEvidence.scenarios) ? visualEvidence.scenarios : [];
  const values = scenarios.map((scenario) => {
    const dom = scenario.dom ?? {};
    const conversation = dom.conversation ?? {};
    const ambient = dom.ambient ?? {};
    const writeGuardrails = dom.writeGuardrails ?? {};
    const axe = scenario.axe ?? {};
    return {
      id: scenario.id,
      ok: scenario.ok === true,
      opened_preview: scenario.opened_preview === true,
      entity_refs: number(conversation.entityRefs),
      citations: number(conversation.citations),
      confidence_signals: number(conversation.confidence),
      evidence_summaries: number(ambient.evidence),
      preview_only_visible: writeGuardrails.previewOnlyText === true,
      service_now_blocked_visible: writeGuardrails.serviceNowBlockedText === true,
      disabled_confirm_count: number(writeGuardrails.disabledConfirmCount),
      serious_axe_violations: number(axe.serious_or_critical_count),
      horizontal_overflow: number(dom.horizontalOverflow),
      unnamed_interactive: Array.isArray(dom.unnamedInteractive) ? dom.unnamedInteractive.length : 0
    };
  });

  return {
    scenarios: values,
    min: {
      confidence_signals: min(values, 'confidence_signals'),
      evidence_summaries: min(values, 'evidence_summaries')
    },
    max: {
      entity_refs: max(values, 'entity_refs'),
      citations: max(values, 'citations'),
      disabled_confirm_count: max(values, 'disabled_confirm_count'),
      horizontal_overflow: max(values, 'horizontal_overflow')
    },
    total: {
      serious_axe_violations: sum(values, 'serious_axe_violations'),
      unnamed_interactive: sum(values, 'unnamed_interactive')
    },
    all: {
      preview_only_visible: values.length > 0 && values.every((scenario) => scenario.preview_only_visible),
      service_now_blocked_visible: values.length > 0 && values.every((scenario) => scenario.service_now_blocked_visible)
    }
  };
}

function countOccurrences(text, needle) {
  return text.split(needle).length - 1;
}

function number(value) {
  return Number.isFinite(Number(value)) ? Number(value) : 0;
}

function min(values, key) {
  return values.length ? Math.min(...values.map((value) => number(value[key]))) : 0;
}

function max(values, key) {
  return values.length ? Math.max(...values.map((value) => number(value[key]))) : 0;
}

function sum(values, key) {
  return values.reduce((total, value) => total + number(value[key]), 0);
}
