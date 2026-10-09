import type { LibraryRecordingTrack } from '@daydream/shared';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Minus, Plus, ShieldOff, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatDuration } from '../../library-format';
import {
  ActivityTrackLane,
  type ActivityTrackLaneProps,
} from './activity-track-lane';
import type { StudioTimeRangeSelection } from './studio-timeline-store';
import { TrackLane, type TrackLaneProps } from './track-lane';
import {
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  type RulerTick,
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
} from './track-timeline-math';

export function TrackTimelineToolbar({
  canZoomIn,
  canZoomOut,
  hasTimeRangeSelection,
  onRedactSelection,
  onUnredactSelection,
  redactionPending,
  onZoomIn,
  onZoomOut,
  selectedRangeRedactionCount,
  statusText,
  zoomPercent,
}: {
  canZoomIn: boolean;
  canZoomOut: boolean;
  hasTimeRangeSelection: boolean;
  onRedactSelection: () => void;
  onUnredactSelection: () => void;
  redactionPending: boolean;
  onZoomIn: () => void;
  onZoomOut: () => void;
  selectedRangeRedactionCount: number;
  statusText: string;
  zoomPercent: number;
}) {
  return (
    <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b px-6 py-4">
      <div className="text-sm font-medium">Tracks</div>
      <div className="flex min-w-0 flex-wrap items-center gap-3">
        {hasTimeRangeSelection ? (
          <div className="flex shrink-0 items-center gap-1">
            <Button
              disabled={redactionPending}
              size="sm"
              type="button"
              variant="outline"
              onClick={onRedactSelection}
            >
              <ShieldX data-icon="inline-start" />
              Redact
            </Button>
            {selectedRangeRedactionCount > 0 ? (
              <Button
                disabled={redactionPending}
                size="sm"
                type="button"
                variant="outline"
                onClick={onUnredactSelection}
              >
                <ShieldOff data-icon="inline-start" />
                Unredact
              </Button>
            ) : null}
          </div>
        ) : null}
        <div className="text-muted-foreground text-xs">{statusText}</div>
        <div
          aria-label="Track zoom"
          className="flex shrink-0 items-center gap-1"
        >
          <Button
            aria-label="Zoom out tracks"
            disabled={!canZoomOut}
            size="icon"
            type="button"
            variant="ghost"
            onClick={onZoomOut}
          >
            <Minus className="size-3" />
          </Button>
          <div
            aria-live="polite"
            className="text-muted-foreground w-12 text-center text-xs tabular-nums"
          >
            {zoomPercent}%
          </div>
          <Button
            aria-label="Zoom in tracks"
            disabled={!canZoomIn}
            size="icon"
            type="button"
            variant="ghost"
            onClick={onZoomIn}
          >
            <Plus className="size-3" />
          </Button>
        </div>
      </div>
    </div>
  );
}

