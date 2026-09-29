# Claude Code Guide

This repository uses the same operating rules for Claude Code, Codex, and human
contributors.

Start with:

```bash
scripts/appfw doctor
scripts/appfw validate --json
```

Use `AGENTS.md` for repository rules, `docs/start/cli-quickstart.md` for the
first command path, `docs/reference/cli.md` for the supported command contract,
`docs/start/agent-task-map.md` for task routing, and
`docs/start/generated-ownership.md` to choose the right edit surface. Use
`docs/lifecycle/product-golden-path.md` for the end-to-end downstream product
developer flow and `docs/lifecycle/application-lifecycle.md` for downstream app
bootstrap and upstream framework upgrade planning.

Key expectations:

- Prefer `_config`, `app_gen/src`, or `app_gen/_templates` changes over direct
  edits to generated output.
- Use `docs/start/agent-task-map.md` to route the task before editing.
- Use `docs/start/generated-ownership.md` before editing generated-looking files.
- Verify generator or template changes with `scripts/appfw validate --json`,
  `scripts/appfw generate`, `scripts/appfw generate --check --json`, and
  `scripts/appfw test`.
- Use `scripts/appfw docs-check --json` when changing docs or agent-facing CLI
  examples.
- Finish substantial work with `scripts/appfw handoff --json`.
- Do not commit secrets or local environment files.
- Keep generated and human-owned boundaries intact.

For normal verification:

```bash
scripts/appfw validate --json
scripts/appfw test
scripts/appfw handoff --json
```
