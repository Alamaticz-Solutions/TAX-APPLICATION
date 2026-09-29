import fs from 'node:fs';

const displayName = 'Tax Document Routing';
const allowedProductIdentity = [
  'tax-doc-routing',
  'tax_routing',
  'Tax Document Routing',
  'Tax'
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

// `src/generated/appfw-ui-contract.ts` is deliberately NOT scanned: it is the
// compiled shadow of this product's own model, so it legitimately carries this
// product's entity and field names (the Tax model has entities such as
// `TaxDocument` and `ExceptionTask`). The
// residue words below only indicate sample leakage in hand-owned,
// product-facing files. The contract is still a required file (see `required`).
const residueScanFiles = [
  'README.md',
  'package.json',
  'index.html',
  'vite.config.ts',
  'tsconfig.json',
  '.appfw-ui/ownership.json',
  '.appfw-ui/scaffold-manifest.json',
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
  normalized = removePathLikeResidueFragments(normalized);
  for (const allowed of uniqueAllowedProductIdentity) {
    normalized = normalized.split(allowed).join(' ');
  }
  for (const fragment of ignoredTechnicalFragments) {
    normalized = normalized.split(fragment).join(' ');
  }
  return normalized;
}

function removePathLikeResidueFragments(line) {
  return line
    .split(/\s+/)
    .map((token) => {
      const trimmed = token.replace(/^[\\"'([{]+|[\\\\"')},;]+$/g, '');
      if (
        trimmed.includes('../') ||
        trimmed.includes('..\\\\') ||
        trimmed.startsWith('/') ||
        trimmed.includes('/private/') ||
        trimmed.includes('/users/') ||
        trimmed.includes('appfw_ui/pds_health') ||
        trimmed.includes('@appfw/pds-health-components')
      ) {
        return ' ';
      }
      return token;
    })
    .join(' ');
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
// Checks that the product consumes the real, vendored
// @appfw/pds-health-components package directly (the framework's own reference
// apps import it directly; there is no product-local re-export or alias), so
// every screen renders with the actual PDS design system.
const pdsChecks = [
  {
    id: 'pds-component-import',
    path: 'src/main.tsx',
    pattern: "from '@appfw/pds-health-components'"
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
    id: 'pds-generated-form-example',
    path: 'src/main.tsx',
    pattern: 'FormLayout'
  },
  {
    id: 'pds-style-import',
    path: 'src/styles.css',
    pattern: "@appfw/pds-health-components/styles.css"
  },
  {
    id: 'pds-token-import',
    path: 'src/styles.css',
    pattern: "@appfw/pds-health-components/tokens.css"
  },
  {
    // The package name must resolve from node_modules, never through a
    // product-local path override.
    id: 'pds-name-not-shimmed-vite',
    path: 'vite.config.ts',
    mustNotContain: "'@appfw/pds-health-components':"
  },
  {
    id: 'pds-name-not-shimmed-tsconfig',
    path: 'tsconfig.json',
    mustNotContain: '"@appfw/pds-health-components":'
  },
  {
    // The former product-local `@ui-kit` indirection is gone.
    id: 'no-ui-kit-alias-vite',
    path: 'vite.config.ts',
    mustNotContain: '@ui-kit'
  },
  {
    id: 'no-ui-kit-alias-tsconfig',
    path: 'tsconfig.json',
    mustNotContain: '@ui-kit'
  },
  {
    id: 'no-ui-kit-alias-vitest',
    path: 'vitest.config.ts',
    mustNotContain: '@ui-kit'
  }
];
{
  // The product must actually depend on the real, vendored PDS package —
  // no more hand-rolled lookalike standing in for it.
  const pkg = fs.existsSync(scaffoldUrl('package.json'))
    ? JSON.parse(fs.readFileSync(scaffoldUrl('package.json'), 'utf8'))
    : {};
  const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };
  if (!('@appfw/pds-health-components' in allDeps)) {
    pdsChecks.push({
      id: 'client-owns-design-system-dependency',
      path: 'package.json',
      pattern: '__must-contain-@appfw/pds-health-components-dependency__'
    });
  }
}
const missingPdsChecks = pdsChecks.filter((check) => {
  const url = scaffoldUrl(check.path);
  if (!fs.existsSync(url)) return true;
  const content = fs.readFileSync(url, 'utf8');
  if (check.mustNotContain !== undefined) return content.includes(check.mustNotContain);
  return !content.includes(check.pattern);
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
    console.error(`Frontend scaffold is missing UI kit wiring; see ${artifactUrl.pathname}`);
  }
  process.exit(1);
}

console.log(`${displayName} frontend scaffold OK`);
console.log(`Evidence: ${artifactUrl.pathname}`);
