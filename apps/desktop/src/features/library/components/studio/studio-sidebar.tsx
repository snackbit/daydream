import { useMemo, useState, type ReactNode } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
  RecallEvidence,
  RecordingActivity,
} from '@daydream/shared';
import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ocrFrameProcessingRevision } from '@/features/ai/ocr/ocr-processing-revision';
import { buildInspectorContent } from './inspector/content/inspector-content';
import type { InspectorContentEntry } from './inspector/content/inspector-content';
import { PlaybackInspectorFeed } from './inspector/content/playback-inspector-feed';
import { RecallEvidenceInspector } from './inspector/recall-evidence-inspector';
import { RecordingInspectorSummary } from './inspector/recording-inspector-summary';
import { SelectedSourceInspector } from './inspector/selected-source-inspector';
import {
  SourceLocationInspector,
  type StudioSourceLocation,
} from './inspector/source-location-inspector';
import { TimelineSelectionInspector } from './inspector/timeline-selection-inspector';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { studioRecallEvidenceTrackIds } from './studio-recall-navigation';
import { type StudioSidebarSegmentSelection } from './studio-sidebar-inspector-sections';
import {
  useStudioTimelineStore,
  type StudioLaneDraft,
} from './studio-timeline-store';
import type { StudioVisualFrame } from './studio-visual-observations';

