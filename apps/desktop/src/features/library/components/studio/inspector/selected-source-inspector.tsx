import { useCallback } from 'react';
import { X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatDuration, formatTimestamp } from '../../../library-format';
import type { StudioInspectorSelection } from '../studio-inspector-selection';
import {
  activityEventLabel,
  activitySourceLabel,
  formatTimelineOffset,
  transcriptionResultLabel,
  transcriptionSourceLabel,
} from './inspector-format';
import { InspectorField, InspectorSection } from './inspector-section';

export interface SelectedSourceInspectorProps {
  onClearSelection: () => void;
  onSeek: (timelineOffsetMs: number) => void;
  selection: StudioInspectorSelection;
  debugEnabled?: boolean;
}

export function SelectedSourceInspector({
  onClearSelection,
  onSeek,
  selection,
  debugEnabled = false,
}: SelectedSourceInspectorProps) {
  const handleSeek = useCallback(() => {
    onSeek(selection.timelineOffsetMs);
  }, [onSeek, selection.timelineOffsetMs]);
  const title = inspectorTitle(selection);

  return (
    <InspectorSection title={title}>
      {!debugEnabled ? (
        <ReadableSelectedSource selection={selection} />
      ) : selection.kind === 'activity-interval' ? (
        <ActivityIntervalFields selection={selection} />
      ) : selection.kind === 'activity-event' ? (
        <ActivityEventFields selection={selection} />
      ) : (
        <TranscriptSegmentFields selection={selection} />
      )}

      <div className="flex gap-1 border-t pt-2">
        <Button
          className="min-w-0 flex-1"
          onClick={handleSeek}
          size="sm"
          type="button"
          variant="outline"
        >
          Jump to source
        </Button>
        <Button
          aria-label="Clear selected source"
          onClick={onClearSelection}
          size="icon-sm"
          type="button"
          variant="ghost"
        >
          <X aria-hidden="true" />
        </Button>
      </div>
    </InspectorSection>
  );
}

function ReadableSelectedSource({
  selection,
}: {
  selection: StudioInspectorSelection;
}) {
  if (selection.kind === 'transcript-segment')
    return (
      <>
        <p className="text-muted-foreground text-xs">
          {transcriptionSourceLabel(selection.sourceRole)} ·{' '}
          {formatTimelineOffset(selection.startOffsetMs)}–
          {formatTimelineOffset(selection.endOffsetMs)}
        </p>
        <p className="text-sm leading-6 break-words whitespace-pre-wrap">
          {selection.text}
        </p>
      </>
    );
  if (selection.kind === 'activity-event')
    return (
      <>
        <p className="font-medium">
          {selection.applicationName ?? 'Window focus changed'}
        </p>
        {selection.title ? (
          <p className="leading-6 break-words">{selection.title}</p>
        ) : null}
        <p className="text-muted-foreground text-xs">
          {formatTimelineOffset(selection.offsetMs)}
        </p>
      </>
    );
  return (
    <>
      <p>
        {activitySourceLabel(selection.sourceKind)} ·{' '}
        {selection.state === 'active' ? 'Active' : 'Idle'}
      </p>
      <p className="text-muted-foreground text-xs">
        {formatTimelineOffset(selection.startOffsetMs)}–
        {formatTimelineOffset(selection.endOffsetMs)} ·{' '}
        {formatDuration(selection.endOffsetMs - selection.startOffsetMs)}
      </p>
    </>
  );
}

function ActivityIntervalFields({
  selection,
}: {
  selection: Extract<StudioInspectorSelection, { kind: 'activity-interval' }>;
}) {
  return (
    <dl className="space-y-1.5">
      <InspectorField
        label="Activity source"
        value={activitySourceLabel(selection.sourceKind)}
      />
      <InspectorField label="State" value={selection.state} />
      <InspectorField
        label="Start"
        value={formatTimelineOffset(selection.startOffsetMs)}
      />
      <InspectorField
        label="End"
        value={formatTimelineOffset(selection.endOffsetMs)}
      />
      <InspectorField
        label="Duration"
        value={formatDuration(selection.endOffsetMs - selection.startOffsetMs)}
      />
      {selection.score === null ? null : (
        <InspectorField
          label="Signal score"
          value={selection.score.toLocaleString(undefined, {
            maximumFractionDigits: 3,
          })}
        />
      )}
      <InspectorField
        label="Track ID"
        value={<code>{selection.trackId}</code>}
      />
    </dl>
  );
}

function ActivityEventFields({
  selection,
}: {
  selection: Extract<StudioInspectorSelection, { kind: 'activity-event' }>;
}) {
  return (
    <dl className="space-y-1.5">
      <InspectorField label="Track" value={selection.trackLabel} />
      <InspectorField
        label="Event"
        value={activityEventLabel(selection.eventKind)}
      />
      <InspectorField
        label="Application"
        value={selection.applicationName ?? 'Unavailable'}
      />
      <InspectorField
        label="Window title"
        value={selection.title ?? 'Unavailable'}
      />
      <InspectorField
        label="Occurred"
        value={formatTimestamp(selection.occurredAtMs)}
      />
      <InspectorField
        label="Timeline offset"
        value={formatTimelineOffset(selection.offsetMs)}
      />
      <InspectorField
        label="Track ID"
        value={<code>{selection.trackId}</code>}
      />
    </dl>
  );
}

function TranscriptSegmentFields({
  selection,
}: {
  selection: Extract<StudioInspectorSelection, { kind: 'transcript-segment' }>;
}) {
  return (
    <>
      <dl className="space-y-1.5">
        <InspectorField
          label="Source"
          value={transcriptionSourceLabel(selection.sourceRole)}
        />
        <InspectorField label="Track" value={selection.trackLabel} />
        <InspectorField
          label="Track ID"
          value={<code>{selection.trackId}</code>}
        />
        <InspectorField
          label="Segment"
          value={<code>{selection.segmentId}</code>}
        />
        <InspectorField
          label="Time range"
          value={`${formatTimelineOffset(selection.startOffsetMs)} – ${formatTimelineOffset(selection.endOffsetMs)}`}
        />
        <InspectorField
          label="Result"
          value={transcriptionResultLabel(selection.chunkResult)}
        />
        <InspectorField label="Provider" value={selection.provider} />
        <InspectorField
          label="Model"
          value={selection.model ?? 'Unavailable'}
        />
        <InspectorField
          label="Confidence"
          value={
            selection.confidence === null
              ? 'Not provided'
              : selection.confidence.toLocaleString(undefined, {
                  maximumFractionDigits: 3,
                })
          }
        />
      </dl>

      <div className="border-t pt-2">
        <div className="text-muted-foreground mb-1 text-xs font-medium">
          Transcript text
        </div>
        <p className="bg-muted/50 max-h-36 overflow-auto px-2 py-1.5 text-sm leading-4 whitespace-pre-wrap">
          {selection.text}
        </p>
      </div>
    </>
  );
}

function inspectorTitle(selection: StudioInspectorSelection): string {
  switch (selection.kind) {
    case 'activity-interval':
      return `${activitySourceLabel(selection.sourceKind)} activity interval`;
    case 'activity-event':
      return activityEventLabel(selection.eventKind);
    case 'transcript-segment':
      return `${transcriptionSourceLabel(selection.sourceRole)} transcript`;
  }
}