export function SortableTrackLane({
  visibleRange,
  aiProcessingState,
  clipDrafts,
  detail,
  lane,
  mutedTrackIds,
  onSelectClip,
  onSelectLane,
  onSelectRedaction,
  onSelectVisualFrame,
  onToggleTrackMuted,
  onInspectTranscript,
  playbackAssets,
  selectedClipIds,
  selectedLane,
  selectedRedactionIds,
  selectedVisualFrameIds,
  thumbnailStatus,
  timelineWidthPx,
  timeRanges,
  totalDurationMs,
  track,
  waveformTiles,
  visualFrames,
}: Omit<TrackLaneProps, 'dragHandleProps'>) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: track.id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      className="group/lane grid"
      style={{
        ...style,
        gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${timelineWidthPx}px`,
        width: TRACK_LABEL_WIDTH_PX + timelineWidthPx,
      }}
    >
      <TrackLane
        visibleRange={visibleRange}
        aiProcessingState={aiProcessingState}
        clipDrafts={clipDrafts}
        detail={detail}
        dragHandleProps={{
          ...attributes,
          ...listeners,
        }}
        lane={lane}
        mutedTrackIds={mutedTrackIds}
        playbackAssets={playbackAssets}
        selectedClipIds={selectedClipIds}
        selectedLane={selectedLane}
        selectedRedactionIds={selectedRedactionIds}
        selectedVisualFrameIds={selectedVisualFrameIds}
        thumbnailStatus={thumbnailStatus}
        timelineWidthPx={timelineWidthPx}
        timeRanges={timeRanges}
        totalDurationMs={totalDurationMs}
        track={track}
        waveformTiles={waveformTiles}
        visualFrames={visualFrames}
        onInspectTranscript={onInspectTranscript}
        onSelectClip={onSelectClip}
        onSelectLane={onSelectLane}
        onSelectRedaction={onSelectRedaction}
        onSelectVisualFrame={onSelectVisualFrame}
        onToggleTrackMuted={onToggleTrackMuted}
      />
    </div>
  );
}

export function SortableActivityTrackLane({
  activity,
  segments,
  lane,
  onInspectActivity,
  onSelectLane,
  onSelectRedaction,
  redactions,
  selectedLane,
  selectedRedactionIds,
  timelineWidthPx,
  timeRanges,
  totalDurationMs,
}: Omit<ActivityTrackLaneProps, 'dragHandleProps'>) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: lane.trackId });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      className="group/lane grid"
      style={{
        ...style,
        gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${timelineWidthPx}px`,
        width: TRACK_LABEL_WIDTH_PX + timelineWidthPx,
      }}
    >
      <ActivityTrackLane
        activity={activity}
        segments={segments}
        dragHandleProps={{
          ...attributes,
          ...listeners,
        }}
        lane={lane}
        redactions={redactions}
        selectedLane={selectedLane}
        selectedRedactionIds={selectedRedactionIds}
        timelineWidthPx={timelineWidthPx}
        timeRanges={timeRanges}
        totalDurationMs={totalDurationMs}
        onInspectActivity={onInspectActivity}
        onSelectLane={onSelectLane}
        onSelectRedaction={onSelectRedaction}
      />
    </div>
  );
}

export function TimeRuler({
  ticks,
  timelineWidthPx,
}: {
  ticks: RulerTick[];
  timelineWidthPx: number;
}) {
  return (
    <div
      aria-label="Time ruler"
      className="bg-background relative sticky top-0 z-20 border-b"
      data-time-ruler
      style={{ height: TRACK_RULER_HEIGHT_PX, width: timelineWidthPx }}
    >
      {ticks.map((tick) => (
        <div
          className="border-muted-foreground/40 absolute bottom-0 border-l"
          key={`${tick.timeMs}-${tick.major ? 'major' : 'minor'}`}
          style={{
            height: tick.major ? 22 : 10,
            left: tick.leftPx,
          }}
        >
          {tick.major ? (
            <div className="text-muted-foreground mt-0.5 ml-1 text-xs tabular-nums">
              {formatDuration(tick.timeMs)}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}

export function TimelineRangeLabels({
  ranges,
  timelineWidthPx,
  totalDurationMs,
}: {
  ranges: StudioTimeRangeSelection[];
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  return (
    <>
      {ranges.map((range) => {
        const left = (range.startMs / totalDurationMs) * timelineWidthPx;
        const width =
          ((range.endMs - range.startMs) / totalDurationMs) * timelineWidthPx;

        return (
          <div
            aria-label="Selected time range"
            className="bg-destructive/80 text-destructive-foreground pointer-events-none absolute top-0 z-40 rounded-b-sm px-1 py-px text-xs font-medium"
            key={range.id}
            style={{
              left: TRACK_LABEL_WIDTH_PX + left + 4,
              maxWidth: Math.max(1, width - 8),
            }}
          >
            {formatDuration(range.startMs)} - {formatDuration(range.endMs)}
          </div>
        );
      })}
    </>
  );
}

export function horizontalWheelDeltaPx(event: WheelEvent): number {
  const rawDelta = event.deltaX !== 0 ? event.deltaX : event.deltaY;
  if (event.deltaMode === 1) {
    return rawDelta * 16;
  }
  if (event.deltaMode === 2) {
    return rawDelta * DEFAULT_TRACK_VIEWPORT_WIDTH_PX;
  }

  return rawDelta;
}

export function orderedDetailTracks(
  tracks: LibraryRecordingTrack[],
  laneOrder: string[],
): LibraryRecordingTrack[] {
  const byId = new Map(tracks.map((track) => [track.id, track]));
  const ordered = laneOrder
    .map((trackId) => byId.get(trackId))
    .filter((track): track is LibraryRecordingTrack => Boolean(track));
  const orderedIds = new Set(ordered.map((track) => track.id));
  return [...ordered, ...tracks.filter((track) => !orderedIds.has(track.id))];
}
