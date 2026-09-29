#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import {
  createReadStream,
  existsSync,
  mkdirSync,
  statSync,
  writeFileSync
} from 'node:fs';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const frontendRoot = resolve(scriptDir, '..');
const productRoot = resolve(frontendRoot, '..');
const repoRoot = resolve(productRoot, '../../..');
const distRoot = join(productRoot, 'backend/product_dist');
const targetRoot = join(frontendRoot, 'target/appfw');
const outputPath = join(targetRoot, 'nexus-visual-evidence.json');
const screenshotsDir = join(targetRoot, 'nexus-visual-evidence/screenshots');
const args = parseArgs(process.argv.slice(2));
const jsonOutput = args.json === true;
const scenarios = [
  {
    id: 'desktop-light-home',
    label: 'Desktop light Nexus pilot',
    colorScheme: 'light',
    viewport: { width: 1280, height: 900 },
    openPreview: false
  },
  {
    id: 'desktop-dark-preview',
    label: 'Desktop dark Intent Preview guardrail',
    colorScheme: 'dark',
    viewport: { width: 1280, height: 900 },
    openPreview: true
  },
  {
    id: 'mobile-dark-home',
    label: 'Mobile dark Nexus pilot',
    colorScheme: 'dark',
    viewport: { width: 390, height: 844 },
    openPreview: false
  }
];

mkdirSync(targetRoot, { recursive: true });
mkdirSync(screenshotsDir, { recursive: true });

const evidence = {
  command: 'pds-nexus-visual-evidence',
  ok: false,
  generated_at_utc: new Date().toISOString(),
  product_root: relative(productRoot),
  frontend_root: relative(frontendRoot),
  artifacts: {
    output: relative(outputPath),
    screenshots_dir: relative(screenshotsDir)
  },
  build: null,
  server: null,
  scenarios: [],
  checks: []
};

let server;
let browser;

try {
  if (!args.skipBuild) {
    evidence.build = await runCommand('npm', ['run', 'build'], frontendRoot);
  } else {
    evidence.build = {
      command: 'npm run build',
      skipped: true,
      ok: existsSync(join(distRoot, 'index.html'))
    };
  }

  if (!evidence.build.ok) {
    throw new Error('Nexus frontend build failed.');
  }

  server = await startStaticServer(distRoot, args.port ? Number(args.port) : 0);
  evidence.server = {
    url: server.url,
    dist_root: relative(distRoot)
  };

  const playwrightModule = await importFromKnownRoots('@playwright/test');
  const chromium = playwrightModule.chromium ?? playwrightModule.default?.chromium;
  if (!chromium) throw new Error('Unable to load chromium from @playwright/test.');

  const axeModule = await importFromKnownRoots('@axe-core/playwright');
  const AxeBuilder = axeModule.default ?? axeModule.AxeBuilder;
  if (!AxeBuilder) throw new Error('Unable to load AxeBuilder from @axe-core/playwright.');

  browser = await chromium.launch();
  for (const scenario of scenarios) {
    evidence.scenarios.push(await runScenario({ AxeBuilder, browser, scenario, url: server.url }));
  }

  evidence.checks = evidence.scenarios.flatMap((scenario) =>
    scenario.checks.map((check) => ({
      id: `${scenario.id}:${check.id}`,
      ok: check.ok,
      detail: Object.fromEntries(Object.entries(check).filter(([key]) => key !== 'id' && key !== 'ok'))
    }))
  );
  evidence.ok =
    Boolean(evidence.build?.ok)
    && evidence.scenarios.every((scenario) => scenario.ok)
    && evidence.checks.every((check) => check.ok);
} catch (error) {
  evidence.error = error instanceof Error ? error.message : String(error);
} finally {
  if (browser) await browser.close();
  if (server) await new Promise((resolveClose) => server.instance.close(resolveClose));
}

writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, 'utf8');

