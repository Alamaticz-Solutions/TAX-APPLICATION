# Product Frontend

Use this layer for product UI scaffolding, generated UI contracts, data grids,
forms, dashboards, accessibility, and frontend release evidence.

| Topic | Guide |
| --- | --- |
| PDS Health enterprise design system | [PDS Health Design System](pds-health-design-system.md) |
| Enterprise product frontend contract | [Product Frontend](product-frontend.md) |
| Agentic UX for invoked and ambient AI | [Agentic UX](agentic-ux.md) |
| React Native mobile app contract | [React Native Mobile](mobile-react-native.md) |

The canonical PDS catalog manifest lives at
`appfw_ui/pds_health/reference/catalog.json`; the static reference page lives at
`appfw_ui/pds_health/reference/index.html` for quick product-neutral visual
review of the shared component class surface. Run
`node scripts/serve-pds-reference.mjs` and open
`http://127.0.0.1:5175/reference/` when a browser-accessible preview is needed.

For a live searchable component catalog, run `npm run dev` from
`appfw_ui/pds_health/catalog-app`. The interactive catalog renders the same
manifest and framework-owned component source; it is not a place for product
fixture language.

The backend-hosted admin UI is not a product frontend template. Product
frontends and mobile apps consume generated model/API contracts and live in the
product repo. CRM is a reference implementation and framework E2E fixture, not
the base UI for new products.
