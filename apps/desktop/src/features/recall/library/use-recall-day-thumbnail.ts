import { useEffect, useState } from 'react';
import type { RecallDayCard } from '@daydream/shared';
import { prepareLibraryRecordingThumbnail } from '@/features/library/library-client';

// Uncached previews launch FFmpeg. Match the existing Library's single request budget.
let pendingPreparation: Promise<void> = Promise.resolve();

/** The native thumbnail adapter applies current source protection and redactions. */
export function useRecallDayThumbnail(day: RecallDayCard): {
  path: string | null;
  state: 'ready' | 'loading' | 'hidden' | 'unavailable';
} {
  const dayId = day.dayId;
  const privacyRevision = day.privacyRevision;
  const [preview, setPreview] = useState<{
    dayId: string;
    recordingId: string;
    privacyRevision: number;
    path: string | null;
  } | null>(null);
  const recordingId = day.thumbnailRecordingId;
  const cachedPath = day.thumbnailPath;
  useEffect(() => {
    if (cachedPath !== null || recordingId === null) return;
    const candidateId = recordingId;
    let active = true;
    async function prepare() {
      if (!active) return;
      try {
        const thumbnail = await prepareLibraryRecordingThumbnail({
          recordingId: candidateId,
        });
        if (active)
          setPreview({
            dayId,
            recordingId: candidateId,
            privacyRevision,
            path:
              thumbnail.error === null && thumbnail.recordingId === candidateId
                ? thumbnail.path
                : null,
          });
      } catch {
        if (active)
          setPreview({
            dayId,
            recordingId: candidateId,
            privacyRevision,
            path: null,
          });
      }
    }
    pendingPreparation = pendingPreparation.then(prepare);
    return () => {
      active = false;
    };
  }, [recordingId, cachedPath, dayId, privacyRevision]);
  const path =
    cachedPath ??
    (preview?.dayId === dayId &&
    preview.recordingId === recordingId &&
    preview.privacyRevision === privacyRevision
      ? preview.path
      : null);
  return {
    path,
    state: path
      ? 'ready'
      : recordingId !== null &&
          !(
            preview?.dayId === dayId &&
            preview.recordingId === recordingId &&
            preview.privacyRevision === privacyRevision
          )
        ? 'loading'
        : day.redactionCount > 0 && recordingId === null
          ? 'hidden'
          : 'unavailable',
  };
}
