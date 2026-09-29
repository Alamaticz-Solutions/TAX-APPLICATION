import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * SK-4: "Required" is not an error. Coral is for blocked, failed or dangerous
 * marks, and dozens of danger badges on an unanswered form read as failure.
 * A required marker uses the neutral tone; danger is kept for answers still
 * missing after a submit attempt (the form's missing-answers summary, GV-2).
 */

const SRC = join(__dirname, '..');

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === 'generated' ? [] : files(path);
    return /\.tsx$/.test(name) && !/\.test\.tsx$/.test(name) ? [path] : [];
  });
}

describe('a required marker is not drawn as an error (SK-4)', () => {
  it('finds no danger-toned "Required" badge', () => {
    const offenders: string[] = [];
    for (const path of files(SRC)) {
      const text = readFileSync(path, 'utf8');
      if (/<Badge\b[^>]*\btone=(?:"danger"|\{['"]danger['"]\})[^>]*>\s*Required\s*<\/Badge>/.test(text)) offenders.push(relative(SRC, path));
    }
    expect(offenders).toEqual([]);
  });
});
