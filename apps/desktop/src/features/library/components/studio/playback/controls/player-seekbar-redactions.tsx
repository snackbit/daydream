import type { LibraryRecordingDetail } from '@daydream/shared';
import { formatDuration } from '../../../../library-format';

export interface PlayerSeekbarRedactionsProps {
  detail: Pick<LibraryRecordingDetail, 'redactions' | 'tracks'>;
  totalDurationMs: number;
}

export function PlayerSeekbarRedactions({
  detail,
  totalDurationMs,
}: PlayerSeekbarRedactionsProps) {
  const mediaTracks = detail.tracks;
  return (
    <>
      {detail.redactions.map((redaction) => {
        const affected = mediaTracks.filter((track) =>
          redaction.trackIds.includes(track.id),
        );
        if (affected.length === 0) return null;
        const video = affected.some((track) => track.kind === 'video');
        const audio = affected.some((track) => track.kind !== 'video');
        const label =
          video && audio ? 'video and audio' : video ? 'video' : 'audio';
        const start = Math.max(
          0,
          Math.min(totalDurationMs, redaction.startOffsetMs),
        );
        const end = Math.max(
          start,
          Math.min(totalDurationMs, redaction.endOffsetMs),
        );
        return (
          <span
            aria-label={`Redacted ${label}, ${formatDuration(start)} to ${formatDuration(end)}`}
            className="studio-redaction-pattern bg-muted border-muted-foreground/60 pointer-events-none absolute inset-y-0 z-10 border-x"
            data-playback-redaction={
              affected.length === mediaTracks.length ? 'full' : 'partial'
            }
            key={redaction.id}
            style={{
              left: `${(start / totalDurationMs) * 100}%`,
              width: `${((end - start) / totalDurationMs) * 100}%`,
            }}
            title={`Redacted ${label}`}
          />
        );
      })}
    </>
  );
}
