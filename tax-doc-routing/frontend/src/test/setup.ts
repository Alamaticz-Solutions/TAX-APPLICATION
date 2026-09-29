import '@testing-library/jest-dom/vitest';
import { expect } from 'vitest';
import { toHaveNoViolations } from 'jest-axe';

expect.extend(toHaveNoViolations);

// Node 25+ ships its own experimental `localStorage`/`sessionStorage` globals,
// which shadow jsdom's and are `undefined` unless Node is started with
// `--localstorage-file`. Give tests a working in-memory Storage in that case,
// so the suite behaves the same on every supported Node (see package.json engines).
function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => Array.from(items.keys())[index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, String(value))
  };
}
for (const name of ['localStorage', 'sessionStorage'] as const) {
  let usable = false;
  try {
    usable = typeof window[name]?.getItem === 'function';
  } catch {
    usable = false;
  }
  if (!usable) Object.defineProperty(window, name, { value: memoryStorage(), configurable: true });
}

// jsdom has no real viewport, so `window.matchMedia` isn't implemented
// (SignInScreen's responsive breakpoint hook calls it on mount).
if (!window.matchMedia) {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false
    }) as MediaQueryList;
}
