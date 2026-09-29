import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * X-2: signal colours are not decoration. An icon beside a heading or an
 * action inherits the text colour. Green on "Microsoft Teams meeting" read as
 * success. Chart series fills are not icons and are not covered here.
 *
 * X-7: text takes semantic ink (`--pds-color-state-*`, `--pds-color-text-*`).
 * A vivid signal fill is not assumed to be readable as text colour.
 */

const SRC = join(__dirname, '..');

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === 'generated' ? [] : files(path);
    return /\.tsx$/.test(name) && !/\.test\.tsx$/.test(name) ? [path] : [];
  });
}

function offenders(pattern: RegExp): string[] {
  const found: string[] = [];
  for (const path of files(SRC)) {
    const rel = relative(SRC, path);
    if (rel === 'main.tsx') continue;
    readFileSync(path, 'utf8')
      .split('\n')
      .forEach((line, i) => {
        if (pattern.test(line)) found.push(`${rel}:${i + 1}`);
      });
  }
  return found;
}

describe('signal colours are not decoration (X-2, X-7)', () => {
  it('paints no icon with a signal colour', () => {
    expect(offenders(/\bcolor="var\(--pds-color-signal-/)).toEqual([]);
  });

  it('colours no text with a signal fill', () => {
    // Same-line `color: …signal…`, including a ternary; `background-color` is a fill.
    expect(offenders(/(?<![-\w])color:[^;\n]*--pds-color-signal-/)).toEqual([]);
  });
});
