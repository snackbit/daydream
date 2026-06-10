import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from 'react';
import { useNavigate } from '@tanstack/react-router';
import type {
  LibraryRecordingSortKey,
  LibraryRecordingSummary,
  LibraryRecordingViewMode,
} from '@daydream/shared';
import { AlertTriangle, Grid2X2, List, RefreshCw, Trash2 } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
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
  errorMessage,
  isRecordingDeletable,
  isRecordingRenameable,
  isRecordingSelectable,
  keepThumbnailPaths,
  selectedRange,
  sortRecordings,
  uniqueIds,
} from '@/features/library/components/overview/library-overview-utils';
import { RecordingLibraryItem } from '@/features/library/components/overview/recording-library-item';
import {
  deleteLibraryRecordings,
  deleteLibraryRecording,
  listLibraryRecordings,
  prepareLibraryRecordingThumbnail,
  renameLibraryRecording,
} from '@/features/library/library-client';
import {
  formatFileSize,
  formatTimestamp,
} from '@/features/library/library-format';

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
