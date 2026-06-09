import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from 'react';
import { Link, useNavigate } from '@tanstack/react-router';
import type {
  LibraryRecordingSortKey,
  LibraryRecordingSummary,
  LibraryRecordingViewMode,
} from '@daydream/shared';
import {
  AlertTriangle,
  Grid2X2,
  List,
  Lock,
  Pencil,
  Play,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  deleteLibraryRecordings,
  deleteLibraryRecording,
  listLibraryRecordings,
  localAssetUrl,
  prepareLibraryRecordingThumbnail,
  renameLibraryRecording,
} from '@/features/library/library-client';
import {
  availabilityLabel,
  formatDuration,
  formatFileSize,
  formatTimestamp,
} from '@/features/library/library-format';
import { cn } from '@/lib/utils';

interface DeleteTarget {
  ids: string[];
  label: string;
}

interface RenameTarget {
  fallbackTitle: string;
  id: string;
}

const RECORDING_TITLE_MAX_CHARS = 120;

export function LibraryScreen() {
  const navigate = useNavigate();
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [renameSaving, setRenameSaving] = useState(false);
  const [renameTarget, setRenameTarget] = useState<RenameTarget | null>(null);
  const [renameTitle, setRenameTitle] = useState('');
  const thumbnailLoadRunId = useRef(0);
  const [selectionAnchorId, setSelectionAnchorId] = useState<string | null>(
    null,
  );
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [recordings, setRecordings] = useState<LibraryRecordingSummary[]>([]);
  const [sortKey, setSortKey] = useState<LibraryRecordingSortKey>('started');
  const [status, setStatus] = useState<'error' | 'idle' | 'loading' | 'ready'>(
    'idle',
  );
  const [thumbnailPaths, setThumbnailPaths] = useState<Record<string, string>>(
    {},
  );
  const [viewMode, setViewMode] = useState<LibraryRecordingViewMode>('grid');

  const sortedRecordings = useMemo(
    () => sortRecordings(recordings, sortKey),
    [recordings, sortKey],
  );
  const selectedRecordings = useMemo(
    () => recordings.filter((recording) => selectedIds.includes(recording.id)),
    [recordings, selectedIds],
  );
  const selectedSizeBytes = selectedRecordings.reduce(
    (total, recording) => total + recording.sizeBytes,
    0,
  );
  const librarySizeBytes = recordings.reduce(
    (total, recording) => total + recording.sizeBytes,
    0,
  );

  const loadOverviewThumbnails = useCallback(
    async (nextRecordings: LibraryRecordingSummary[], runId: number) => {
      for (const recording of nextRecordings) {
        if (recording.availability !== 'available') {
          continue;
        }
        const thumbnail = await prepareLibraryRecordingThumbnail({
          recordingId: recording.id,
        }).catch(() => null);
        if (thumbnailLoadRunId.current !== runId) {
          return;
        }
        const thumbnailPath = thumbnail?.path;
        if (thumbnail?.error === null && thumbnailPath) {
          setThumbnailPaths((current) => ({
            ...current,
            [thumbnail.recordingId]: thumbnailPath,
          }));
        }
      }
    },
    [],
  );

  const load = useCallback(async () => {
    setError(null);
    setStatus('loading');
    try {
      const nextRecordings = await listLibraryRecordings();
      const nextRecordingIds = new Set(
        nextRecordings
          .filter(isRecordingSelectable)
          .map((recording) => recording.id),
      );
      setRecordings(nextRecordings);
      setThumbnailPaths((current) =>
        keepThumbnailPaths(current, nextRecordings),
      );
      setSelectedIds((current) =>
        current.filter((recordingId) => nextRecordingIds.has(recordingId)),
      );
      setStatus('ready');
      const nextThumbnailRunId = thumbnailLoadRunId.current + 1;
      thumbnailLoadRunId.current = nextThumbnailRunId;
      void loadOverviewThumbnails(nextRecordings, nextThumbnailRunId);
    } catch (loadError) {
      setError(errorMessage(loadError));
      setStatus('error');
    }
  }, [loadOverviewThumbnails]);

  useEffect(() => {
    void Promise.resolve().then(load);
    return () => {
      thumbnailLoadRunId.current += 1;
    };
  }, [load]);

  async function confirmDelete() {
    if (!deleteTarget) {
      return;
    }
    setError(null);
    try {
      if (deleteTarget.ids.length === 1) {
        await deleteLibraryRecording({ recordingId: deleteTarget.ids[0]! });
      } else {
        await deleteLibraryRecordings({ recordingIds: deleteTarget.ids });
      }
      setRecordings((current) =>
        current.filter((recording) => !deleteTarget.ids.includes(recording.id)),
      );
      setSelectedIds((current) =>
        current.filter(
          (recordingId) => !deleteTarget.ids.includes(recordingId),
        ),
      );
      setDeleteTarget(null);
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  }

  async function confirmRename() {
    if (!renameTarget) {
      return;
    }
    const trimmedTitle = renameTitle.trim();
    setError(null);
    setRenameSaving(true);
    try {
      const detail = await renameLibraryRecording({
        recordingId: renameTarget.id,
        title: trimmedTitle.length > 0 ? trimmedTitle : null,
      });
      setRecordings((current) =>
        current.map((recording) =>
          recording.id === detail.id
            ? {
                ...recording,
                title: detail.title,
              }
            : recording,
        ),
      );
      setRenameTarget(null);
      setRenameTitle('');
    } catch (renameError) {
      setError(errorMessage(renameError));
    } finally {
      setRenameSaving(false);
    }
  }

  function selectRecording(
    recording: LibraryRecordingSummary,
    event: MouseEvent<HTMLElement>,
  ) {
    if (!isRecordingSelectable(recording)) {
      return;
    }

    const modifierPressed = event.ctrlKey || event.metaKey;
    if (event.shiftKey && selectionAnchorId) {
      const nextRange = selectedRange(
        sortedRecordings,
        selectionAnchorId,
        recording.id,
      );
      setSelectedIds((current) =>
        modifierPressed ? uniqueIds([...current, ...nextRange]) : nextRange,
      );
      return;
    }

    if (modifierPressed) {
      setSelectedIds((current) =>
        current.includes(recording.id)
          ? current.filter((id) => id !== recording.id)
          : [...current, recording.id],
      );
      setSelectionAnchorId(recording.id);
      return;
    }

    setSelectedIds([recording.id]);
    setSelectionAnchorId(recording.id);
  }

  function deleteSelectionTarget(
    recording: LibraryRecordingSummary,
  ): DeleteTarget | null {
    const selectedRecordingIds = selectedIds.includes(recording.id)
      ? selectedIds
      : [recording.id];
    const deletableRecordingIds = selectedRecordingIds.filter((recordingId) => {
      const selectedRecording = recordings.find(
        (candidate) => candidate.id === recordingId,
      );

      return selectedRecording
        ? isRecordingDeletable(selectedRecording)
        : false;
    });

    if (deletableRecordingIds.length === 0) {
      return null;
    }

    return {
      ids: deletableRecordingIds,
      label:
        deletableRecordingIds.length === 1
          ? '1 recording'
          : `${deletableRecordingIds.length} recordings`,
    };
  }

  function openDeleteDialog(target: DeleteTarget | null) {
    if (target) {
      setDeleteTarget(target);
    }
  }

  function openRenameDialog(recording: LibraryRecordingSummary) {
    if (!isRecordingRenameable(recording)) {
      return;
    }
    setRenameTarget({
      fallbackTitle: formatTimestamp(recording.startedAtMs),
      id: recording.id,
    });
    setRenameTitle(recording.title ?? '');
  }

  function openRecording(recording: LibraryRecordingSummary) {
    if (recording.availability !== 'available') {
      return;
    }

    void navigate({
      params: { recordingId: recording.id },
      to: '/library/$recordingId',
    });
  }

  return (
    <PageLayout>
      <PageHeader
        title="Library"
        description="Local recordings stored on this machine."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className="text-muted-foreground mr-2 text-xs">
              {selectedIds.length > 0
                ? `${selectedIds.length} selected · ${formatFileSize(
                    selectedSizeBytes,
                  )}`
                : `${recordings.length} recordings · ${formatFileSize(
                    librarySizeBytes,
                  )} library size`}
            </div>
            {selectedIds.length > 0 ? (
              <Button
                className="gap-2"
                variant="destructive"
                onClick={() => {
                  openDeleteDialog({
                    ids: selectedIds,
                    label:
                      selectedIds.length === 1
                        ? '1 recording'
                        : `${selectedIds.length} recordings`,
                  });
                }}
              >
                <Trash2 className="size-3.5" />
                Delete
              </Button>
            ) : null}
            <Select
              onValueChange={(value) =>
                setSortKey(value as LibraryRecordingSortKey)
              }
              value={sortKey}
            >
              <SelectTrigger aria-label="Sort recordings">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="started">Newest first</SelectItem>
                <SelectItem value="duration">Duration</SelectItem>
                <SelectItem value="status">Status</SelectItem>
                <SelectItem value="availability">Availability</SelectItem>
              </SelectContent>
            </Select>
            <div className="flex rounded-md border">
              <Button
                aria-label="Grid view"
                size="icon"
                variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
                onClick={() => setViewMode('grid')}
              >
                <Grid2X2 className="size-4" />
              </Button>
              <Button
                aria-label="List view"
                size="icon"
                variant={viewMode === 'list' ? 'secondary' : 'ghost'}
                onClick={() => setViewMode('list')}
              >
                <List className="size-4" />
              </Button>
            </div>
            <Button
              className="gap-2"
              variant="outline"
              onClick={() => void load()}
            >
              <RefreshCw className="size-3.5" />
              Refresh
            </Button>
          </div>
        }
      />

      {error ? (
        <Alert variant="destructive">
          <AlertTriangle />
          <AlertTitle>Library failed</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}

      {status === 'loading' ? (
        <div className="text-muted-foreground border p-4 text-sm">
          Loading recordings...
        </div>
      ) : null}

      {status !== 'loading' && sortedRecordings.length === 0 ? (
        <div className="text-muted-foreground border p-4 text-sm">
          No recordings yet.
        </div>
      ) : null}

      {sortedRecordings.length > 0 ? (
        <div
          className={
            viewMode === 'grid'
              ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-4'
              : 'flex flex-col gap-2'
          }
        >
          {sortedRecordings.map((recording) => (
            <RecordingLibraryItem
              key={recording.id}
              isSelected={selectedIds.includes(recording.id)}
              recording={recording}
              thumbnailPath={thumbnailPaths[recording.id] ?? null}
              viewMode={viewMode}
              onDelete={() =>
                openDeleteDialog(deleteSelectionTarget(recording))
              }
              onOpen={() => openRecording(recording)}
              onRename={() => openRenameDialog(recording)}
              onSelect={(event) => selectRecording(recording, event)}
            />
          ))}
        </div>
      ) : null}

      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete recording</DialogTitle>
            <DialogDescription>
              This removes {deleteTarget?.label ?? 'the selected recordings'}
              and any recording files still present in Daydream storage.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={() => void confirmDelete()}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={renameTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setRenameTarget(null);
            setRenameTitle('');
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename recording</DialogTitle>
            <DialogDescription>
              Set a display title for{' '}
              {renameTarget?.fallbackTitle ?? 'this recording'}. Leave it blank
              to use the recording timestamp.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="recording-title">Title</Label>
            <Input
              id="recording-title"
              maxLength={RECORDING_TITLE_MAX_CHARS + 1}
              value={renameTitle}
              onChange={(event) => setRenameTitle(event.target.value)}
            />
            <div className="text-muted-foreground text-xs">
              {renameTitle.trim().length}/{RECORDING_TITLE_MAX_CHARS}
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setRenameTarget(null);
                setRenameTitle('');
              }}
            >
              Cancel
            </Button>
            <Button
              disabled={
                renameSaving ||
                renameTitle.trim().length > RECORDING_TITLE_MAX_CHARS
              }
              onClick={() => void confirmRename()}
            >
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageLayout>
  );
}

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

