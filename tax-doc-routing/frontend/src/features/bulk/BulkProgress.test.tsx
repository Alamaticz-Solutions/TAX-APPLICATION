import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BulkProgress } from './BulkProgress';

describe('BulkProgress', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('says what is happening and how far it has got', () => {
    render(<BulkProgress title="Processing documents, please wait…" step="Checking the clients' folders" done={37} total={200} startedAt={Date.now()} note="Nothing is created yet." />);
    expect(screen.getByText('Processing documents, please wait…')).toBeTruthy();
    expect(screen.getByText(/Checking the clients' folders\. Running for 0 s\./)).toBeTruthy();
    expect(screen.getByText('37 of 200')).toBeTruthy();
    expect(screen.getByText('Nothing is created yet.')).toBeTruthy();
  });

  it('shows no bar while the amount of work is not known yet', () => {
    render(<BulkProgress title="Creating requests, please wait…" step="Reading the files" done={0} total={0} startedAt={Date.now()} />);
    expect(screen.queryByText(/ of /)).toBeNull();
  });

  it('keeps counting, and says so plainly when it is taking longer than usual', () => {
    render(<BulkProgress title="Creating requests, please wait…" step="3 of 100 requests created" done={3} total={100} startedAt={Date.now()} />);
    expect(screen.queryByText(/taking longer than usual/)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(25_000);
    });
    expect(screen.getByText(/Running for 25 s\./)).toBeTruthy();
    expect(screen.getByText(/taking longer than usual/)).toBeTruthy();
  });

  it('never shows more done than the total', () => {
    render(<BulkProgress title="Creating requests, please wait…" step="" done={120} total={100} startedAt={Date.now()} />);
    expect(screen.getByText('100 of 100')).toBeTruthy();
  });
});
