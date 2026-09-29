# Chat Markdown Sanitizer Decision

Status: Wave 4 CH5 decision recorded; implementation remains gated.

## Decision

Use `react-markdown` with `rehype-sanitize` for the PDS `StreamingText`
markdown adapter when live conversational rendering is implemented.

This is a client-side rendering decision only. The framework still treats all
model output, tool output, citations, and generated labels as untrusted input.
The component package must not wire a markdown renderer until the adapter has
its own tests, catalog evidence, and release evidence.

## Required Controls

- Raw HTML is disabled. Do not enable `rehype-raw` for model-authored content.
- Sanitization uses a framework-owned `rehype-sanitize` schema. Product apps
  must not pass custom schemas directly to `StreamingText`.
- Allowed markdown is intentionally small: paragraphs, emphasis, strong text,
  inline code, code blocks, ordered and unordered lists, tables only when the
  product surface opts in, and links only after URL policy checks.
- URL policy is allowlist-based. Permit `https:` and generated `appfw://`
  references only after framework resolution. Block `javascript:`, `data:`,
  raw relative URLs, and model-authored deep links.
- Entity references remain refs-as-pointers. Model output may name
  `appfw://` refs, but navigation cards are created only after generated
  contract resolution and policy trimming.
- Incremental streaming must sanitize each rendered snapshot before it reaches
  the DOM. Never append unsanitized HTML chunks.
- The adapter must expose screen-reader friendly loading/streaming status and
  preserve citation affordances from the answer envelope.

## Dependency Posture

`react-markdown` and `rehype-sanitize` are expected to be optional PDS
component-package dependencies or peer dependencies, not product-direct
imports. They require the normal dependency, license, vulnerability, bundle, and
accessibility review before the live adapter is released.

No server-side Rust runtime dependency is introduced by this decision. The
server emits the governed answer-envelope / stream contract; the PDS client
adapter renders sanitized presentation.

## Evidence Contract

`scripts/check-pds-components.mjs --json` records
`conversation_markdown_sanitizer` in `target/appfw/pds-component-check.json`.
That section must remain `live_ready:false` until:

1. the adapter exists in the PDS component package;
2. sanitizer tests cover malicious HTML, bad URL schemes, raw relative links,
   malformed markdown, incremental streaming snapshots, and citation/ref
   rendering;
3. catalog visual/a11y evidence covers sanitized markdown states; and
4. release evidence proves the dependency and bundle posture is acceptable.

Until then, `StreamingText` may render plain trusted examples in the catalog,
but product live chat must not render model-authored markdown as HTML.
