import {
  useRef,
  type ButtonHTMLAttributes,
  type MouseEvent,
  type PointerEvent,
} from 'react';
import type {
  ActivityCaptureState,
  ActivityInterval,
  LibraryRecordingRedaction,
  RecordingActivity,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import {
  ActivityIntervalSpan,
  FocusActivityMarker,
} from './activity-track-markers';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from './studio-timeline-store';
import { TrackLaneHeader } from './track-lane-header';
import { RedactionOverlay, TimeRangeOverlay } from './track-lane-overlays';
import {
  TRACK_DERIVED_LANE_HEIGHT_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
} from './track-timeline-math';

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
}: ActivityTrackLaneProps) {
  const laneContentClickRef = useRef<{
    moved: boolean;
    x: number;
    y: number;
  } | null>(null);
  const laneRedactions = redactions.filter((redaction) =>
    redaction.trackIds.includes(lane.trackId),
  );

  function handleLaneContentPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (
      event.target instanceof Element &&
      event.target.closest('[data-library-action]')
    ) {
      laneContentClickRef.current = null;
      return;
    }

    laneContentClickRef.current = {
      moved: false,
      x: event.clientX,
      y: event.clientY,
    };
  }

  function handleLaneContentPointerMove(event: PointerEvent<HTMLDivElement>) {
    const clickState = laneContentClickRef.current;
    if (!clickState) {
      return;
    }

    if (
      Math.hypot(event.clientX - clickState.x, event.clientY - clickState.y) >
      TRACK_SELECTION_DRAG_THRESHOLD_PX
    ) {
      clickState.moved = true;
    }
  }

  function handleLaneContentClick(event: MouseEvent<HTMLDivElement>) {
    if (
      event.target instanceof Element &&
      event.target.closest('[data-library-action]')
    ) {
      return;
    }

    const clickState = laneContentClickRef.current;
    laneContentClickRef.current = null;
    if (clickState?.moved) {
      return;
    }

    onSelectLane(
      lane.trackId,
      event.shiftKey || event.metaKey || event.ctrlKey,
    );
  }

  return (
    <>
      <TrackLaneHeader
        audioTrack={false}
        dragHandleProps={dragHandleProps}
        heightPx={TRACK_DERIVED_LANE_HEIGHT_PX}
        lane={lane}
        muted={false}
        selectedLane={selectedLane}
        trackId={lane.trackId}
        onSelectLane={onSelectLane}
        onToggleTrackMuted={() => undefined}
      />
      <div
        className={cn(
          'bg-muted/30 group-hover/lane:bg-muted/50 relative cursor-pointer overflow-hidden border-b transition-colors',
          selectedLane && 'bg-accent/20',
        )}
        data-lane-content
        data-track-id={lane.trackId}
        style={{ height: TRACK_DERIVED_LANE_HEIGHT_PX, width: timelineWidthPx }}
        onClick={handleLaneContentClick}
        onPointerDown={handleLaneContentPointerDown}
        onPointerMove={handleLaneContentPointerMove}
      >
        <ActivityLaneContent
          activity={activity}
          lane={lane}
          redactions={laneRedactions}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
          onInspectActivity={onInspectActivity}
        />
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
      </div>
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
}: {
  activity: RecordingActivity | null;
  lane: StudioLaneDraft;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  redactions: LibraryRecordingRedaction[];
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  if (lane.kind === 'activity-focus') {
    return (
      <>
        {activity?.events
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
      </>
    );
  }

  const sourceKind = lane.kind === 'activity-mouse' ? 'mouse' : 'keyboard';
  const intervals =
    activity?.intervals.filter(
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
      {intervals.map((interval) => (
        <ActivityIntervalSpan
          interval={interval}
          key={`${interval.id}:${interval.startOffsetMs}:${interval.endOffsetMs}`}
          lane={lane}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
          onInspect={handleInspectInterval}
        />
      ))}
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
      className="text-muted-foreground absolute inset-x-2 top-1/2 -translate-y-1/2 truncate text-[11px]"
      title={`${lane.label} activity unavailable: ${captureState.reason ?? 'unknown'}`}
    >
      Unavailable
    </div>
  );
}
