import { useState } from 'react';
import { Link } from '@tanstack/react-router';
import type {
  LibraryDayCard as LibraryDayCardData,
  LibraryDaysPage,
} from '@daydream/shared';
import { ArrowUpRight, CalendarDays } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { localAssetUrl } from '@/features/library/library-client';
import {
  recallDateLabel,
  recallDayStatusLabel,
  recordedDuration,
} from '../recall-format';
import { useRecallDayThumbnail } from './use-recall-day-thumbnail';

export interface LibraryDayCardProps {
  day: LibraryDayCardData;
  prominent?: boolean;
  status?: Pick<
    LibraryDaysPage,
    'generationReady' | 'generationUnavailableReason'
  >;
}
export function LibraryDayCard({
  day,
  prominent = false,
  status,
}: LibraryDayCardProps) {
  const preview = useRecallDayThumbnail(day);
  const [failedPath, setFailedPath] = useState<string | null>(null);
  const thumbnailPath = preview.path;
  return (
    <article
      className="border-border hover:bg-muted/50 focus-within:ring-ring relative flex flex-col gap-6 rounded-[20px] border p-6 focus-within:ring-2 sm:flex-row sm:items-center"
      data-prominent={prominent}
    >
      <div className="bg-muted flex size-24 shrink-0 items-center justify-center overflow-hidden rounded-2xl">
        {thumbnailPath && failedPath !== thumbnailPath ? (
          <img
            alt="Privacy-aware recording preview"
            className="size-full object-cover"
            src={localAssetUrl(thumbnailPath)}
            onError={() => setFailedPath(thumbnailPath)}
          />
        ) : (
          <div className="text-muted-foreground grid justify-items-center gap-2 p-2 text-center text-xs">
            <CalendarDays aria-hidden="true" className="size-7" />
            <span>
              {preview.state === 'hidden'
                ? 'Preview hidden'
                : preview.state === 'loading'
                  ? 'Loading preview…'
                  : 'Preview unavailable'}
            </span>
          </div>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <h2 className="text-2xl font-light tracking-tight">
          {recallDateLabel(day.dayId)}
        </h2>
        <p className="text-muted-foreground mt-3 text-xs">
          {day.recordingCount} recordings ·{' '}
          {recordedDuration(day.capturedDurationMs)} recorded ·{' '}
          {day.evidenceCount} text and transcript entries
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-4 sm:flex-col sm:items-end">
        <span className="text-muted-foreground text-xs">
          {day.reviewed ? 'Reviewed' : 'Not reviewed'}
        </span>
        <Badge variant="outline">
          {status?.generationUnavailableReason === 'recall_generation_disabled'
            ? 'Summaries off'
            : day.status === 'failed'
              ? 'Summary failed'
              : status && !status.generationReady
                ? 'Summary setup incomplete'
                : day.recordingActive
                  ? 'Recording'
                  : day.reviewed &&
                      (day.status === 'ready' || day.status === 'partial')
                    ? 'Reviewed'
                    : recallDayStatusLabel[day.status]}
        </Badge>
        {status && !status.generationReady ? (
          <Link
            className="relative z-10 text-xs underline"
            to="/settings"
            search={{ tab: 'ai', focus: 'recall-generation' }}
          >
            Summary settings
          </Link>
        ) : null}
        <Link
          className="flex items-center gap-2 text-sm after:absolute after:inset-0 after:rounded-[20px]"
          to="/day/$date"
          params={{ date: day.dayId }}
          aria-label={`Open day ${recallDateLabel(day.dayId)}`}
        >
          Open day <ArrowUpRight aria-hidden="true" />
        </Link>
      </div>
    </article>
  );
}
