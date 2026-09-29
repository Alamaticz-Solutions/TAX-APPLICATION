import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

import {
  assertPdsIxPresentation,
  pdsIxPresentationSchema,
  validatePdsIxPresentation
} from "../src/index.js";

const fixtureUrl = new URL("../fixtures/working-brief.presentation.json", import.meta.url);
const schemaUrl = new URL("../schema/pds.ix.presentation.v1.schema.json", import.meta.url);

async function readJson(url) {
  return JSON.parse(await readFile(url, "utf8"));
}

function withValueAtPath(value, path, replacement) {
  const copy = JSON.parse(JSON.stringify(value));
  let cursor = copy;
  for (const segment of path.slice(0, -1)) cursor = cursor[segment];
  cursor[path.at(-1)] = replacement;
  return copy;
}

function codePointString(pattern, length) {
  const points = Array.from(pattern);
  return Array.from({ length }, (_, index) => points[index % points.length]).join("");
}

function displayPath(path) {
  return path.map((segment) => typeof segment === "number" ? `[${segment}]` : `.${segment}`).join("");
}

test("golden fixture validates and round-trips without losing portable semantics", async () => {
  const fixture = await readJson(fixtureUrl);
  const validated = assertPdsIxPresentation(fixture);
  assert.equal(validated.identity.schemaVersion, pdsIxPresentationSchema);
  assert.equal(validated.context.gaps[0], "Approval owner is not yet resolved.");
  assert.equal(validated.context.evidence.items[0].sourceRef, "fixture-source-a");
  assert.equal(validated.response.regions[0].status, "ready");
  assert.equal(validated.announcement, "Working brief revision 1 is ready.");
  assert.deepEqual(JSON.parse(JSON.stringify(validated)), fixture);
});

test("runtime validator matches JSON Schema for valid and adversarial transport", async () => {
  const fixture = await readJson(fixtureUrl);
  const schema = await readJson(schemaUrl);
  const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const candidates = [
    fixture,
    { ...fixture, context: undefined, workStatus: undefined },
    { ...fixture, runId: "not-presentation-authority" },
    { ...fixture, identity: { ...fixture.identity, revision: -1 } },
    { ...fixture, response: { ...fixture.response, regions: [{ ...fixture.response.regions[0], status: "completed" }] } },
    { ...fixture, context: { ...fixture.context, gaps: [""] } },
    { ...fixture, response: { ...fixture.response, announcement: 42 } },
    { ...fixture, identity: { ...fixture.identity, revision: Number.MAX_SAFE_INTEGER + 1 } },
    { ...fixture, identity: { ...fixture.identity, presentationId: "x".repeat(129) } },
    { ...fixture, context: { ...fixture.context, gaps: Array.from({ length: 33 }, (_, index) => `gap-${index}`) } },
    { ...fixture, response: { ...fixture.response, regions: Array.from({ length: 65 }, () => fixture.response.regions[0]) } },
    { ...fixture, announcement: "x".repeat(4097) },
    { ...fixture, response: { ...fixture.response, regions: [{ ...fixture.response.regions[0], unknown: true }] } }
  ];
  for (const candidate of candidates) {
    const transport = JSON.parse(JSON.stringify(candidate));
    assert.equal(
      validatePdsIxPresentation(transport).ok,
      validateSchema(transport),
      JSON.stringify({ candidate: transport, schemaErrors: validateSchema.errors })
    );
  }
});

test("runtime rejects duplicate response region identifiers before rendering", async () => {
  const fixture = await readJson(fixtureUrl);
  const duplicated = {
    ...fixture,
    response: {
      ...fixture.response,
      regions: [
        fixture.response.regions[0],
        {
          ...fixture.response.regions[0],
          title: "Different content with the same identity"
        }
      ]
    }
  };
  const validation = validatePdsIxPresentation(duplicated);
  assert.equal(validation.ok, false);
  assert.match(validation.errors.join("\n"), /id must be unique within the response/);
  assert.throws(() => assertPdsIxPresentation(duplicated), /id must be unique/);
});

