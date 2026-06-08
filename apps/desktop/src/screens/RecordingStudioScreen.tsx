import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryWaveforms,
} from '@daydream/shared';
import { AlertTriangle, RefreshCw, Trash2 } from 'lucide-react';
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
import { RecordingStudio } from '@/features/library/components/studio/recording-studio';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
  deleteLibraryRecording,
  getLibraryRecording,
  getRecordingWaveforms,
} from '@/features/library/library-client';
import {
  availabilityLabel,
  formatDuration,
  formatTimestamp,
} from '@/features/library/library-format';

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
  const [waveforms, setWaveforms] = useState<LibraryWaveforms | null>(null);
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
    setWaveforms(null);
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
        getRecordingWaveforms({ recordingId })
          .then((nextWaveforms) => {
            if (!isCurrentLoad() || nextWaveforms.recordingId !== recordingId) {
              return;
            }
            setWaveforms(nextWaveforms);
          })
          .catch((waveformError: unknown) => {
            if (!isCurrentLoad()) {
              return;
            }
            setWaveforms({
              error: errorMessage(waveformError),
              recordingId,
              tracks: [],
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
    <section className="flex min-h-full flex-col">
      <div className="flex shrink-0 items-center justify-between gap-3 border-b px-4 py-3">
        <div className="min-w-0">
          <h1 className="truncate text-lg font-semibold tracking-normal">
            {detail ? formatTimestamp(detail.startedAtMs) : 'Recording'}
          </h1>
          <div className="text-muted-foreground mt-1 flex flex-wrap items-center gap-2 text-xs">
            {detail ? (
              <>
                <Badge variant="outline">
                  {availabilityLabel(detail.availability)}
                </Badge>
                <span>{formatDuration(detail.durationMs)}</span>
                <span>{detail.segments.length} segments</span>
              </>
            ) : (
              <span>Loading studio...</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
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
          waveforms={waveforms}
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
    </section>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecordingDeletable(detail: LibraryRecordingDetail): boolean {
  return detail.status === 'completed' || detail.status === 'failed';
}
