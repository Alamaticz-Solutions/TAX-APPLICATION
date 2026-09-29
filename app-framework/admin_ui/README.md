# Admin UI

The admin console is a React/Vite, runtime model-driven SPA served by the
backend at `/admin`.

## Source And Build Output

- Edit source in `admin_ui`.
- Do not edit `backend/admin_dist`; it is Vite build output.
- The production build writes to `backend/admin_dist` because
  `admin_ui/vite.config.ts` sets `build.outDir` to `../backend/admin_dist`.
- The production build also normalizes the generated `index.html` for the
  backend-hosted route by using same-origin asset tags without Vite dev-server
  CORS attributes.
- The backend serves `backend/admin_dist/index.html` at `/admin` and static
  assets under `/admin/assets`.
- The backend model endpoint is `/admin/model`.

## PDS Health Design Tokens

The admin UI consumes the framework-owned PDS Health frontend token contract:

- `../appfw_ui/pds_health/tokens/pdsTokens.css` is the canonical CSS token
  source for the `--pds-*` prefix.
- `../appfw_ui/pds_health/tokens/pdsTokens.ts` exports the same token families
  plus typed CSS var handles for React code, future generated UI contracts, and
  CRM reference app scaffolding.
- `src/design/pdsTokens.*` must not be recreated. Use `npm run tokens:check`
  before admin UI handoff to prove the admin UI imports the canonical source and
  has not forked local token copies.
- `src/styles.css` maps its existing local aliases to the `--pds-*` tokens so
  new frontend work can use the stable token names without forcing a full admin
  UI restyle.

Frontend starters and reference apps should consume `--pds-*` tokens first,
then add product-specific aliases only when a screen needs local semantic names.

## Production Build

```bash
cd admin_ui
npm install
npm run build
```

Then run the backend from the repository root:

```bash
APP_ADMIN_UI_ENABLED=true ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

Open:

```text
http://127.0.0.1:8080/admin
```

When testing against `ENV_NAME=compose` with explicit local test auth enabled,
set a session-scoped authorization header in the browser before loading the
admin UI:

```js
sessionStorage.setItem(
  "appfw.admin.authorization",
  "Bearer appfw-local:user=local-admin;tenant=local;roles=admin"
);
```

## Vite Dev Server

For frontend iteration, run the backend on port `8080`, then run:

```bash
cd admin_ui
npm run dev
```

Open:

```text
http://127.0.0.1:5173/admin/
```

In dev mode Vite serves the React app and proxies `/admin/model`, `/crm`, and
`/system` to the backend on `8080`.

## Verification

```bash
cd admin_ui
npm run tokens:check
npm run build
```

From the repository root:

```bash
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test
```
