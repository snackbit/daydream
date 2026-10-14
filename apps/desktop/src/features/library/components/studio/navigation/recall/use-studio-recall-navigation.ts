import { useEffect, useRef } from 'react';
import type { RecallEvidence } from '@daydream/shared';
import type { StudioVisualFrame } from '../../observations/studio-visual-observations';
import type { StudioVideoSources } from '../../playback/sources/use-studio-video-sources';
import { useStudioTimelineStore } from '../../timeline/state/studio-timeline-store';

export interface StudioRecallNavigationInput {
  evidence: RecallEvidence | null;
  playbackReady: boolean;
  seek: (timestamp: number, options: { resume: boolean }) => void;
  sources: Pick<StudioVideoSources, 'selectedTrackId' | 'select'>;
  visualFrames: StudioVisualFrame[];
}

/** Select the canonical source before seeking its media, exactly once per evidence revision. */
export function useStudioRecallNavigation({
  evidence,
  playbackReady,
  seek,
  sources,
  visualFrames,
}: StudioRecallNavigationInput) {
  const applied = useRef<string | null>(null);
  useEffect(() => {
    if (!evidence || !playbackReady) return;
    const key = `${evidence.evidenceId}:${evidence.contentRevision}`;
    if (applied.current === key) return;
    if (
      evidence.sourceKind === 'ocr' &&
      sources.selectedTrackId !== evidence.sourceTrackId
    ) {
      sources.select(evidence.sourceTrackId);
      return;
    }
    applied.current = key;
    seek(evidence.startOffsetMs, { resume: false });
    const store = useStudioTimelineStore.getState();
    const frame = visualFrames.find(
      (candidate) =>
        candidate.segmentId === evidence.segmentId &&
        candidate.sourceTrackId === evidence.sourceTrackId &&
        candidate.frameTimestampMs === evidence.startOffsetMs &&
        (evidence.documentId === null ||
          candidate.ocrDocumentId === evidence.documentId),
    );
    store.setSelection({
      ...store.selection,
      laneIds: [evidence.sourceTrackId],
      clipIds: [],
      redactionIds: [],
      ranges: [],
      visualFrameIds: frame ? [frame.id] : [],
    });
  }, [evidence, playbackReady, seek, sources, visualFrames]);
}
