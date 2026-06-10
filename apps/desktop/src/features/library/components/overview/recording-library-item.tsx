import type { MouseEvent } from 'react';
import { Link } from '@tanstack/react-router';
import type {
  LibraryRecordingSummary,
  LibraryRecordingViewMode,
} from '@daydream/shared';
import { Lock, Pencil, Play, Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
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
  recordingTitle,
} from './library-overview-utils';

interface RecordingLibraryItemProps {
  isSelected: boolean;
  onDelete: () => void;
  onOpen: () => void;
  onRename: () => void;
  onSelect: (event: MouseEvent<HTMLElement>) => void;
  recording: LibraryRecordingSummary;
  thumbnailPath: string | null;
  viewMode: LibraryRecordingViewMode;
}

export function RecordingLibraryItem({
  isSelected,
  onDelete,
  onOpen,
  onRename,
  onSelect,
  recording,
  thumbnailPath,
  viewMode,
}: RecordingLibraryItemProps) {
  const playable = recording.availability === 'available';
  const locked = recording.availability === 'locked';
  const displayTitle = recordingTitle(recording);
  const timestampTitle = formatTimestamp(recording.startedAtMs);

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <Card
          aria-disabled={locked}
          aria-selected={isSelected}
          className={cn(
            'cursor-default',
            viewMode === 'list'
              ? 'grid grid-cols-[12rem_1fr] gap-0 py-0'
              : 'py-0',
            locked && 'opacity-75',
            isSelected && 'ring-primary ring-2',
          )}
          onClick={(event) => {
            if (event.target instanceof Element) {
              if (event.target.closest('[data-library-action]')) {
                return;
              }
            }
            onSelect(event);
          }}
          onContextMenu={(event) => {
            if (!isSelected && !locked) {
              onSelect(event);
            }
          }}
          onDoubleClick={(event) => {
            if (event.target instanceof Element) {
              if (event.target.closest('[data-library-action]')) {
                return;
              }
            }
            onOpen();
          }}
        >
          <RecordingThumbnail
            recording={recording}
            thumbnailPath={thumbnailPath}
          />
          <div className="flex min-w-0 flex-col">
            <CardHeader className="min-w-0 pt-4">
              <CardTitle className="truncate">{displayTitle}</CardTitle>
              <CardDescription>
                {recording.title ? `${timestampTitle} · ` : null}
                {formatDuration(recording.durationMs)} ·{' '}
                {recording.segmentCount} segment
                {recording.segmentCount === 1 ? '' : 's'} ·{' '}
                {formatFileSize(recording.sizeBytes)}
              </CardDescription>
              <CardAction>
                <Badge variant={playable ? 'secondary' : 'outline'}>
                  {locked ? <Lock data-icon="inline-start" /> : null}
                  {availabilityLabel(recording.availability)}
                </Badge>
              </CardAction>
            </CardHeader>
            <CardContent className="flex flex-1 items-end justify-between gap-3 pb-4">
              <div className="text-muted-foreground min-w-0 text-xs">
                {recording.missingReasons[0] ?? recording.status}
              </div>
              {playable ? (
                <Button asChild className="gap-2" size="sm">
                  <Link
                    data-library-action
                    params={{ recordingId: recording.id }}
                    to="/library/$recordingId"
                  >
                    <Play className="size-3.5" />
                    Play
                  </Link>
                </Button>
              ) : (
                <Button
                  className="gap-2"
                  data-library-action
                  disabled
                  size="sm"
                  variant="outline"
                >
                  <Play className="size-3.5" />
                  Play
                </Button>
              )}
            </CardContent>
          </div>
        </Card>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem
          className="gap-2"
          disabled={!isRecordingRenameable(recording)}
          onSelect={onRename}
        >
          <Pencil className="size-3.5" />
          Rename recording
        </ContextMenuItem>
        <ContextMenuItem
          className="gap-2"
          disabled={!isRecordingDeletable(recording)}
          onSelect={onDelete}
          variant="destructive"
        >
          <Trash2 className="size-3.5" />
          Delete recording
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

function RecordingThumbnail({
  recording,
  thumbnailPath,
}: {
  recording: LibraryRecordingSummary;
  thumbnailPath: string | null;
}) {
  if (recording.availability !== 'available') {
    return (
      <div className="bg-muted text-muted-foreground flex aspect-video min-h-28 items-center justify-center text-xs">
        {recording.availability === 'locked'
          ? 'Recording in progress'
          : 'Missing video files'}
      </div>
    );
  }

  if (!thumbnailPath) {
    return (
      <div className="bg-muted text-muted-foreground flex aspect-video min-h-28 items-center justify-center text-xs">
        Thumbnail loading...
      </div>
    );
  }

  return (
    <img
      aria-label="Recording thumbnail preview"
      className="aspect-video min-h-28 w-full bg-black object-cover"
      src={localAssetUrl(thumbnailPath)}
    />
  );
}
