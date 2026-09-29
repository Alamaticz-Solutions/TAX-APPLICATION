#!/usr/bin/env node

import assert from "node:assert/strict";

import {
  classifyDocsChangedSurfaceInput,
  docsChangedSurfaceRuleForPath,
  evaluatePathSafetyCommand,
} from "./product-increment-path-safety.mjs";

{
  const classifications = classifyDocsChangedSurfaceInput(
    [
      "",
      "docs/specs/product-increment-portfolio.json\r",
      "scripts/appfw\r",
      "docs/README.md",
      "   \r",
      "",
    ].join("\n"),
  );
  assert.deepEqual(classifications, [
    {
      path: "docs/specs/product-increment-portfolio.json",
      rule: "focused:product-increment-delivery",
    },
    {
      path: "scripts/appfw",
      rule: "product-increment-authority-contract",
    },
    { path: "docs/README.md", rule: null },
  ]);
}

assert.equal(
  docsChangedSurfaceRuleForPath(
    "docs/specs/example.product-increment.json",
  ),
  "focused:product-increment-delivery",
  "focused Product Increment routing must precede the broader sensitive rule",
);
assert.equal(docsChangedSurfaceRuleForPath("docs/README.md"), null);

{
  const result = evaluatePathSafetyCommand(
    ["--classify-docs-surface-batch"],
    "\r\ndocs/specs/example.product-increment.json\r\nscripts/appfw\r\ndocs/README.md\r\n",
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    result.stdout,
    [
      "docs/specs/example.product-increment.json|focused:product-increment-delivery",
      "scripts/appfw|product-increment-authority-contract",
      "docs/README.md|",
      "",
    ].join("\n"),
  );
}

{
  const result = evaluatePathSafetyCommand([
    "--classify-docs-subcheck",
    "docs/specs/example.product-increment.json",
  ]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, "product-increment-delivery\n");
}

{
  const result = evaluatePathSafetyCommand([
    "--classify-sensitive",
    "scripts/appfw",
  ]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, "product-increment-authority-contract\n");
}

for (const command of ["--classify-docs-subcheck", "--classify-sensitive"]) {
  const result = evaluatePathSafetyCommand([command, "docs/README.md"]);
  assert.equal(result.status, 1);
  assert.equal(result.stdout, "");
}

{
  const result = evaluatePathSafetyCommand([
    "--classify-docs-surface-batch",
    "unexpected-argument",
  ]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /Usage:/);
}

console.log("product-increment path-safety tests passed");
