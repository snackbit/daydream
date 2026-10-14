export interface DriftCorrection {
  rate: number;
  seek: boolean;
  majorSinceMs: number | null;
}
export function driftCorrection(
  driftSeconds: number,
  nowMs: number,
  majorSinceMs: number | null,
): DriftCorrection {
  const since = Math.abs(driftSeconds) > 1 ? (majorSinceMs ?? nowMs) : null;
  const seek = since !== null && nowMs - since >= 2000;
  return {
    rate:
      seek || Math.abs(driftSeconds) < 0.04
        ? 1
        : Math.max(0.97, Math.min(1.03, 1 + driftSeconds * 0.1)),
    seek,
    majorSinceMs: seek ? null : since,
  };
}
