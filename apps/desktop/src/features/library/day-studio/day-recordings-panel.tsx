import { useCallback } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { formatDuration, formatTimestamp } from '../library-format';
import type { DayTimelineModel } from './day-timeline';

export interface DayRecordingsPanelProps {
  model: DayTimelineModel;
  activeId: string | null;
  onSelect: (time: number) => void;
}
export function DayRecordingsPanel({
  model,
  activeId,
  onSelect,
}: DayRecordingsPanelProps) {
  return (
    <div className="grid gap-1 p-3">
      {model.manifest.recordings.map((recording) => {
        const spans = model.spans.filter((s) => s.recordingId === recording.id),
          first = spans[0];
        return first ? (
          <RecordingRow
            key={recording.id}
            recording={recording}
            start={first.startMs}
            time={first.startMs - model.originMs}
            duration={spans.reduce((sum, s) => sum + s.durationMs, 0)}
            active={recording.id === activeId}
            onSelect={onSelect}
          />
        ) : null;
      })}
    </div>
  );
}
interface RecordingRowProps {
  recording: LibraryRecordingDetail;
  start: number;
  time: number;
  duration: number;
  active: boolean;
  onSelect: (time: number) => void;
}
function RecordingRow({
  recording,
  start,
  time,
  duration,
  active,
  onSelect,
}: RecordingRowProps) {
  const select = useCallback(() => onSelect(time), [onSelect, time]);
  return (
    <Button
      variant="ghost"
      className="h-auto w-full justify-start rounded-xl px-3 py-3 text-left"
      aria-current={active ? 'true' : undefined}
      onClick={select}
    >
      <span className="grid min-w-0 gap-1">
        <span className="truncate font-medium">
          {recording.title ?? formatTimestamp(start)}
        </span>
        <span className="text-muted-foreground text-xs">
          {formatTimestamp(start)} · {formatDuration(duration)}
        </span>
      </span>
    </Button>
  );
}
