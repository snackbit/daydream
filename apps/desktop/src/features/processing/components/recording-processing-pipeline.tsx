import type {
  RecordingProcessingLane,
  RecordingProcessingLaneKind,
} from '@daydream/shared';
import { ArrowRight } from 'lucide-react';
import { ProcessingPipelineNode } from './processing-pipeline-node';

export interface RecordingProcessingPipelineProps {
  lanes: readonly RecordingProcessingLane[];
  selected: RecordingProcessingLaneKind | null;
  onSelect: (kind: RecordingProcessingLaneKind) => void;
}
export function RecordingProcessingPipeline({
  lanes,
  selected,
  onSelect,
}: RecordingProcessingPipelineProps) {
  const independent = lanes.filter(
    (lane) => lane.kind !== 'ocr' && lane.kind !== 'vlm',
  );
  const screen = lanes
    .filter((lane) => lane.kind === 'ocr' || lane.kind === 'vlm')
    .toSorted((a) => (a.kind === 'ocr' ? -1 : 1));
  const dependency = screen.some(
    (lane) => lane.kind === 'ocr' && lane.total > 0,
  );
  return (
    <div className="mt-4 space-y-3" aria-label="Processing pipeline">
      <ul
        aria-label="Independent processing"
        className="grid gap-2 sm:grid-cols-3"
      >
        {independent.map((lane) => (
          <ProcessingPipelineNode
            key={lane.kind}
            lane={lane}
            selected={selected === lane.kind}
            onSelect={onSelect}
          />
        ))}
      </ul>
      <ul
        aria-label="Screen processing"
        className="grid grid-cols-1 items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]"
      >
        {screen.map((lane) => (
          <ProcessingPipelineNode
            key={lane.kind}
            lane={lane}
            selected={selected === lane.kind}
            onSelect={onSelect}
          />
        ))}
        <li
          aria-hidden="true"
          className="col-start-1 row-start-2 justify-self-center sm:col-start-2 sm:row-start-1"
        >
          {dependency ? (
            <ArrowRight className="text-muted-foreground size-4 rotate-90 sm:rotate-0" />
          ) : (
            <span className="block w-4" />
          )}
        </li>
      </ul>
    </div>
  );
}
