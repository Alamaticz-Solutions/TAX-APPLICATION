import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const layoutSource = await readFile(
  resolve(componentRoot, "src/layout.tsx"),
  "utf8"
);
const styles = await readFile(resolve(componentRoot, "src/styles.css"), "utf8");

test("AppShell exposes additive viewport and responsive modes", () => {
  const appShellSource = layoutSource.slice(
    layoutSource.indexOf("export type AppShellProps"),
    layoutSource.indexOf("export type BreadcrumbItem")
  );

  for (const property of [
    "navigationLabel?: string",
    "responsiveCollapse?: boolean",
    "viewportBounded?: boolean",
    "sidebarResizable?: boolean",
    "sidebarWidth?: number",
    "defaultSidebarWidth?: number",
    "sidebarMinWidth?: number",
    "sidebarMaxWidth?: number",
    "onSidebarWidthChange?: (width: number) => void",
    "onSidebarWidthCommit?: (width: number) => void"
  ]) {
    assert.ok(
      appShellSource.includes(property),
      `missing AppShell property: ${property}`
    );
  }

  assert.match(appShellSource, /data-responsive-collapse=/);
  assert.match(appShellSource, /data-viewport-bounded=/);
  assert.match(appShellSource, /data-sidebar-resizable=/);
  assert.match(appShellSource, /responsiveCollapse = false/);
  assert.match(appShellSource, /viewportBounded = false/);
  assert.match(appShellSource, /sidebarResizable = false/);
  assert.match(appShellSource, /defaultSidebarWidth = 288/);
  assert.match(appShellSource, /sidebarMinWidth = 240/);
  assert.match(appShellSource, /sidebarMaxWidth = 360/);
  assert.match(appShellSource, /role="separator"/);
  assert.match(appShellSource, /aria-valuemin=/);
  assert.match(appShellSource, /aria-valuemax=/);
  assert.match(appShellSource, /aria-valuenow=/);
  assert.match(appShellSource, /event\.key === "ArrowLeft"/);
  assert.match(appShellSource, /event\.key === "ArrowRight"/);
  assert.match(appShellSource, /event\.key === "Home"/);
  assert.match(appShellSource, /event\.key === "End"/);
});

test("viewport-bounded shell gives navigation and content independent scroll regions", () => {
  assert.match(
    styles,
    /\.pds-app-shell\[data-viewport-bounded="true"\]\s*\{[\s\S]*?height:\s*100dvh;[\s\S]*?overflow:\s*hidden;/
  );
  assert.match(
    styles,
    /\.pds-app-shell\[data-viewport-bounded="true"\] \.pds-app-shell__nav,[\s\S]*?\.pds-app-shell\[data-viewport-bounded="true"\] \.pds-app-shell__main\s*\{[\s\S]*?overflow:\s*auto;/
  );
  assert.match(styles, /overscroll-behavior:\s*contain;/);
  assert.match(styles, /scrollbar-gutter:\s*stable;/);
});

test("responsive shell swaps the sidebar for the supplied topbar at the established breakpoint", () => {
  assert.match(
    styles,
    /\.pds-app-shell\[data-responsive-collapse="true"\] \.pds-app-shell__topbar\s*\{\s*display:\s*none;/
  );
  const responsiveStyles = styles.slice(
    styles.lastIndexOf("@media (max-width: 960px)")
  );
  assert.match(
    responsiveStyles,
    /\.pds-app-shell\[data-responsive-collapse="true"\][\s\S]*?\.pds-app-shell__sidebar\s*\{\s*display:\s*none;/
  );
  assert.match(
    responsiveStyles,
    /\.pds-app-shell\[data-responsive-collapse="true"\][\s\S]*?\.pds-app-shell__topbar\s*\{\s*display:\s*flex;/
  );
  assert.match(
    responsiveStyles,
    /\.pds-app-shell__sidebar-resizer\s*\{\s*display:\s*none;/
  );
});
