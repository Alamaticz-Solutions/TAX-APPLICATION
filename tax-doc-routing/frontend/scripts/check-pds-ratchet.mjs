#!/usr/bin/env node
// PDS ratchet: the goal is 100% PDS (components + `--pds-*` tokens, nothing
// product-local). This check counts the things that are NOT PDS, per file, and
// fails if any count goes UP versus scripts/pds-ratchet-baseline.json. Counts can
// only go down; when they do, run `--update` to lock the new, lower baseline.
//
//   node scripts/check-pds-ratchet.mjs                   # check (exit 1 on regression)
//   node scripts/check-pds-ratchet.mjs --update           # lower the baseline (never raises it)
//   node scripts/check-pds-ratchet.mjs --update --force   # rebaseline after widening/fixing a
//                                                          # detection pattern (counts can jump up
//                                                          # because more real code is now measured,
//                                                          # not because new non-PDS code was added)
//   node scripts/check-pds-ratchet.mjs --json             # machine-readable report
//
// Categories (per hand-owned file under src/, excluding src/generated/):
//   inlineStyle      every JSX `style={...}` attribute — a literal object
//                     (`style={{ ... }}`) *and* a reference to a named
//                     `CSSProperties` constant or call (`style={fooStyle}`,
//                     `style={barStyle(x)}`). Moving a literal into a named
//                     const and passing it by reference is still hand-authored
//                     CSS-in-JS, not a PDS component — counting only the
//                     literal form let that refactor look like progress
//                     without removing anything, so both forms count equally.
//   nativeControl    <input> <select> <textarea> <button> <table>
//   hardColor        hex / rgb() / rgba() / hsl() / named colour keywords
//   legacyToken      `--gov-*` design tokens or `@ui-kit` imports
//
// There are no exemptions: CSS mask layers use `currentColor` (only alpha matters
// in a mask), and colour mixing uses `--pds-*` tokens, never white/black keywords.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const srcRoot = path.join(root, 'src');
const baselinePath = path.join(root, 'scripts', 'pds-ratchet-baseline.json');
const args = new Set(process.argv.slice(2));

const CATEGORIES = ['inlineStyle', 'nativeControl', 'hardColor', 'legacyToken'];

const patterns = {
  inlineStyle: [/style=\{/g],
  nativeControl: [/<(?:input|select|textarea|button|table)\b/g],
  hardColor: [
    /#[0-9a-fA-F]{3,8}\b/g,
    /\brgba?\(/g,
    /\bhsla?\(/g,
    /['"](?:white|black|red|green|blue|gray|grey|orange|yellow)['"]/g,
    /,\s*(?:white|black)\s*\)/g
  ],
  // `--gov-angle` is a private animation variable (not a design token).
  legacyToken: [/--gov-(?!angle\b)[a-z0-9-]+/g, /@ui-kit/g]
};

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === 'generated' || e.name === 'node_modules') continue;
      walk(p, out);
    } else if (/\.(ts|tsx|css)$/.test(e.name)) out.push(p);
  }
  return out;
}

function countFile(text) {
  const counts = Object.fromEntries(CATEGORIES.map((c) => [c, 0]));
  const cleaned = text;
  for (const cat of CATEGORIES) {
    for (const re of patterns[cat]) {
      const m = cleaned.match(re);
      if (m) counts[cat] += m.length;
    }
  }
  return counts;
}

function measure() {
  const result = {};
  for (const file of walk(srcRoot)) {
    const rel = path.relative(root, file).split(path.sep).join('/');
    const isTsx = file.endsWith('.tsx');
    let text = fs.readFileSync(file, 'utf8');
    // Ignore comments so documentation about a pattern is not counted as use.
    text = text
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/(^|[^:'"`])\/\/.*$/gm, '$1');
    const c = countFile(text);
    // Native controls / inline styles only exist in TSX; hex etc. in TS/TSX/CSS.
    if (!isTsx) {
      c.inlineStyle = 0;
      c.nativeControl = 0;
    }
    if (CATEGORIES.some((k) => c[k] > 0)) result[rel] = c;
  }
  return result;
}

const current = measure();
const totals = (obj) =>
  Object.fromEntries(
    CATEGORIES.map((c) => [c, Object.values(obj).reduce((n, f) => n + (f[c] ?? 0), 0)])
  );

const baseline = fs.existsSync(baselinePath)
  ? JSON.parse(fs.readFileSync(baselinePath, 'utf8')).files
  : null;

const regressions = [];
if (baseline) {
  for (const [file, counts] of Object.entries(current)) {
    for (const cat of CATEGORIES) {
      const was = baseline[file]?.[cat] ?? 0;
      if (counts[cat] > was) regressions.push({ file, category: cat, was, now: counts[cat] });
    }
  }
}

if (args.has('--update')) {
  if (baseline && regressions.length && !args.has('--force')) {
    console.error('Refusing to update: the baseline can only go down. Regressions:');
    for (const r of regressions) console.error(`  ${r.file} ${r.category}: ${r.was} -> ${r.now}`);
    console.error('If these are not new non-PDS code but a corrected/widened detection pattern, re-run with --force.');
    process.exit(1);
  }
  fs.writeFileSync(
    baselinePath,
    `${JSON.stringify({ note: 'Lower-only ratchet. Regenerate with: node scripts/check-pds-ratchet.mjs --update', totals: totals(current), files: current }, null, 2)}\n`
  );
  console.log('PDS ratchet baseline written.');
  console.log(totals(current));
  process.exit(0);
}

const report = {
  command: 'pds-ratchet',
  ok: regressions.length === 0,
  totals: totals(current),
  baselineTotals: baseline ? totals(baseline) : null,
  regressions
};
fs.mkdirSync(path.join(root, 'target', 'appfw'), { recursive: true });
fs.writeFileSync(path.join(root, 'target', 'appfw', 'pds-ratchet.json'), `${JSON.stringify(report, null, 2)}\n`);

if (args.has('--json')) {
  console.log(JSON.stringify(report, null, 2));
} else {
  const t = report.totals;
  const b = report.baselineTotals;
  console.log('PDS ratchet (lower is better, goal is 0):');
  for (const c of CATEGORIES) console.log(`  ${c.padEnd(14)} ${String(t[c]).padStart(5)}${b ? `  (baseline ${b[c]})` : ''}`);
  if (!baseline) console.log('No baseline yet. Run with --update to create it.');
}
if (regressions.length) {
  console.error('PDS ratchet FAILED — these got worse (new non-PDS code):');
  for (const r of regressions) console.error(`  ${r.file} ${r.category}: ${r.was} -> ${r.now}`);
  process.exit(1);
}
