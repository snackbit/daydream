import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import type {
  LibraryRecordingDebugData,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import {
  AlertTriangle,
  Bug,
  Clipboard,
  Loader2,
  Pencil,
  RefreshCw,
  Sparkles,
  Trash2,
} from 'lucide-react';
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
import { startRecordingAiProcessing } from '@/features/ai/ai-client';
import { useRecordingAiProcessingState } from '@/features/ai/use-recording-ai-processing-state';
import { RecordingStudio } from '@/features/library/components/studio/recording-studio';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
  deleteLibraryRecording,
  getLibraryRecordingDebug,
  getLibraryRecording,
  renameLibraryRecording,
} from '@/features/library/library-client';
import {
  availabilityLabel,
  formatDuration,
  formatTimestamp,
} from '@/features/library/library-format';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';

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
  const [processingStartBusy, setProcessingStartBusy] = useState(false);
  const [debugDialogOpen, setDebugDialogOpen] = useState(false);
  const [debugData, setDebugData] = useState<LibraryRecordingDebugData | null>(
    null,
  );
  const [debugError, setDebugError] = useState<string | null>(null);
  const [debugLoading, setDebugLoading] = useState(false);
  const [debugCopyStatus, setDebugCopyStatus] = useState<
    'copied' | 'failed' | 'idle'
  >('idle');
  const debugEnabled = useDebugEnabled();
  const aiProcessing = useRecordingAiProcessingState(recordingId);
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

  async function startAiProcessing() {
    if (!detail) {
      return;
    }
    setError(null);
    setProcessingStartBusy(true);
    try {
      await startRecordingAiProcessing({ recordingId: detail.id });
      await aiProcessing.reload();
    } catch (processingError) {
      setError(errorMessage(processingError));
    } finally {
      setProcessingStartBusy(false);
    }
  }

  async function openDebugDialog() {
    setDebugDialogOpen(true);
    setDebugCopyStatus('idle');
    setDebugError(null);
    setDebugLoading(true);
    try {
      setDebugData(await getLibraryRecordingDebug({ recordingId }));
    } catch (debugLoadError) {
      setDebugError(errorMessage(debugLoadError));
    } finally {
      setDebugLoading(false);
    }
  }

  async function copyDebugData() {
    if (!debugData) {
      return;
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(debugData, null, 2));
      setDebugCopyStatus('copied');
    } catch (_error) {
      setDebugCopyStatus('failed');
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

  const processingJob = aiProcessing.state?.job ?? null;
  const hasActiveProcessingChunks =
    aiProcessing.state?.chunks.some(
      (chunk) => chunk.status === 'queued' || chunk.status === 'running',
    ) ?? false;
  const processingActive =
    processingJob?.status === 'queued' ||
    processingJob?.status === 'running' ||
    hasActiveProcessingChunks;
  const processingSucceeded =
    processingJob?.status === 'succeeded' && !hasActiveProcessingChunks;
  const processingRetryable =
    !hasActiveProcessingChunks &&
    (processingJob?.status === 'cancelled' ||
      processingJob?.status === 'failed');
  const processAiLabel = processingStartBusy
    ? 'Starting...'
    : processingActive
      ? 'Processing...'
      : processingSucceeded
        ? 'Processed'
        : processingRetryable
          ? 'Retry AI'
          : 'Process AI';

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
          {detail ? (
            <Button
              className="gap-2"
              disabled={
                !isAiProcessingStartable(detail) ||
                processingStartBusy ||
                processingActive ||
                processingSucceeded
              }
              variant="outline"
              onClick={() => void startAiProcessing()}
            >
              {processingStartBusy || processingActive ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <Sparkles className="size-3.5" />
              )}
              {processAiLabel}
            </Button>
          ) : null}
          {detail && debugEnabled ? (
            <Button
              className="gap-2"
              variant="outline"
              onClick={() => void openDebugDialog()}
            >
              <Bug className="size-3.5" />
              Debug
            </Button>
          ) : null}
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
          aiProcessingState={aiProcessing.state}
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

      <Dialog open={debugDialogOpen} onOpenChange={setDebugDialogOpen}>
        <DialogContent className="max-w-4xl">
          <DialogHeader>
            <DialogTitle>Recording debug data</DialogTitle>
            <DialogDescription>
              Raw local state for this recording. Transcript text may be
              included.
            </DialogDescription>
          </DialogHeader>
          {debugLoading ? (
            <div className="text-muted-foreground border p-4 text-sm">
              Loading debug data.
            </div>
          ) : null}
          {debugError ? (
            <Alert variant="destructive">
              <AlertTriangle />
              <AlertTitle>Debug data failed</AlertTitle>
              <AlertDescription>{debugError}</AlertDescription>
            </Alert>
          ) : null}
          <pre className="bg-muted max-h-[60vh] overflow-auto p-3 text-xs leading-5">
            {JSON.stringify(debugData ?? { status: 'loading' }, null, 2)}
          </pre>
          <DialogFooter>
            <p
              aria-live="polite"
              className="text-muted-foreground min-h-5 flex-1 text-xs"
            >
              {debugCopyStatus === 'copied' ? 'Debug JSON copied.' : null}
              {debugCopyStatus === 'failed'
                ? 'Unable to copy debug JSON.'
                : null}
            </p>
            <Button
              className="gap-2"
              disabled={!debugData}
              variant="outline"
              onClick={() => void copyDebugData()}
            >
              <Clipboard className="size-3.5" />
              Copy JSON
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

function isAiProcessingStartable(detail: LibraryRecordingDetail): boolean {
  return detail.status === 'completed' && detail.availability === 'available';
}

function recordingTitle(detail: LibraryRecordingDetail): string {
  return detail.title ?? formatTimestamp(detail.startedAtMs);
}
