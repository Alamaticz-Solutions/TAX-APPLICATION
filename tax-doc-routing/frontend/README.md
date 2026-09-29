# Tax Document Routing — Frontend

React/TypeScript single-page app (Vite), built on the PDS Health design system: the vendored
`@appfw/pds-health-components` package (`vendor/`), imported directly from its public entry points,
with its `--pds-*` tokens. Light/dark mode and the Apple-like/Material-like styles come from the
package's own `AppearanceProvider`. No Tailwind, no product-local component kit.

The screens currently run on mock data and mock services (`src/features/shared/services`), so the
whole workflow can be reviewed before the GraphQL API is wired. Replacing a mock service with a call
through `src/lib/appfwClient.ts` does not change any screen.

## Layout

| Path | Ownership |
|---|---|
| `src/features/**`, `src/app/`, `src/components/`, `src/lib/` | Product |
| `src/generated/` | Generated UI contract: do not hand-edit |

See `src/features/README.md` for the feature folders.

## Styling rules (from the framework's frontend contract)

- Use a PDS component before writing your own; check
  `app-framework/appfw_ui/pds_health/reference/catalog.json` first.
- Product CSS (`src/styles.css`) is for layout and product-owned classes, using `--pds-*` tokens
  only. Do not target PDS's own `.pds-*` classes.
- No raw colours, home-made tints, gradients or blur on work screens.
- The PDS ratchet (`npm run pds:ratchet`) only lets inline-style and native-control counts go down.
- Import PDS by component family (`@appfw/pds-health-components/primitives`, `/layout`,
  `/surfaces`, ...). Only the dev scaffold `src/main.tsx` imports the package root, because
  `npm run appfw:check` requires it; `src/test/pdsFamilyImports.test.ts` enforces this.
- Text-bearing badges and alerts do not use the accent tone (`src/test/noAccentTextTone.test.ts`).

## Commands

```bash
npm install
npm run test:frontend   # appfw:check, pds:ratchet, typecheck, lint, test, build
npm run dev             # Vite on :5173, proxies /tax-routing /system /admin to the backend (VITE_BACKEND_URL, default :8080)
```

`npm run build` emits the deployable SPA bundle to `../backend/product_dist`. The backend serves it
at `/` when `APP_PRODUCT_UI_ENABLED=true`.

The `/scaffold` route is the framework's UI-kit reference screen. It exists only in development
builds.
