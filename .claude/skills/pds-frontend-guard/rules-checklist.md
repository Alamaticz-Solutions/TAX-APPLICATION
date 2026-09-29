# PDS frontend rules: the compact checklist

Distilled once (2026-09-24) from the 788 rules in
`tax-doc-routing/docs/architecture/pds-design-review-2026-09-24/rules/RB-A..D.md`, which
quote every rule verbatim with its source. **Read this file, not the rulebooks.** Open a
rulebook entry only when you must quote a rule's exact wording.

Paths: `design-system` = `app-framework/docs/frontend/pds-health-design-system.md`,
`ux-strategy` = `app-framework/docs/frontend/ux-design-strategy.md`, `agentic-ux` and
`product-frontend` likewise under `app-framework/docs/frontend/`, `catalog` =
`app-framework/appfw_ui/pds_health/reference/catalog.json`, `README` =
`app-framework/appfw_ui/pds_health/components/README.md`, `ca/` =
`app-framework/appfw_ui/pds_health/catalog-app/src/`.

**Enforced by:** `[rule: …]` = a test in `frontend/src/test/pdsRules.test.ts`; `[guard: file]`
= another guard test; `[ratchet]`, `[phi]`, `[entities]`, `[scaffold]` = the gate scripts;
`[e2e: spec]` = a Playwright spec; `[review]` = nothing checks it, so you must.

## 0. Settled: don't undo, don't "fix"

Owner decisions are in `docs/architecture/pds-design-review-2026-09-24/DECISIONS.md`. The ones a
change is most likely to trip over:

- Universal navigation: the same destinations for every role. Roles change what a screen shows, never the nav.
- Forms open in a PDS `Dialog` (like *New project*); `Drawer` is only for the notification panel and the project-list filters. `[rule: opens forms in a Dialog]`
- Project rows open the full project page at every width (no quick-look drawer).
- Retired pages stay retired: gate workspace, Team Inbox, Audit log, Entity browser.
- No sample or invented data anywhere; empty means `EmptyState`. `[rule: shows no invented data]`
- Analytics: neutral tones, no "SLA", no invented targets, "No data" instead of 0% (decision 3).
- A rejected step can be reworked like a returned one (decision 11); decisions notify the submitter (12).
- Gate order and stage codes are provisional (open decision P5). Don't hard-code their meaning.
- **Owner deviations from PDS, kept on purpose:** the sign-in marketing panel (decision 5, against `product-frontend:97`). Don't change it without the owner.
- **Blocked, don't build:** role home lists *Your open steps* / *Awaiting my decision* / *Not started* (J-2, J-3, J-4; decision 4), and Analytics "committee" grouping (D-4; decision 13).
- **Waiting on PDS, don't build locally** (ADR 0016: a PDS limitation doesn't justify local mechanics, RB-A-198):
  phone-width navigation drawer (Finding W; RB-C-152), `Tabs` overflow (AC; the tab row scrolls in `.gov-project-tabs`),
  `ProcessStepper` has no `skipped` (X; use `warning` + a "Skipped" caption), accent tone unreadable in dark (Y; use
  `neutral` on text-bearing badges and alerts), `WorkQueueItem` status overlap below 720px (AD; status goes in the
  meta line at compact width), `Popover` is always modal (R). New PDS defects go into
  `docs/architecture/framework-issues-for-pds.md`, never into a `.pds-*` override.
- **Known gaps still to build (product-owned):** an offline state and a designed "session expired" state
  (RB-A-96, RB-B-164). Include them when you touch the shell or the API client.

## 1. Components and imports: catalog first

- Before adding any product-local component, control or style, check `catalog.json` (10 families) and use the PDS part if one covers the need. RB-B-116 `product-frontend:247`, RB-C-13. `[review]`
  The family list is in `docs/architecture/pds-catalog-notes.md`. PDS has no generic layout (Stack/Grid/Box) and no text/heading primitive, so `style={}` for grid/flex layout and token-based typography are the accepted fallbacks.
