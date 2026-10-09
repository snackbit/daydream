import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
  RecallEvidence,
} from '@daydream/shared';
import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ocrFrameProcessingRevision } from '@/features/ai/ocr/ocr-processing-revision';
import { OcrDocumentInspector } from './inspector/ocr/ocr-document-inspector';
import { RecallEvidenceInspector } from './inspector/recall-evidence-inspector';
import { RecordingInspectorSummary } from './inspector/recording-inspector-summary';
import { SelectedSourceInspector } from './inspector/selected-source-inspector';
import { VisualSourceInspector } from './inspector/visual-source-inspector';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import {
  StudioSidebarComponentSelection,
  StudioSidebarSectionLane,
  StudioSidebarSectionSegment,
  type StudioSidebarSegmentSelection,
} from './studio-sidebar-inspector-sections';
import { StudioSidebarRedactionSection } from './studio-sidebar-redaction-section';
import {
  useStudioTimelineStore,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';
import type { StudioVisualFrame } from './studio-visual-observations';

export interface StudioSidebarActions {
  onClearActivitySelection: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onSeek: (timelineOffsetMs: number) => void;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
}

export interface StudioSidebarModel {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  selection: StudioInspectorSelection | null;
  visualFrames: StudioVisualFrame[];
  debugEnabled?: boolean;
  timelineOffsetMs?: number;
  recallEvidence?: RecallEvidence | null;
}

export interface StudioSidebarProps {
  actions: StudioSidebarActions;
  model: StudioSidebarModel;
}

export function StudioSidebar({ actions, model }: StudioSidebarProps) {
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const timelineSelection = useStudioTimelineStore((state) => state.selection);
  const clearSelection = useStudioTimelineStore(
    (state) => state.clearSelection,
  );

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

  const timelineContent = renderTimelineSelection({
    actions,
    aiProcessingState: model.aiProcessingState,
    clearSelection,
    detail: model.detail,
    ranges: timelineSelection.ranges,
    selectedLanes,
    selectedRedactions,
    selectedSegments,
  });

  return (
    <aside className="bg-muted text-foreground rounded-panel flex h-full min-h-0 flex-col overflow-hidden">
      <div className="shrink-0 border-b px-6 py-5">
        <h2 className="text-sm font-medium">Inspector</h2>
      </div>
      <ScrollArea className="min-h-0 flex-1 overflow-hidden">
        <div className="text-sm">
          {model.recallEvidence ? (
            <>
              <RecallEvidenceInspector
                evidence={model.recallEvidence}
                onSeek={actions.onSeek}
                onClear={actions.onClearActivitySelection}
                onCreateRedactions={actions.onCreateRedactions}
              />
              {model.recallEvidence.sourceKind === 'ocr' ? (
                <OcrDocumentInspector
                  context={{
                    recordingId: model.detail.id,
                    sourceTrackId: model.recallEvidence.sourceTrackId,
                    privacyRevision: JSON.stringify(model.detail.redactions),
                    processingRevision: model.recallEvidence.contentRevision,
                    debugEnabled: model.debugEnabled === true,
                  }}
                  timestampMs={model.recallEvidence.startOffsetMs}
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
            />
          ) : selectedVisualFrames.length > 0 ? (
            selectedVisualFrames.map((frame) => (
              <VisualSourceInspector
                context={{
                  recordingId: model.detail.id,
                  sourceTrackId:
                    frame.sourceTrackId ??
                    model.detail.tracks.find((track) => track.kind === 'video')
                      ?.id ??
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
                  <OcrDocumentInspector
                    context={{
                      recordingId: model.detail.id,
                      sourceTrackId: lane.trackId,
                      privacyRevision: JSON.stringify(model.detail.redactions),
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
                    timestampMs={Math.round(model.timelineOffsetMs ?? 0)}
                  />
                ))}
            </>
          )}

          {!model.recallEvidence &&
          model.selection === null &&
          !hasTimelineSelection ? (
            <RecordingInspectorSummary
              aiProcessingState={model.aiProcessingState}
              detail={model.detail}
            />
          ) : null}
        </div>
      </ScrollArea>
    </aside>
  );
}

function renderTimelineSelection({
  actions,
  aiProcessingState,
  clearSelection,
  detail,
  ranges,
  selectedLanes,
  selectedRedactions,
  selectedSegments,
}: {
  actions: StudioSidebarActions;
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  ranges: StudioTimeRangeSelection[];
  selectedLanes: StudioLaneDraft[];
  selectedRedactions: LibraryRecordingRedaction[];
  selectedSegments: StudioSidebarSegmentSelection[];
}) {
  if (ranges.length > 0) {
    return (
      <StudioSidebarComponentSelection
        aiProcessingState={aiProcessingState}
        clearSelection={clearSelection}
        detail={detail}
        onCreateRedactions={actions.onCreateRedactions}
        onUnredactRanges={actions.onUnredactRanges}
        selections={ranges}
      />
    );
  }

  if (selectedRedactions.length > 0) {
    return selectedRedactions.map((redaction) => (
      <StudioSidebarRedactionSection
        detail={detail}
        key={redaction.id}
        onDeleteRedaction={actions.onDeleteRedaction}
        redaction={redaction}
      />
    ));
  }

  if (selectedSegments.length > 0) {
    return selectedSegments.map((segment) => (
      <StudioSidebarSectionSegment
        aiProcessingState={aiProcessingState}
        key={segment.clipId}
        segment={segment}
      />
    ));
  }

  if (selectedLanes.length > 0) {
    return selectedLanes.map((lane) => (
      <StudioSidebarSectionLane
        aiProcessingState={aiProcessingState}
        key={lane.trackId}
        lane={lane}
      />
    ));
  }

  return null;
}
