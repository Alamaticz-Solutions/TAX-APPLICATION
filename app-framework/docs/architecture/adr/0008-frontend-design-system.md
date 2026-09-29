# ADR 0008: PDS Health Frontend Design System

## Status

Accepted (provisional - exercised by the CRM reference frontend and initial
PDS component source).

## Context

Enterprise consistency and theming require a shared design layer. Product teams
also need a clean base UI that carries the PDS Health brand without inheriting
CRM domain concepts. Agent productivity requires components agents can read and
edit directly, rather than configure through opaque, prop-heavy APIs.

PDS design tokens already exist in admin UI and CRM frontend consumers, but
those files are not yet governed as one framework-owned source. CRM has proven
useful as a reference implementation and E2E fixture, but it must not be the
source copied into new products.

## Decision

The design system is specifically PDS Health branded. Theme is driven by
canonical `--pds-*` design tokens under `appfw_ui/pds_health`. Reusable
component source lives under `appfw_ui/pds_health/components` as readable,
source-in-repo React primitives and CSS. Product scaffolds and reference apps
may use utility CSS locally, but shared framework components should converge on
the PDS component source.

Heavyweight component frameworks (for example MUI) are not used as the base,
because their source is not in-repo and is harder for agents to customize.

New product frontends should start from a CRM-neutral PDS Health scaffold. CRM
remains the reference app and framework E2E fixture that exercises shared
tokens, components, generated UI contracts, grids, forms, dashboards, and
accessibility paths.

## Consequences

- Agents edit real component source; few-shot from existing components is
  effective.
- Branding and theming are centralized in tokens; product frontends inherit the
  enterprise look by default.
- Accessibility derives from component APIs, token-backed styles, and explicit
  checks instead of being re-implemented per product.
- The shared component source is framework-owned. Generated product scaffolds
  consume or vendor it with provenance recorded in `.appfw-ui` manifests.
- Framework stewards own the PDS design-system source, component maturity gates,
  token drift checks, component contract checks, and shared scaffold evidence.
- Product developers own product workflow screens and copy; they consume the PDS
  system instead of forking it.
- CRM-specific strings, routes, fixtures, class names, and storage keys are not
  allowed in the base product scaffold.
