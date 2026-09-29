# Model-Driven Scaffold

This directory is the canonical model-driven UI reference for generated App
Framework applications.

Keep these components generic. They should depend on the generated UI contract,
the shared client, and shared UI primitives, but not on CRM-specific feature
folders.

## Component Map

- `EntityListView.tsx` orchestrates the list, query state, route identity, and
  selected-record workflow.
- `GridControls.tsx` owns server-side search, advanced filters, sorting,
  visible columns, and resize behavior.
- `EditPanel.tsx` owns form lifecycle, save/cancel/delete state, access
  blockers, and related collections.
- `RecordFormFields.tsx` owns generated field rendering, validation annotation,
  lookup selectors, currency formatting, and dirty-state input handling.
- `RelatedCollections.tsx` owns related entity grids and parent-scoped creation.
- `entityScaffoldModel.ts` keeps query, sort, field-selection, and operation
  helpers out of React components.
- `gridPreferences.ts` and `routeIdentity.ts` isolate browser persistence and
  URL-safe record references.
- `useLookupOptions.ts` loads lookup values without coupling controls to one
  entity type.

## Extension Rules

- Add product-specific dashboards or alternate record views through
  `EntityRecordInsight`.
- Add first-class workflow screens under `../features` only when they compose
  multiple entities, custom methods, charts, or opinionated business UX.
- Do not import feature modules into scaffold modules.
- Prefer small helper modules over making `EntityListView.tsx`,
  `GridControls.tsx`, or `EditPanel.tsx` own new domain logic.
