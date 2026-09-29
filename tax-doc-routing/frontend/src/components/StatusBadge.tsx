import { AlertTriangle, CheckCircle2, Circle, Clock, Loader2, XCircle, type LucideIcon } from 'lucide-react';
import { Badge } from '@appfw/pds-health-components/primitives';
import { STATUS_META, type StatusIcon } from '../features/shared/config/statuses';
import type { RecordStatus } from '../features/shared/types';

const ICONS: Record<StatusIcon, LucideIcon> = {
  circle: Circle,
  clock: Clock,
  loader: Loader2,
  alert: AlertTriangle,
  check: CheckCircle2,
  x: XCircle
};

/**
 * The one place record statuses are rendered. PDS Badge tone + an icon + the text, so status
 * never depends on colour alone.
 */
export function StatusBadge({ status }: { status: RecordStatus }) {
  const meta = STATUS_META[status];
  const Icon = ICONS[meta.icon];
  return (
    <Badge tone={meta.tone} className="tax-status-badge">
      <Icon size={13} aria-hidden="true" className={meta.icon === 'loader' ? 'tax-spin' : undefined} />
      {status}
    </Badge>
  );
}
