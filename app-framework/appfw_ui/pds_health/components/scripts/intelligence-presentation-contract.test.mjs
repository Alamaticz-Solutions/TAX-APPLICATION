import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  EvidenceDisclosure,
  ProgressiveResponse,
  ResolvedContextDisclosure,
  WorkStatus
} from "../dist/intelligence-presentation.js";
import {
  assertPdsIxPresentation,
  pdsIxPresentationSchema,
  validatePdsIxPresentation
} from "../dist/intelligence-presentation-model.js";

test("evidence and resolved context remain presentational and inspectable", () => {
  const evidence = renderToStaticMarkup(React.createElement(EvidenceDisclosure, {
    model: {
      summary: "Inspect source and derivation",
      items: [
        { label: "Source", value: "Sanitized fixture" },
        { label: "How derived", value: "Deterministic rule" }
      ]
    }
  }));
  assert.match(evidence, /<details class="pds-evidence-disclosure">/);
  assert.match(evidence, /Inspect source and derivation/);
  assert.match(evidence, /Sanitized fixture/);

  const context = renderToStaticMarkup(React.createElement(ResolvedContextDisclosure, {
    model: {
      eyebrow: "Working from",
      title: "Current request",
      detail: "Two authorized sources",
      announcement: "Context resolved with one known gap.",
      gaps: ["Approval owner is not yet resolved."],
      evidence: {
        summary: "View 2 sources",
        items: [{ sourceRef: "fixture-source-a", label: "Freshness", value: "Current" }]
      },
      metaLabel: "Evaluated now",
      actionLabel: "Check context"
    },
    meta: React.createElement("span", null, "Evaluated now"),
    action: React.createElement("button", { type: "button" }, "Check context")
  }));
  assert.match(context, /aria-label="Context used for this work"/);
  assert.match(context, /View 2 sources/);
  assert.match(context, /data-source-ref="fixture-source-a"/);
  assert.match(context, /Context gaps/);
  assert.match(context, /Check context/);
});

test("work status and progressive response expose live announcements without lifecycle policy", () => {
  const status = renderToStaticMarkup(React.createElement(WorkStatus, {
    model: {
      label: "Checking evidence",
      detail: "Resolving two sources",
      active: true,
      actionLabel: "Pause"
    },
    action: React.createElement("button", { type: "button" }, "Pause")
  }));
  assert.match(status, /data-active="true"/);
  assert.match(status, /role="status" aria-live="polite" aria-atomic="true"/);
  assert.match(status, /Pause/);

  const response = renderToStaticMarkup(React.createElement(ProgressiveResponse, {
    model: {
      eyebrow: "Progressive response",
      title: "Working brief",
      announcement: "Working brief revision 2. Updated: Current signal.",
      metaLabel: "Revision 2",
      active: true,
      emptyState: "Useful sections will appear when ready.",
      regions: [{
        id: "signal",
        status: "partial",
        label: "Signal",
        title: "Current signal",
        body: "A useful bounded finding.",
        whyItMatters: "It changes the next decision.",
        evidence: {
          summary: "Where did this come from?",
          items: [{ label: "Source", value: "Fixture A" }]
        },
        actionLabel: "Challenge this",
        changed: true
      }]
    },
    meta: React.createElement("span", null, "Revision 2"),
    renderAction: (region) => React.createElement("button", { type: "button" }, region.actionLabel)
  }));
  assert.match(response, /class="pds-assistive-announcement" role="status" aria-live="polite"/);
  assert.match(response, /data-changed="true"/);
  assert.match(response, /data-status="partial"/);
  assert.match(response, /Why this matters:/);
  assert.match(response, /Challenge this/);
});

test("progressive response retains a truthful empty state", () => {
  const html = renderToStaticMarkup(React.createElement(ProgressiveResponse, {
    model: {
      eyebrow: "Progressive response",
      title: "Answer",
      announcement: "No sections yet.",
      active: false,
      regions: [],
      emptyState: "Useful sections will appear when ready."
    }
  }));
  assert.equal(html.match(/role="status"/g)?.length, 1);
  assert.match(html, /No sections yet\./);
  assert.match(html, /Useful sections will appear when ready/);
});

