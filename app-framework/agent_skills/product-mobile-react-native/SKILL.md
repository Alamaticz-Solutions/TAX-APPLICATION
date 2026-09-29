---
name: product-mobile-react-native
audience: product
phase: mobile
cli_namespace: product
artifacts: .appfw/target/appfw/mobile-rn-conversion-plan.json,target/appfw/agent-handoff.json
description: Use when converting a mobile HTML mockup, mobile screenshots, or a mobile workflow brief into a React Native + Expo product app that consumes App Framework contracts.
---

# Product Mobile React Native

## Use When

- Converting an HTML mockup of a mobile app into React Native source.
- Creating or changing product-owned `mobile/` screens, navigation, native
  app shell, secure storage, push, offline, or Expo/EAS configuration.
- Deciding how mobile-native layouts relate to an existing product web app.

## Procedure

1. Read `docs/frontend/mobile-react-native.md` for the mobile architecture,
   layout relationship, ownership boundary, and publish path.
2. Run `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`
   before writing React Native source; retain the generated plan.
3. Treat the HTML mockup as workflow and visual evidence only. Do not copy DOM,
   CSS, mock data, local storage, or unauthenticated behavior into RN.
4. Map every screen and action to `.appfw/model`, generated API operations,
   product services, or a documented integration boundary.
5. Preserve workflow parity with web while using mobile-native navigation,
   screen composition, touch behavior, sheets, modals, safe areas, and back
   behavior.
6. Use PDS native tokens and generated contracts; keep product-specific mobile
   features under product-owned `mobile/` modules.
7. Gate secure storage, biometrics, push, offline cache, file/device APIs, and
   local persistence with data-classification and retention decisions.
8. Verify with product validation, generated drift checks, `mobile-test`
   diagnostics, `mobile-test --run-local` for RN type/test/Expo doctor and
   runtime-audit diagnostics when dependencies are installed,
   `mobile-test --device-preflight` for local tooling posture, separately
   retained device/simulator and store-track observations, and product handoff.
   These legacy inputs remain non-authoritative; a future source-bound checker
   owns candidate evidence, and named human authorities own distribution and
   release decisions.

## Proof

```bash
scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product mobile-test --json
scripts/appfw product mobile-test --run-local --json
scripts/appfw product mobile-test --device-preflight --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not treat mobile as a responsive clone of the web layout.
- Do not use PWA evidence as proof of native mobile UX.
- Do not use static scaffold or device-preflight evidence as simulator,
  physical-device, or store-track release certification.
- Treat every compatibility-named U5 `mobile-test` input and staged artifact as
  diagnostic and non-authoritative, even when it was produced from the target
  app build and carries platform/build/runtime identity plus a workflow matrix.
- Keep `candidate_ready:false` and `release_ready:false` in all legacy mobile
  evidence. A future source-bound checker, not `mobile-test`, owns candidate
  evidence; named humans own distribution, OTA, store/MDM, and release
  decisions. Legacy JSON must parse strictly with unique object keys; malformed,
  duplicate-key, or non-boolean readiness fields are rejected and never staged.
- Do not treat placeholder `device-evidence.json` or
  `store-track-evidence.json` files as candidate or release evidence when they
  retain `status:not-run`, `static_scaffold_only:true`, `template_only:true`,
  or `release_ready:false`; replace placeholders with accurate retained
  build/device/store observations while keeping the legacy readiness fields
  false.
- Do not promote `npm-audit-disposition.template.json` to authority evidence;
  create `npm-audit-disposition.json` only with named release/security approval
  and a matching retained audit summary. In `mobile-test` it remains a
  non-authoritative diagnostic input.
- Do not consume `frontend/src/generated/appfw-ui-contract.ts` as canonical
  mobile input. Use `mobile/src/generated/appfw-mobile-contract.ts`; any Web
  generated TypeScript used during migration is diagnostic evidence only.
- Do not cache PHI/ePHI or tenant data on-device without explicit governance.
- Do not use EAS Update for native-code, permission, SDK, entitlement, or
  native dependency changes.
