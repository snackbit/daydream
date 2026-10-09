import type { RecallDayCard } from '@daydream/shared';

export const recallTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
export function localDayId(now = new Date()): string {
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
}
export function recallDateLabel(dayId: string): string {
  return new Date(`${dayId}T12:00:00`).toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
  });
}
export function recallTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

export const recallDayStatusLabel: Record<RecallDayCard['status'], string> = {
  empty: 'No evidence yet',
  pending: 'Processing',
  ready: 'Ready to review',
  stale: 'Update needed',
  failed: 'Needs attention',
  unconfigured: 'Summary setup incomplete',
  generating: 'Generating overview',
  partial: 'Partially processed',
  invalidated: 'Privacy update needed',
  insufficient: 'Not enough evidence',
};

export function recordedDuration(durationMs: number | null): string {
  if (durationMs === null) return 'Timing unavailable';
  const seconds = Math.round(durationMs / 1000);
  if (seconds < 60) return `${seconds} seconds`;
  const minutes = Math.floor(seconds / 60);
  return minutes < 60
    ? `${minutes} min`
    : `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}
