import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = await readFile(
  resolve(componentRoot, "src/work-surfaces.tsx"),
  "utf8"
);

test("work-surface APIs remain additive and product-neutral", () => {
  for (const exportName of [
    "ButtonLink",
    "InteractiveCard",
    "CardLink",
    "WorkQueueItem"
  ]) {
    assert.match(
      source,
      new RegExp(`export function ${exportName}\\(`),
      `missing ${exportName} export`
    );
  }

  for (const propsName of [
    "ButtonLinkProps",
    "InteractiveCardProps",
    "CardLinkProps",
    "WorkQueueItemProps"
  ]) {
    assert.match(
      source,
      new RegExp(`export type ${propsName} =`),
      `missing ${propsName} contract`
    );
  }

  assert.doesNotMatch(
    source,
    /Nexus|CRM|request|workflow/i,
    "shared work surfaces must not contain product or workflow copy"
  );
});

test("link and command activations use native semantics", () => {
  assert.match(source, /kind: "link"/);
  assert.match(source, /kind: "button"/);
  assert.match(source, /<a[\s\S]*href=\{activation\.href\}/);
  assert.match(source, /<button[\s\S]*type=\{type \?\? "button"\}/);
  assert.doesNotMatch(source, /role=["'](?:link|button)["']/);
  assert.match(source, /aria-disabled=\{isDisabled \|\| undefined\}/);
  assert.match(source, /if \(isDisabled\) \{\s*event\.preventDefault\(\)/);
});

test("whole-surface activation and trailing actions are siblings", () => {
  const interactiveCard = source.slice(
    source.indexOf("export function InteractiveCard("),
    source.indexOf("export type CardLinkProps")
  );
  const queueItem = source.slice(
    source.indexOf("export function WorkQueueItem(")
  );

  for (const componentSource of [interactiveCard, queueItem]) {
    const activationIndex = componentSource.indexOf("<ActivationOverlay");
    const trailingIndex = componentSource.indexOf("trailingAction ? (");
    assert.ok(activationIndex >= 0, "missing whole-surface activation overlay");
    assert.ok(
      trailingIndex > activationIndex,
      "trailing action must be rendered as a sibling after the activation overlay"
    );
  }

  assert.doesNotMatch(interactiveCard, /<(?:a|button)\b/);
  assert.doesNotMatch(queueItem, /<(?:a|button)\b/);
});

test("variant, state, and density contracts are exposed through data attributes", () => {
  for (const attribute of [
    "data-activation",
    "data-completed",
    "data-density",
    "data-disabled",
    "data-selected",
    "data-tone",
    "data-variant"
  ]) {
    assert.match(source, new RegExp(attribute), `missing ${attribute}`);
  }

  assert.match(source, /aria-labelledby=/);
  assert.match(source, /aria-describedby=/);
  assert.match(source, /disabled=\{isDisabled\}/);
});
