import type { LibraryRecordingAvailability } from '@daydream/shared';

export function formatDuration(durationMs: number | null): string {
  if (durationMs === null) {
    return '--:--';
  }
  const totalSeconds = Math.max(0, Math.round(durationMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${padTime(minutes)}:${padTime(seconds)}`;
  }

  return `${minutes}:${padTime(seconds)}`;
}

export function formatTimestamp(timestampMs: number | null): string {
  if (timestampMs === null) {
    return 'Not started';
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'medium',
  }).format(new Date(timestampMs));
}

export function formatFileSize(sizeBytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  let value = Math.max(0, sizeBytes);
  let unitIndex = 0;
  while (value >= 1000 && unitIndex < units.length - 1) {
    value /= 1000;
    unitIndex += 1;
  }

  if (unitIndex === 0) {
    return `${value} ${units[unitIndex]}`;
  }

  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${
    units[unitIndex]
  }`;
}

export function availabilityLabel(
  availability: LibraryRecordingAvailability,
): string {
  switch (availability) {
    case 'available':
      return 'Ready';
    case 'incomplete':
      return 'Incomplete';
    case 'locked':
      return 'In progress';
    case 'missing-files':
      return 'Missing files';
  }
}

function padTime(value: number): string {
  return value.toString().padStart(2, '0');
}
