#!/usr/bin/env node
// Points this clone's git hooks at the guard's pre-push hook, so every push —
// from Claude, a terminal, an IDE or SourceTree — that touches the frontend must
// pass verify-all.sh.
//
// Runs automatically, so teammates do nothing: from the plugin's SessionStart
// hook (hooks.json) and from `npm install` in tax-doc-routing/frontend
// (its `prepare` script). It only sets core.hooksPath when it is unset or still
// points at the old `.githooks` location; a developer's own hooks path is left
// alone. Silent unless it changes something (SessionStart stdout becomes context).

import { execFileSync } from 'node:child_process';

const HOOKS = '.claude/skills/pds-frontend-guard/git-hooks';
const git = (...args) => execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

try {
  git('rev-parse', '--show-toplevel');
} catch {
  process.exit(0); // not inside a git clone (for example an npm pack): nothing to do
}

let current = '';
try {
  current = git('config', '--local', '--get', 'core.hooksPath');
} catch {
  current = ''; // unset
}

if (current === HOOKS) process.exit(0);
if (current === '' || current === '.githooks') {
  git('config', '--local', 'core.hooksPath', HOOKS);
  console.log(`pds-frontend-guard: git pushes now run the PDS verification (core.hooksPath = ${HOOKS}).`);
} else {
  console.log(
    `pds-frontend-guard: core.hooksPath is "${current}", so the PDS pre-push check is NOT active in this clone. ` +
      `To enable it: git config core.hooksPath ${HOOKS}`
  );
}
