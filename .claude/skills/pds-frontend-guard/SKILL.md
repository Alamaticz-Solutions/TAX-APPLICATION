---
name: pds-frontend-guard
description: Keeps every Tax Document Routing frontend change inside the PDS framework rules and proves it end to end before anything is pushed. Use for ANY change under tax-doc-routing/frontend (screens, components, styles, copy, tests), for UI user stories, before committing or pushing frontend work, and when asked "does this follow PDS?" or to check compliance.
---

# PDS frontend guard

The frontend must follow the PDS App Framework's written rules (788 of them, already distilled).
This folder is a self-contained Claude Code plugin that loads automatically for anyone who opens
the repository in Claude Code; nobody installs anything. Work inline; don't spawn workflows.

## What's in this folder

| Path | What it does |
|---|---|
| `rules-checklist.md` | **The** rules: every applicable PDS rule on one page, with its source and what enforces it. Read it at the start of every frontend task instead of the 430 KB rulebooks. |
| `agents/pds-reviewer.md` | Read-only reviewer that checks a diff against the checklist from a fresh context. |
| `hooks/hooks.json` | Session start: turns on the git pre-push check in this clone (`enable-git-hook.mjs`). Before Claude runs a push: refuses unless the commit passed the frontend verification (`push-gate.mjs`; a push with no frontend change is let through), and always refuses `--no-verify`. |
| `git-hooks/pre-push` | Git's own hook: blocks any push (terminal, IDE, SourceTree, Claude) that touches the frontend and whose commit hasn't passed `verify-all.sh`. A push with no frontend change goes through. |
| `scripts/verify-all.sh` | The frontend verification, about 3 minutes: the frontend gate with the PDS rule tests, then the Playwright suite on the mocked API. No database, no Rust build. `--full` and `--ci` (the Bitbucket mode) also run the backend checks, kept for when the backend gets its own guard. |
| `evals/` | Test cases for this guard itself (`claude plugin eval`). |
| `bitbucket/pull-request-description.md` | The PR checklist, pasted once as Bitbucket's default PR description. |

Also relevant outside this folder: `tax-doc-routing/frontend/src/test/pdsRules.test.ts` (the
machine-checked rules, which have to sit with the app's tests), the owner decisions and documented
exceptions in `tax-doc-routing/docs/architecture/pds-design-review-2026-09-24/DECISIONS.md`, and
`bitbucket-pipelines.yml` at the repository root (Bitbucket only reads it there).

## Workflow for a frontend change

1. **Orient (cheap).** Read `rules-checklist.md`. Name the floor plan the screen belongs to and the
   PDS parts the change needs. If the change touches anything in checklist section 0 (settled or
   blocked), stop and ask the user rather than overriding an owner decision.
2. **Build with PDS parts.** Catalog first: grep `app-framework/appfw_ui/pds_health/reference/catalog.json`
   for the need and read the real props in `tax-doc-routing/frontend/node_modules/@appfw/pds-health-components/dist/<family>.d.ts`.
   `style={}` only for grid/flex layout with `--pds-space-*` tokens. Sections on the page, not cards.
   Forms in a `Dialog`. `ButtonLink` for navigation. Every data screen gets loading, empty, error and
   denied states; every write gets preview, receipt and undo posture. If PDS truly lacks something,
   record the exception in DECISIONS.md first, then add it to the allowlist in `pdsRules.test.ts`
   with the same reason. Never patch a `.pds-*` class; write a framework finding instead.
3. **Self-review the diff** against the checklist's `[review]` items it touches. For a larger change,
   or when asked, also run the `pds-reviewer` agent on the diff for a second opinion.
4. **Fast loop** (from `tax-doc-routing/frontend`): `npx vitest run <file>`, `npx playwright test <spec>`,
   `npm run test:frontend` (the full gate, about 2 min). Add or extend a test for each new behaviour.
   To see it in the browser: `preview_start` with the frontend dev server (port 5173), then use the role switcher in the top bar (Tax Staff / Tax Admin). There is no sign-in yet: the screens run on mock data.
5. **Commit, then verify.** The stamp needs a clean tree, so commit first. Only the frontend has a
   guard for now: a push that touches `tax-doc-routing/frontend/` (compared with the remote branch)
   runs the frontend gate with the PDS rule tests, then the Playwright suite against the mocked API,
   about 3 minutes, no database and no Rust build. A push with no frontend change has nothing to check
   and goes through as an ordinary push. (`--full` and `--ci` still run the backend checks, kept for
   when the backend gets its own rules.) Then run
   `bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh` **in the background**
   (`run_in_background: true`; it takes several minutes) and wait for the notification. It needs
   Docker with `tax-doc-routing-postgres` running; it snapshots and restores the database, and restarts
   the user's backend if it was running. On failure read `tax-doc-routing/target/verify/<step>.log`,
   fix, commit, run again.
6. **Push** after "VERIFY PASSED": the hooks see the stamp and let it through at once. Never use
   `--no-verify`, never skip or weaken a rule test, never raise a ratchet baseline or add an allowlist
   entry just to make a check pass.
7. **Report**: what changed, the rules touched and how each is met, the verify result, and anything
   not verified (for example no Graph or OpenAI credentials).

## Changing the guard itself

When the checklist, the reviewer agent or the hooks change (or PDS is upgraded, or a new owner
decision lands): update `rules-checklist.md` and DECISIONS.md, add a rule test where a rule can be
checked mechanically and prove it catches a planted violation, then run the evals:

```bash
claude plugin eval .claude/skills/pds-frontend-guard --runs 1 --no-publish
```

Evals cost model calls (a few cents per case), so run them when the guard changes, not on every push.
Add a case to `evals/` for any mistake the reviewer is found to make.
