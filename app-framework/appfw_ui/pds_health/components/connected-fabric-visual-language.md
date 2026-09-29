# Connected Fabric Visual Language

Date ratified: 2026-07-16

Status: accepted expressive web language and catalog reference implementation.
Product rollout is optional and evidence-gated.

Accepted implementation baseline: `connected-fabric/1.0.0`, recorded in
`connected-fabric.accepted-baseline.json` on 2026-07-17.

## Meaning

Connected Fabric gives the application-fabric strategy a restrained visual
signature. The hex topology represents connected enterprise capabilities. A
temporary circuit represents a connection being established and acknowledged:

1. a source ring appears;
2. a colored signal follows mesh edges to a destination;
3. a destination ring appears;
4. a compact bright-blue acknowledgment returns over the same route;
5. the resolved circuit holds briefly and fades uniformly.

This sequence is ambient visual language, not an execution trace. It must not
imply that a real API call, workflow, agent, write, or integration succeeded.

## Interaction Contract

- A faint fixed mesh is always available as the fabric baseline.
- Fine-pointer movement increases mesh presence with smooth positional lag; it
  does not stretch, tilt, distort, or thicken the topology.
- Pointer-local circuits are sparse and rate-limited. Autonomous circuits occur
  independently every 10–30 seconds at lower opacity.
- A circuit follows five to eight mesh edges. Concurrent visible circuits may
  meet at nodes but cannot reuse an occupied edge. A route that cannot satisfy
  the invariant is not shown.
- Source and destination rings remain compact. The routed signal is more
  prominent than the mesh, and the acknowledgment is more prominent than the
  resting route only while it travels.
- Completed circuits hold briefly, then dissolve over the full route. They do
  not accumulate into persistent visual residue.

## Accessibility And Restraint

- The canvas is `aria-hidden`, cannot receive focus, and uses
  `pointer-events: none`.
- Business meaning always uses accessible components, text, status, evidence,
  and operation state outside the canvas.
- Panel interiors mask the fabric by default. A panel may reveal only a faint
  local mesh response; content contrast always wins.
- Reduced-motion disables responsive and circuit motion while preserving a
  calm static composition. Coarse-pointer and hidden-document states do not
  run interactive signals.
- Native mobile receives a channel-native equivalent only when a product brief
  justifies it; this web canvas is not ported mechanically to React Native.
- No sound, haptic, cursor capture, interaction dependency, or loading delay is
  attached to the effect.

## Ownership And Adoption

The compiled PDS component implementation is the canonical reference, owned by
`appfw_ui/pds_health/components/src/connected-fabric.tsx`. The catalog and
product web surfaces consume the same reviewed package boundary. Do not fork
its geometry, timing, or palette into catalog-local or product-local copies.

Reviewed product compositions may supply three optional, stable CSS selectors
to the canonical component: `contentRootSelector` for mutation and
resize observation, `panelSelector` for default interior masking, and
`scrollRootSelector` for independently scrolling workspaces. Catalog defaults
remain unchanged when these selectors are omitted. Product compositions must
not use this boundary to change mesh geometry, timing, palette, semantics, or
motion policy.

Required product evidence includes representative light/dark screenshots,
reduced-motion behavior, keyboard and screen-reader equivalence, frame-time and
CPU review on target hardware, canvas fallback, responsive layout, and proof
that the effect does not obscure or misrepresent work state.

## Preservation And Change Control

The accepted behavior includes the faint resting mesh, smooth pointer lag,
panel masking, pointer-local and autonomous routing, exclusive visible edges,
source and destination rings, multicolor route construction, return
acknowledgment, resolved hold, and uniform fade. These behaviors are one
coherent signature asset; a visual-theme or interaction-substrate migration
must not silently restyle, simplify, or replace them.

`connected-fabric.accepted-baseline.json` records the accepted constants and
source markers. `scripts/check-pds-components.mjs` protects that static
baseline, while `scripts/check-pds-catalog-evidence.mjs` proves the runtime
canvas contract and edge invariant in a browser. A change to the baseline
requires all of the following:

1. an explicit Product Owner visual review;
2. an updated visual-language rationale and baseline version;
3. retained light, dark, reduced-motion, and route-exclusivity evidence; and
4. proof that content legibility, frame time, and semantic restraint did not
   regress.

The baseline protects behavior, not an implementation monopoly. The shared
primitive replaced the catalog-owned module only after the Nexus journey
established the public appearance and selector configuration boundary; future
implementation changes remain subject to this accepted contract.