test("runtime and JSON Schema use identical Unicode code-point bounds at every string site", async () => {
  const fixture = await readJson(fixtureUrl);
  const schema = await readJson(schemaUrl);
  const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const identifierPaths = [
    ["identity", "presentationId"],
    ["context", "evidence", "items", 0, "id"],
    ["context", "evidence", "items", 0, "sourceRef"],
    ["response", "regions", 0, "id"],
    ["response", "regions", 0, "evidence", "items", 0, "id"],
    ["response", "regions", 0, "evidence", "items", 0, "sourceRef"]
  ];
  const nonEmptyStringPaths = [
    ["announcement"],
    ["context", "eyebrow"],
    ["context", "title"],
    ["context", "detail"],
    ["context", "announcement"],
    ["context", "gaps", 0],
    ["context", "evidence", "summary"],
    ["context", "evidence", "items", 0, "label"],
    ["context", "evidence", "items", 0, "value"],
    ["context", "metaLabel"],
    ["context", "actionLabel"],
    ["workStatus", "label"],
    ["workStatus", "detail"],
    ["workStatus", "actionLabel"],
    ["response", "eyebrow"],
    ["response", "title"],
    ["response", "metaLabel"],
    ["response", "announcement"],
    ["response", "regions", 0, "label"],
    ["response", "regions", 0, "title"],
    ["response", "regions", 0, "body"],
    ["response", "regions", 0, "whyItMatters"],
    ["response", "regions", 0, "evidence", "summary"],
    ["response", "regions", 0, "evidence", "items", 0, "label"],
    ["response", "regions", 0, "evidence", "items", 0, "value"],
    ["response", "regions", 0, "actionLabel"],
    ["response", "emptyState"],
    ["response", "footerText"]
  ];
  const patterns = [
    ["ASCII", "x"],
    ["BMP", "界"],
    ["astral", "😀"],
    ["combining", "e\u0301"],
    ["zero-width joiner", "👩‍💻"]
  ];

  function assertParity(path, replacement, expected, label) {
    const candidate = withValueAtPath(fixture, path, replacement);
    const runtimeResult = validatePdsIxPresentation(candidate).ok;
    const schemaResult = validateSchema(candidate);
    assert.equal(schemaResult, expected, `${label} Schema result at $${displayPath(path)}: ${JSON.stringify(validateSchema.errors)}`);
    assert.equal(runtimeResult, expected, `${label} runtime result at $${displayPath(path)}`);
  }

  for (const path of identifierPaths) {
    assertParity(path, "x", true, "one-code-point identifier");
    assertParity(path, "", false, "empty identifier");
    for (const [label, pattern] of patterns) {
      assertParity(path, codePointString(pattern, 128), true, `${label} 128-code-point identifier`);
      assertParity(path, codePointString(pattern, 129), false, `${label} 129-code-point identifier`);
    }
    assertParity(path, "😀".repeat(65), true, "65-emoji identifier regression");
  }

  for (const path of nonEmptyStringPaths) {
    assertParity(path, "x", true, "one-code-point string");
    assertParity(path, "", false, "empty string");
    for (const [label, pattern] of patterns) {
      assertParity(path, codePointString(pattern, 4096), true, `${label} 4096-code-point string`);
      assertParity(path, codePointString(pattern, 4097), false, `${label} 4097-code-point string`);
    }
    assertParity(path, "😀".repeat(2049), true, "2049-emoji string regression");
  }
});

test("runtime rejects huge malformed strings without allocating a code-point copy", async () => {
  const fixture = await readJson(fixtureUrl);
  const implementation = await readFile(new URL("../src/index.js", import.meta.url), "utf8");
  assert.doesNotMatch(implementation, /Array\.from\s*\(/);
  assert.match(implementation, /for \(const _codePoint of value\)[\s\S]*if \(length > maximum\) return false/);
  const hugeAnnouncement = "😀".repeat(1_000_000);
  const result = validatePdsIxPresentation({ ...fixture, announcement: hugeAnnouncement });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /\$\.announcement must be a string between 1 and 4096 characters/);
});

test("JSON Schema is closed at every object boundary represented by the runtime validator", async () => {
  const schema = await readJson(schemaUrl);
  assert.equal(schema.additionalProperties, false);
  for (const name of ["identity", "evidenceItem", "evidence", "context", "workStatus", "region", "response"]) {
    assert.equal(schema.$defs[name].additionalProperties, false, name);
  }
  assert.equal(schema.properties.identity.$ref, "#/$defs/identity");
  assert.equal(schema.$defs.identity.properties.schemaVersion.const, pdsIxPresentationSchema);
  assert.equal(schema.$defs.nonEmptyString.maxLength, 4096);
  assert.equal(schema.$defs.identifier.maxLength, 128);
  assert.equal(schema.$defs.identity.properties.revision.maximum, Number.MAX_SAFE_INTEGER);
  assert.equal(schema.$defs.context.properties.gaps.maxItems, 32);
  assert.equal(schema.$defs.evidence.properties.items.maxItems, 64);
  assert.equal(schema.$defs.response.properties.regions.maxItems, 64);
  assert.equal(schema.$defs.response.properties.regions.uniqueItems, true);
});

test("contract implementation and declarations are React, DOM, provider, and product free", async () => {
  const paths = [new URL("../src/index.js", import.meta.url), new URL("../src/index.d.ts", import.meta.url)];
  for (const path of paths) {
    const source = await readFile(path, "utf8");
    assert.doesNotMatch(source, /\b(?:react|document|window|servicenow|workday|nexus)\b/i);
  }
});
