import type { MouseEvent } from 'react';

/** Ctrl/Cmd/Shift/middle-click open a link in a new tab; those must not be hijacked by client routing. */
export function isModifiedClick(e: MouseEvent): boolean {
  return e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button !== 0;
}
