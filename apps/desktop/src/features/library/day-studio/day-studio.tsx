import { useCallback, useEffect, useMemo, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  RecallSourceKind,
  ResizeLibraryRedactionInput,
  LibraryWaveformTileRequest,
} from '@daydream/shared';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { TooltipProvider } from '@/components/ui/tooltip';
import { startRecordingAiProcessing } from '@/features/ai/ai-client';
import { DayReviewPanel } from '@/features/recall/day/day-review-panel';
import { recallDateLabel } from '@/features/recall/recall-format';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';
import { useStudioSelectionDismissal } from '../components/studio/interactions/use-studio-selection-dismissal';
import { RecordingStudioLayout } from '../components/studio/layout/recording-studio-layout';
import { ReprocessingDialog } from '../components/studio/reprocessing/reprocessing-dialog';
import type { StudioInspectorSelection } from '../components/studio/studio-inspector-selection';
import { StudioSidebar } from '../components/studio/studio-sidebar';
import { StudioSourcePlayer } from '../components/studio/studio-source-player';
import { useStudioTimelineStore } from '../components/studio/studio-timeline-store';
import { TrackTimeline } from '../components/studio/track-timeline';
import { useRecordingPlaybackController } from '../components/studio/use-recording-playback-controller';
import { useStudioVideoSources } from '../components/studio/use-studio-video-sources';
import {
  deleteLibraryRecording,
  renameLibraryRecording,
} from '../library-client';
import { RecordingDebugDialog } from '../recording-studio-route/recording-debug-dialog';
import { RecordingDeleteDialog } from '../recording-studio-route/recording-delete-dialog';
import { RecordingRenameDialog } from '../recording-studio-route/recording-rename-dialog';
import { RecordingStudioToolbar } from '../recording-studio-route/recording-studio-toolbar';
import { DayRecordingAudioCleanup } from './day-recording-audio-cleanup';
import { DayRecordingsPanel } from './day-recordings-panel';
import { projectDayResults } from './day-results';
import {
  sourceRedaction,
  readDayWaveformTiles,
  prepareDayRedactionResize,
} from './day-studio-actions';
import {
  dayPlaybackAssets,
  splitDayRanges,
  type DayTimelineModel,
} from './day-timeline';
import type {
  DayMutationController,
  DayPrivacyOperation,
} from './use-day-mutations';
import { useDayNavigation } from './use-day-navigation';
import { useDayPlaybackSessions } from './use-day-playback-sessions';
import { useDayReprocessing } from './use-day-reprocessing';
import { useDayResults } from './use-day-results';

