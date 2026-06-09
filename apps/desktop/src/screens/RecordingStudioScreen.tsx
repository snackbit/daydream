import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { AlertTriangle, Pencil, RefreshCw, Trash2 } from 'lucide-react';
import { FullHeightLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
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
import { RecordingStudio } from '@/features/library/components/studio/recording-studio';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
  deleteLibraryRecording,
  getLibraryRecording,
  renameLibraryRecording,
} from '@/features/library/library-client';
import {
  availabilityLabel,
  formatDuration,
  formatTimestamp,
} from '@/features/library/library-format';

const RECORDING_TITLE_MAX_CHARS = 120;

export function RecordingStudioScreen() {
  const { recordingId } = useParams({ strict: false }) as {
    recordingId: string;
  };
  const navigate = useNavigate();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [detail, setDetail] = useState<LibraryRecordingDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [playbackAssets, setPlaybackAssets] =
    useState<LibraryPlaybackAssets | null>(null);
  const [status, setStatus] = useState<'error' | 'idle' | 'loading' | 'ready'>(
    'idle',
  );
  const [thumbnailStatus, setThumbnailStatus] = useState<
    'error' | 'idle' | 'loading' | 'ready'
  >('idle');
  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [renameSaving, setRenameSaving] = useState(false);
  const [renameTitle, setRenameTitle] = useState('');
  const loadRunRef = useRef(0);
  const playbackSessionIdRef = useRef<string | null>(null);

  const load = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    const isCurrentLoad = () => loadRunRef.current === loadRunId;
    const previousSessionId = playbackSessionIdRef.current;
    playbackSessionIdRef.current = null;
    if (previousSessionId) {
      void closeLibraryPlaybackSession({ sessionId: previousSessionId });
    }

    setError(null);
    setStatus('loading');
    setThumbnailStatus('idle');
    setPlaybackAssets(null);
    try {
      const nextDetail = await getLibraryRecording(recordingId);
      if (!isCurrentLoad()) {
        return;
      }
      setDetail(nextDetail);
      setStatus('ready');
      if (nextDetail.availability === 'available') {
        createLibraryPlaybackSession({ recordingId })
          .then((assets) => {
            if (!isCurrentLoad() || assets.recordingId !== recordingId) {
              if (assets.sessionId) {
                void closeLibraryPlaybackSession({
                  sessionId: assets.sessionId,
                });
              }
              return;
            }
            playbackSessionIdRef.current = assets.sessionId;
            setPlaybackAssets(assets);
            setThumbnailStatus(assets.error === null ? 'ready' : 'error');
          })
          .catch((playbackError: unknown) => {
            if (!isCurrentLoad()) {
              return;
            }
            setPlaybackAssets({
              error: errorMessage(playbackError),
              recordingId,
              segments: [],
              sessionId: '',
            });
          });
      }
    } catch (loadError) {
      if (!isCurrentLoad()) {
        return;
      }
      setError(errorMessage(loadError));
      setStatus('error');
    }
  }, [recordingId]);

  async function confirmDelete() {
    setError(null);
    try {
      await deleteLibraryRecording({ recordingId });
      await navigate({ to: '/library' });
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  }

  async function confirmRename() {
    if (!detail) {
      return;
    }
    const trimmedTitle = renameTitle.trim();
    setError(null);
    setRenameSaving(true);
    try {
      const nextDetail = await renameLibraryRecording({
        recordingId: detail.id,
        title: trimmedTitle.length > 0 ? trimmedTitle : null,
      });
      setDetail(nextDetail);
      setRenameDialogOpen(false);
    } catch (renameError) {
      setError(errorMessage(renameError));
    } finally {
      setRenameSaving(false);
    }
  }

  useEffect(() => {
    void Promise.resolve().then(load);
  }, [load]);

  useEffect(
    () => () => {
      loadRunRef.current += 1;
      const sessionId = playbackSessionIdRef.current;
      playbackSessionIdRef.current = null;
      if (sessionId) {
        void closeLibraryPlaybackSession({ sessionId });
      }
    },
    [],
  );

  return (
    <FullHeightLayout>
      <div className="flex shrink-0 items-center justify-between gap-3 border-b px-4 py-3">
        <div className="min-w-0">
          <h1 className="truncate text-lg font-semibold tracking-normal">
            {detail ? recordingTitle(detail) : 'Recording'}
          </h1>
          <div className="text-muted-foreground mt-1 flex flex-wrap items-center gap-2 text-xs">
            {detail ? (
              <>
                <Badge variant="outline">
                  {availabilityLabel(detail.availability)}
                </Badge>
                {detail.title ? (
                  <span>{formatTimestamp(detail.startedAtMs)}</span>
                ) : null}
                <span>{formatDuration(detail.durationMs)}</span>
                <span>{detail.segments.length} segments</span>
              </>
            ) : (
              <span>Loading studio...</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {detail && isRecordingRenameable(detail) ? (
            <Button
              className="gap-2"
              variant="outline"
              onClick={() => {
                setRenameTitle(detail.title ?? '');
                setRenameDialogOpen(true);
              }}
            >
              <Pencil className="size-3.5" />
              Rename
            </Button>
          ) : null}
          {detail && isRecordingDeletable(detail) ? (
            <Button
              className="gap-2"
              variant="destructive"
              onClick={() => setDeleteDialogOpen(true)}
            >
              <Trash2 className="size-3.5" />
              Delete
            </Button>
          ) : null}
          <Button
            className="gap-2"
            variant="outline"
            onClick={() => void load()}
          >
            <RefreshCw className="size-3.5" />
            Refresh
          </Button>
        </div>
      </div>

      {error ? (
        <div className="p-4">
          <Alert variant="destructive">
            <AlertTriangle />
            <AlertTitle>Recording failed to load</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        </div>
      ) : null}

      {status === 'loading' ? (
        <div className="text-muted-foreground p-4 text-sm">
          Loading recording...
        </div>
      ) : null}

      {detail ? (
        <RecordingStudio
          detail={detail}
          playbackAssets={playbackAssets}
          thumbnailStatus={thumbnailStatus}
        />
      ) : null}

      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete recording</DialogTitle>
            <DialogDescription>
              This removes the broken Library entry and any remaining files in
              Daydream storage.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button variant="destructive" onClick={() => void confirmDelete()}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={renameDialogOpen}
        onOpenChange={(open) => {
          setRenameDialogOpen(open);
          if (!open) {
            setRenameTitle('');
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename recording</DialogTitle>
            <DialogDescription>
              Leave the title blank to use the recording timestamp.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="studio-recording-title">Title</Label>
            <Input
              id="studio-recording-title"
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
                setRenameDialogOpen(false);
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
    </FullHeightLayout>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecordingDeletable(detail: LibraryRecordingDetail): boolean {
  return detail.status === 'completed' || detail.status === 'failed';
}

function isRecordingRenameable(detail: LibraryRecordingDetail): boolean {
  return detail.status === 'completed' || detail.status === 'failed';
}

function recordingTitle(detail: LibraryRecordingDetail): string {
  return detail.title ?? formatTimestamp(detail.startedAtMs);
}
