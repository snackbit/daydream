import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { LibraryPlaybackAssets } from '@daydream/shared';
import type { RecordingPlaybackController } from '../components/studio/use-recording-playback-controller';
import { useStudioEntryNavigation } from '../components/studio/use-studio-entry-navigation';
import { useStudioRecallEvidence } from '../components/studio/use-studio-recall-evidence';
import { useStudioSpacePlayback } from '../components/studio/use-studio-space-playback';
import type { StudioVideoSources } from '../components/studio/use-studio-video-sources';
import { adjacentLibraryFrame } from '../library-client';
import type { DayStudioLocation } from './day-studio';
import {
  dayPosition,
  sourcePosition,
  type DayTimelineModel,
} from './day-timeline';

interface DayNavigationInput {
  model: DayTimelineModel;
  location: DayStudioLocation;
  sessions: Map<string, LibraryPlaybackAssets>;
  sources: StudioVideoSources;
  playback: RecordingPlaybackController;
  activeId: string | null;
  evidenceDismissed: boolean;
  onError: (error: string) => void;
}
export function useDayNavigation({
  model,
  location,
  sessions,
  sources,
  playback,
  activeId,
  evidenceDismissed,
  onError,
}: DayNavigationInput) {
  const seekTimeline = playback.actions.seekTimeline;
  const seek = useCallback(
    (time: number) => seekTimeline(time, { resume: false }),
    [seekTimeline],
  );
  const resolveSource = useCallback(
    (trackId: string, time: number) => {
      const position = sourcePosition(model, time);
      if (!position) return undefined;
      const original = [
        ...(model.trackIds.get(position.span.recordingId) ?? []),
      ].find(([, id]) => id === trackId)?.[0];
      if (!original) return undefined;
      return {
        recordingId: position.span.recordingId,
        sourceTrackId: original,
        timestampMs: position.offsetMs,
        toTimeline: (time: number) =>
          dayPosition(model, position.span.recordingId, time),
      };
    },
    [model],
  );
  const generation = useRef(0),
    pending = useRef(false);
  useEffect(() => {
    generation.current++;
    return () => {
      generation.current++;
    };
  }, [model, playback.state.seekRevision, sources.selectedTrackId]);
  const step = useCallback(
    async (direction: -1 | 1) => {
      if (pending.current) return;
      const source = resolveSource(
        playback.state.currentSegment?.videoTrackId ??
          sources.selectedTrackId ??
          '',
        playback.state.displayedTimelineMs,
      );
      if (!source) return;
      const request = generation.current;
      pending.current = true;
      try {
        const time = await adjacentLibraryFrame(
          source.recordingId,
          source.timestampMs,
          direction,
          source.sourceTrackId,
        );
        if (request === generation.current && time !== null) {
          const target = source.toTimeline(time);
          if (target !== null) seek(target);
        }
      } catch {
        if (request === generation.current)
          onError('The next video frame could not be read. Try again.');
      } finally {
        pending.current = false;
      }
    },
    [
      resolveSource,
      playback.state.currentSegment,
      playback.state.displayedTimelineMs,
      sources.selectedTrackId,
      seek,
      onError,
    ],
  );
  useStudioSpacePlayback(playback.actions.togglePlayback, step);
  const entryNavigate = useStudioEntryNavigation(
    model.view.id,
    String(model.manifest.privacyRevision),
    sources,
    true,
    seek,
  );
  const recall = useStudioRecallEvidence(
    location,
    location.recordingId ?? activeId ?? '',
    String(model.manifest.privacyRevision),
  );
  const evidence = recall.state?.evidence,
    initialApplied = useRef<string | null>(null);
  useEffect(() => {
    const id = location.recordingId;
    if (!id || (location.evidenceId && !evidence)) return;
    const position = dayPosition(
      model,
      id,
      evidence?.startOffsetMs ??
        location.seek ??
        model.spans.find((s) => s.recordingId === id)?.sourceOffsetMs ??
        0,
    );
    const key = JSON.stringify(location);
    if (initialApplied.current === key) return;
    // Seek before preparation so a deep link can admit its own recording session.
    if (position !== null) {
      initialApplied.current = key;
      if (evidence?.sourceKind === 'ocr') {
        const track = model.trackIds.get(id)?.get(evidence.sourceTrackId);
        if (track) sources.select(track);
      }
      seek(position);
    }
  }, [location, evidence, model, sources, seek, sessions]);
  const shownEvidence = useMemo(() => {
    if (!evidence || evidenceDismissed) return null;
    const start = dayPosition(
        model,
        evidence.recordingId,
        evidence.startOffsetMs,
      ),
      end = dayPosition(
        model,
        evidence.recordingId,
        Math.max(evidence.startOffsetMs, evidence.endOffsetMs - 1),
      );
    return start === null || end === null
      ? null
      : {
          ...evidence,
          startOffsetMs: start,
          endOffsetMs: end + 1,
          sourceTrackId:
            model.trackIds
              .get(evidence.recordingId)
              ?.get(evidence.sourceTrackId) ?? evidence.sourceTrackId,
        };
  }, [evidence, evidenceDismissed, model]);
  return {
    seek,
    resolveSource,
    entryNavigate,
    shownEvidence,
    recallUnavailable: recall.state?.status === 'unavailable',
  };
}
