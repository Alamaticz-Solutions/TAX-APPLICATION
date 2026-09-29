import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = new URL("./", import.meta.url);

function luminance(hex) {
  const channels = hex.slice(1).match(/.{2}/g).map((part) => Number.parseInt(part, 16) / 255);
  const linear = channels.map((value) => value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(a, b) {
  const [lighter, darker] = [luminance(a), luminance(b)].sort((left, right) => right - left);
  return (lighter + 0.05) / (darker + 0.05);
}

test("native design data is a deterministic projection of the canonical token source", async () => {
  const output = execFileSync(process.execPath, [
    fileURLToPath(new URL("generate-pds-native-design-data.mjs", root)),
    "--check",
    "--json"
  ], { encoding: "utf8" });
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  const projection = JSON.parse(await readFile(new URL("pdsNativeDesignData.json", root), "utf8"));
  const sourceBytes = await readFile(new URL("tokens.dtcg.json", root));
  assert.equal(projection.source.sha256, `sha256:${createHash("sha256").update(sourceBytes).digest("hex")}`);
  assert.equal(projection.schemaVersion, "pds.native.design-data@1");
  assert.equal(projection.visualThemes["apple-like"].light.color.action, "#0077a8");
  assert.equal(projection.sources["color.action"].light, "--pds-color-brand-blue-deep");
  assert.equal(projection.sources["color.action"].dark, "--pds-color-brand-blue-bright");
  assert.equal(projection.visualThemes["apple-like"].light.spacing["20"], 20);
  assert.equal(projection.visualThemes["apple-like"].light.shape.panel, 14);
  assert.notEqual(
    projection.visualThemes["apple-like"].light.color.text,
    projection.visualThemes["apple-like"].dark.color.text
  );
  assert.deepEqual(Object.keys(projection.visualThemes), ["apple-like"]);
  assert.deepEqual(projection.defaultSelection, { visualTheme: "apple-like", colorScheme: "light" });
  assert.deepEqual(projection.platforms["native-ios"], {
    visualTheme: "apple-like",
    qualification: "not-qualified"
  });
  assert.deepEqual(projection.platforms["native-android"], {
    visualTheme: null,
    qualification: "not-qualified"
  });
});

test("projection digest binds all source-derived design data", async () => {
  const projection = JSON.parse(await readFile(new URL("pdsNativeDesignData.json", root), "utf8"));
  const { projectionSha256, ...withoutDigest } = projection;
  const actual = `sha256:${createHash("sha256").update(`${JSON.stringify(withoutDigest)}\n`).digest("hex")}`;
  assert.equal(projectionSha256, actual);
});

test("light and dark projections have exact semantic parity and contrast-safe actions", async () => {
  const projection = JSON.parse(await readFile(new URL("pdsNativeDesignData.json", root), "utf8"));
  const light = projection.visualThemes["apple-like"].light;
  const dark = projection.visualThemes["apple-like"].dark;
  assert.deepEqual(Object.keys(light), Object.keys(dark));
  for (const key of Object.keys(light)) {
    assert.deepEqual(Object.keys(light[key]), Object.keys(dark[key]), key);
  }
  assert.ok(contrast(light.color.action, light.color.onAction) >= light.fontPolicy.minimumContrastRatio);
  assert.ok(contrast(dark.color.action, dark.color.onAction) >= dark.fontPolicy.minimumContrastRatio);
  assert.ok(contrast(light.color.text, light.color.canvas) >= light.fontPolicy.minimumContrastRatio);
  assert.ok(contrast(dark.color.text, dark.color.canvas) >= dark.fontPolicy.minimumContrastRatio);
});

test("projection is numeric, accessible, reduced-motion aware, and native-safe", async () => {
  const text = await readFile(new URL("pdsNativeDesignData.json", root), "utf8");
  const projection = JSON.parse(text);
  for (const visualTheme of Object.values(projection.visualThemes)) {
    for (const theme of Object.values(visualTheme)) {
    assert.deepEqual(Object.keys(theme.spacing), ["4", "8", "12", "16", "20", "24", "32"]);
    assert.ok(Object.values(theme.spacing).every(Number.isFinite));
    assert.ok(Object.values(theme.shape).every(Number.isFinite));
    for (const role of Object.values(theme.typography)) {
      assert.ok([role.fontSize, role.lineHeight, role.fontWeight].every(Number.isFinite));
    }
    assert.ok(theme.geometry.minimumTarget >= 44);
    assert.ok(theme.geometry.comfortableTarget >= theme.geometry.minimumTarget);
    assert.equal(theme.motion.reduced.enabled, false);
    assert.deepEqual(
      [theme.motion.reduced.fastDurationMs, theme.motion.reduced.standardDurationMs, theme.motion.reduced.slowDurationMs],
      [0, 0, 0]
    );
    assert.equal(theme.fontPolicy.allowFontScaling, true);
    assert.equal("maximumFontScale" in theme.fontPolicy, false);
    assert.ok(["light", "dark"].includes(theme.identity.colorScheme));
    assert.equal(theme.identity.visualTheme, "apple-like");
    }
  }
  assert.doesNotMatch(text, /var\(|light-dark\(|\bpx\b|\b(?:css|dom|window|document)\b/i);
});
