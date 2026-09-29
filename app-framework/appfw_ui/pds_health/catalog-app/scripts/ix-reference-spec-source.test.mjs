import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCanonicalIxReferenceSpec,
  canonicalIxReferenceSpecPath
} from "./ix-reference-spec-source.mjs";

test("canonical IX reference source is required", () => {
  assert.doesNotThrow(() => assertCanonicalIxReferenceSpec(
    canonicalIxReferenceSpecPath,
    "# Canonical IX reference\nSelf-contained obligations."
  ));
  assert.throws(
    () => assertCanonicalIxReferenceSpec(canonicalIxReferenceSpecPath, undefined),
    /canonical source is missing/
  );
  assert.throws(
    () => assertCanonicalIxReferenceSpec(
      "docs/specs/nexus-pds-design-system-foundation-r1.md",
      "historical"
    ),
    /requires canonical source/
  );
});

test("canonical IX reference source may not reintroduce historical document paths", () => {
  for (const historicalPath of [
    "docs/specs/nexus-pds-design-system-foundation-r1.md",
    "docs/specs/nexus-ix-lifecycle-adapter-r1.md"
  ]) {
    assert.throws(
      () => assertCanonicalIxReferenceSpec(
        canonicalIxReferenceSpecPath,
        `See ${historicalPath}`
      ),
      /reintroduced historical path/
    );
  }
});
