# PDS IX Presentation Contract

`@appfw/pds-ix-presentation-contract` is the standalone, React-free source of
truth for `pds.ix.presentation@1`, `pds.ix.recipe_registry@1`, and
`pds.ix.recipe_registration@1`. It contains closed JSON Schemas, runtime
validators, TypeScript declarations, the canonical eight-recipe registry, and
a sanitized presentation fixture shared by Web and native renderers.

The contract describes presentation only. It carries no Intelligent Experience
lifecycle, provider, authorization, navigation, product, or action-effect
semantics. Channel renderers may adapt this data, but may not silently discard
context gaps, stable source references, region status, or announcements.

The recipe registry adds stable IX identity, intent, renderer, required
capability, and projection-readiness metadata for:

1. Analyze Why
2. Contextual Conversation
3. Adaptive Composition
4. Working Goal Plan
5. Adaptive Information Lens
6. Situation to Strategy
7. Attention Stewardship
8. Agents Helping You (`ambient-agent-continuity`)

Products own `artifactType`, domain data, language, orchestration, actions, and
policy. A capability in a recipe registration is a compatibility obligation,
not evidence that the behavior executed. Native iOS and Android projections
remain explicitly `not-qualified`.

Use the root package or `@appfw/pds-ix-presentation-contract/recipes` for the
recipe APIs. The canonical registry and both recipe schemas are exported as
package JSON subpaths.

Run `npm test` and `npm run check:package` from this directory.
