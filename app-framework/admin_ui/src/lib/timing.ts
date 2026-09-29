export function formatResponseTime(responseMs: number) {
  if (!Number.isFinite(responseMs)) return "-";
  if (responseMs < 1) return "<1 ms";
  if (responseMs < 1000) return `${Math.round(responseMs)} ms`;
  return `${(responseMs / 1000).toFixed(2)} s`;
}
