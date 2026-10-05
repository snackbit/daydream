import { useMemo, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { AlertTriangle, LoaderCircle, Video } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { groupRecordingsByDate } from './library-overview-utils';
import { RecordingLibraryItem } from './recording-library-item';
import type {
  LibraryOverviewActions,
  LibraryOverviewModel,
} from './use-library-overview-controller';

interface LibraryOverviewContentProps {
  actions: Pick<
    LibraryOverviewActions,
    | 'openRecording'
    | 'refresh'
    | 'requestRecordingDelete'
    | 'requestRecordingRename'
    | 'selectRecording'
    | 'setRecordingSelected'
  >;
  model: Pick<
    LibraryOverviewModel,
    | 'error'
    | 'selectedIdSet'
    | 'sortKey'
    | 'sortedRecordings'
    | 'status'
    | 'thumbnailPaths'
    | 'viewMode'
  >;
}

export function LibraryOverviewContent({
  actions,
  model,
}: LibraryOverviewContentProps) {
  const [nowMs] = useState(() => Date.now());
  const recordingGroups = useMemo(
    () => groupRecordingsByDate(model.sortedRecordings, model.sortKey, nowMs),
    [model.sortKey, model.sortedRecordings, nowMs],
  );

  return (
    <div className="grid gap-5">
      {model.error ? (
        <Alert className="p-4" variant="destructive">
          <AlertTriangle aria-hidden="true" className="size-4" />
          <AlertTitle>
            {model.sortedRecordings.length === 0
              ? "Couldn't load recordings"
              : "Couldn't refresh recordings"}
          </AlertTitle>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <AlertDescription>{model.error}</AlertDescription>
            <Button
              disabled={model.status === 'loading'}
              onClick={actions.refresh}
              size="sm"
              variant="outline"
            >
              Try again
            </Button>
          </div>
        </Alert>
      ) : null}

      {model.status === 'idle' || model.status === 'loading' ? (
        <div
          aria-busy="true"
          className="flex min-h-12 items-center gap-2 border-y px-2"
          role="status"
        >
          <LoaderCircle
            aria-hidden="true"
            className="text-muted-foreground size-3.5 animate-spin"
          />
          <span className="text-muted-foreground text-xs">
            Loading your local recordings…
          </span>
        </div>
      ) : null}

      {model.status === 'ready' && model.sortedRecordings.length === 0 ? (
        <div className="border-y px-2 py-4">
          <h2 className="text-[13px] font-medium">Your Library is empty</h2>
          <p className="text-muted-foreground mt-1 text-xs leading-5">
            Finished sessions appear here when you stop capture.
          </p>
          <Button asChild className="mt-2 gap-1.5" size="sm">
            <Link to="/capture">
              <Video aria-hidden="true" className="size-3" />
              Set up a recording
            </Link>
          </Button>
        </div>
      ) : null}

      {recordingGroups.map((group) => (
        <section
          aria-labelledby={`recording-group-${group.id}`}
          className="grid gap-3"
          key={group.id}
        >
          <div className="flex items-center justify-between gap-3 border-b pb-2">
            <h2
              className="text-[13px] font-semibold tracking-tight"
              id={`recording-group-${group.id}`}
            >
              {group.label}
            </h2>
            <span className="text-muted-foreground text-[11px] tabular-nums">
              {group.recordings.length}{' '}
              {group.recordings.length === 1 ? 'recording' : 'recordings'}
            </span>
          </div>
          <div
            className={
              model.viewMode === 'grid'
                ? 'grid grid-cols-1 gap-3 min-[520px]:grid-cols-2 xl:grid-cols-3'
                : 'flex flex-col gap-3'
            }
          >
            {group.recordings.map((recording) => (
              <RecordingLibraryItem
                actions={actions}
                isSelected={model.selectedIdSet.has(recording.id)}
                key={recording.id}
                recording={recording}
                thumbnailPath={model.thumbnailPaths[recording.id] ?? null}
                viewMode={model.viewMode}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
