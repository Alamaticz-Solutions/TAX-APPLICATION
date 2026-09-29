import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * X-1: in Apple-like dark, the accent tone keeps --pds-color-brand-blue-deeper
 * on a dark fill (1.7:1, framework Finding Y). Until PDS fixes it, text-bearing
 * `Badge`s and `InlineAlert`s use `neutral`. `main.tsx` is the dev-only
 * scaffold and is excluded; KPI rules and chart series carry no text.
 */

const SRC = join(__dirname, '..');

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === 'generated' ? [] : files(path);
    return /\.tsx$/.test(name) && !/\.test\.tsx$/.test(name) ? [path] : [];
  });
}

describe('no accent tone on text-bearing badges and alerts (X-1)', () => {
  it('finds none outside the dev scaffold', () => {
    const offenders: string[] = [];
    for (const path of files(SRC)) {
      const rel = relative(SRC, path);
      if (rel === 'main.tsx') continue;
      const text = readFileSync(path, 'utf8');
      const pattern = /<(Badge|InlineAlert)\b[^>]*\btone=(?:"accent"|\{['"]accent['"]\})/g;
      for (const match of text.matchAll(pattern)) offenders.push(`${rel}: ${match[1]}`);
    }
    expect(offenders).toEqual([]);
  });

  it('keeps status tone maps free of accent', () => {
    const statuses = readFileSync(join(SRC, 'features/shared/config/statuses.ts'), 'utf8');
    const fileStatuses = readFileSync(join(SRC, 'components/DocumentGrid.tsx'), 'utf8');
    expect(statuses).not.toMatch(/tone: 'accent'/);
    expect(fileStatuses).not.toMatch(/: 'accent'/);
  });
});
