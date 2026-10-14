import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ocrFrameProcessingRevision } from '@/features/ai/ocr/ocr-processing-revision';
import { studioRecallEvidenceTrackIds } from '../../navigation/recall/studio-recall-navigation';
import type { InspectorContentIndex } from '../content/inspector-content';
import { RecordingInspectorSummary } from '../recording-inspector-summary';
import type {
  StudioSidebarActions,
  StudioSidebarModel,
} from '../studio-sidebar';
import { RecallEvidenceInspector } from './recall-evidence-inspector';
import { SelectedSourceInspector } from './selected-source-inspector';
import { SourceLocationInspector } from './source-location-inspector';
import { TimelineSelectionInspector } from './timeline-selection-inspector';
import type { useStudioInspectorSelection } from './use-studio-inspector-selection';

export interface StudioSidebarDetailsProps {
  active: boolean;
  actions: StudioSidebarActions;
  model: StudioSidebarModel;
  content: InspectorContentIndex;
  selection: ReturnType<typeof useStudioInspectorSelection>;
}

// Pure layout renderer; selection and tab state stay with the sidebar owner.
export function renderStudioSidebarDetails({
  active,
  actions,
  model,
  content,
  selection,
}: StudioSidebarDetailsProps) {
  const {
    selectedLanes,
    selectedSegments,
    selectedRedactions,
    selectedVisualFrames,
    hasTimelineSelection,
    timelineSelection,
    clearSelection,
  } = selection;
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
                privacyRevision: JSON.stringify(model.detail.redactions),
                processingRevision: model.recallEvidence.contentRevision,
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
            model.detail.availability === 'locked' ? 'default' : 'destructive'
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
              />
            ))}
        </>
      )}

      {!model.recallEvidence &&
      model.selection === null &&
      !hasTimelineSelection ? (
        <RecordingInspectorSummary
          active={active}
          aiProcessingState={
            model.summaryProcessingState ?? model.aiProcessingState
          }
          detail={model.summaryDetail ?? model.detail}
          debugEnabled={model.debugEnabled === true}
        />
      ) : null}
    </div>
  );
}
