# PDS Health Native Components

`@appfw/pds-health-native` is the PDS-owned React Native renderer
for the channel-neutral `pds.ix.presentation@1` contract. It provides
`EvidenceDisclosure`, `ResolvedContextDisclosure`, `WorkStatus`,
`ProgressiveResponse`, and the thin `PdsIxPresentation` composition.

The public `@appfw/pds-health-native/ix-recipes` subpath exposes
`PdsIxRecipePresentation`, its props, and the resolver that projects any exact
`pds.ix.recipe_registration@1` registration to either `native-ios` or
`native-android`. The separate `@appfw/pds-health-native/ix-recipe-projection`
subpath exposes that resolver without loading React Native so admission tooling
and detached non-rendering consumers can execute the projection contract
directly. The generic component delegates caller-owned presentation data to
the package's native composition. Together they support all eight canonical
recipe identities without taking ownership of product language, data, policy,
actions, or journeys. Invalid registration or presentation data uses the
existing non-actionable accessible fallback.

The package consumes the standalone contract and deterministic
`pds.native.design-data@1` token projection directly. It intentionally owns no
Expo shell, safe-area policy, navigation, deep links, secure storage, source
resolution, product copy, callback effects, provider behavior, or Intelligent
Experience lifecycle semantics. Those remain application or runtime concerns.

The `@appfw/pds-health-native/design-data` subpath exposes the versioned,
generated native design-data projection. The first projection provides an
Apple-like light/dark visual theme selected independently from renderer
projection. iOS and Android remain not-qualified until retained platform
evidence exists; Android does not yet claim a Material-like implementation.
This package is local-only. It has no device, distribution, release, or
product-adoption claim.

`npm run proof:detached-consumer` delegates to the clean-checkout three-archive
proof. That proof begins without `node_modules`, creates a fresh writable npm
cache, populates it from the three exact package locks, then repeats build,
pack, install, strict typecheck, and runtime imports offline for contract
`0.2.0`, Web `0.12.0`, and native `0.2.0`. The proof establishes package
portability only; it does not qualify either native platform.

The native renderer owns one programmatic announcement for each accepted
presentation revision. Nested native views do not create a second revision
announcement. Context and work-status accessibility labels remain available
without claiming separate lifecycle authority.
