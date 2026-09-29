import fs from 'node:fs';

const displayName = 'PDS Nexus';
const allowedProductIdentity = [
  'pds-nexus',
  'PDS Nexus',
  'nexus',
  'Nexus'
].map((value) => value.toLowerCase()).filter(Boolean);
const uniqueAllowedProductIdentity = [...new Set(allowedProductIdentity)];
const required = [
  '.appfw-ui/ownership.json',
  '.appfw-ui/scaffold-manifest.json',
  'src/generated/appfw-ui-contract.ts',
  'src/main.tsx',
  'src/styles.css',
  'vite.config.ts',
  'tsconfig.json'
];

const residueScanFiles = [
  'README.md',
  'package.json',
  'index.html',
  'vite.config.ts',
  'tsconfig.json',
  '.appfw-ui/ownership.json',
  '.appfw-ui/scaffold-manifest.json',
  'src/generated/appfw-ui-contract.ts',
  'src/main.tsx',
  'src/styles.css'
];

const residueWords = new Set([
  'crm',
  'account',
  'accounts',
  'activity',
  'activities',
  'contact',
  'contacts',
  'lead',
  'leads',
  'opportunity',
  'opportunities',
  'pipeline',
  'pipelines',
  'quote',
  'quotes'
]);
const residuePhrases = ['customer relationship management'];
const ignoredTechnicalFragments = [
  'atlassian/pipelines/agent/build',
  'atlassian\\pipelines\\agent\\build'
];

function scaffoldUrl(relativePath) {
  return new URL(`../${relativePath}`, import.meta.url);
}

function normalizeResidueLine(line) {
  let normalized = line.toLowerCase();
  for (const allowed of uniqueAllowedProductIdentity) {
    normalized = normalized.split(allowed).join(' ');
  }
  for (const fragment of ignoredTechnicalFragments) {
    normalized = normalized.split(fragment).join(' ');
  }
  return normalized;
}

function residueMatches(relativePath, text) {
  const matches = [];
  const lines = text.split(/\r?\n/);
  lines.forEach((line, index) => {
    const normalized = normalizeResidueLine(line);
    const words = new Set(normalized.split(/[^a-z0-9]+/).filter(Boolean));
    for (const term of residueWords) {
      if (words.has(term)) {
        matches.push({
          path: relativePath,
          line: index + 1,
          term,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
    for (const phrase of residuePhrases) {
      if (normalized.includes(phrase)) {
        matches.push({
          path: relativePath,
          line: index + 1,
          term: phrase,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
  });
  return matches;
}

const missing = required.filter((path) => !fs.existsSync(scaffoldUrl(path)));
const pdsChecks = [
  {
    id: 'pds-component-import',
    path: 'src/main.tsx',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-command-palette',
    path: 'src/main.tsx',
    pattern: 'CommandPalette'
  },
  {
    id: 'pds-app-shell',
    path: 'src/main.tsx',
    pattern: 'AppShell'
  },
  {
    id: 'pds-overlay-example',
    path: 'src/main.tsx',
    pattern: 'Dialog'
  },
  {
    id: 'pds-analytics-example',
    path: 'src/main.tsx',
    pattern: 'KpiTile'
  },
  {
    id: 'pds-data-grid-example',
    path: 'src/main.tsx',
    pattern: 'DataGridShell'
  },
  {
    id: 'pds-conversation-example',
    path: 'src/main.tsx',
    pattern: 'MessageThread'
  },
  {
    id: 'pds-generated-view-example',
    path: 'src/main.tsx',
    pattern: 'GeneratedViewShell'
  },
  {
    id: 'pds-flow-graph-example',
    path: 'src/main.tsx',
    pattern: 'FlowGraphShell'
  },
  {
    id: 'pds-agent-timeline-example',
    path: 'src/main.tsx',
    pattern: 'AgentTimeline'
  },
  {
    id: 'pds-ambient-recommendation-example',
    path: 'src/main.tsx',
    pattern: 'RecommendationCard'
  },
  {
    id: 'pds-preview-only-guardrail',
    path: 'src/main.tsx',
    pattern: 'Intent Preview'
  },
  {
    id: 'pds-generated-form-example',
    path: 'src/main.tsx',
    pattern: 'FormLayout'
  },
  {
    id: 'pds-style-import',
    path: 'src/styles.css',
    pattern: '@appfw/pds-health-components/styles.css'
  },
  {
    id: 'pds-tsconfig-alias',
    path: 'tsconfig.json',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-vite-alias',
    path: 'vite.config.ts',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-manifest-package',
    path: '.appfw-ui/scaffold-manifest.json',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-nexus-visual-evidence-script',
    path: 'package.json',
    pattern: 'appfw:evidence'
  }
];
const missingPdsChecks = pdsChecks.filter((check) => {
  const url = scaffoldUrl(check.path);
  return !fs.existsSync(url) || !fs.readFileSync(url, 'utf8').includes(check.pattern);
});
const residue = [];
for (const relativePath of residueScanFiles) {
  const url = scaffoldUrl(relativePath);
  if (!fs.existsSync(url)) {
    continue;
  }
  residue.push(...residueMatches(relativePath, fs.readFileSync(url, 'utf8')));
}

const artifactUrl = new URL('../target/appfw/frontend-scaffold-check.json', import.meta.url);
fs.mkdirSync(new URL('../target/appfw/', import.meta.url), { recursive: true });

const report = {
  command: 'appfw:check',
  ok: missing.length === 0 && missingPdsChecks.length === 0 && residue.length === 0,
  generated_at_utc: new Date().toISOString(),
  display_name: displayName,
  required_files: required,
  missing_files: missing,
  design_system: {
    ok: missingPdsChecks.length === 0,
    checks: pdsChecks,
    missing: missingPdsChecks
  },
  residue_check: {
    ok: residue.length === 0,
    scan_files: residueScanFiles,
    terms: [...residueWords, ...residuePhrases],
    allowed_product_identity: uniqueAllowedProductIdentity,
    matches: residue
  },
  artifact: 'target/appfw/frontend-scaffold-check.json'
};

fs.writeFileSync(artifactUrl, `${JSON.stringify(report, null, 2)}\n`);

if (!report.ok) {
  if (missing.length > 0) {
    console.error(`Missing frontend scaffold files: ${missing.join(', ')}`);
  }
  if (residue.length > 0) {
    console.error(`Frontend scaffold contains sample residue; see ${artifactUrl.pathname}`);
  }
  if (missingPdsChecks.length > 0) {
    console.error(`Frontend scaffold is missing PDS design-system wiring; see ${artifactUrl.pathname}`);
  }
  process.exit(1);
}

console.log(`${displayName} frontend scaffold OK`);
console.log(`Evidence: ${artifactUrl.pathname}`);
