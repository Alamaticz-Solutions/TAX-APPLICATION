import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * CV-12: product code imports PDS by family (`…/primitives`, `…/layout`), as
 * the PDS components README asks. `main.tsx` is the dev-only scaffold, and
 * `npm run appfw:check` requires it to keep the package-root import.
 */

const SRC = join(__dirname, '..');

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === 'generated' ? [] : files(path);
    return /\.tsx?$/.test(name) ? [path] : [];
  });
}

describe('PDS family imports (CV-12)', () => {
  it('imports no PDS component from the package root outside the scaffold', () => {
    const offenders = files(SRC)
      .filter((path) => relative(SRC, path) !== 'main.tsx')
      .filter((path) => /from ['"]@appfw\/pds-health-components['"]/.test(readFileSync(path, 'utf8')))
      .map((path) => relative(SRC, path));
    expect(offenders).toEqual([]);
  });
});
