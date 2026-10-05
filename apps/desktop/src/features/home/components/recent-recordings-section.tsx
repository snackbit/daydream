import { useCallback } from 'react';
import { Link } from '@tanstack/react-router';
import {
  AlertCircle,
  ArrowUpRight,
  Check,
  CircleDot,
  LoaderCircle,
  TriangleAlert,
} from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  availabilityLabel,
  formatDuration,
  formatFileSize,
  formatTimestamp,
} from '@/features/library/library-format';
import type {
  HomeRecordingsActions,
  HomeRecordingsModel,
} from '../use-home-recordings-controller';

interface RecentRecordingsSectionProps {
  actions: HomeRecordingsActions;
  model: HomeRecordingsModel;
}

export function RecentRecordingsSection({
  actions,
  model,
}: RecentRecordingsSectionProps) {
  const refresh = actions.refresh;
  const handleRefresh = useCallback(() => {
    void refresh();
  }, [refresh]);
  const isLoading = model.status === 'idle' || model.status === 'loading';

  return (
    <section aria-labelledby="recent-recordings-heading" className="min-w-0">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="flex min-w-0 items-baseline gap-2">
          <h2
            className="text-[13px] font-semibold tracking-tight"
            id="recent-recordings-heading"
          >
            Recent recordings
          </h2>
          {model.status === 'ready' ? (
            <span className="text-muted-foreground text-[11px] tabular-nums">
              {model.recordingCount} · {formatFileSize(model.librarySizeBytes)}
            </span>
          ) : null}
        </div>
        <Button asChild className="gap-1.5" size="sm" variant="ghost">
          <Link to="/library">
            View all
            <ArrowUpRight aria-hidden="true" className="size-3" />
          </Link>
        </Button>
      </div>

      {model.error ? (
        <Alert className="p-3" variant="destructive">
          <AlertCircle aria-hidden="true" className="size-3.5" />
          <AlertTitle>Could not load recent recordings</AlertTitle>
          <div className="flex flex-wrap items-start justify-between gap-2">
            <AlertDescription>{model.error}</AlertDescription>
            <Button
              disabled={model.status === 'loading'}
              onClick={handleRefresh}
              size="sm"
              variant="outline"
            >
              Try again
            </Button>
          </div>
        </Alert>
      ) : null}

      {isLoading && model.recentRecordings.length === 0 ? (
        <div
          aria-busy="true"
          className="flex min-h-14 items-center gap-2 border-y px-2"
          role="status"
        >
          <LoaderCircle
            aria-hidden="true"
            className="text-muted-foreground size-3.5 animate-spin"
          />
          <span className="text-muted-foreground text-xs">
            Loading local recordings…
          </span>
        </div>
      ) : null}

      {isLoading && model.recentRecordings.length > 0 ? (
        <p
          className="text-muted-foreground border-y px-2 py-1.5 text-[11px]"
          role="status"
        >
          Updating recordings…
        </p>
      ) : null}

      {model.status === 'ready' && model.recordingCount === 0 ? (
        <div className="border-y px-2 py-4" role="status">
          <p className="text-[13px] font-medium">No recordings yet</p>
          <p className="text-muted-foreground mt-1 text-xs leading-5">
            Finished sessions appear here after you start and stop a recording.
          </p>
        </div>
      ) : null}

      {model.recentRecordings.length > 0 ? (
        <div className="divide-y border-y">
          {model.recentRecordings.map((recording) => (
            <RecentRecordingRow key={recording.id} recording={recording} />
          ))}
        </div>
      ) : null}
    </section>
  );
}

function RecentRecordingRow({
  recording,
}: {
  recording: HomeRecordingsModel['recentRecordings'][number];
}) {
  const title = recording.title ?? 'Recording';
  const rowContents = (
    <>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-xs font-medium">{title}</span>
        <span className="text-muted-foreground mt-0.5 block truncate text-[11px] tabular-nums">
          {formatTimestamp(recording.startedAtMs)} ·{' '}
          {formatDuration(recording.durationMs)} · {recording.segmentCount}{' '}
          {recording.segmentCount === 1 ? 'segment' : 'segments'} ·{' '}
          {formatFileSize(recording.sizeBytes)}
        </span>
      </span>
      <RecordingAvailability recording={recording} />
    </>
  );

  if (recording.availability === 'available') {
    return (
      <Link
        aria-label={`Open ${title}, ${availabilityLabel(recording.availability)}, ${formatTimestamp(recording.startedAtMs)}`}
        className="hover:bg-muted/60 focus-visible:ring-ring flex min-h-12 min-w-0 items-center gap-3 px-2 py-1.5 outline-none focus-visible:ring-2 focus-visible:ring-inset"
        params={{ recordingId: recording.id }}
        to="/library/$recordingId"
      >
        {rowContents}
      </Link>
    );
  }

  return (
    <div className="flex min-h-12 min-w-0 items-center gap-3 px-2 py-1.5">
      {rowContents}
    </div>
  );
}

function RecordingAvailability({
  recording,
}: {
  recording: HomeRecordingsModel['recentRecordings'][number];
}) {
  const Icon =
    recording.availability === 'available'
      ? Check
      : recording.availability === 'locked'
        ? CircleDot
        : TriangleAlert;
  const className =
    recording.availability === 'locked'
      ? 'text-primary'
      : recording.availability === 'missing-files'
        ? 'text-destructive'
        : 'text-muted-foreground';

  return (
    <span className={`flex shrink-0 items-center gap-1.5 ${className}`}>
      <Icon aria-hidden="true" className="size-3" />
      <span className="text-[11px]">
        {availabilityLabel(recording.availability)}
      </span>
    </span>
  );
}
