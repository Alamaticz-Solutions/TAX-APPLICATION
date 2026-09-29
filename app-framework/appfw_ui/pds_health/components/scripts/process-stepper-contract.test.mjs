import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ProcessStepper } from "../dist/process.js";

const steps = [
  { id: "review", label: "Review", description: "Review the evidence" },
  { id: "build", label: "Build", description: "Complete the work" },
  { id: "release", label: "Release", disabled: true }
];

test("interactive process steps expose one full-surface native button per step", () => {
  const html = renderToStaticMarkup(
    createElement(ProcessStepper, {
      ariaLabel: "Release process",
      steps,
      currentStepId: "build",
      selectedStepId: "review",
      onStepSelect() {}
    })
  );

  assert.match(html, /data-interactive="true"/);
  assert.match(html, /data-selected-step-id="review"/);

  const buttons = [...html.matchAll(/<button([^>]*)>([\s\S]*?)<\/button>/g)];
  assert.equal(buttons.length, steps.length);

  const [selectedAttributes, selectedContent] = buttons[0].slice(1);
  assert.match(selectedAttributes, /type="button"/);
  assert.match(selectedAttributes, /class="pds-process-stepper__trigger pds-process-stepper__selection-surface"/);
  assert.match(selectedAttributes, /aria-pressed="true"/);
  assert.match(selectedAttributes, /data-selected="true"/);
  assert.match(selectedContent, /class="pds-process-stepper__marker"/);
  assert.match(selectedContent, /class="pds-process-stepper__copy"/);

  const currentAttributes = buttons[1][1];
  assert.match(currentAttributes, /aria-current="step"/);
  assert.match(currentAttributes, /aria-pressed="false"/);
  assert.match(currentAttributes, /data-current="true"/);

  const disabledAttributes = buttons[2][1];
  assert.match(disabledAttributes, /disabled=""/);
  assert.match(disabledAttributes, /aria-disabled="true"/);
});

test("noninteractive process steps preserve ordered-list presentation", () => {
  const html = renderToStaticMarkup(
    createElement(ProcessStepper, {
      ariaLabel: "Release process",
      steps,
      currentStepId: "build"
    })
  );

  assert.doesNotMatch(html, /data-interactive=/);
  assert.doesNotMatch(html, /<button/);
  assert.equal((html.match(/class="pds-process-stepper__marker"/g) ?? []).length, steps.length);
  assert.equal((html.match(/class="pds-process-stepper__trigger"/g) ?? []).length, steps.length);
});

test("interactive process selection can be explicitly cleared without changing the current step", () => {
  const html = renderToStaticMarkup(
    createElement(ProcessStepper, {
      ariaLabel: "Release process",
      steps,
      currentStepId: "build",
      selectedStepId: null,
      onStepSelect() {}
    })
  );

  assert.doesNotMatch(html, /data-selected-step-id=/);
  assert.equal((html.match(/aria-pressed="false"/g) ?? []).length, steps.length);
  assert.equal((html.match(/data-selected="true"/g) ?? []).length, 0);
  assert.equal((html.match(/data-current="true"/g) ?? []).length, 2);
});

test("milestone process steps expose compact status symbols and independent selection", () => {
  const milestoneSteps = [
    { id: "intake", label: "Deal entry", status: "complete" },
    { id: "lease", label: "Lease execution", status: "current", metadata: "9 days" },
    { id: "setup", label: "Project setup", status: "upcoming" },
    { id: "build", label: "Permitting and build", status: "warning" }
  ];
  const html = renderToStaticMarkup(
    createElement(ProcessStepper, {
      ariaLabel: "Office milestones",
      steps: milestoneSteps,
      currentStepId: "lease",
      selectedStepId: "setup",
      variant: "milestone",
      onStepSelect() {}
    })
  );

  assert.match(html, /data-variant="milestone"/);
  assert.match(html, /data-selected-step-id="setup"/);
  assert.match(html, /data-symbol="complete"[^>]*><span>✓<\/span>/);
  assert.match(html, /data-symbol="current"[^>]*><span>•<\/span>/);
  assert.match(html, /data-symbol="upcoming"[^>]*><span>3<\/span>/);
  assert.match(html, /data-symbol="warning"[^>]*><span>!<\/span>/);
  assert.match(html, /class="pds-process-stepper__status-label"/);
  assert.match(html, /class="pds-process-stepper__metadata">9 days<\/span>/);
  assert.equal((html.match(/aria-pressed="true"/g) ?? []).length, 1);
});

test("compact process presentation keeps the labeled stepper and renders semantic segments", () => {
  const html = renderToStaticMarkup(
    createElement(ProcessStepper, {
      ariaLabel: "Release process",
      steps,
      currentStepId: "build",
      compactPresentation: "segments"
    })
  );

  assert.match(html, /class="pds-process-stepper-adaptive"/);
  assert.match(html, /class="pds-process-stepper-adaptive__full"/);
  assert.match(html, /class="pds-process-stepper"/);
  assert.match(
    html,
    /class="pds-process-progress pds-process-stepper-adaptive__compact"/
  );
  assert.match(html, /aria-hidden="true"/);
  assert.match(html, /role="progressbar"/);
  assert.match(html, /aria-valuetext="Step 2 of 3"/);
  assert.equal(
    (html.match(/class="pds-process-stepper__step"/g) ?? []).length,
    steps.length
  );
  assert.equal(
    (html.match(/class="pds-process-progress__segments"/g) ?? []).length,
    1
  );
  assert.equal(
    (html.match(/<span data-status="complete"><\/span>/g) ?? []).length,
    1
  );
  assert.equal(
    (html.match(/<span data-status="current"><\/span>/g) ?? []).length,
    1
  );
  assert.equal(
    (html.match(/<span data-status="upcoming"><\/span>/g) ?? []).length,
    1
  );
});