- Any new exception needs an entry in DECISIONS.md "Documented custom exceptions" **first**, then the allowlist in `pdsRules.test.ts`. RB-A-7 `design-system:102`, RB-B-117.
- Import PDS by family subpath (`@appfw/pds-health-components/forms`, `/layout`, …); only `main.tsx` uses the root. RB-C-5 `README:181`. `[guard: pdsFamilyImports]`
- No React Aria, AG Grid, MUI, Fluent, chart libraries, CSS-in-JS or checkout-relative `components/src`. RB-A-21/22/23/44, RB-C-2/3. `[rule: imports no other UI kit]`
- Don't restyle, extend or override PDS components; never target a `.pds-*` class. RB-A-20 `design-system:172`, RB-C-16. `[rule: never targets a PDS class]`
- Product CSS lives only in `src/styles.css`. `[rule: keeps product CSS in the one stylesheet]`
- Thin wrappers over PDS in `components/ui.tsx` are allowed; a parallel component kit is not. RB-A-43 `design-system:308`. `[review]`
- Clickable cards and queue rows: `WorkQueueItem`, `InteractiveCard` (command) or `CardLink` (navigation). No secondary button nested in a whole-card link. RB-A-50, RB-C-49, RB-D-209. `[review]`

## 2. Composition

- Hierarchy comes from alignment, spacing, typography and stable state, not from cards, borders, shadows or effects. RB-A-5 `design-system:81`, RB-B-18 `ux-strategy:91`. `[review]`
- Containment (a card or `Surface`) only for repeated actionable items, transient overlays and genuine tools. A section is a `PageSection` on the page, not a card. RB-A-6 `design-system:83`, RB-B-19. `[review]`
- Never nest cards in cards (for example, a card around `KpiTile`s). RB-B-9 `ux-strategy:68`, RB-A-99, RB-C-130. `[review]`
- Chrome (borders, fills, radii, shadows) comes from PDS parts, never hand-drawn in `style={}`; hairline dividers on work surfaces are allowed. RB-B-116, RB-B-8, RB-D-182. `[rule: draws chrome only in documented exceptions]` `[ratchet]`
- `Surface` variants mean different things: elevated = related work, filled = grouped content, outlined = light containment. `Surface` clips; `overflow="visible"` only when it hosts an anchored overlay. RB-D-199, RB-C-118. `[review]`
- Compose each screen from a floor plan: queue/detail (My Work), record 360, guided task and approval, decision workspace (dashboard), admin console. RB-A-98, RB-D-1..19. `[review]`
- Record pages keep identity and status visible on every tab and width. RB-D-11. `[review]`
- Each screen has one evident next action, and shows state and consequence. RB-A-8 `design-system:110`, RB-B-30. `[review]`

## 3. Colour, type, tokens, motion

- Colour only through `--pds-*` tokens; no hex, rgb or named colours. RB-A-97, RB-C-23/24. `[ratchet: hardColor]`
- PDS blue for primary action, focus and selection; danger/success/warning/teal for status only, never decoration. RB-A-62/63. `[review]`
- Signal colours carry state or data only, always with a non-colour cue; never as text colour. RB-B-10/11 `ux-strategy:69`, RB-A-144. `[rule: uses signal colours only where they carry state]` `[guard: noSignalDecoration]`
- No accent tone on text-bearing `Badge`/`InlineAlert` (Finding Y). `[guard: noAccentTextTone]`
- "Required" is neutral; danger only for a real error. `[guard: requiredNotDanger]`
- Violet or AI styling only where AI was actually involved. RB-A-76, RB-A-139, RB-A-222. `[review]`
- Fonts only via `--pds-font-family-*` tokens; numbers in tables and metrics use tabular numerals. RB-A-187 (ADR 0015: type comes from canonical PDS data), RB-B-45 (token values only), RB-B-37. `[rule: sets fonts only through PDS family tokens]`
- No gradients, glow, blur or glass on work surfaces; no decorative motion; motion only to explain a state change, with a reduced-motion equivalent. RB-A-3/37/78/79, RB-B-9/27. `[review]`
- A control never changes size when its state changes. Keep button labels constant while pending (use `isLoading`). RB-A-166/167. `[review]`

