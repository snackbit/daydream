import { useCallback } from 'react';
import type { RecorderStatus, RecorderRecordingStats } from '@daydream/shared';
import { Pause, Play, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatStorageSize } from '@/features/settings/settings-format';

export interface RecorderSessionControlsProps {
  status: Pick<RecorderStatus, 'state' | 'finalization'> | null;
  stats: Pick<
    RecorderRecordingStats,
    'recordedDurationMs' | 'totalSizeBytes'
  > | null;
  locked: boolean;
  pause: () => Promise<unknown>;
  resume: () => Promise<unknown>;
  stop: () => Promise<unknown>;
  error?: string | null;
}
export function RecorderSessionControls({
  status,
  stats,
  locked,
  pause,
  resume,
  stop,
  error,
}: RecorderSessionControlsProps) {
  const handlePause = useCallback(() => void pause(), [pause]);
  const handleResume = useCallback(() => void resume(), [resume]);
  const handleStop = useCallback(() => void stop(), [stop]);
  if (
    status?.state !== 'recording' &&
    status?.state !== 'paused' &&
    !status?.finalization
  )
    return null;
  const seconds = Math.floor((stats?.recordedDurationMs ?? 0) / 1_000);
  const duration = `${Math.floor(seconds / 3_600)
    .toString()
    .padStart(
      2,
      '0',
    )}:${Math.floor(seconds / 60) % 60 < 10 ? '0' : ''}${Math.floor(seconds / 60) % 60}:${(seconds % 60).toString().padStart(2, '0')}`;
  return (
    <div
      aria-label="Global recorder controls"
      className="flex shrink-0 items-center gap-2"
    >
      <span aria-label="Recorded duration" className="tabular-nums">
        {stats?.recordedDurationMs === undefined ? '--:--:--' : duration}
      </span>
      <span aria-label="Recording size" className="tabular-nums">
        {stats ? formatStorageSize(stats.totalSizeBytes) : 'Size unavailable'}
      </span>
      {status.finalization ? (
        <span role="status">Saving recording…</span>
      ) : null}
      {status.state === 'paused' ? (
        <Button
          aria-label="Resume recording"
          disabled={locked}
          onClick={handleResume}
          size="icon-sm"
          variant="ghost"
        >
          <Play aria-hidden />
        </Button>
      ) : (
        <Button
          aria-label="Pause recording"
          disabled={locked}
          onClick={handlePause}
          size="icon-sm"
          variant="ghost"
        >
          <Pause aria-hidden />
        </Button>
      )}
      <Button
        aria-label="Stop recording"
        disabled={locked}
        onClick={handleStop}
        size="icon-sm"
        variant="ghost"
      >
        <Square aria-hidden />
      </Button>
      {error ? (
        <span
          role="alert"
          className="text-destructive max-w-64 truncate"
          title={error}
        >
          {error}
        </span>
      ) : null}
    </div>
  );
}
