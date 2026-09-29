// Theme control for the catalog. PDS tokens are defined with the CSS
// `light-dark()` function and keyed off `color-scheme`, which the token sheet
// binds to `:root[data-theme="light"|"dark"]`. So switching the whole palette
// is just a matter of setting (or clearing) that attribute. "system" clears it
// and lets the OS preference drive the bare `:root { color-scheme: light dark }`.
export type ThemeMode = "system" | "light" | "dark";
export type VisualTheme = "apple-like" | "material-like";

const STORAGE_KEY = "pds-catalog-theme";
const VISUAL_THEME_STORAGE_KEY = "pds-catalog-visual-theme";
export const THEME_MODES: readonly ThemeMode[] = ["system", "light", "dark"];
export const VISUAL_THEMES: readonly VisualTheme[] = ["apple-like", "material-like"];

export function readStoredTheme(): ThemeMode {
  if (typeof localStorage === "undefined") return "system";
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" || stored === "system" ? stored : "system";
}

export function applyTheme(mode: ThemeMode): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (mode === "system") {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = mode;
  }
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, mode);
  }
}

export function readStoredVisualTheme(): VisualTheme {
  if (typeof localStorage === "undefined") return "apple-like";
  const stored = localStorage.getItem(VISUAL_THEME_STORAGE_KEY);
  return stored === "material-like" || stored === "apple-like" ? stored : "apple-like";
}

export function applyVisualTheme(theme: VisualTheme): void {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.visualTheme = theme;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(VISUAL_THEME_STORAGE_KEY, theme);
  }
}

export function resolvedScheme(mode: ThemeMode): "light" | "dark" {
  if (mode !== "system") return mode;
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
