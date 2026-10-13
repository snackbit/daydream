import { useCallback, useState } from 'react';
import { Link } from '@tanstack/react-router';
import type {
  RecordingProcessingGroup,
  RecordingProcessingLaneKind,
} from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { formatElapsed, taskStatusLabel } from '../task-format';
import { useRecordingProcessingActions } from '../use-recording-processing-actions';
import { ProcessingActivityList } from './processing-activity-list';
import { RecordingProcessingLaneRow } from './recording-processing-lane';
import { RecordingProcessingPipeline } from './recording-processing-pipeline';

export interface RecordingProcessingCardProps {
  group: RecordingProcessingGroup;
  onInspect: (recordingId: string, kind: RecordingProcessingLaneKind) => void;
  onChanged?: () => Promise<void>;
  now: number;
}
export function RecordingProcessingCard({
  group,
  onInspect,
  onChanged,
  now,
}: RecordingProcessingCardProps) {
  const { pending, error, perform } = useRecordingProcessingActions(
    group.recordingId,
    onChanged,
  );
  const [selected, setSelected] = useState<RecordingProcessingLaneKind | null>(
    () => group.lanes.find((lane) => lane.failed)?.kind ?? null,
  );
  const select = useCallback(
    (kind: RecordingProcessingLaneKind) =>
      setSelected((current) => (current === kind ? null : kind)),
    [],
  );
  const selectedLane = group.lanes.find((lane) => lane.kind === selected);
  const runningCount = group.lanes.reduce((sum, lane) => sum + lane.running, 0);
  const queuedCount = group.lanes.reduce((sum, lane) => sum + lane.queued, 0);
  const pause = useCallback(() => void perform({ action: 'pause' }), [perform]);
  const resume = useCallback(
    () => void perform({ action: 'resume' }),
    [perform],
  );
  const cancel = useCallback(
    () => void perform({ action: 'cancel' }),
    [perform],
  );
  const retry = useCallback(
    (sourceKind: RecordingProcessingLaneKind, useCurrentSettings: boolean) =>
      void perform({ action: 'retry', sourceKind, useCurrentSettings }),
    [perform],
  );
  const inspect = useCallback(
    (kind: RecordingProcessingLaneKind) => onInspect(group.recordingId, kind),
    [group.recordingId, onInspect],
  );
  const running = group.status === 'running' || group.status === 'queued';
  const coverage = group.indexCoverage;
  return (
    <li
      className="bg-card rounded-panel min-w-0 p-4"
      aria-label={group.title}
      aria-busy={pending}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold wrap-break-word">
            {group.title}
          </h2>
          <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
            <span>
              {group.lanes.some((lane) => lane.applicability === 'protected')
                ? 'Waiting for recording protection'
                : taskStatusLabel(group.status)}
            </span>
            {group.startedAtMs !== null ? (
              <time dateTime={new Date(group.startedAtMs).toISOString()}>
                {new Date(group.startedAtMs).toLocaleString()}
              </time>
            ) : null}
            <span>
              {group.inputOpen
                ? 'Live recording · no fixed finish time'
                : runningCount || queuedCount
                  ? `${runningCount} running · ${queuedCount} queued`
                  : 'Processing finished'}
            </span>
            {group.etaMs !== null && !group.inputOpen ? (
              <span>About {formatElapsed(group.etaMs)} remaining</span>
            ) : null}
          </div>
        </div>
        <div className="flex flex-wrap gap-1">
          {running && group.jobIds.length ? (
            <Button
              size="sm"
              variant="outline"
              disabled={pending}
              onClick={pause}
            >
              Pause recording processing
            </Button>
          ) : null}
          {group.status === 'paused' ||
          group.lanes.some((lane) => lane.status === 'paused') ? (
            <Button
              size="sm"
              variant="outline"
              disabled={pending}
              onClick={resume}
            >
              Resume recording processing
            </Button>
          ) : null}
          {['running', 'queued', 'paused'].includes(group.status) &&
          group.jobIds.length ? (
            <Button
              size="sm"
              variant="ghost"
              disabled={pending}
              onClick={cancel}
            >
              Cancel recording processing
            </Button>
          ) : null}
          <Button asChild size="sm" variant="ghost">
            <Link
              to="/library/$recordingId"
              params={{ recordingId: group.recordingId }}
            >
              Open recording
            </Link>
          </Button>
        </div>
      </div>
      {error ? (
        <p role="alert" className="text-destructive mt-3 text-sm">
          {error}
        </p>
      ) : null}
      <RecordingProcessingPipeline
        lanes={group.lanes}
        selected={selected}
        onSelect={select}
      />
      {selectedLane ? (
        <div
          className="bg-muted/40 mt-3 rounded-lg px-3"
          aria-label="Selected stage details"
        >
          <ul aria-label={`${group.title} processing lanes`}>
            <RecordingProcessingLaneRow
              lane={selectedLane}
              pending={pending}
              onInspect={inspect}
              onRetry={retry}
            />
          </ul>
          <ProcessingActivityList
            activity={selectedLane.activity ?? []}
            now={now}
          />
        </div>
      ) : null}
      <SharedDisclosure
        label="Search readiness"
        status={
          coverage.applicability === 'available'
            ? `${coverage.indexed}/${coverage.total} ready`
            : coverage.applicability === 'disabled'
              ? 'Disabled'
              : 'No text or transcripts available'
        }
        className="bg-muted/40 mt-1"
      >
        {coverage.applicability === 'available' ? (
          <>
            <Progress
              aria-label="Available text and transcripts indexed"
              value={
                coverage.total
                  ? (100 * coverage.indexed) / coverage.total
                  : null
              }
            />
            <p className="text-muted-foreground mt-2 text-xs">
              {coverage.pending
                ? `${coverage.pending} eligible passages waiting for the shared index worker.`
                : 'Current eligible passages are ready for semantic search.'}{' '}
              New processing results can add more text and transcripts.
            </p>
          </>
        ) : (
          <p className="text-muted-foreground text-xs">
            {coverage.applicability === 'disabled'
              ? 'Configure Recall embeddings in AI settings to enable semantic search.'
              : 'Eligible OCR and transcript results appear here as they become available.'}
          </p>
        )}
        <Button asChild size="sm" variant="link" className="mt-2 px-0">
          <Link to="/settings" search={{ tab: 'ai', focus: 'recall' }}>
            Recall settings
          </Link>
        </Button>
      </SharedDisclosure>
    </li>
  );
}
