import { useCallback, useState, type MouseEvent } from 'react';
import type {
  LibraryRecordingSummary,
  LibraryRecordingViewMode,
} from '@daydream/shared';
import { Clock3, FileWarning } from 'lucide-react';
import { Checkbox } from '@/components/ui/checkbox';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import { localAssetUrl } from '@/features/library/library-client';
import {
  availabilityLabel,
  formatDuration,
  formatFileSize,
  formatTimestamp,
} from '@/features/library/library-format';
import { cn } from '@/lib/utils';
import {
  isRecordingDeletable,
  isRecordingRenameable,
  isRecordingSelectable,
} from './library-overview-utils';
import { RecordingLibraryItemActions } from './recording-library-item-actions';
import type { LibraryOverviewActions } from './use-library-overview-controller';

interface RecordingLibraryItemProps {
  actions: Pick<
    LibraryOverviewActions,
    | 'openRecording'
    | 'requestRecordingDelete'
    | 'requestRecordingRename'
    | 'selectRecording'
    | 'setRecordingSelected'
  >;
  isSelected: boolean;
  recording: LibraryRecordingSummary;
  thumbnailPath: string | null;
  thumbnailState?: 'loading' | 'ready' | 'hidden' | 'unavailable';
  viewMode: LibraryRecordingViewMode;
}

