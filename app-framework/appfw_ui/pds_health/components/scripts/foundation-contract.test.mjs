import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = await readFile(
  resolve(componentRoot, "src/foundation.tsx"),
  "utf8"
);
const styles = await readFile(resolve(componentRoot, "src/styles.css"), "utf8");

test("foundation APIs remain additive and product-neutral", () => {
  for (const exportName of [
    "IconSlot",
    "NavigationItem",
    "Avatar",
    "IdentitySummary",
    "AppearanceProvider",
    "useAppearance"
  ]) {
    assert.match(
      source,
      new RegExp(`export function ${exportName}\\(`),
      `missing ${exportName} export`
    );
  }

  for (const propsName of [
    "IconSlotProps",
    "NavigationItemProps",
    "AvatarProps",
    "IdentitySummaryProps",
    "AppearanceProviderProps"
  ]) {
    assert.match(
      source,
      new RegExp(`export type ${propsName} =`),
      `missing ${propsName} contract`
    );
  }

  assert.doesNotMatch(
    source,
    /Nexus|CRM|patient|request|workflow/i,
    "shared foundation components must not contain product copy"
  );
});

test("navigation items use native link and button semantics with consistent state", () => {
  assert.match(source, /<a[\s\S]*?href=\{href\}/);
  assert.match(source, /<button[\s\S]*?type=\{type \?\? "button"\}/);
  assert.doesNotMatch(source, /role=["'](?:link|button)["']/);
  assert.match(source, /aria-current=\{current \? "page" : undefined\}/);
  assert.match(source, /aria-disabled=\{disabled \|\| undefined\}/);
  assert.match(source, /disabled=\{disabled\}/);
  assert.match(source, /if \(disabled\) \{\s*event\.preventDefault\(\)/);
  assert.match(source, /labelBehavior\?: "wrap" \| "truncate"/);
  assert.match(source, /labelBehavior = "wrap"/);
  assert.match(source, /overflowTooltip\?: ReactNode \| false/);
  assert.match(source, /<OverflowTooltip/);
  assert.match(source, /overflowTargetRef=\{labelRef\}/);

  assert.match(
    styles,
    /\.pds-navigation-item\s*\{[\s\S]*?grid-template-columns:\s*var\(--pds-space-6\) minmax\(0, 1fr\) auto;/
  );
  assert.match(
    styles,
    /\.pds-navigation-item__icon\s*\{[\s\S]*?width:\s*var\(--pds-space-6\);/
  );
  assert.match(
    styles,
    /\.pds-navigation-item\[data-label-behavior="wrap"\][\s\S]*?-webkit-line-clamp:\s*2;/
  );
  assert.match(
    styles,
    /\.pds-navigation-item\[data-label-behavior="truncate"\][\s\S]*?text-overflow:\s*ellipsis;/
  );
  assert.match(styles, /font-size:\s*var\(--pds-font-navigation-label-size\);/);
});

test("icon and identity primitives provide deterministic sizing and accessible names", () => {
  assert.match(source, /className=\{composeClassNames\("pds-icon-slot"/);
  assert.match(source, /role=\{label \? "img" : undefined\}/);
  assert.match(source, /aria-hidden=\{label \? undefined : true\}/);
  assert.match(source, /role=\{decorative \? undefined : "img"\}/);
  assert.match(source, /aria-label=\{decorative \? undefined : name\}/);
  assert.match(source, /initialsForName\(name\)/);
  assert.match(styles, /\.pds-icon-slot\[data-size="sm"\]/);
  assert.match(styles, /\.pds-icon-slot\[data-size="lg"\]/);
  assert.match(styles, /\.pds-avatar\[data-size="sm"\]/);
  assert.match(styles, /\.pds-avatar\[data-size="lg"\]/);
});

test("appearance provider keeps visual and color axes independent", () => {
  assert.match(source, /export type AppearanceColorMode = "system" \| "light" \| "dark"/);
  assert.match(
    source,
    /export type AppearanceVisualTheme = "apple-like" \| "material-like"/
  );
  assert.match(source, /prefers-color-scheme: dark/);
  assert.match(source, /target\.setAttribute\("data-theme", resolvedColorMode\)/);
  assert.match(source, /target\.setAttribute\("data-visual-theme", visualTheme\)/);
  assert.match(source, /const shouldPersist = persist \?\? Boolean\(storageKey\)/);
  assert.match(source, /function writeStoredAppearance/);
  assert.match(source, /window\.localStorage\.setItem/);
  assert.match(source, /Keep the active session usable when storage is unavailable or blocked/);
  assert.match(source, /useAppearance must be used within an AppearanceProvider/);
});

test("segmented controls honor reduced-motion preferences", () => {
  assert.match(styles, /@media \(prefers-reduced-motion: reduce\)/);
  assert.match(
    styles,
    /\.pds-segmented-control__option \{\n\s+transition: none;/
  );
});

test("new foundation styles use PDS tokens", () => {
  const foundationStyles = styles.slice(
    styles.indexOf("/* Shell behavior and application identity */")
  );
  assert.ok(foundationStyles.length > 0, "missing foundation style block");
  assert.doesNotMatch(
    foundationStyles,
    /#[0-9a-f]{3,8}\b/i,
    "foundation styles must not introduce raw color values"
  );
  assert.match(foundationStyles, /var\(--pds-color-/);
  assert.match(foundationStyles, /var\(--pds-space-/);
  assert.match(foundationStyles, /var\(--pds-motion-/);
});
