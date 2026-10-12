import { useCallback, useEffect, useEffectEvent } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import { AudioCleanupDialog } from '../components/studio/audio-cleanup/audio-cleanup-dialog';
import { useAudioCleanup } from '../components/studio/audio-cleanup/use-audio-cleanup';
import { ReprocessingDialog } from '../components/studio/reprocessing/reprocessing-dialog';
import { useStudioReprocessing } from '../components/studio/reprocessing/use-studio-reprocessing';

export interface DayRecordingAudioCleanupProps {
  detail: LibraryRecordingDetail;
  requestRevision: number;
  readRange: () => { startOffsetMs: number; endOffsetMs: number };
  pausePlayback: () => void;
  refreshPlayback: () => void;
}
/** The day view edits one recording's audio, never its synthetic day ID. */
export function DayRecordingAudioCleanup({
  detail,
  requestRevision,
  readRange,
  pausePlayback,
  refreshPlayback,
}: DayRecordingAudioCleanupProps) {
  const cleanup = useAudioCleanup({
    detail,
    readRange,
    pausePlayback,
    refreshPlayback,
  });
  const reprocessing = useStudioReprocessing(detail.id);
  const open = useEffectEvent(() => cleanup.open());
  useEffect(() => {
    void open();
  }, [requestRevision]);
  const regenerate = useCallback(() => {
    reprocessing.open(
      detail.tracks
        .filter((track) => track.kind === 'microphone')
        .map((track) => ({ trackId: track.id, ranges: null })),
    );
    reprocessing.operations(['transcription']);
  }, [detail.tracks, reprocessing]);
  return (
    <>
      <AudioCleanupDialog
        controller={cleanup}
        onRegenerate={regenerate}
        scopeLabel={detail.title ?? 'Active recording'}
        unavailableReason={
          detail.status === 'recording' || detail.status === 'paused'
            ? 'Finish recording before removing speaker bleed.'
            : null
        }
      />
      <ReprocessingDialog controller={reprocessing} />
      {cleanup.state?.transcriptsOutdated ? (
        <p className="text-muted-foreground text-xs" role="status">
          {detail.title ?? 'Recording'} · Microphone transcripts use previous
          audio settings.
        </p>
      ) : null}
    </>
  );
}
