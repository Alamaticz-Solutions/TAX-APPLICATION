#!/usr/bin/env node
/**
 * Nexus My Work IX real-journey check script.
 *
 * EXECUTION: DEFERRED_TO_B_HOST (decision-0002 / amendment packet 000004).
 * The B_PRODUCT leaf authors and commits this gate; the B_HOST leaf executes
 * it once the chat-gated IX runtime (IxRuntimeService + IxJwtVerifier +
 * ix_transport_routes) is constructed and mounted in the product backend
 * against a real PostgreSQL. Nothing here mocks, intercepts, or replays: the
 * script verifies the live composition and then runs the committed
 * Playwright journey spec against it.
 *
 * Required environment (B_HOST provides real values):
 *   IX_JOURNEY_BASE_URL      served product frontend URL
 *   IX_JOURNEY_API_URL       product backend base URL (JWT ingress + /chat/*)
 *   IX_JOURNEY_BEARER        signed local JWT, principal WITH work context
 *   IX_JOURNEY_ORIGIN        bound Browser Origin; must equal Host
 *                            NEXUS_IX_FRONTEND_ORIGIN / HostIxCompose.frontend_origin
 *   IX_JOURNEY_TENANT        tenant id (defaults to tenant_a)
 *   IX_JOURNEY_EMPTY_BEARER  signed local JWT, principal WITHOUT context
 */

import { spawnSync } from 'node:child_process';
import process from 'node:process';

const required = [
  'IX_JOURNEY_BASE_URL',
  'IX_JOURNEY_API_URL',
  'IX_JOURNEY_BEARER',
  'IX_JOURNEY_ORIGIN'
];
const missing = required.filter((name) => !process.env[name] || !process.env[name].trim());

if (missing.length > 0) {
  console.error(
    [
      'ix-real-journey: DEFERRED_TO_B_HOST — live composition not present.',
      `Missing environment: ${missing.join(', ')}.`,
      'This gate executes on the B_HOST leaf against the running host binary',
      'with the mounted IX transport and a real PostgreSQL. The B_PRODUCT',
      'leaf authors this script but does not execute it (decision-0002).'
    ].join('\n')
  );
  process.exit(2);
}

const apiUrl = process.env.IX_JOURNEY_API_URL.replace(/\/$/, '');
const bearer = process.env.IX_JOURNEY_BEARER;
const tenant = process.env.IX_JOURNEY_TENANT ?? 'tenant_a';
const origin = process.env.IX_JOURNEY_ORIGIN.trim();

async function preflight() {
  // 1. Unauthenticated + bound Origin must prove JWT ingress (401).
  //    Host composes IxClientProfile::Browser; authorize_origin requires
  //    exactly this Origin. Do not guess a second allowed origin.
  const unauthenticated = await fetch(`${apiUrl}/chat/stream`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      origin
    },
    body: JSON.stringify({
      schemaVersion: 'appfw.ix_run_request@1',
      intentKey: 'pds.ix.intent.attention-stewardship@1',
      focus: { kind: 'nexus.work-queue', id: 'my-work' }
    })
  });
  if (unauthenticated.status !== 401) {
    throw new Error(
      `unauthenticated+Origin /chat/stream must be 401 ingress; received ${unauthenticated.status}`
    );
  }

  // 2. Authenticated + bound Origin must serve SSE 200.
  const controller = new AbortController();
  const authenticated = await fetch(`${apiUrl}/chat/stream`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      accept: 'text/event-stream',
      authorization: `Bearer ${bearer}`,
      'x-tenant-id': tenant,
      origin
    },
    body: JSON.stringify({
      schemaVersion: 'appfw.ix_run_request@1',
      intentKey: 'pds.ix.intent.attention-stewardship@1',
      focus: { kind: 'nexus.work-queue', id: 'my-work' }
    }),
    signal: controller.signal
  });
  const contentType = authenticated.headers.get('content-type') ?? '';
  if (!authenticated.ok || !contentType.includes('text/event-stream')) {
    throw new Error(
      `authenticated /chat/stream must serve text/event-stream; received ` +
        `${authenticated.status} ${contentType}`
    );
  }
  controller.abort();
  console.log('ix-real-journey: transport preflight passed (ingress + SSE live).');
}

try {
  await preflight();
} catch (error) {
  console.error(`ix-real-journey: preflight failed: ${error.message}`);
  process.exit(1);
}

const result = spawnSync(
  'npx',
  ['playwright', 'test', 'tests/ix-real-journey.spec.ts'],
  {
    stdio: 'inherit',
    env: process.env,
    cwd: new URL('..', import.meta.url).pathname
  }
);

if (result.status !== 0) {
  console.error('ix-real-journey: journey spec failed against the live composition.');
  process.exit(result.status ?? 1);
}

console.log('ix-real-journey: PASS — real journey proven against the live composition.');