export interface DayStudioLocation {
  recordingId: string | null;
  seek: number | null;
  evidenceId: string | null;
  source: RecallSourceKind | null;
}
export interface DayStudioProps {
  model: DayTimelineModel;
  location: DayStudioLocation;
  onRefresh: () => void;
  mutations: DayMutationController;
}
export function DayStudio({
  model,
  location,
  onRefresh,
  mutations,
}: DayStudioProps) {
  const sessions = useDayPlaybackSessions(model),
    derived = useDayResults(model);
  const projected = useMemo(
    () => projectDayResults(model, derived.results),
    [model, derived.results],
  );
  const assets = useMemo(
    () => dayPlaybackAssets(model, sessions.sessions),
    [model, sessions.sessions],
  );
  const [muted, setMuted] = useState<Set<string>>(() => new Set()),
    [selection, setSelection] = useState<StudioInspectorSelection | null>(null);
  const [rename, setRename] = useState(false),
    [deleting, setDeleting] = useState(false),
    [debug, setDebug] = useState(false),
    [busy, setBusy] = useState(false),
    [deletePending, setDeletePending] = useState(false),
    [error, setError] = useState<string | null>(null),
    [evidenceDismissed, setEvidenceDismissed] = useState(false);
  const inspectedIds = useStudioTimelineStore(
    (s) => s.selection.visualFrameIds,
  );
  const inspectedSource = projected.frames.find(
    (frame) => frame.id === inspectedIds.at(-1),
  )?.sourceTrackId;
  const sources = useStudioVideoSources(model.view, assets, inspectedSource);
  const playback = useRecordingPlaybackController({
    detail: sources.selectedDetail,
    mutedTrackIds: muted,
    playbackAssets: sources.selectedAssets,
    shortGaps: true,
    skipOnlyWhenAllSources: sources.grid,
  });
  const span =
    model.spans.find(
      (s) => s.segmentId === playback.state.currentSegment?.id,
    ) ?? model.spans[0];
  const active =
    model.manifest.recordings.find((r) => r.id === span?.recordingId) ?? null;
  const [cleanupRequest, setCleanupRequest] = useState<{
    recordingId: string;
    revision: number;
  } | null>(null);
  const cleanupRecording = model.manifest.recordings.find(
    (recording) => recording.id === cleanupRequest?.recordingId,
  );
  const openAudioCleanup = useCallback(() => {
    if (active)
      setCleanupRequest((current) => ({
        recordingId: active.id,
        revision: (current?.revision ?? 0) + 1,
      }));
  }, [active]);
  const readCleanupRange = useCallback(() => {
    const time = Math.round(playback.actions.readPlaybackTimeMs());
    const chosen = useStudioTimelineStore.getState().selection.ranges.at(-1);
    const ranges = splitDayRanges(model, [
      {
        startOffsetMs: chosen?.startMs ?? time,
        endOffsetMs: Math.min(
          chosen?.endMs ?? time + 15_000,
          (chosen?.startMs ?? time) + 15_000,
        ),
        trackIds: model.view.tracks
          .filter((track) => track.kind === 'microphone')
          .map((track) => track.id),
      },
    ]);
    const range = ranges.get(cleanupRequest?.recordingId ?? '')?.[0];
    if (!range)
      throw new Error(
        'Select microphone audio in this recording to preview cleanup.',
      );
    return {
      startOffsetMs: range.startOffsetMs,
      endOffsetMs: range.endOffsetMs,
    };
  }, [model, cleanupRequest?.recordingId, playback.actions]);
  const debugEnabled = useDebugEnabled();
  const initialize = useStudioTimelineStore((s) => s.initializeFromDetail),
    clearTimeline = useStudioTimelineStore((s) => s.clearSelection);
  useEffect(() => {
    initialize(model.view);
  }, [model, initialize]);
  const ensure = sessions.ensure;
  const preparePlayback = useCallback(() => {
    const index = model.spans.findIndex(
      (s) => s.segmentId === playback.state.currentSegment?.id,
    );
    const current = model.spans[Math.max(0, index)],
      next = model.spans[Math.max(0, index) + 1];
    if (current) void ensure(current.recordingId).catch(() => undefined);
    if (next && next.recordingId !== current?.recordingId)
      void ensure(next.recordingId).catch(() => undefined);
  }, [model, playback.state.currentSegment?.id, ensure]);
  useEffect(preparePlayback, [preparePlayback]);
  const reloadResults = derived.reload;
  const refreshPlayback = useCallback(() => {
    onRefresh();
    preparePlayback();
    reloadResults();
  }, [onRefresh, preparePlayback, reloadResults]);
  const seekTimeline = playback.actions.seekTimeline;
  const {
    seek,
    resolveSource,
    entryNavigate,
    shownEvidence,
    recallUnavailable,
  } = useDayNavigation({
    model,
    location,
    sessions: sessions.sessions,
    sources,
    playback,
    activeId: active?.id ?? null,
    evidenceDismissed,
    onError: setError,
  });
  const selectRecording = useCallback(
    (time: number) => {
      setSelection(null);
      setEvidenceDismissed(true);
      clearTimeline();
      seekTimeline(time);
    },
    [clearTimeline, seekTimeline],
  );
  const clear = useCallback(() => {
    setSelection(null);
    setEvidenceDismissed(true);
  }, []);
  useStudioSelectionDismissal(clear);
  const inspect = useCallback((value: StudioInspectorSelection) => {
    setSelection(value);
    setEvidenceDismissed(true);
  }, []);
  const toggleMuted = useCallback(
    (id: string) =>
      setMuted((current) => {
        const next = new Set(current);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    [],
  );
  const mutate = useCallback(
    async (action: () => Promise<unknown>) => {
      playback.actions.pausePlayback();
      setDeletePending(true);
      setError(null);
      try {
        await action();
      } finally {
        setDeletePending(false);
        onRefresh();
      }
    },
    [onRefresh, playback.actions],
  );
  const hide = useCallback(
    (ranges: CreateLibraryRedactionRangeInput[]) => {
      playback.actions.pausePlayback();
      return mutations.run(
        [...splitDayRanges(model, ranges)].map(
          ([recordingId, ranges]): DayPrivacyOperation => ({
            kind: 'ranges',
            recordingId,
            ranges,
            hide: true,
          }),
        ),
      );
    },
    [model, mutations, playback.actions],
  );
  const unhide = useCallback(
    (ranges: CreateLibraryRedactionRangeInput[]) => {
      playback.actions.pausePlayback();
      return mutations.run(
        [...splitDayRanges(model, ranges)].map(
          ([recordingId, ranges]): DayPrivacyOperation => ({
            kind: 'ranges',
            recordingId,
            ranges,
            hide: false,
          }),
        ),
      );
    },
    [model, mutations, playback.actions],
  );
  const remove = useCallback(
    (id: string) => {
      playback.actions.pausePlayback();
      const { recording, cut } = sourceRedaction(model, id);
      return mutations.run([
        { kind: 'delete', recordingId: recording.id, redactionId: cut.id },
      ]);
    },
    [model, mutations, playback.actions],
  );
  const resize = useCallback(
    (input: ResizeLibraryRedactionInput) => {
      playback.actions.pausePlayback();
      return mutations.run([
        { kind: 'resize', input: prepareDayRedactionResize(model, input) },
      ]);
    },
    [model, mutations, playback.actions],
  );
  const waveforms = useCallback(
    (input: LibraryWaveformTileRequest) => readDayWaveformTiles(model, input),
    [model],
  );
  const reprocessing = useDayReprocessing(model);
  const openRename = useCallback(() => setRename(true), []),
    openDelete = useCallback(() => setDeleting(true), []),
    openDebug = useCallback(() => setDebug(true), []);
  const renameRecording = useCallback(
    async (title: string) => {
      if (!active) return;
      await renameLibraryRecording({ recordingId: active.id, title });
      setRename(false);
      onRefresh();
    },
    [active, onRefresh],
  );
  const deleteRecording = useCallback(async () => {
    if (!active) return;
    await mutate(() => deleteLibraryRecording({ recordingId: active.id }));
    setDeleting(false);
  }, [active, mutate]);
  const process = useCallback(() => {
    if (!active) return;
    setBusy(true);
    setError(null);
    void startRecordingAiProcessing({ recordingId: active.id })
      .then(onRefresh)
      .catch(() => setError('Processing could not start. Try again.'))
      .finally(() => setBusy(false));
  }, [active, onRefresh]);
  const player = (
    <StudioSourcePlayer
      detail={model.view}
      assets={assets}
      sources={sources}
      playback={playback}
      onRefreshPlayback={refreshPlayback}
    />
  );
  const timeline = (
    <TrackTimeline
      timelineOriginMs={model.originMs}
      detail={model.view}
      activity={projected.activity}
      aiProcessingState={projected.ai}
      mutedTrackIds={muted}
      playback={playback}
      playbackAssets={assets}
      thumbnailStatus="ready"
      visualFrames={projected.frames}
      onCreateRedactions={hide}
      onUnredactRanges={unhide}
      onToggleTrackMuted={toggleMuted}
      onClearInspectorSelection={clear}
      onInspectActivity={inspect}
      onInspectTranscript={inspect}
      onRefreshRedactions={onRefresh}
      readWaveformTiles={waveforms}
      onResizeRedaction={resize}
      onReprocess={reprocessing.open}
      onAudioCleanup={active ? openAudioCleanup : undefined}
      suspendFollow={reprocessing.request !== null}
    />
  );
  const inspector = (
    <StudioSidebar
      recordings={
        <DayRecordingsPanel
          model={model}
          activeId={active?.id ?? null}
          onSelect={selectRecording}
        />
      }
      day={<DayReviewPanel date={model.manifest.dayId} />}
      actions={{
        readPlaybackTimeMs: playback.actions.readPlaybackTimeMs,
        onClearActivitySelection: clear,
        onCreateRedactions: hide,
        onDeleteRedaction: remove,
        onSeek: seek,
        onNavigateEntry: entryNavigate,
        onUnredactRanges: unhide,
      }}
      model={{
        detail: model.view,
        summaryDetail: active ?? undefined,
        summaryProcessingState: active
          ? derived.results.get(active.id)?.ai
          : undefined,
        resolveSource,
        aiProcessingState: projected.ai,
        activity: projected.activity,
        visualFrames: projected.frames,
        selection,
        debugEnabled,
        timelineOffsetMs: playback.state.displayedTimelineMs,
        isPlaying: playback.state.isPlaying,
        seekRevision: playback.state.seekRevision,
        recallEvidence: shownEvidence,
      }}
    />
  );
  return (
    <TooltipProvider delayDuration={300}>
      <RecordingStudioToolbar
        title={recallDateLabel(model.manifest.dayId)}
        detail={active}
        aiProcessing={{
          state: active ? (derived.results.get(active.id)?.ai ?? null) : null,
        }}
        debugEnabled={debugEnabled}
        loadStatus="ready"
        processingStartBusy={busy}
        onRename={openRename}
        onDelete={openDelete}
        onDebug={openDebug}
        onStartAiProcessing={process}
        onRefresh={refreshPlayback}
      />
      <div
        data-studio-root
        className="relative flex min-h-[36rem] w-full flex-1 flex-col gap-6 px-5 pb-6 lg:px-8"
      >
        {error || sessions.error || derived.error || recallUnavailable ? (
          <Alert variant="destructive">
            <AlertDescription>
              {error ??
                sessions.error ??
                (recallUnavailable
                  ? 'This source is unavailable.'
                  : 'Some text or activity could not be loaded. Refresh the day to try again.')}
            </AlertDescription>
          </Alert>
        ) : null}
        <RecordingStudioLayout
          player={player}
          timeline={timeline}
          inspector={inspector}
          initiallyOpen={shownEvidence !== null}
        />
        {deletePending || mutations.pending ? (
          <div
            role="status"
            className="bg-background absolute inset-0 z-50 flex items-center justify-center"
          >
            Updating recording privacy…
          </div>
        ) : null}
      </div>
      <ReprocessingDialog controller={reprocessing} />
      {cleanupRecording && cleanupRequest ? (
        <DayRecordingAudioCleanup
          key={cleanupRecording.id}
          detail={cleanupRecording}
          requestRevision={cleanupRequest.revision}
          readRange={readCleanupRange}
          pausePlayback={playback.actions.pausePlayback}
          refreshPlayback={refreshPlayback}
        />
      ) : null}
      <RecordingDeleteDialog
        open={deleting}
        onOpenChange={setDeleting}
        onConfirm={deleteRecording}
      />
      {rename ? (
        <RecordingRenameDialog
          open={rename}
          detail={active}
          onOpenChange={setRename}
          onSave={renameRecording}
        />
      ) : null}
      {debug && active ? (
        <RecordingDebugDialog
          open={debug}
          recordingId={active.id}
          onOpenChange={setDebug}
        />
      ) : null}
    </TooltipProvider>
  );
}
