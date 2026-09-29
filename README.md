# Tax Application

Monorepo for the **Tax Document Routing** application, built on the PDS App Framework. It follows the same structure and conventions as the Project Governance monorepo.

```
Tax-Application/
├── app-framework/         # vendored PDS App Framework (do not edit casually)
├── tax-doc-routing/       # the application (backend + frontend + model)
├── docs/                  # machine setup and team git workflow
├── .claude/skills/        # pds-frontend-guard: PDS rules, push gate
├── .github/CODEOWNERS
└── bitbucket-pipelines.yml
```

- Start with `CLAUDE.md` for the architecture split (generated vs hand-owned) and commands.
- `docs/LOCAL_DEV_SETUP.md` and `docs/TEAM_GIT_WORKFLOW.md` cover setup and the PR process.
- `tax-doc-routing/README.md` describes the application itself.
