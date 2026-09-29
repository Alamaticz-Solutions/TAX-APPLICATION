const DATE = new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
const TIME = new Intl.DateTimeFormat('en-US', { hour: 'numeric', minute: '2-digit' });

export const formatDate = (iso: string | null | undefined): string => (iso ? DATE.format(new Date(iso)) : '—');

export const formatDateTime = (iso: string | null | undefined): string =>
  iso ? `${DATE.format(new Date(iso)).replace(/, \d{4}$/, '')}, ${TIME.format(new Date(iso))}` : '—';

export const nowIso = (): string => new Date().toISOString().slice(0, 16);

export function formatBytes(bytes: number | undefined): string {
  if (bytes === undefined) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
