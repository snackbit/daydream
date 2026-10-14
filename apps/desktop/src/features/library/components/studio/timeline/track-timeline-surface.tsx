import { DndContext, closestCenter } from '@dnd-kit/core';
import {
  SortableContext,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { ScrollArea } from '@/components/ui/scroll-area';
import { GroupedRedactions } from '../redactions/grouped-redactions';
import { TimelineReprocessingContextMenu } from '../reprocessing/timeline-reprocessing-context-menu';
import { renderTrackTimelineControls } from './controls/track-timeline-controls';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
} from './interactions/track-timeline-math';
import { TimelineGaps } from './lanes/timeline-gaps';
import { TrackTimelineLanes } from './lanes/track-timeline-lanes';
import { TimeRuler } from './ruler/time-ruler';
import { TimelineRangeLabels } from './ruler/timeline-range-labels';
import { TimelinePlayhead } from './timeline-playhead';
import type { TrackTimelineProps } from './track-timeline';
import type { TrackTimelineController } from './use-track-timeline-controller';

export interface TrackTimelineSurfaceProps {
  source: TrackTimelineProps;
  controller: TrackTimelineController;
}

// Pure layout renderer: viewport hooks stay with the public timeline owner.
export function renderTrackTimelineSurface({
  source: props,
  controller,
}: TrackTimelineSurfaceProps) {
  const {
    actions,
    state,
    clipsById,
    lanesById,
    selection,
    totalDurationMs,
    contentRef,
    followPlayhead,
    keepPlayheadVisible,
    viewportRef,
    viewModel,
    menuIsOpen,
    menuOpen,
    waveformTiles,
    sensors,
    handleDragEnd,
    handleTimelinePointerDown,
    handleTimelinePointerEnd,
    handleTimelinePointerMove,
    handleTimelineClickCapture,
    handleSelectClip,
    handleSelectLane,
    handleSelectRedaction,
    selectVisualFrameForInspection,
    handleInspectActivity,
    selectTranscriptForInspection,
    zoomSelection,
    zoomRange,
  } = controller;
  const {
    timelineOriginMs,
    onResizeRedaction,
    activity,
    aiProcessingState,
    plannedProcessingScopes,
    detail,
    mutedTrackIds,
    onToggleTrackMuted,
    playbackAssets,
    thumbnailStatus,
    visualFrames,
    onRefreshRedactions,
    onReprocess,
    onAudioCleanup,
    suspendFollow = false,
    readTranscriptPreview,
    transcriptPreviewRevision,
    timelinePendingRanges,
    timelineFailedRanges,
    transcriptReadStatus = 'ready',
  } = props;

  return (
    <div className="bg-background flex h-full flex-col">
      {renderTrackTimelineControls({
        controller,
        onReprocess: props.onReprocess,
      })}
      <ScrollArea
        className="isolate min-h-0 flex-1"
        scrollbars="both"
        viewportRef={viewportRef}
      >
        <TimelineReprocessingContextMenu
          contentRef={contentRef}
          ranges={selection.ranges}
          tracks={detail.tracks}
          timelineWidthPx={viewModel.timelineWidthPx}
          totalDurationMs={viewModel.totalDurationMs}
          onReprocess={onReprocess}
          onOpenChange={menuOpen}
          onZoomSelection={zoomSelection}
        >
          <div
            ref={contentRef}
            aria-label="Track timeline"
            className="relative cursor-crosshair"
            role="group"
            style={{
              width: TRACK_LABEL_WIDTH_PX + viewModel.timelineWidthPx,
            }}
            onPointerCancel={handleTimelinePointerEnd}
            onClickCapture={handleTimelineClickCapture}
            onPointerDown={handleTimelinePointerDown}
            onPointerMove={handleTimelinePointerMove}
            onPointerUp={handleTimelinePointerEnd}
          >
            <TimelinePlayhead
              time={state.snapshot.displayedTimelineMs}
              readTime={actions.readPlaybackTimeMs}
              playing={state.snapshot.isPlaying}
              width={viewModel.timelineWidthPx}
              duration={totalDurationMs}
              follow={followPlayhead && !suspendFollow && !menuIsOpen}
              keepVisible={keepPlayheadVisible}
            />
            <TimelineRangeLabels
              ranges={selection.ranges}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={viewModel.totalDurationMs}
            />
            <GroupedRedactions
              onResize={onResizeRedaction}
              redactions={detail.redactions}
              lanes={viewModel.activeLaneLayouts}
              selectedIds={viewModel.selectedRedactionIds}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={totalDurationMs}
              onSelect={handleSelectRedaction}
              onSaved={onRefreshRedactions}
            />
            <TimelineGaps
              segments={detail.segments}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={totalDurationMs}
            />
            <div
              className="grid"
              style={{
                gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${viewModel.timelineWidthPx}px`,
              }}
            >
              <div
                className="bg-background sticky left-0 z-60 border-r border-b px-3"
                style={{ height: TRACK_RULER_HEIGHT_PX }}
              />
              <TimeRuler
                originMs={timelineOriginMs}
                ticks={viewModel.rulerTicks}
                timelineWidthPx={viewModel.timelineWidthPx}
              />
            </div>
            <DndContext
              collisionDetection={closestCenter}
              sensors={sensors}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={viewModel.activeLaneIds}
                strategy={verticalListSortingStrategy}
              >
                <TrackTimelineLanes
                  view={viewModel}
                  lanesById={lanesById}
                  media={{
                    readTranscriptPreview,
                    transcriptPreviewRevision,
                    timelinePendingRanges:
                      timelinePendingRanges ??
                      (transcriptReadStatus === 'loading'
                        ? [
                            {
                              startOffsetMs:
                                viewModel.paintRange.startTimelineMs,
                              endOffsetMs: viewModel.paintRange.endTimelineMs,
                            },
                          ]
                        : []),
                    timelineFailedRanges:
                      timelineFailedRanges ??
                      (transcriptReadStatus === 'error'
                        ? [
                            {
                              startOffsetMs:
                                viewModel.paintRange.startTimelineMs,
                              endOffsetMs: viewModel.paintRange.endTimelineMs,
                            },
                          ]
                        : []),
                    visibleRange: viewModel.paintRange,
                    aiProcessingState,
                    plannedProcessingScopes,
                    clipDrafts: clipsById,
                    detail,
                    mutedTrackIds,
                    playbackAssets,
                    selectedClipIds: viewModel.selectedClipIds,
                    selectedRedactionIds: viewModel.selectedRedactionIds,
                    selectedVisualFrameIds: viewModel.selectedVisualFrameIds,
                    thumbnailStatus,
                    timelineWidthPx: viewModel.timelineWidthPx,
                    totalDurationMs: viewModel.totalDurationMs,
                    waveformTiles,
                    visualFrames,
                    onInspectTranscript: selectTranscriptForInspection,
                    onSelectClip: handleSelectClip,
                    onSelectLane: handleSelectLane,
                    onSelectRedaction: handleSelectRedaction,
                    onSelectVisualFrame: selectVisualFrameForInspection,
                    onToggleTrackMuted,
                    onReprocess,
                    onAudioCleanup,
                    onZoomRange: zoomRange,
                  }}
                  activity={{
                    overview: aiProcessingState?.timelineOverview,
                    visibleRange: viewModel.paintRange,
                    onZoomRange: zoomRange,
                    activity,
                    segments: detail.segments,
                    redactions: detail.redactions,
                    selectedRedactionIds: viewModel.selectedRedactionIds,
                    timelineWidthPx: viewModel.timelineWidthPx,
                    totalDurationMs: viewModel.totalDurationMs,
                    onInspectActivity: handleInspectActivity,
                    onSelectLane: handleSelectLane,
                    onSelectRedaction: handleSelectRedaction,
                  }}
                />
              </SortableContext>
            </DndContext>
          </div>
        </TimelineReprocessingContextMenu>
      </ScrollArea>
    </div>
  );
}
