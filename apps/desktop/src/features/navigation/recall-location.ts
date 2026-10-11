import type { SearchSchemaInput } from '@tanstack/react-router';
import type { RecallSourceKind } from '@daydream/shared';

export function validateRecallDate(value: unknown): string {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    throw new Error('Choose a valid calendar date.');
  }
  const parsed = new Date(`${value}T00:00:00Z`);
  if (
    !Number.isFinite(parsed.getTime()) ||
    parsed.toISOString().slice(0, 10) !== value
  ) {
    throw new Error('Choose a valid calendar date.');
  }
  return value;
}

function source(value: unknown): RecallSourceKind | null {
  if (value === undefined || value === null || value === '') return null;
  if (value === 'ocr' || value === 'desktop-audio' || value === 'microphone')
    return value;
  throw new Error('Choose a valid recall source.');
}

export function validateRecallSearch(
  input: SearchSchemaInput & { q?: unknown; source?: unknown; date?: unknown },
) {
  const q = input.q ?? '';
  if (typeof q !== 'string' || new TextEncoder().encode(q).length > 4096)
    throw new Error('Search query is invalid.');
  return {
    q,
    source: source(input.source),
    date:
      input.date == null || input.date === ''
        ? null
        : validateRecallDate(input.date),
  };
}

export function validateStudioSearch(
  input: SearchSchemaInput & {
    evidenceId?: unknown;
    source?: unknown;
    seek?: unknown;
  },
) {
  const evidenceId = input.evidenceId ?? null;
  if (
    evidenceId !== null &&
    (typeof evidenceId !== 'string' ||
      evidenceId.length === 0 ||
      evidenceId.length > 256)
  ) {
    throw new Error('Source link is invalid.');
  }
  const seek = input.seek ?? null;
  if (
    seek !== null &&
    (typeof seek !== 'number' || !Number.isSafeInteger(seek) || seek < 0)
  ) {
    throw new Error('Source position is invalid.');
  }
  return { evidenceId, source: source(input.source), seek };
}