export function RecordingLibraryItem({
  actions,
  isSelected,
  recording,
  thumbnailPath,
  thumbnailState = 'loading',
  viewMode,
}: RecordingLibraryItemProps) {
  const {
    openRecording,
    requestRecordingDelete,
    requestRecordingRename,
    selectRecording,
    setRecordingSelected,
  } = actions;
  const displayTitle = recording.title ?? 'Recording';
  const timestampTitle = formatTimestamp(recording.startedAtMs);
  const actionTitle = `${displayTitle}, ${timestampTitle}`;
  const playable = recording.availability === 'available';

  const handleSelect = useCallback(
    (event: MouseEvent<HTMLElement>) => {
      if (!isLibraryAction(event.target)) {
        selectRecording(recording, event);
      }
    },
    [recording, selectRecording],
  );

  const handleContextMenu = useCallback(
    (event: MouseEvent<HTMLElement>) => {
      if (!isSelected && isRecordingSelectable(recording)) {
        selectRecording(recording, event);
      }
    },
    [isSelected, recording, selectRecording],
  );

  const handleOpenFromDoubleClick = useCallback(
    (event: MouseEvent<HTMLElement>) => {
      if (!isLibraryAction(event.target)) {
        openRecording(recording);
      }
    },
    [openRecording, recording],
  );

  const handleDelete = useCallback(() => {
    requestRecordingDelete(recording);
  }, [recording, requestRecordingDelete]);

  const handleOpen = useCallback(() => {
    openRecording(recording);
  }, [openRecording, recording]);

  const handleRename = useCallback(() => {
    requestRecordingRename(recording);
  }, [recording, requestRecordingRename]);

  const handleSelectionChange = useCallback(
    (checked: boolean | 'indeterminate') => {
      setRecordingSelected(recording, checked === true);
    },
    [recording, setRecordingSelected],
  );

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <article
          className={cn(
            'hover:bg-muted/35 data-[selected=true]:bg-muted/60 min-w-0 border-b transition-colors',
            viewMode === 'grid'
              ? 'flex flex-col'
              : 'flex items-center gap-3 px-2 py-2',
          )}
          data-selected={isSelected}
          onClick={handleSelect}
          onContextMenu={handleContextMenu}
          onDoubleClick={handleOpenFromDoubleClick}
        >
          <RecordingThumbnail
            displayTitle={displayTitle}
            recording={recording}
            thumbnailPath={thumbnailPath}
            thumbnailState={thumbnailState}
            viewMode={viewMode}
          />
          <div
            className={cn(
              'min-w-0 gap-x-2.5 gap-y-1.5',
              viewMode === 'grid'
                ? 'grid grid-cols-[16px_minmax(0,1fr)] px-2 py-2.5 sm:px-3'
                : 'flex flex-1 items-center gap-3 px-1 py-0.5',
            )}
          >
            <Checkbox
              aria-label={`Select ${actionTitle}`}
              checked={isSelected}
              className="mt-0.5"
              data-library-action
              disabled={!isRecordingSelectable(recording)}
              onCheckedChange={handleSelectionChange}
            />

            <div className="min-w-0">
              <div className="flex min-w-0 items-start justify-between gap-2">
                <span
                  className={cn(
                    'min-w-0 text-xs leading-4 font-medium',
                    viewMode === 'grid' ? 'line-clamp-2' : 'truncate',
                  )}
                >
                  {displayTitle}
                </span>
                <RecordingAvailability recording={recording} />
              </div>
              <div className="text-muted-foreground mt-0.5 flex min-w-0 flex-wrap gap-x-1.5 text-[10px] leading-4 tabular-nums">
                <span>{timestampTitle}</span>
                <span aria-hidden="true">·</span>
                <span>{formatDuration(recording.durationMs)}</span>
                <span aria-hidden="true">·</span>
                <span>
                  {recording.segmentCount}{' '}
                  {recording.segmentCount === 1 ? 'segment' : 'segments'}
                </span>
                <span aria-hidden="true">·</span>
                <span>{formatFileSize(recording.sizeBytes)}</span>
              </div>
              {recording.missingReasons[0] ? (
                <p className="text-muted-foreground mt-0.5 truncate text-[11px]">
                  {recording.missingReasons[0]}
                </p>
              ) : null}
            </div>

            <div
              className={
                viewMode === 'grid'
                  ? 'col-start-2 flex justify-end border-t pt-1'
                  : ''
              }
            >
              <RecordingLibraryItemActions
                deletable={isRecordingDeletable(recording)}
                onDelete={handleDelete}
                onOpen={handleOpen}
                onRename={handleRename}
                playable={playable}
                playUnavailableReason={
                  recording.availability === 'locked'
                    ? 'Recording is still in progress.'
                    : (recording.missingReasons[0] ??
                      availabilityLabel(recording.availability))
                }
                recordingTitle={actionTitle}
                renameable={isRecordingRenameable(recording)}
              />
            </div>
          </div>
        </article>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem
          disabled={!isRecordingRenameable(recording)}
          onSelect={handleRename}
        >
          Rename recording
        </ContextMenuItem>
        <ContextMenuItem
          disabled={!isRecordingDeletable(recording)}
          onSelect={handleDelete}
          variant="destructive"
        >
          Delete recording
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

function RecordingThumbnail({
  displayTitle,
  recording,
  thumbnailPath,
  thumbnailState,
  viewMode,
}: {
  displayTitle: string;
  recording: LibraryRecordingSummary;
  thumbnailPath: string | null;
  thumbnailState: 'loading' | 'ready' | 'hidden' | 'unavailable';
  viewMode: LibraryRecordingViewMode;
}) {
  const [failedPath, setFailedPath] = useState<string | null>(null);
  const sizeClassName =
    viewMode === 'grid'
      ? 'aspect-video w-full'
      : 'aspect-video w-24 shrink-0 sm:w-28';

  if (recording.availability === 'locked') {
    return (
      <div
        className={cn(
          'bg-muted text-muted-foreground flex items-center justify-center gap-1.5 text-[11px]',
          sizeClassName,
        )}
      >
        <Clock3 aria-hidden="true" className="size-3" />
        Recording
      </div>
    );
  }

  if (recording.availability !== 'available') {
    return (
      <div
        className={cn(
          'bg-muted text-muted-foreground flex items-center justify-center gap-1.5 text-[11px]',
          sizeClassName,
        )}
      >
        <FileWarning aria-hidden="true" className="size-3" />
        Files unavailable
      </div>
    );
  }

  if (!thumbnailPath || failedPath === thumbnailPath) {
    return (
      <div
        aria-label={`${thumbnailState === 'loading' ? 'Preparing preview' : 'Preview unavailable'} for ${displayTitle}`}
        className={cn(
          'bg-muted text-muted-foreground flex items-center justify-center text-[11px]',
          sizeClassName,
        )}
        role="status"
      >
        {thumbnailState === 'hidden'
          ? 'Preview hidden'
          : thumbnailState === 'loading' && failedPath !== thumbnailPath
            ? 'Loading preview…'
            : 'Preview unavailable'}
      </div>
    );
  }

  return (
    <img
      alt={`Preview of ${displayTitle}`}
      className={cn('bg-black object-cover', sizeClassName)}
      src={localAssetUrl(thumbnailPath)}
      onError={() => setFailedPath(thumbnailPath)}
    />
  );
}

function RecordingAvailability({
  recording,
}: {
  recording: LibraryRecordingSummary;
}) {
  const availability = recording.availability;
  const textClassName =
    availability === 'locked'
      ? 'text-primary'
      : availability === 'missing-files'
        ? 'text-destructive'
        : 'text-muted-foreground';

  return (
    <span className={cn('shrink-0 text-[10px]', textClassName)}>
      {availabilityLabel(availability)}
    </span>
  );
}

function isLibraryAction(target: EventTarget): boolean {
  return (
    target instanceof Element &&
    target.closest('[data-library-action]') !== null
  );
}
