import { useCallback } from 'react';
import type {
  RecordingProcessingLane,
  RecordingProcessingLaneKind,
} from '@daydream/shared';
import { Check, Circle, Loader2, Pause, TriangleAlert } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { processingLaneLabel } from '../processing-activity';
import { stageLabel } from '../task-format';

export interface ProcessingPipelineNodeProps {
  lane: RecordingProcessingLane;
  selected: boolean;
  onSelect: (kind: RecordingProcessingLaneKind) => void;
}
export function ProcessingPipelineNode({
  lane,
  selected,
  onSelect,
}: ProcessingPipelineNodeProps) {
  const select = useCallback(() => onSelect(lane.kind), [lane.kind, onSelect]);
  const Icon = lane.failed
    ? TriangleAlert
    : lane.status === 'paused'
      ? Pause
      : lane.running
        ? Loader2
        : lane.status === 'succeeded'
          ? Check
          : Circle;
  return (
    <li className="min-w-0 flex-1">
      <Button
        variant="outline"
        aria-label={`Inspect ${stageLabel(lane.kind)}`}
        aria-pressed={selected}
        onClick={select}
        className="bg-background aria-pressed:border-foreground aria-pressed:bg-muted h-auto w-full min-w-0 items-start justify-start gap-2 rounded-lg px-3 py-3 text-left whitespace-normal"
      >
        <Icon aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
        <span className="min-w-0 flex-1">
          <span className="block text-xs font-medium">
            {stageLabel(lane.kind)}
          </span>
          <span className="text-muted-foreground mt-1 block text-xs font-normal">
            {processingLaneLabel(lane)}
          </span>
          {lane.total ? (
            <span className="text-muted-foreground mt-2 flex flex-wrap gap-x-2 gap-y-1 text-xs font-normal tabular-nums">
              <span>
                {lane.completed + lane.skipped}/{lane.total} done
              </span>
              {lane.running ? <span>{lane.running} running</span> : null}
              {lane.queued ? <span>{lane.queued} queued</span> : null}
              {lane.failed ? (
                <span className="text-destructive">{lane.failed} failed</span>
              ) : null}
            </span>
          ) : null}
        </span>
      </Button>
    </li>
  );
}
