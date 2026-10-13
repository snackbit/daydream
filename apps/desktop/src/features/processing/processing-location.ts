export function validateProcessingLocation(search: Record<string, unknown>): {
  recordings?: string;
} {
  if (typeof search.recordings !== 'string') return {};
  const ids = search.recordings
    .split(',')
    .filter((id) => id.length > 0 && id.length <= 128);
  if (!ids.length || ids.length > 2000) return {};
  return { recordings: [...new Set(ids)].sort().join(',') };
}