export interface StudioSidebarActions {
  readPlaybackTimeMs?: (() => number) | undefined;
  onClearActivitySelection: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onSeek: (timelineOffsetMs: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
}

export interface StudioSidebarModel {
  resolveSource?:
    | ((trackId: string, time: number) => StudioSourceLocation | undefined)
    | undefined;
  summaryDetail?: LibraryRecordingDetail | undefined;
  summaryProcessingState?: RecordingAiProcessingState | null | undefined;
  seekRevision?: number | undefined;
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  selection: StudioInspectorSelection | null;
  visualFrames: StudioVisualFrame[];
  debugEnabled?: boolean;
  timelineOffsetMs?: number;
  recallEvidence?: RecallEvidence | null;
  activity?: RecordingActivity | null;
  isPlaying?: boolean;
  resetRevision?: number;
}

export interface StudioSidebarProps {
  actions: StudioSidebarActions;
  model: StudioSidebarModel;
  recordings?: ReactNode;
  day?: ReactNode;
}

export function StudioSidebar({
  actions,
  model,
  recordings,
  day,
}: StudioSidebarProps) {
  const content = useMemo(
    () =>
      buildInspectorContent({
        detail: model.detail,
        state: model.aiProcessingState,
        activity: model.activity ?? null,
        visualFrames: model.visualFrames,
      }),
    [model.detail, model.aiProcessingState, model.activity, model.visualFrames],
  );
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const timelineSelection = useStudioTimelineStore((state) => state.selection);
  const clearSelection = useStudioTimelineStore(
    (state) => state.clearSelection,
  );
  const [choice, setChoice] = useState<{
    recordingId: string;
    selection: StudioInspectorSelection | null;
    timelineSelection: typeof timelineSelection;
    evidenceId: string | undefined;
    resetRevision: number | undefined;
    mode: 'teleprompter' | 'inspector' | 'recordings' | 'day';
  } | null>(null);

  const selectedLanes = timelineSelection.laneIds
    .map((laneId) => lanesById[laneId])
    .filter((lane): lane is StudioLaneDraft => Boolean(lane));
  const selectedSegments = timelineSelection.clipIds
    .map((clipId): StudioSidebarSegmentSelection | null => {
      const clip = clipsById[clipId];
      return clip ? { ...clip, clipId } : null;
    })
    .filter((segment): segment is StudioSidebarSegmentSelection =>
      Boolean(segment),
    );
  const selectedRedactions = timelineSelection.redactionIds
    .map((redactionId) =>
      model.detail.redactions.find((redaction) => redaction.id === redactionId),
    )
    .filter((redaction): redaction is LibraryRecordingRedaction =>
      Boolean(redaction),
    );
  const selectedVisualFrames = timelineSelection.visualFrameIds
    .map((frameId) => model.visualFrames.find((frame) => frame.id === frameId))
    .filter((frame): frame is StudioVisualFrame => Boolean(frame));
  const hasTimelineSelection =
    selectedLanes.length > 0 ||
    selectedSegments.length > 0 ||
    selectedRedactions.length > 0 ||
    selectedVisualFrames.length > 0 ||
    timelineSelection.ranges.length > 0;
  const available = model.detail.availability === 'available';
  const manual =
    choice?.recordingId === model.detail.id &&
    choice.selection === model.selection &&
    choice.timelineSelection === timelineSelection &&
    choice.evidenceId === model.recallEvidence?.evidenceId &&
    choice.resetRevision === model.resetRevision;
  const mode = !available
    ? 'inspector'
    : manual
      ? choice.mode
      : model.recallEvidence || model.selection || hasTimelineSelection
        ? 'inspector'
        : recordings
          ? 'recordings'
          : 'teleprompter';
  function changeMode(value: string) {
    if (
      value !== 'teleprompter' &&
      value !== 'inspector' &&
      value !== 'recordings' &&
      value !== 'day'
    )
      return;
    setChoice({
      recordingId: model.detail.id,
      selection: model.selection,
      timelineSelection,
      evidenceId: model.recallEvidence?.evidenceId,
      resetRevision: model.resetRevision,
      mode: value,
    });
  }

  const timelineContent = (
    <TimelineSelectionInspector
      actions={actions}
      aiProcessingState={model.aiProcessingState}
      clearSelection={clearSelection}
      detail={model.detail}
      ranges={timelineSelection.ranges}
      selectedLanes={selectedLanes}
      selectedRedactions={selectedRedactions}
      selectedSegments={selectedSegments}
      debugEnabled={model.debugEnabled === true}
      content={content}
    />
  );

  return (
    <aside
      data-studio-preserve-selection
      className="bg-muted text-foreground rounded-panel flex h-full min-h-0 min-w-0 flex-col overflow-hidden"
    >
      <Tabs
        value={mode}
        onValueChange={changeMode}
        className="min-h-0 flex-1 gap-0"
      >
        <div className="shrink-0 border-b px-6 py-5">
          <TabsList
            aria-label="Studio sidebar"
            className={
              recordings || day
                ? 'grid h-auto w-full grid-cols-2'
                : 'flex h-auto flex-wrap'
            }
          >
            {recordings ? (
              <TabsTrigger value="recordings">Recordings</TabsTrigger>
            ) : null}
            {day ? <TabsTrigger value="day">Day</TabsTrigger> : null}
            <TabsTrigger value="teleprompter" disabled={!available}>
              Teleprompter
            </TabsTrigger>
            <TabsTrigger value="inspector">Inspector</TabsTrigger>
          </TabsList>
        </div>
        {recordings ? (
          <TabsContent
            value="recordings"
            className="min-h-0 flex-1 overflow-hidden"
          >
            <ScrollArea className="h-full">{recordings}</ScrollArea>
          </TabsContent>
        ) : null}
        {day ? (
          <TabsContent value="day" className="min-h-0 flex-1 overflow-hidden">
            <ScrollArea className="h-full">{day}</ScrollArea>
          </TabsContent>
        ) : null}
        <TabsContent
          value="teleprompter"
          className="flex min-h-0 flex-1 flex-col overflow-hidden"
        >
          <PlaybackInspectorFeed
            index={content}
            time={model.timelineOffsetMs ?? 0}
            isPlaying={model.isPlaying === true}
            onSeek={actions.onSeek}
            onNavigateEntry={actions.onNavigateEntry}
            readPlaybackTimeMs={actions.readPlaybackTimeMs}
            seekRevision={model.seekRevision}
          />
        </TabsContent>
        <TabsContent
          value="inspector"
          className="flex min-h-0 flex-1 flex-col overflow-hidden"
        >
          <ScrollArea className="min-h-0 flex-1 overflow-hidden">
            <div className="text-sm">
              {model.recallEvidence ? (
                <>
                  <RecallEvidenceInspector
                    evidence={model.recallEvidence}
                    sourceTrackIds={studioRecallEvidenceTrackIds(
                      model.recallEvidence,
                      model.detail,
                    )}
                    onSeek={actions.onSeek}
                    onClear={actions.onClearActivitySelection}
                    onCreateRedactions={actions.onCreateRedactions}
                    debugEnabled={model.debugEnabled === true}
                  />
                  {model.recallEvidence.sourceKind === 'ocr' ? (
                    <SourceLocationInspector
                      source={model.resolveSource?.(
                        model.recallEvidence.sourceTrackId,
                        model.recallEvidence.startOffsetMs,
                      )}
                      mappingRequired={model.resolveSource !== undefined}
                      time={model.recallEvidence.startOffsetMs}
                      onClear={actions.onClearActivitySelection}
                      context={{
                        recordingId: model.detail.id,
                        sourceTrackId: model.recallEvidence.sourceTrackId,
                        privacyRevision: JSON.stringify(
                          model.detail.redactions,
                        ),
                        processingRevision:
                          model.recallEvidence.contentRevision,
                        debugEnabled: model.debugEnabled === true,
                      }}
                      documentId={model.recallEvidence.documentId}
                      onSeek={actions.onSeek}
                      key={model.recallEvidence.evidenceId}
                    />
                  ) : null}
                </>
              ) : null}
              {model.detail.availability !== 'available' ? (
                <Alert
                  className="m-2"
                  variant={
                    model.detail.availability === 'locked'
                      ? 'default'
                      : 'destructive'
                  }
                >
                  {model.detail.availability === 'locked' ? (
                    <Lock />
                  ) : (
                    <AlertTriangle />
                  )}
                  <AlertTitle>
                    {model.detail.availability === 'locked'
                      ? 'Recording in progress'
                      : 'Missing video files'}
                  </AlertTitle>
                  <AlertDescription>
                    {model.detail.missingReasons.join(' ') ||
                      'Recording files are unavailable.'}
                  </AlertDescription>
                </Alert>
              ) : null}

              {model.recallEvidence ? null : model.selection ? (
                <SelectedSourceInspector
                  onClearSelection={actions.onClearActivitySelection}
                  onSeek={actions.onSeek}
                  selection={model.selection}
                  debugEnabled={model.debugEnabled === true}
                />
              ) : selectedVisualFrames.length > 0 ? (
                selectedVisualFrames.map((frame) => (
                  <SourceLocationInspector
                    mappingRequired={model.resolveSource !== undefined}
                    source={model.resolveSource?.(
                      frame.sourceTrackId ?? '',
                      frame.frameTimestampMs,
                    )}
                    time={frame.frameTimestampMs}
                    context={{
                      recordingId: model.detail.id,
                      sourceTrackId:
                        frame.sourceTrackId ??
                        model.detail.tracks.find(
                          (track) => track.kind === 'video',
                        )?.id ??
                        '',
                      privacyRevision: JSON.stringify(model.detail.redactions),
                      processingRevision: ocrFrameProcessingRevision(
                        model.aiProcessingState,
                        frame.frameTimestampMs,
                        frame.segmentId,
                      ),
                      debugEnabled: model.debugEnabled === true,
                    }}
                    frame={frame}
                    label={
                      model.detail.tracks.find(
                        (track) => track.id === frame.sourceTrackId,
                      )?.label ?? 'Screen'
                    }
                    key={frame.id}
                    onClear={clearSelection}
                    onSeek={actions.onSeek}
                  />
                ))
              ) : (
                <>
                  {timelineContent}
                  {selectedLanes
                    .filter((lane) => lane.kind === 'video')
                    .map((lane) => (
                      <SourceLocationInspector
                        mappingRequired={model.resolveSource !== undefined}
                        source={model.resolveSource?.(
                          lane.trackId,
                          model.timelineOffsetMs ?? 0,
                        )}
                        time={Math.round(model.timelineOffsetMs ?? 0)}
                        onClear={clearSelection}
                        context={{
                          recordingId: model.detail.id,
                          sourceTrackId: lane.trackId,
                          privacyRevision: JSON.stringify(
                            model.detail.redactions,
                          ),
                          processingRevision: ocrFrameProcessingRevision(
                            model.aiProcessingState,
                            Math.round(model.timelineOffsetMs ?? 0),
                            undefined,
                          ),
                          debugEnabled: model.debugEnabled === true,
                        }}
                        key={JSON.stringify([
                          lane.trackId,
                          model.detail.id,
                          model.detail.redactions,
                        ])}
                        onSeek={actions.onSeek}
                      />
                    ))}
                </>
              )}

              {!model.recallEvidence &&
              model.selection === null &&
              !hasTimelineSelection ? (
                <RecordingInspectorSummary
                  aiProcessingState={
                    model.summaryProcessingState ?? model.aiProcessingState
                  }
                  detail={model.summaryDetail ?? model.detail}
                  debugEnabled={model.debugEnabled === true}
                />
              ) : null}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </aside>
  );
}
