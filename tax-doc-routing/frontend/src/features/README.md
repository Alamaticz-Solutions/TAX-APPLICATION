# Features

Human-owned product screens, one folder per domain (not per entity):

| Folder | Screens |
|---|---|
| `dashboard/` | Dashboard: KPI tiles, work queue, exception summary, recent cases |
| `routing/` | Create New Routing Record: taxpayer, documents, confirmation, processing, completed |
| `records/` | Record details, My Cases, All Cases (admin), cancel/withdraw |
| `exceptions/` | Shared exception queue |
| `admin/` | Reports and Administration (read-only reference data) |
| `shared/` | Types, config (document types, statuses, processing steps), mock data and services, business rules (`utils/`), hooks and the prototype state provider |

Rules (see the root `CLAUDE.md` and `.claude/skills/pds-frontend-guard/`):
- Import PDS components by family subpath, e.g. `@appfw/pds-health-components/forms`. The package root is only allowed in `main.tsx`.
- No local design-system layer, no inline `style`, no native `<input>/<button>/<select>/<textarea>/<table>`, no raw colours.
- Business rules live in `shared/config` and `shared/utils` (with unit tests), never inline in components.
- `shared/services` are mock services behind a stable interface; replace them with GraphQL calls via `lib/appfwClient.ts` when the API is wired.