function RecordingLibraryItem({
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
  const deletable = isRecordingDeletable(recording);
  const renameable = isRecordingRenameable(recording);
  const displayTitle = recordingTitle(recording);
  const timestampTitle = formatTimestamp(recording.startedAtMs);

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <Card
          aria-selected={isSelected}
          className={cn(
            'cursor-default',
            viewMode === 'list'
              ? 'grid grid-cols-[12rem_1fr] gap-0 py-0'
              : 'py-0',
            locked && 'opacity-75',
            isSelected && 'ring-primary ring-2',
          )}
          aria-disabled={locked}
          onClick={(event) => {
            if (event.target instanceof Element) {
              if (event.target.closest('[data-library-action]')) {
                return;
              }
            }
            onSelect(event);
          }}
          onDoubleClick={(event) => {
            if (event.target instanceof Element) {
              if (event.target.closest('[data-library-action]')) {
                return;
              }
            }
            onOpen();
          }}
          onContextMenu={(event) => {
            if (!isSelected && !locked) {
              onSelect(event);
            }
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
          disabled={!renameable}
          onSelect={onRename}
        >
          <Pencil className="size-3.5" />
          Rename recording
        </ContextMenuItem>
        <ContextMenuItem
          className="gap-2"
          disabled={!deletable}
          variant="destructive"
          onSelect={onDelete}
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

function sortRecordings(
  recordings: LibraryRecordingSummary[],
  sortKey: LibraryRecordingSortKey,
): LibraryRecordingSummary[] {
  return [...recordings].sort((left, right) => {
    switch (sortKey) {
      case 'availability':
        return left.availability.localeCompare(right.availability);
      case 'duration':
        return (right.durationMs ?? -1) - (left.durationMs ?? -1);
      case 'status':
        return left.status.localeCompare(right.status);
      case 'started':
        return (right.startedAtMs ?? -1) - (left.startedAtMs ?? -1);
    }
  });
}

function selectedRange(
  recordings: LibraryRecordingSummary[],
  anchorId: string,
  recordingId: string,
): string[] {
  const anchorIndex = recordings.findIndex(
    (recording) => recording.id === anchorId,
  );
  const recordingIndex = recordings.findIndex(
    (recording) => recording.id === recordingId,
  );
  if (anchorIndex < 0 || recordingIndex < 0) {
    return [recordingId];
  }
  const [start, end] =
    anchorIndex < recordingIndex
      ? [anchorIndex, recordingIndex]
      : [recordingIndex, anchorIndex];

  return recordings
    .slice(start, end + 1)
    .filter(isRecordingSelectable)
    .map((recording) => recording.id);
}

function isRecordingSelectable(recording: LibraryRecordingSummary): boolean {
  return recording.availability !== 'locked';
}

function isRecordingDeletable(recording: LibraryRecordingSummary): boolean {
  return recording.status === 'completed' || recording.status === 'failed';
}

function isRecordingRenameable(recording: LibraryRecordingSummary): boolean {
  return recording.status === 'completed' || recording.status === 'failed';
}

function recordingTitle(recording: LibraryRecordingSummary): string {
  return recording.title ?? formatTimestamp(recording.startedAtMs);
}

function uniqueIds(ids: string[]): string[] {
  return [...new Set(ids)];
}

function keepThumbnailPaths(
  thumbnailPaths: Record<string, string>,
  recordings: LibraryRecordingSummary[],
): Record<string, string> {
  const recordingIds = new Set(recordings.map((recording) => recording.id));

  return Object.fromEntries(
    Object.entries(thumbnailPaths).filter(([recordingId]) =>
      recordingIds.has(recordingId),
    ),
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