test("repeated evidence disclosures scope generated DOM identifiers", () => {
  const model = {
    summary: "Inspect evidence",
    items: [{ id: "source", label: "Source", value: "Fixture" }]
  };
  const html = renderToStaticMarkup(React.createElement(
    React.Fragment,
    null,
    React.createElement(EvidenceDisclosure, { model }),
    React.createElement(EvidenceDisclosure, { model })
  ));
  const ids = [...html.matchAll(/<dt id="([^"]+)"/g)].map((match) => match[1]);
  assert.equal(ids.length, 2);
  assert.equal(new Set(ids).size, 2);
  assert.equal(ids.some((id) => id === "source"), false);
});

test("progressive response preserves legacy action eligibility and delegates editable regions", () => {
  const region = {
    id: "editable-without-action-label",
    status: "ready",
    title: "Caller-owned edit",
    body: "The caller decides which controls are appropriate.",
    editable: true
  };
  let actionCalls = 0;
  let receivedEditRegion;
  const html = renderToStaticMarkup(React.createElement(ProgressiveResponse, {
    model: {
      eyebrow: "Progressive response",
      title: "Answer",
      announcement: "One editable section is ready.",
      active: false,
      regions: [region],
      emptyState: "No sections yet."
    },
    renderAction() {
      actionCalls += 1;
      return React.createElement("button", { type: "button" }, "Unnamed legacy action");
    },
    renderEditAction(value) {
      receivedEditRegion = value;
      return React.createElement("button", { type: "button" }, "Edit this section");
    }
  }));
  assert.equal(actionCalls, 0);
  assert.equal(receivedEditRegion, region);
  assert.equal(region.actionLabel, undefined);
  assert.match(html, /class="pds-progressive-response__action"/);
  assert.match(html, />Edit this section<\/button>/);
  assert.doesNotMatch(html, /Unnamed legacy action/);

  const withoutAction = renderToStaticMarkup(React.createElement(ProgressiveResponse, {
    model: {
      eyebrow: "Progressive response",
      title: "Answer",
      announcement: "One editable section is ready.",
      active: false,
      regions: [region],
      emptyState: "No sections yet."
    }
  }));
  assert.doesNotMatch(withoutAction, /pds-progressive-response__action/);

  let noneditableCalls = 0;
  renderToStaticMarkup(React.createElement(ProgressiveResponse, {
    model: {
      eyebrow: "Progressive response",
      title: "Answer",
      announcement: "One noneditable section is ready.",
      active: false,
      regions: [{ ...region, editable: false }],
      emptyState: "No sections yet."
    },
    renderEditAction() {
      noneditableCalls += 1;
      return React.createElement("button", { type: "button" }, "Edit this section");
    }
  }));
  assert.equal(noneditableCalls, 0);
});

test("pds.ix.presentation@1 golden fixture round-trips through the canonical validator", async () => {
  const model = JSON.parse(await readFile(
    new URL("../../ix-presentation-contract/fixtures/working-brief.presentation.json", import.meta.url),
    "utf8"
  ));
  assert.equal(validatePdsIxPresentation(model).ok, true);
  assert.deepEqual(assertPdsIxPresentation(model), model);
  assert.deepEqual(JSON.parse(JSON.stringify(model)), model);
  assert.equal(JSON.stringify(model).includes("React"), false);
  const compiledModel = await readFile(new URL("../dist/intelligence-presentation-model.js", import.meta.url), "utf8");
  assert.doesNotMatch(compiledModel, /\breact\b/i);
  assert.match(compiledModel, /from "@appfw\/pds-ix-presentation-contract"/);
  assert.doesNotMatch(compiledModel, /\.\.\/.*\/src\//);
});
