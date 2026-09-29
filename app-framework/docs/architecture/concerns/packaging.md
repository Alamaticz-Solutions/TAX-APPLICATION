# Packaging And Product Boundary

This concern owns the durable packaging direction for App Framework.

The goal is a versioned framework that product apps consume through stable CLI,
runtime, provider, generator, and test harness packages. Product repos should
own product intent and generated output, not framework implementation copies.

## Durable Rules

- Product apps own app identity, manifest, config, generated artifacts,
  handlers, services, policies, product frontend, deployment overlays, and
  product release evidence.
- Framework packages own generator behavior, runtime contracts, provider
  semantics, CLI behavior, reusable test harnesses, docs-check rules, and
  framework certification.
- Product templates should stay slim. Do not copy provider certification,
  framework runtime internals, generator internals, or framework release gates
  into downstream product repos.
- Framework-root workflows may use `examples/products/crm` as an executable
  reference product, but the repository root is not itself a product app.
- Packaged downstream apps should consume `appfw-cli`, `appfw-codegen`,
  `appfw-runtime`, `appfw-provider-*`, and `appfw-test` through versioned
  dependencies as those packages stabilize.

## Current Reference Docs

- [Framework Packaging](../framework-packaging.md)
- [Packaging Boundary Matrix](../packaging-boundary-matrix.md)
- [Product Workspace Contract](../../reference/product-workspace-contract.md)
- [Product Workspace Boundaries](../../reference/product-workspace-boundaries.md)

Historical extraction plans and runtime inventory ledgers live in
`docs/archive/` and should not be used as current operating guidance.
