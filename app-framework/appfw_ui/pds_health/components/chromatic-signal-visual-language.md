# Chromatic Signal Visual Language

Date ratified: 2026-07-16

Status: accepted PDS Health design-system direction. Signal use remains subject
to semantic, contrast, accessibility, and product evidence.

## Intent

Accessible semantic text colors and vivid visual marks have different jobs.
Semantic ink remains contrast-safe for labels and status text. A compact,
brighter palette carries bounded data, progress, state, and intelligence cues
without turning containers or page backgrounds into decoration.

## Signal Palette

| Role | Token | Intended use |
| --- | --- | --- |
| Electric blue | `--pds-color-signal-blue` | Primary data, progress, running state, active controls |
| Clear teal | `--pds-color-signal-teal` | Progress completion edge and operational flow |
| Emerald | `--pds-color-signal-green` | Successful completion and ready data |
| Electric violet | `--pds-color-signal-violet` | Intelligence and model-derived signals |
| Coral | `--pds-color-signal-coral` | Blocked, failed, or dangerous visual marks |
| Amber | `--pds-color-signal-amber` | Pending, stale, or warning visual marks |

## Contract

- Large surfaces remain white or luminous blue-white. Signal colors occupy
  progress, charts, dots, markers, active controls, and semantic borders.
- Accessible semantic ink remains the source for labels and explanatory text;
  a vivid fill is not assumed to be readable text color.
- Blue leads the product identity. Supporting hues appear only when they encode
  a meaningful distinction.
- Progress may use a narrow blue-to-teal transition. Routine charts use
  balanced solid series colors rather than unrelated gradient treatments.
- Timeline, operation, feedback, freshness, and attention markers may use vivid
  fills, with shape, copy, iconography, or position providing a non-color cue.
- Intelligent cards use one quiet AI-tinted surface and compact attribution;
  layered gradients, aura stacks, and ornamental rings are not the AI identity.
- Data series, status, and risk mappings remain stable within a product journey.

## Verification

Every signal use must retain non-color meaning, token-only CSS, light/dark
review, contrast checks for adjacent colors and overlaid content, and
representative color-vision inspection. Product teams may not create parallel
raw-color palettes.
