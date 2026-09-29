#!/usr/bin/env node
// PreToolUse gate for Claude's own shell commands (Bash and PowerShell tools).
//
// A `git push` that touches the frontend is allowed only when the checked-out
// commit already passed verify-all.sh (its stamp in .git/pds-verify-passed) and
// the tree is clean. Otherwise Claude is told to run the verification in the
// background first: it takes a few minutes, longer than a foreground command may.
// A push with no frontend change has nothing to verify and is let through.
// `--no-verify` is always refused. The git pre-push hook still runs on the push
// itself; this gate just stops Claude from starting a push that would block.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const VERIFY = 'bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh';

function deny(reason) {
  process.stdout.write(
    JSON.stringify({ hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'deny', permissionDecisionReason: reason } })
  );
  process.exit(0);
}

let input = {};
try {
  input = JSON.parse(readFileSync(0, 'utf8') || '{}');
} catch {
  process.exit(0); // unreadable input: don't interfere
}
const command = String(input?.tool_input?.command ?? '');

// `git push`, `git -C "some dir" push`, `git --no-pager push`, also inside a && / ; chain.
const ARG = String.raw`(?:[^\s"']|"[^"]*"|'[^']*')+`; // one shell word, quoted parts included
const PUSH = new RegExp(String.raw`\bgit(?:\s+(?:-C\s+${ARG}|-c\s+${ARG}|--[\w-]+(?:=${ARG})?))*\s+push\b`);
if (!PUSH.test(command)) process.exit(0);

// (`git push -n` is --dry-run, not --no-verify; push has no short form for it.)
if (/--no-verify\b/.test(command.slice(command.search(/\bpush\b/)))) {
  deny('Pushing with --no-verify is not allowed in this repository: every push must pass the PDS verification. ' +
    `Run \`${VERIFY}\` (in the background) and push without --no-verify.`);
}

const cwd = input.cwd || process.env.CLAUDE_PROJECT_DIR || process.cwd();
const git = (...args) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

let head, stamp = '', dirty;
try {
  head = git('rev-parse', 'HEAD');
  dirty = git('status', '--porcelain') !== '';
  try {
    stamp = readFileSync(join(git('rev-parse', '--absolute-git-dir'), 'pds-verify-passed'), 'utf8').trim();
  } catch {
    stamp = '';
  }
} catch {
  process.exit(0); // not a git clone we can read: leave it to git's own pre-push hook
}

// Only the frontend has a guard for now: what would this push add, compared with the remote branch?
let touchesFrontend = true; // when the base can't be found, check rather than skip
try {
  let base = '';
  try {
    base = git('rev-parse', '--verify', '-q', '@{upstream}');
  } catch {
    base = git('merge-base', 'origin/main', 'HEAD');
  }
  touchesFrontend = git('diff', '--name-only', base, 'HEAD')
    .split('\n')
    .some((f) => f.startsWith('tax-doc-routing/frontend/'));
} catch {
  /* keep the safe default */
}
if (!touchesFrontend) process.exit(0);

if (dirty) {
  deny('The working tree has uncommitted or untracked changes. Commit (or stash) them first; the verification stamps only a clean commit.');
}
if (stamp !== head) {
  deny(
    `Commit ${head.slice(0, 8)} has not passed the PDS verification yet. Run \`${VERIFY}\` with run_in_background ` +
      '(about 3 minutes for a frontend change), wait for "VERIFY PASSED", then push again; the pre-push hook will then pass instantly.'
  );
}
process.exit(0);
