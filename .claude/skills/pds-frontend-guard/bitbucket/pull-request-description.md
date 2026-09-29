## What changed

## Areas touched
- [ ] Product code only (my assigned folders)
- [ ] Model (`.appfw/model`): regenerated files + `appfw.lock` in this PR
- [ ] Framework (`app-framework/`): regenerated files + `appfw.lock` in this PR

## Verified
- [ ] `bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh` printed **VERIFY PASSED** for the last commit
      (the pre-push hook enforces this, and the Bitbucket pipeline re-runs it on this pull request)
- [ ] Not verified live, and why (for example, no Graph or OpenAI credentials):

## Frontend: PDS rules a machine can't check
(`.claude/skills/pds-frontend-guard/rules-checklist.md` has the full list and sources)
- [ ] Checked `catalog.json` before building anything; any new exception is in `DECISIONS.md` and the `pdsRules.test.ts` allowlist
- [ ] Sections sit on the page (no card per section, no cards in cards); forms open in a `Dialog`
- [ ] `ButtonLink` for navigation, `Button` for commands
- [ ] Every data screen shows loading (after 1s), empty (one action), error (request ID + retry) and denied states
- [ ] Every write: `IntentPreview` before, `ActionAudit` receipt after, undo posture stated
- [ ] Domain nouns and verb + noun labels; no framework words in the UI
- [ ] No owner decision in `DECISIONS.md` was overridden