## 4. States: every data screen

- Wired screens use the typed client (`useAsync`/`useAction`) and render loading, empty, validation, policy-denied, auth and unexpected-error states explicitly; never inspect `errors[0].message`. RB-B-98/107/108 `product-frontend:93,222`. `[review]`
- Loading: nothing for the first second, then a skeleton in the shape of the content (`DelayedLoading`, `Skeleton`, `DataGridLoadingPreview`, `FormLoadingPreview`). RB-B-36 `ux-strategy:125`, RB-A-115. `[review]`
- Empty: PDS `EmptyState` with a title, a line of explanation and **one** action (for example "Clear search and filters"). RB-B-35, RB-D-54, RB-D-193. `[review]`
- Errors: `ClientErrorAlert`/`ErrorState` with the request ID and a retry, next to the failed action. A failed module fails on its own; never show a failure as zero or green. RB-C-95/99, RB-A-103, RB-D-55/56. `[review]`
- Denied: a distinct denied state that hides protected content; unknown permission locks a field and says why (fail closed). RB-B-112, RB-B-156, RB-D-52/90. `[review]`
- Stale and live lists show an "as of" time; an unknown or invalid URL input shows a warning and never falls back to a guessed item. RB-D-57/63/66/83. `[review]`
- Also design: partial, offline, expired, conflict (optimistic-lock `version` mismatch), timeout (keep the query). RB-A-96, RB-B-164, RB-D-48, RB-C-35. `[review]`
- Timestamps render as `<time dateTime>` (`Timestamp` in `ui.tsx`). RB-D-72/112. `[review]`

## 5. Governed actions and trust (every write)

