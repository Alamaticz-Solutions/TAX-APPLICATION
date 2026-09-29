/**
 * Nexus My Work IX real-journey proof.
 *
 * EXECUTION: DEFERRED_TO_B_HOST (decision-0002 / amendment packet 000004).
 * This spec is authored and committed by the B_PRODUCT leaf but is executed
 * only by the B_HOST leaf, where the chat-gated IX runtime
 * (IxRuntimeService + IxJwtVerifier + ix_transport_routes) is constructed
 * and mounted in the product backend against a real PostgreSQL. The spec
 * fails closed (skips loudly) when the live composition is absent.
 *
 * Journey under proof — real transport, no intercepted API, no mocked
 * network, no fixture response:
 *   authorized context -> start -> live SSE render -> idempotent cancel ->
 *   reload/resume -> unavailable posture for an absent-context principal.
 *
 * Required environment (provided by scripts/check-ix-real-journey.mjs):
 *   IX_JOURNEY_BASE_URL      served product frontend (backed by the real API)
 *   IX_JOURNEY_BEARER        signed local JWT for a principal WITH work context
 *   IX_JOURNEY_TENANT        tenant id for both principals
 *   IX_JOURNEY_EMPTY_BEARER  signed local JWT for a principal WITHOUT context
 */

import { expect, test, type Page } from '@playwright/test';

const baseUrl = process.env.IX_JOURNEY_BASE_URL;
const bearer = process.env.IX_JOURNEY_BEARER;
const tenant = process.env.IX_JOURNEY_TENANT ?? 'tenant_a';
const emptyBearer = process.env.IX_JOURNEY_EMPTY_BEARER;

test.skip(
  !baseUrl || !bearer,
  'DEFERRED_TO_B_HOST: set IX_JOURNEY_BASE_URL and IX_JOURNEY_BEARER against the ' +
    'running host binary with the mounted IX runtime and real PostgreSQL ' +
    '(B_HOST composition); the B_PRODUCT leaf authors but does not execute this spec.'
);

async function openJourney(page: Page, token: string): Promise<void> {
  await page.goto('/');
  const journey = page.locator('#my-work-ix');
  await expect(journey).toBeVisible();
  await page.locator('.ix-journey__auth summary').click();
  await page.getByTestId('ix-auth-token').fill(token);
  await page.getByTestId('ix-auth-tenant').fill(tenant);
}

test.describe.serial('My Work IX real journey', () => {
  test('start streams canonical lifecycle into the recipe presentation', async ({ page }) => {
    await openJourney(page, bearer as string);
    await page.getByTestId('ix-start').click();

    // Live SSE: acknowledgement, product-resolved context, phase movement.
    await expect(page.getByTestId('ix-posture')).toHaveAttribute(
      'data-posture',
      /streaming|finished/
    );
    await expect(page.getByTestId('ix-run-id')).not.toHaveText('No active run');
    await expect(page.getByTestId('ix-context')).toBeVisible();
    await expect(page.getByTestId('ix-timeline')).toContainText('Run acknowledged');
    await expect(page.getByTestId('ix-timeline')).toContainText('Context resolved');

    // The validated pds.ix.presentation@1 payload renders through the
    // registered attention-stewardship recipe from the receipted package.
    await expect(page.getByTestId('ix-presentation')).toBeVisible();
    await expect(page.getByTestId('ix-presentation')).toContainText('My Work attention brief');

    // A commit-boundary resume cursor is retained for reload/resume.
    await expect(page.getByTestId('ix-cursor')).toContainText('ix1.');
  });

  test('cancel is idempotent through the real transport', async ({ page }) => {
    await openJourney(page, bearer as string);
    await page.getByTestId('ix-start').click();
    await expect(page.getByTestId('ix-run-id')).not.toHaveText('No active run');

    // First cancel: accepted (or already terminal when the bounded run
    // finished first) — both are truthful transport dispositions.
    await page.getByTestId('ix-cancel').click();
    // Second cancel with the SAME command id must not fork state.
    await page.getByTestId('ix-cancel').click();

    await expect(page.getByTestId('ix-terminal')).toBeVisible();
    const outcome = await page.getByTestId('ix-terminal').getAttribute('data-outcome');
    expect(['cancelled', 'completed', 'partial']).toContain(outcome);

    // A third cancel after terminal must keep the same terminal outcome.
    await page.getByTestId('ix-cancel').click({ force: true }).catch(() => undefined);
    await expect(page.getByTestId('ix-terminal')).toHaveAttribute(
      'data-outcome',
      outcome as string
    );
  });

  test('reload then resume keeps exclusive final-tip history', async ({ page }) => {
    await openJourney(page, bearer as string);
    await page.getByTestId('ix-start').click();
    // Final tip only: do not reload on the first opaque ix1. cursor.
    await expect(page.getByTestId('ix-posture')).toHaveAttribute('data-posture', 'finished');
    await expect(page.getByTestId('ix-timeline')).toContainText('Run finished');
    await expect(page.getByTestId('ix-cursor')).toHaveAttribute('data-final-tip', 'true');
    await expect(page.getByTestId('ix-cursor')).toContainText('ix1.');
    const runId = await page.getByTestId('ix-run-id').textContent();
    const finalTip = await page.getByTestId('ix-cursor').textContent();

    await page.reload();
    await page.locator('.ix-journey__auth summary').click();
    await page.getByTestId('ix-auth-token').fill(bearer as string);
    await page.getByTestId('ix-auth-tenant').fill(tenant);

    const resumeResponsePromise = page.waitForResponse((response) => {
      const request = response.request();
      return (
        request.method() === 'POST' &&
        response.url().includes('/chat/stream') &&
        request.headers()['last-event-id'] === (finalTip ?? '').trim()
      );
    });
    await page.getByTestId('ix-resume').click();
    const resumeResponse = await resumeResponsePromise;
    expect(resumeResponse.status()).toBe(200);
    expect(resumeResponse.request().headers()['last-event-id']).toBe(
      (finalTip ?? '').trim()
    );
    // Exclusive last-event-id of the final tip yields no later envelopes.
    // Hydration may become visible only after this authenticated 2xx.
    await expect(page.getByTestId('ix-run-id')).toHaveText(runId ?? '');
    await expect(page.getByTestId('ix-cursor')).toHaveText(finalTip ?? '');
    await expect(page.getByTestId('ix-cursor')).toHaveAttribute('data-final-tip', 'true');
    await expect(page.getByTestId('ix-timeline')).toContainText('Phase:');
    await expect(page.getByTestId('ix-timeline')).toContainText('Artifact');
    await expect(page.getByTestId('ix-timeline')).toContainText('Run finished');
  });

  test('a principal without work context sees the unavailable posture', async ({ page }) => {
    test.skip(
      !emptyBearer,
      'DEFERRED_TO_B_HOST: IX_JOURNEY_EMPTY_BEARER (absent-context principal) is ' +
        'provisioned by the B_HOST check script.'
    );
    await openJourney(page, emptyBearer as string);
    await page.getByTestId('ix-start').click();

    // The run must end unavailable/failed — never a fabricated brief.
    await expect(page.getByTestId('ix-unavailable')).toBeVisible();
    await expect(page.getByTestId('ix-presentation')).toHaveCount(0);
  });
});
