import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

test("compiled package has public contract and design-data boundaries only", async () => {
  for (const file of ["dist/index.js", "dist/index.d.ts", "dist/design-data.js", "dist/design-data.d.ts"]) {
    const content = await readFile(new URL(file, root), "utf8");
    assert.doesNotMatch(content, /\.\.\/(?:tokens|ix-presentation-contract)|\/src\//, file);
    assert.doesNotMatch(content, /\b(?:expo|servicenow|workday|kafka|okta)\b/i, file);
  }
  const compiled = await readFile(new URL("dist/index.js", root), "utf8");
  assert.match(compiled, /from "react-native"/);
  assert.match(compiled, /from "@appfw\/pds-ix-presentation-contract"/);
  const declarations = await readFile(new URL("dist/index.d.ts", root), "utf8");
  assert.match(declarations, /from "\.\/design-data\.js"/);
  const designDeclarations = await readFile(new URL("dist/design-data.d.ts", root), "utf8");
  assert.match(designDeclarations, /from "\.\/generated\/pdsNativeDesignData\.js"/);
  const designData = await readFile(new URL("dist/design-data.js", root), "utf8");
  assert.match(designData, /pds\.native\.design-data@1/);
});