- Preview before commit: PDS `IntentPreview` showing actor, permission result and the before/after changes. After: `ActionAudit` receipt (request ID, correlation ID, policy) and `UndoCompensationState` stating the undo posture (available, not supported, or manual recovery; hash-chained audit can't be undone). RB-B-31 `ux-strategy:119`, RB-C-36/38/39, RB-D-86..104, RB-D-188..190. `[review]`
- Permission, evidence and freshness sit beside the action they affect. RB-A-90 `design-system:601`, RB-B-32. `[review]`
- Destructive or irreversible: `ConfirmDialog tone="danger"`, a question title, an irreversibility line and a verb label; never `window.confirm`. RB-D-187, RB-C-84. `[rule: confirms with PDS ConfirmDialog]`
- A pending operation can't be dispatched twice; a failure says whether anything changed. RB-D-97/104. `[review]`
- Every service update round-trips the `version` it read (optimistic locking); partial updates are refused. Use the shared read-then-update helpers. `[review]`
- Graph and AI writes stay preview-first; don't claim a live write path is certified without live evidence. RB-A-59, RB-C-65/145. `[review]`

## 6. Forms

- Forms are built from PDS form parts: `FormLayout`, `FieldGroup`, `TextField`, `SelectField`, `DateField`/`TimeField`, `SwitchField`, `CheckboxField`, `FileUpload`, `LookupSelect`, `ValidationSummary`, `FieldMetadata` (provenance). Labels, hints and errors go through props. RB-C-33, RB-C-67..74, RB-A-105. `[review]` `[ratchet: nativeControl]`
- Validate inline, summarise on submit (`FormErrorSummary`), and error text says how to fix it. Handle dirty-close ("discard changes?"). RB-A-38, RB-D-173/195, RB-B-174. `[review]`
- AI-filled values are marked per field and offer "Revert to extracted value" after an edit. RB-B-33. `[review]`

## 7. Lists, grids and queues

- Tables: `DataGridShell` (small) or `DataGrid` with PDS toolbar, filter, sort and pagination; compact density by default, remembered per user. RB-A-34/56, RB-C-30/31/77. `[review]`
- List state (query, filters, sort, page) and the opened item live in the URL; Back restores it and returns focus to the row. RB-D-65/68, PL-1. `[review]`
- Below 600px, tables become `WorkQueueItem` rows (`ProjectQueueList`). Window classes only via `lib/windowClass.ts` (compact <600, medium <840, expanded <1200, large <1600). RB-B-162. `[guard: windowClass]` `[e2e: layout]`

## 8. Navigation, links and URLs

- `ButtonLink`/`RouterButtonLink` for location changes, `Button` for commands; never emulate navigation with a click handler on a non-link. RB-A-49 `design-system:399`, RB-C-49/58. `[rule: changes location with ButtonLink]`
- Nav entries use `NavigationItem`; view switching uses `Tabs` or `SegmentedControl`; the `CommandPalette` carries search and frequent actions (commands act, not browse). RB-A-46/87, RB-C-151, RB-A-216. `[review]`
- A notification opens its related item and marks itself read. RB-D-77. `[review]`

## 9. Overlays

- PDS `Dialog`, `Drawer`, `Popover`, `Tooltip`, `ConfirmDialog` only (focus trap, Escape, focus return). A `Drawer` is contextual detail, never a full workflow or a form. RB-A-107, RB-C-88, RB-D-186. `[rule: opens forms in a Dialog]`

## 10. Dashboards and analytics

- `KpiTile`, `MetricTrend`, `ChartShell`, `ChartLegend` and PDS charts; custom renderers only for multi-series (documented exception). Tiles sit on the page, not in a card. RB-A-110, RB-C-40/130/133. `[review]`
- KPI tones rise only when the value warrants it; no thresholds without a real, owner-set target. RB-A-2, DECISIONS 3. `[review]`

## 11. AI surfaces

- Model output is attributed (`AiAttributionAffordance`), explainable and never blended with system facts; an ungrounded suggestion doesn't render. RB-B-65/66/69, RB-C-54. `[review]`
- AI never writes directly: every write-shaped output is an `IntentPreview`. RB-B-62. `[review]`
- Text sent to OpenAI passes the PHI gate; no PHI or PII literals in source. RB-B-157. `[phi]`
- No raw HTML from any source. RB-B-129. `[rule: renders no raw HTML]`

## 12. Responsive and accessibility

- No horizontal overflow at 390 px and 1280 px; critical actions reachable on phone widths. RB-A-41. `[e2e: layout]`
- Every control has an accessible name, visible focus and a keyboard path; status is never colour-only; icon-only buttons have `ariaLabel` and `tooltip`. RB-A-36/70, RB-C-20/28. `[e2e: a11y]` (axe) + `[review]`
- Programmatic state for selected, current, busy and disabled (`aria-current`, `aria-pressed`, `aria-busy`). RB-A-53. `[review]`
- Evidence beyond axe (forced colours, 200% zoom, reduced motion, screen reader, touch targets) is still owed (CV-6). Don't claim WCAG 2.2 compliance without it. RB-A-100/104.

## 13. Copy

- Domain nouns from the model; never framework words (PDS, appfw, GraphQL, generator, "entity") in the UI. RB-A-71/72. `[review]` `[entities]`
- Verb + noun actions ("Create project", "Record decision…"), never "Submit" or "Get Started". Empty, error and confirmation copy is direct and says what to do. RB-B-38, RB-A-93/215. `[review]`
- No CRM nouns or catalog fixture language. RB-B-101/127, RB-C-21. `[scaffold]`

## 14. Before a change is done

- Unit or component tests for new logic (pure helpers first), and an E2E assertion for any new user-visible behaviour. RB-B-124, RB-B-178. `[review]`
- Only real data paths; no mocks left in product code, no console logging, no prototype routes or flags. RB-B-105/125. `[rule: logs nothing to the console]`
- Run `bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh` (in the background) and push only after "VERIFY PASSED". The git pre-push hook, the Claude push gate and the Bitbucket pipeline all enforce it.