if (jsonOutput) {
  console.log(JSON.stringify(evidence, null, 2));
} else if (evidence.ok) {
  console.log(`PDS Nexus visual evidence OK: ${outputPath}`);
} else {
  console.error(`PDS Nexus visual evidence failed: ${outputPath}`);
  if (evidence.error) console.error(evidence.error);
}

process.exit(evidence.ok ? 0 : 1);

async function runScenario({ AxeBuilder, browser, scenario, url }) {
  const context = await browser.newContext({
    viewport: scenario.viewport,
    colorScheme: scenario.colorScheme,
    reducedMotion: 'reduce'
  });
  const page = await context.newPage();
  const screenshotPath = join(screenshotsDir, `${scenario.id}.png`);

  try {
    await page.goto(url, { waitUntil: 'networkidle' });
    if (scenario.openPreview) {
      await page.getByRole('button', { name: 'Open Intent Preview' }).first().click();
      await page.getByRole('dialog', { name: 'Preview recommendation' }).waitFor();
    }

    const axe = await new AxeBuilder({ page }).analyze();
    await page.screenshot({ path: screenshotPath, fullPage: false });
    const dom = await page.evaluate(() => {
      const horizontalOverflow = Math.max(
        0,
        document.documentElement.scrollWidth - document.documentElement.clientWidth
      );
      const duplicateIds = duplicateElementIds();
      const unnamedInteractive = findUnnamedInteractive();

      return {
        title: document.title,
        horizontalOverflow,
        duplicateIds,
        unnamedInteractive,
        shell: {
          appShellCount: document.querySelectorAll('.pds-app-shell').length,
          pageHeaderCount: document.querySelectorAll('.pds-page-header').length,
          commandPaletteCount: document.querySelectorAll('.pds-command-palette').length
        },
        conversation: {
          threadCount: document.querySelectorAll('.pds-message-thread[aria-label]').length,
          composerCount: document.querySelectorAll('.pds-message-composer').length,
          streamingStatusCount: document.querySelectorAll(".pds-streaming-text[role='status'][aria-live='polite']").length,
          toolStatusCount: document.querySelectorAll(".pds-tool-call-status[role='status']").length,
          entityRefs: document.querySelectorAll('.pds-entity-ref-card[aria-label]').length,
          citations: document.querySelectorAll('.pds-citation-list[aria-label]').length,
          confidence: document.querySelectorAll(".pds-confidence-signal[role='status']").length,
          timeline: document.querySelectorAll('.pds-agent-timeline[aria-label]').length,
          flowGraph: document.querySelectorAll(".pds-flow-graph-shell__viewport[role='img'][aria-label]").length
        },
        ambient: {
          generatedView: document.querySelectorAll('.pds-generated-view-shell[data-grounding]').length,
          recommendation: document.querySelectorAll('.pds-recommendation-card[data-rank]').length,
          evidence: document.querySelectorAll('.pds-evidence-summary[aria-label]').length,
          freshness: document.querySelectorAll(".pds-freshness-indicator[role='status']").length,
          attribution: document.querySelectorAll(".pds-ai-attribution[role='status'][aria-live='polite']").length,
          attention: document.querySelectorAll(".pds-attention-marker[role='status'], .pds-attention-marker[role='alert']").length
        },
        writeGuardrails: {
          previewOnlyText: document.body.textContent?.includes('Preview only') ?? false,
          intentPreviewCount: document.querySelectorAll('.pds-intent-preview').length,
          disabledConfirmCount: Array.from(document.querySelectorAll('button')).filter((button) =>
            button.disabled && button.textContent?.includes('Confirm disabled')
          ).length,
          serviceNowBlockedText: document.body.textContent?.includes('cannot write to ServiceNow') ?? false
        },
        dataAndAnalytics: {
          gridCount: document.querySelectorAll('.pds-data-grid').length,
          kpiCount: document.querySelectorAll('.pds-kpi-tile').length,
          chartCount: document.querySelectorAll('.pds-chart-shell').length
        }
      };

      function duplicateElementIds() {
        const counts = new Map();
        for (const element of Array.from(document.querySelectorAll('[id]'))) {
          counts.set(element.id, (counts.get(element.id) ?? 0) + 1);
        }
        return Array.from(counts.entries())
          .filter(([, count]) => count > 1)
          .map(([id, count]) => ({ id, count }));
      }

      function findUnnamedInteractive() {
        return Array.from(document.querySelectorAll("button, a, input, select, textarea, [role='button'], [role='tab']"))
          .filter((element) => isVisible(element) && !accessibleName(element))
          .slice(0, 20)
          .map((element) => ({
            tag: element.tagName.toLowerCase(),
            type: element.getAttribute('type'),
            role: element.getAttribute('role'),
            class: element.getAttribute('class'),
            text: element.textContent?.trim().slice(0, 80) ?? ''
          }));
      }

      function accessibleName(element) {
        const ariaLabel = element.getAttribute('aria-label');
        if (ariaLabel?.trim()) return ariaLabel.trim();
        const labelledBy = element.getAttribute('aria-labelledby');
        if (labelledBy) {
          const value = labelledBy
            .split(/\s+/)
            .map((id) => document.getElementById(id)?.textContent?.trim() ?? '')
            .filter(Boolean)
            .join(' ')
            .trim();
          if (value) return value;
        }
        const labels = element.labels ? Array.from(element.labels) : [];
        const labelText = labels.map((label) => label.textContent?.trim() ?? '').filter(Boolean).join(' ').trim();
        if (labelText) return labelText;
        const text = element.textContent?.trim();
        if (text) return text;
        const title = element.getAttribute('title');
        if (title?.trim()) return title.trim();
        return '';
      }

      function isVisible(element) {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none';
      }
    });

    const seriousViolations = axe.violations.filter((violation) =>
      violation.impact === 'critical' || violation.impact === 'serious'
    );
    const checks = [
      {
        id: 'axe-no-serious-violations',
        ok: seriousViolations.length === 0,
        count: seriousViolations.length
      },
      {
        id: 'no-horizontal-overflow',
        ok: dom.horizontalOverflow <= 1,
        pixels: dom.horizontalOverflow
      },
      {
        id: 'no-duplicate-ids',
        ok: dom.duplicateIds.length === 0,
        count: dom.duplicateIds.length
      },
      {
        id: 'named-interactive-controls',
        ok: dom.unnamedInteractive.length === 0,
        count: dom.unnamedInteractive.length
      },
      {
        id: 'nexus-shell-rendered',
        ok: dom.shell.appShellCount >= 1 && dom.shell.pageHeaderCount >= 1 && dom.shell.commandPaletteCount >= 1,
        actual: dom.shell
      },
      {
        id: 'chat-answer-surfaces-rendered',
        ok:
          dom.conversation.threadCount >= 1
          && dom.conversation.composerCount >= 1
          && dom.conversation.streamingStatusCount >= 1
          && dom.conversation.toolStatusCount >= 1
          && dom.conversation.entityRefs >= 3
          && dom.conversation.citations >= 1
          && dom.conversation.confidence >= 1
          && dom.conversation.timeline >= 1
          && dom.conversation.flowGraph >= 1,
        actual: dom.conversation
      },
      {
        id: 'ambient-and-evidence-surfaces-rendered',
        ok:
          dom.ambient.generatedView >= 1
          && dom.ambient.recommendation >= 1
          && dom.ambient.evidence >= 1
          && dom.ambient.freshness >= 1
          && dom.ambient.attribution >= 1
          && dom.ambient.attention >= 1,
        actual: dom.ambient
      },
      {
        id: 'data-and-analytics-rendered',
        ok: dom.dataAndAnalytics.gridCount >= 1 && dom.dataAndAnalytics.kpiCount >= 3 && dom.dataAndAnalytics.chartCount >= 1,
        actual: dom.dataAndAnalytics
      },
      {
        id: 'preview-only-write-guardrail-visible',
        ok:
          dom.writeGuardrails.previewOnlyText
          && dom.writeGuardrails.serviceNowBlockedText
          && (!scenario.openPreview || (dom.writeGuardrails.intentPreviewCount >= 1 && dom.writeGuardrails.disabledConfirmCount >= 1)),
        actual: dom.writeGuardrails
      }
    ];

    return {
      id: scenario.id,
      label: scenario.label,
      ok: checks.every((check) => check.ok),
      color_scheme: scenario.colorScheme,
      viewport: scenario.viewport,
      screenshot: relative(screenshotPath),
      opened_preview: scenario.openPreview,
      checks,
      axe: {
        violation_count: axe.violations.length,
        serious_or_critical_count: seriousViolations.length,
        violations: seriousViolations.map((violation) => ({
          id: violation.id,
          impact: violation.impact,
          description: violation.description,
          nodes: violation.nodes.map((node) => node.target)
        }))
      },
      dom
    };
  } finally {
    await context.close();
  }
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith('--')) continue;
    const key = arg.slice(2);
    const next = argv[index + 1];
    if (!next || next.startsWith('--')) {
      parsed[key] = true;
    } else {
      parsed[key] = next;
      index += 1;
    }
  }
  return parsed;
}

