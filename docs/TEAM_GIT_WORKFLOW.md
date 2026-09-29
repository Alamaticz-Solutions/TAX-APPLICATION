# Tax Document Routing: How the Team Works on Git

## Branches

`main` is the only integration branch. Commit to it only through a pull request (squash-merge,
1 approval, linear history). Rebase your branch on `origin/main` every morning. Work on
`feat/<area>` branches; model changes use `model/<change>`.

## Who edits what

Folder ownership keeps merge conflicts rare:

| Area | Typical branch | Paths |
|---|---|---|
| Frontend | `feat/frontend-*` | `tax-doc-routing/frontend/src/**` (except `src/generated/`) |
| Backend: routing engine | `feat/backend-engine` | `backend/src/services/routing_engine/**` and the matching `*_impl` functions in `backend/src/handlers/tax_routing/<entity>.rs` |
| Backend: integrations | `feat/backend-integrations` | `backend/src/services/{document_store,pdf_protection,notifications}/**` |
| Model | `model/<change>` | `tax-doc-routing/.appfw/model/**` and every regenerated file |

## Changing the data model (coordinated)

One model change in flight team-wide at a time; announce it first. Then:

```bash
# from the repo root, on Windows via the rust-appfw container (docs/LOCAL_DEV_SETUP.md section 6)
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json      # must pass
cargo check --workspace --all-targets && cargo test --workspace
```

Commit the model change, **every** regenerated file and `appfw.lock` together in one commit. Do not
hand-edit generated files (see the root `CLAUDE.md` for the list).

## Before any push

```bash
bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh
```

A push that touches `tax-doc-routing/frontend/` runs the frontend gate (scaffold check, PDS ratchet,
PDS rule tests, typecheck, lint, unit tests, build). A push with no frontend change is an ordinary
push. The git pre-push hook is switched on by `npm install` in `frontend/`.

## Merge conflicts

- Generated files: take `main`'s version (`git checkout --theirs`), then regenerate; do not hand-merge.
- `Cargo.lock` / `package-lock.json`: take theirs, then re-resolve with `cargo check` / `npm install`.

## Secrets

Never commit `.env`, tokens, Box/SharePoint credentials or PDF passwords. `backend/.env.example`
holds only local-development placeholders.
