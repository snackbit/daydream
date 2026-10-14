import { useMemo, type ButtonHTMLAttributes } from 'react';
import type {
  ActivityCaptureState,
  ActivityInterval,
  LibraryRecordingRedaction,
  RecordingActivity,
  StudioTimelineBucket,
  LibraryRecordingSegment,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { FocusAnnotationStrip } from '../../../annotations/focus-annotation-strip';
import type { StudioInspectorSelection } from '../../../navigation/studio-inspector-selection';
import { trackLaneHeightPxForKind } from '../../interactions/track-timeline-math';
import { SourceOverviewStrip } from '../../overview/source-overview-strip';
import {
  groupTimelineEntries,
  TimelineEntryGroupButton,
} from '../../overview/timeline-entry-groups';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from '../../state/studio-timeline-store';
import { TimelineLaneWindow } from '../timeline-lane-window';
import { TrackLaneHeader } from '../track-lane-header';
import { RedactionOverlay, TimeRangeOverlay } from '../track-lane-overlays';
import {
  ActivityIntervalSpan,
  FocusActivityMarker,
} from './activity-track-markers';

export interface ActivityTrackLaneProps {
  activity: RecordingActivity | null;
  dragHandleProps: ButtonHTMLAttributes<HTMLButtonElement>;
  lane: StudioLaneDraft;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  onSelectLane: (trackId: string, additive: boolean) => void;
  onSelectRedaction: (redactionId: string, additive: boolean) => void;
  redactions: LibraryRecordingRedaction[];
  selectedLane: boolean;
  selectedRedactionIds: Set<string>;
  timelineWidthPx: number;
  timeRanges: StudioTimeRangeSelection[];
  totalDurationMs: number;
  segments: LibraryRecordingSegment[];
  visibleRange?: { startTimelineMs: number; endTimelineMs: number } | undefined;
  onZoomRange?: ((start: number, end: number) => void) | undefined;
  overview?: StudioTimelineBucket[] | undefined;
}

export function ActivityTrackLane({
  activity,
  dragHandleProps,
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
  segments,
  visibleRange,
  onZoomRange,
  overview,
}: ActivityTrackLaneProps) {
  const laneRedactions = redactions.filter((redaction) =>
    redaction.trackIds.includes(lane.trackId),
  );

  return (
    <>
      <TrackLaneHeader
        audioTrack={false}
        dragHandleProps={dragHandleProps}
        heightPx={trackLaneHeightPxForKind(lane.kind)}
        lane={lane}
        muted={false}
        selectedLane={selectedLane}
        trackId={lane.trackId}
        onSelectLane={onSelectLane}
        onToggleTrackMuted={() => undefined}
      />
      <TimelineLaneWindow
        className={cn(
          'bg-muted/30 group-hover/lane:bg-muted/50 relative isolate z-0 cursor-pointer overflow-hidden border-b transition-colors',
          selectedLane && 'bg-accent/20',
        )}
        trackId={lane.trackId}
        height={trackLaneHeightPxForKind(lane.kind)}
        width={timelineWidthPx}
        duration={totalDurationMs}
        range={visibleRange}
      >
        {overview !== undefined && onZoomRange ? (
          <SourceOverviewStrip
            range={visibleRange}
            buckets={overview}
            trackId={lane.trackId}
            width={timelineWidthPx}
            duration={totalDurationMs}
            onZoom={onZoomRange}
          />
        ) : (
          <ActivityLaneContent
            activity={activity}
            lane={lane}
            redactions={laneRedactions}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            onInspectActivity={onInspectActivity}
            segments={segments}
            visibleRange={visibleRange}
            onZoomRange={onZoomRange}
          />
        )}
        {timeRanges.map((range) => (
          <TimeRangeOverlay
            key={range.id}
            range={range}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
          />
        ))}
        {laneRedactions.map((redaction) => (
          <RedactionOverlay
            key={redaction.id}
            redaction={redaction}
            selected={selectedRedactionIds.has(redaction.id)}
            trackLabel={lane.label}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            onSelectRedaction={onSelectRedaction}
          />
        ))}
      </TimelineLaneWindow>
    </>
  );
}

function ActivityLaneContent({
  activity,
  lane,
  onInspectActivity,
  redactions,
  timelineWidthPx,
  totalDurationMs,
  segments,
  visibleRange,
  onZoomRange,
}: {
  activity: RecordingActivity | null;
  lane: StudioLaneDraft;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  redactions: LibraryRecordingRedaction[];
  timelineWidthPx: number;
  totalDurationMs: number;
  segments: LibraryRecordingSegment[];
  visibleRange?: { startTimelineMs: number; endTimelineMs: number } | undefined;
  onZoomRange?: ((start: number, end: number) => void) | undefined;
}) {
  const content = useMemo(() => {
    const from = visibleRange?.startTimelineMs ?? 0;
    const to = visibleRange?.endTimelineMs ?? totalDurationMs;
    return {
      events: (activity?.events ?? []).filter(
        (event) => event.offsetMs >= from && event.offsetMs < to,
      ),
      intervals: (activity?.intervals ?? []).filter(
        (interval) =>
          interval.startOffsetMs < to && interval.endOffsetMs > from,
      ),
    };
  }, [activity, visibleRange, totalDurationMs]);
  const pixelsPerMs = timelineWidthPx / totalDurationMs;
  if (lane.kind === 'activity-focus') {
    if (onZoomRange && pixelsPerMs * 30_000 < 24) {
      const groups = groupTimelineEntries(
        content.events.map((event) => ({
          id: event.id,
          startMs: event.offsetMs,
          endMs: event.offsetMs + 1,
        })),
        pixelsPerMs,
      );
      return (
        <div className="absolute inset-0">
          {groups.map((group) => (
            <TimelineEntryGroupButton
              key={group.id}
              group={group}
              label="window changes"
              pixelsPerMs={pixelsPerMs}
              onZoom={onZoomRange}
            />
          ))}
        </div>
      );
    }
    return (
      <div className="relative h-6">
        {content.events
          .filter((event) => event.kind === 'focused-window')
          .map((event) => (
            <FocusActivityMarker
              event={event}
              key={event.id}
              lane={lane}
              timelineWidthPx={timelineWidthPx}
              totalDurationMs={totalDurationMs}
              onInspectActivity={onInspectActivity}
            />
          ))}
        <FocusAnnotationStrip
          events={content.events}
          lane={lane}
          redactions={redactions}
          segments={segments}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
          onInspectActivity={onInspectActivity}
        />
      </div>
    );
  }

  const sourceKind = lane.kind === 'activity-mouse' ? 'mouse' : 'keyboard';
  const intervals =
    content.intervals.filter(
      (interval) =>
        interval.sourceKind === sourceKind && interval.state === 'active',
    ) ?? [];
  const unavailableCaptureState = (activity?.captureStates ?? []).find(
    (captureState) =>
      captureState.status === 'unavailable' &&
      inputActivityUnavailableReasons.has(captureState.reason ?? '') &&
      !redactions.some((redaction) =>
        redactionContainsOffset(redaction, captureState.offsetMs),
      ),
  );

  function handleInspectInterval(interval: ActivityInterval) {
    onInspectActivity({
      endOffsetMs: interval.endOffsetMs,
      kind: 'activity-interval',
      score: interval.score,
      sourceKind: interval.sourceKind,
      startOffsetMs: interval.startOffsetMs,
      state: interval.state,
      timelineOffsetMs: interval.startOffsetMs,
      trackId: lane.trackId,
      trackLabel: lane.label,
    });
  }

  return (
    <>
      {unavailableCaptureState ? (
        <InputActivityUnavailableMarker
          captureState={unavailableCaptureState}
          lane={lane}
        />
      ) : null}
      {groupTimelineEntries(
        intervals.map((interval) => ({
          ...interval,
          startMs: interval.startOffsetMs,
          endMs: interval.endOffsetMs,
        })),
        pixelsPerMs,
      ).map((group) =>
        group.id.startsWith('group:') && onZoomRange ? (
          <TimelineEntryGroupButton
            key={group.id}
            group={group}
            label={`${sourceKind} activity intervals`}
            pixelsPerMs={pixelsPerMs}
            onZoom={onZoomRange}
          />
        ) : (
          group.items.map((interval) => (
            <ActivityIntervalSpan
              interval={interval}
              key={`${interval.id}:${interval.startOffsetMs}:${interval.endOffsetMs}`}
              lane={lane}
              timelineWidthPx={timelineWidthPx}
              totalDurationMs={totalDurationMs}
              onInspect={handleInspectInterval}
            />
          ))
        ),
      )}
    </>
  );
}

const inputActivityUnavailableReasons = new Set([
  'dev-input-missing',
  'input-device-access-unavailable',
  'no-supported-devices',
  'permission-denied',
  'unsupported-platform',
]);

function redactionContainsOffset(
  redaction: LibraryRecordingRedaction,
  offsetMs: number,
): boolean {
  return (
    offsetMs >= redaction.startOffsetMs && offsetMs < redaction.endOffsetMs
  );
}

function InputActivityUnavailableMarker({
  captureState,
  lane,
}: {
  captureState: ActivityCaptureState;
  lane: StudioLaneDraft;
}) {
  return (
    <div
      aria-label={`${lane.label} activity unavailable`}
      className="text-muted-foreground absolute inset-x-2 top-1/2 -translate-y-1/2 truncate text-[13px]"
      title={`${lane.label} activity unavailable: ${captureState.reason ?? 'unknown'}`}
    >
      Unavailable
    </div>
  );
}
