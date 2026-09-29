import type { RecordStatus, Tone } from '../types';

// Single source of truth for record statuses (business spec section 4 / dashboard statuses).
export const STATUS = {
  OPEN: 'Open',
  IN_PROGRESS: 'In Progress',
  PROCESSING: 'Processing',
  EXCEPTION: 'Exception',
  COMPLETED: 'Resolved - Completed',
  CANCELLED: 'Resolved - Cancelled'
} as const satisfies Record<string, RecordStatus>;

export type StatusIcon = 'circle' | 'clock' | 'loader' | 'alert' | 'check' | 'x';

// Tone drives the PDS Badge colour; an icon and the label are always rendered too, so status
// never relies on colour alone. `accent` is deliberately not used: text-bearing badges avoid it
// (X-1, low contrast in Apple-like dark), so Open/Processing/Cancelled are told apart by icon + label.
export const STATUS_META: Record<RecordStatus, { tone: Tone; icon: StatusIcon }> = {
  [STATUS.OPEN]: { tone: 'neutral', icon: 'circle' },
  [STATUS.IN_PROGRESS]: { tone: 'warning', icon: 'clock' },
  [STATUS.PROCESSING]: { tone: 'neutral', icon: 'loader' },
  [STATUS.EXCEPTION]: { tone: 'danger', icon: 'alert' },
  [STATUS.COMPLETED]: { tone: 'success', icon: 'check' },
  [STATUS.CANCELLED]: { tone: 'neutral', icon: 'x' }
};

export const ALL_STATUSES: RecordStatus[] = Object.values(STATUS);
export const TERMINAL_STATUSES: RecordStatus[] = [STATUS.COMPLETED, STATUS.CANCELLED];
export const CANCEL_RESOLUTIONS = ['Cancelled', 'Withdrawn'] as const;
