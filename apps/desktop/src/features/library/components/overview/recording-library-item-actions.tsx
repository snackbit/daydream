import { Pencil, Play, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';

export interface RecordingLibraryItemActionsProps {
  deletable: boolean;
  onDelete: () => void;
  onOpen: () => void;
  onRename: () => void;
  playable: boolean;
  playUnavailableReason: string | null;
  recordingTitle: string;
  renameable: boolean;
}

export function RecordingLibraryItemActions({
  deletable,
  onDelete,
  onOpen,
  onRename,
  playable,
  playUnavailableReason,
  recordingTitle,
  renameable,
}: RecordingLibraryItemActionsProps) {
  return (
    <div className="flex shrink-0 items-center gap-1">
      {renameable ? (
        <Button
          aria-label={`Rename ${recordingTitle}`}
          data-library-action
          onClick={onRename}
          size="icon-sm"
          variant="ghost"
        >
          <Pencil aria-hidden="true" />
        </Button>
      ) : null}
      {deletable ? (
        <Button
          aria-label={`Delete ${recordingTitle}`}
          data-library-action
          onClick={onDelete}
          size="icon-sm"
          variant="ghost"
        >
          <Trash2 aria-hidden="true" />
        </Button>
      ) : null}
      {playable ? (
        <Button
          aria-label={`Play ${recordingTitle}`}
          className="gap-1.5"
          data-library-action
          onClick={onOpen}
          size="sm"
          variant="ghost"
        >
          <Play aria-hidden="true" className="size-3" />
          Play
        </Button>
      ) : (
        <Button
          aria-label={`Play ${recordingTitle}: ${playUnavailableReason}`}
          data-library-action
          disabled
          size="sm"
          variant="ghost"
        >
          <Play aria-hidden="true" className="size-3" />
          Play
        </Button>
      )}
    </div>
  );
}
