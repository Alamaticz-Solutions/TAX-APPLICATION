# ADR 0005: Frontend Scaffold Stack And Rendering Model

## Status

Accepted (provisional — exercised by the CRM reference frontend; revisit only if
server-side rendering or SEO requirements emerge).

## Context

Product frontends live outside the backend repository today; the only in-repo UI
is the runtime-generic admin console. Citizen-developer artifacts arrive as
self-contained HTML with inline JavaScript and CDN charting. Converting such an
artifact into an enterprise frontend is fast and consistent only if the target
stack is fixed once, so the work becomes a bounded mapping rather than an
open-ended design exercise. Pilot product frontends already made these calls.

## Decision

Enterprise product frontends are React + TypeScript single-page applications
built with Vite, talking to the generated backend over GraphQL. Charting is
Chart.js, preserving citizen-developer visual identity and avoiding re-teaching
agents a new library. Server-side rendering and file-based routing (Next.js) are
explicitly deferred; the SPA connects to the generated GraphQL API and is served
as static assets behind the platform ingress.

## Consequences

- One stack to scaffold, lint, test, and document; agents map artifacts onto a
  known target.
- Reuses model-to-UI patterns already proven in the admin console.
- No SSR; first-paint and SEO tradeoffs are accepted for internal enterprise
  apps.
- A future SSR need is a new ADR, not a silent divergence.
