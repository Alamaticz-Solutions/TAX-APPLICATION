import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WorkingSpinner } from './WorkingSpinner';

describe('WorkingSpinner', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('keeps turning on a timer, not on a CSS animation that reduced motion would switch off', () => {
    render(<WorkingSpinner />);
    const turn = () => screen.getByTestId('working-spinner').querySelector('g')?.getAttribute('transform');
    const first = turn();
    act(() => {
      vi.advanceTimersByTime(300);
    });
    const later = turn();
    expect(first).toBe('rotate(0 12 12)');
    expect(later).not.toBe(first);
    act(() => {
      vi.advanceTimersByTime(90 * 12);
    });
    expect(turn()).toBe(later);
  });

  it('is hidden from screen readers (the words beside it carry the meaning)', () => {
    render(<WorkingSpinner />);
    expect(screen.getByTestId('working-spinner').getAttribute('aria-hidden')).toBe('true');
  });
});