function runCommand(command, commandArgs, cwd) {
  return new Promise((resolveCommand) => {
    const child = spawn(command, commandArgs, {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk.toString(); });
    child.stderr.on('data', (chunk) => { stderr += chunk.toString(); });
    child.on('close', (status) => {
      resolveCommand({
        command: `${command} ${commandArgs.join(' ')}`,
        cwd: relative(cwd),
        ok: status === 0,
        status,
        stdout: stdout.trim(),
        stderr: stderr.trim()
      });
    });
  });
}

function startStaticServer(root, requestedPort) {
  return new Promise((resolveServer, rejectServer) => {
    const instance = createServer((request, response) => {
      const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
      const rawPath = decodeURIComponent(requestUrl.pathname);
      const candidatePath = resolve(root, rawPath === '/' ? 'index.html' : `.${rawPath}`);
      const allowedRoot = resolve(root);
      const filePath = candidatePath === allowedRoot || !candidatePath.startsWith(`${allowedRoot}${sep}`)
        ? join(root, 'index.html')
        : candidatePath;
      const servedPath = existsSync(filePath) && statSync(filePath).isFile()
        ? filePath
        : join(root, 'index.html');

      response.setHeader('Cache-Control', 'no-store');
      response.setHeader('Content-Type', contentType(servedPath));
      createReadStream(servedPath).pipe(response);
    });
    instance.once('error', rejectServer);
    instance.listen(requestedPort, '127.0.0.1', () => {
      const address = instance.address();
      if (!address || typeof address === 'string') {
        rejectServer(new Error('Unable to determine static server address.'));
        return;
      }
      resolveServer({ instance, url: `http://127.0.0.1:${address.port}/` });
    });
  });
}

function contentType(filePath) {
  switch (extname(filePath)) {
    case '.html':
      return 'text/html; charset=utf-8';
    case '.js':
      return 'text/javascript; charset=utf-8';
    case '.css':
      return 'text/css; charset=utf-8';
    case '.svg':
      return 'image/svg+xml';
    case '.png':
      return 'image/png';
    case '.jpg':
    case '.jpeg':
      return 'image/jpeg';
    default:
      return 'application/octet-stream';
  }
}

async function importFromKnownRoots(packageName) {
  const roots = [
    join(frontendRoot, 'node_modules'),
    join(repoRoot, 'appfw_ui/pds_health/catalog-app/node_modules'),
    join(repoRoot, 'node_modules')
  ];
  const errors = [];
  for (const root of roots) {
    try {
      const requireFromRoot = createRequire(pathToFileURL(join(root, 'noop.js')));
      const resolvedPath = requireFromRoot.resolve(packageName);
      return await import(pathToFileURL(resolvedPath).href);
    } catch (error) {
      errors.push(`${root}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  throw new Error(`Unable to resolve ${packageName}. Tried:\n${errors.join('\n')}`);
}

function relative(path) {
  return path.replace(`${repoRoot}${sep}`, '');
}
