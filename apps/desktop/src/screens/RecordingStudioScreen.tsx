import {
  type KeyboardEvent,
  type PointerEvent,
  type WheelEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import type {
  LibraryPlaybackAssets,
  LibraryPlaybackSegment,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveforms,
} from '@daydream/shared';
import {
  AlertTriangle,
  Lock,
  Maximize,
  Minus,
  Pause,
  Play,
  Plus,
  RefreshCw,
  SkipBack,
  SkipForward,
  Trash2,
  Volume2,
  VolumeX,
} from 'lucide-react';
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
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
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
import {
  segmentForTimeline,
  timelineDurationMs,
} from '@/features/library/library-timeline';
import { cn } from '@/lib/utils';

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
            if (assets.error !== null) {
              return;
            }
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

interface RecordingStudioProps {
  detail: LibraryRecordingDetail;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  waveforms: LibraryWaveforms | null;
}

const INITIAL_TRACK_WINDOW_MS = 180_000;
const MIN_TRACK_WINDOW_MS = 10_000;
const TRACK_LABEL_WIDTH_PX = 160;
const TRACK_RULER_HEIGHT_PX = 32;
const TRACK_LANE_HEIGHT_PX = 56;
const TRACK_ZOOM_STEP = 1.25;
const TRACK_FOLLOW_EDGE_PX = 48;
const DEFAULT_TRACK_VIEWPORT_WIDTH_PX = 800;
const RULER_TARGET_TICK_PX = 90;

interface TrackPlaybackCommands {
  commitTimelineInput: (nextTimelineMs: number) => void;
  handleTimelineInput: (nextTimelineMs: number) => void;
  seekTimeline: (
    nextTimelineMs: number,
    options?: { resume?: boolean },
  ) => void;
}

interface TrackPlaybackSnapshot {
  displayedTimelineMs: number;
  isPlaying: boolean;
  seekRevision: number;
  totalDurationMs: number;
}

function RecordingStudio({
  detail,
  playbackAssets,
  thumbnailStatus,
  waveforms,
}: RecordingStudioProps) {
  const [mutedTrackIds, setMutedTrackIds] = useState<Set<string>>(
    () => new Set(),
  );
  const playbackCommandsRef = useRef<TrackPlaybackCommands | null>(null);
  const [playbackSnapshot, setPlaybackSnapshot] =
    useState<TrackPlaybackSnapshot>({
      displayedTimelineMs: 0,
      isPlaying: false,
      seekRevision: 0,
      totalDurationMs: Math.max(1, timelineDurationMs(detail.segments)),
    });

  function toggleTrackMuted(trackId: string) {
    setMutedTrackIds((current) => {
      const next = new Set(current);
      if (next.has(trackId)) {
        next.delete(trackId);
      } else {
        next.add(trackId);
      }

      return next;
    });
  }

  return (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize={74} minSize={50}>
        <ResizablePanelGroup orientation="vertical">
          <ResizablePanel defaultSize={68} minSize={45}>
            <StudioPlayer
              detail={detail}
              mutedTrackIds={mutedTrackIds}
              onPlaybackSnapshotChange={setPlaybackSnapshot}
              playbackCommandsRef={playbackCommandsRef}
              playbackAssets={playbackAssets}
            />
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize={32} minSize={18}>
            <TrackTimeline
              detail={detail}
              key={detail.id}
              mutedTrackIds={mutedTrackIds}
              onToggleTrackMuted={toggleTrackMuted}
              playbackCommandsRef={playbackCommandsRef}
              playbackSnapshot={playbackSnapshot}
              playbackAssets={playbackAssets}
              thumbnailStatus={thumbnailStatus}
              waveforms={waveforms}
            />
          </ResizablePanel>
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize={26} minSize={18}>
        <StudioSidebar detail={detail} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function StudioPlayer({
  detail,
  mutedTrackIds,
  onPlaybackSnapshotChange,
  playbackCommandsRef,
  playbackAssets,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onPlaybackSnapshotChange: (snapshot: TrackPlaybackSnapshot) => void;
  playbackCommandsRef: { current: TrackPlaybackCommands | null };
  playbackAssets: LibraryPlaybackAssets | null;
}) {
  const audioRefs = useRef(new Map<string, HTMLAudioElement>());
  const playbackIntentRef = useRef(false);
  const resumeTimerRef = useRef<number | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const playableSegments = useMemo(
    () =>
      detail.segments.filter(
        (segment) => segment.state === 'finalized' && segment.fileExists,
      ),
    [detail.segments],
  );
  const [currentIndex, setCurrentIndex] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [isMuted, setIsMuted] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackEndedAtEnd, setPlaybackEndedAtEnd] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [pendingAutoplay, setPendingAutoplay] = useState(false);
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
  const [scrubTimelineMs, setScrubTimelineMs] = useState<number | null>(null);
  const [seekRevision, setSeekRevision] = useState(0);
  const [streamRevision, setStreamRevision] = useState(0);
  const [streamStartMs, setStreamStartMs] = useState(0);
  const [volume, setVolume] = useState(0.8);
  const currentSegment = playableSegments[currentIndex] ?? null;
  const currentPlaybackSegment = currentSegment
    ? playbackAssets?.segments.find(
        (segment) => segment.segmentId === currentSegment.id,
      )
    : null;
  const totalDurationMs = timelineDurationMs(playableSegments);
  const currentTimelineMs = currentSegment
    ? currentSegment.playbackOffsetMs + currentTime * 1000
    : 0;
  const displayedTimelineMs = scrubTimelineMs ?? currentTimelineMs;
  const streamStartSeconds = streamStartMs / 1000;

  function updateMediaSettings(media: HTMLMediaElement, trackId?: string) {
    media.volume = volume;
    media.muted = isMuted || (trackId ? mutedTrackIds.has(trackId) : false);
  }

  function setMediaCurrentTime(media: HTMLMediaElement, seconds: number) {
    if (media.readyState === HTMLMediaElement.HAVE_NOTHING) {
      return;
    }
    try {
      media.currentTime = seconds;
    } catch {
      // Generated stream replacements may not expose a seekable range yet.
    }
  }

  function syncAudioElements(currentTimeSeconds: number, force = false) {
    for (const [trackId, audio] of audioRefs.current) {
      updateMediaSettings(audio, trackId);
      if (force || Math.abs(audio.currentTime - currentTimeSeconds) > 0.25) {
        setMediaCurrentTime(audio, currentTimeSeconds);
      }
    }
  }

  async function playMediaElements(): Promise<boolean> {
    const video = videoRef.current;
    if (!video) {
      return false;
    }
    setPlaybackError(null);
    syncAudioElements(video.currentTime, true);
    try {
      await video.play();
    } catch (error) {
      playbackIntentRef.current = false;
      setIsPlaying(false);
      setPendingAutoplay(false);
      setPlaybackError(`Playback failed: ${errorMessage(error)}`);
      return false;
    }
    const audioResults = await Promise.allSettled(
      [...audioRefs.current.entries()].map(async ([trackId, audio]) => {
        updateMediaSettings(audio, trackId);
        await audio.play();
      }),
    );
    const audioFailure = audioResults.find(
      (result) => result.status === 'rejected',
    );
    if (audioFailure?.status === 'rejected') {
      setPlaybackError(
        `Audio playback failed: ${errorMessage(audioFailure.reason)}`,
      );
    }

    playbackIntentRef.current = true;
    return true;
  }

  function pauseMediaElements() {
    videoRef.current?.pause();
    for (const audio of audioRefs.current.values()) {
      audio.pause();
    }
  }

  function clearResumeTimer() {
    if (resumeTimerRef.current !== null) {
      window.clearInterval(resumeTimerRef.current);
      resumeTimerRef.current = null;
    }
  }

  function advanceToSegment(nextIndex: number, shouldPlay: boolean) {
    clearResumeTimer();
    playbackIntentRef.current = shouldPlay;
    setCurrentIndex(nextIndex);
    setCurrentTime(0);
    setPlaybackEndedAtEnd(false);
    setPlaybackError(null);
    setPendingAutoplay(shouldPlay);
    setResumeProgress(null);
    setStreamStartMs(0);
    setStreamRevision((revision) => revision + 1);
    setIsPlaying(shouldPlay);
  }

  useEffect(() => {
    const video = videoRef.current;
    if (!video) {
      return;
    }
    updateMediaSettings(video);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isMuted, volume]);

  useEffect(() => {
    for (const [trackId, audio] of audioRefs.current) {
      updateMediaSettings(audio, trackId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isMuted, mutedTrackIds, volume]);

  useEffect(
    () => () => {
      if (resumeTimerRef.current !== null) {
        window.clearInterval(resumeTimerRef.current);
        resumeTimerRef.current = null;
      }
    },
    [],
  );

  useEffect(() => {
    onPlaybackSnapshotChange({
      displayedTimelineMs,
      isPlaying,
      seekRevision,
      totalDurationMs: Math.max(1, totalDurationMs),
    });
  }, [
    displayedTimelineMs,
    isPlaying,
    onPlaybackSnapshotChange,
    seekRevision,
    totalDurationMs,
  ]);

  if (detail.availability !== 'available' || !currentSegment) {
    playbackCommandsRef.current = null;
    const locked = detail.availability === 'locked';
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        {locked ? (
          <Lock className="size-8" />
        ) : (
          <AlertTriangle className="size-8" />
        )}
        <div>
          <div className="text-foreground text-sm font-medium">
            {locked ? 'Recording in progress' : 'Missing video files'}
          </div>
          <p className="mt-1 max-w-md text-xs">
            {locked
              ? 'This recording is still controlled by the recorder and cannot be played or deleted from Library.'
              : 'This recording exists in the database, but Daydream cannot find the local MKV files needed for playback.'}
          </p>
        </div>
      </div>
    );
  }

  if (!playbackAssets) {
    playbackCommandsRef.current = null;
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <div className="text-foreground text-sm font-medium">
          Preparing playback...
        </div>
        <p className="max-w-md text-xs">
          Daydream is preparing browser-playable media from the local MKV
          segment.
        </p>
      </div>
    );
  }

  if (playbackAssets.error || !currentPlaybackSegment) {
    playbackCommandsRef.current = null;
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <AlertTriangle className="size-8" />
        <div>
          <div className="text-foreground text-sm font-medium">
            Playback preparation failed
          </div>
          <p className="mt-1 max-w-md text-xs">
            {playbackAssets.error ??
              'Daydream could not create playable media for this segment.'}
          </p>
        </div>
      </div>
    );
  }

  async function togglePlayback() {
    const video = videoRef.current;
    if (resumeProgress !== null) {
      return;
    }
    if (playbackEndedAtEnd) {
      seekTimeline(0, { resume: true });
      return;
    }
    if (!video) {
      return;
    }
    if (video.paused) {
      setPlaybackEndedAtEnd(false);
      const played = await playMediaElements();
      setIsPlaying(played);
    } else {
      setPendingAutoplay(false);
      pauseMediaElements();
      playbackIntentRef.current = false;
      setIsPlaying(false);
    }
  }

  function seekTimeline(
    nextTimelineMs: number,
    options: { resume?: boolean } = {},
  ) {
    if (nextTimelineMs >= totalDurationMs) {
      clearResumeTimer();
      const lastIndex = playableSegments.length - 1;
      const lastSegment = playableSegments[lastIndex];
      if (!lastSegment) {
        return;
      }
      pauseMediaElements();
      setCurrentIndex(lastIndex);
      setCurrentTime((lastSegment.durationMs ?? 0) / 1000);
      setPendingAutoplay(false);
      playbackIntentRef.current = false;
      setIsPlaying(false);
      setPlaybackEndedAtEnd(true);
      setPlaybackError(null);
      setResumeProgress(null);
      setStreamStartMs(lastSegment.durationMs ?? 0);
      setSeekRevision((revision) => revision + 1);
      setStreamRevision((revision) => revision + 1);
      return;
    }

    const target = segmentForTimeline(playableSegments, nextTimelineMs);
    if (!target) {
      return;
    }
    const shouldResume = options.resume ?? isPlaying;
    clearResumeTimer();
    playbackIntentRef.current = shouldResume;
    pauseMediaElements();
    setCurrentIndex(target.index);
    setCurrentTime(target.offsetMs / 1000);
    setIsPlaying(shouldResume);
    setPendingAutoplay(shouldResume);
    setPlaybackEndedAtEnd(false);
    setPlaybackError(null);
    setResumeProgress(null);
    setStreamStartMs(target.offsetMs);
    setSeekRevision((revision) => revision + 1);
    setStreamRevision((revision) => revision + 1);
  }

  function handleTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(clampTimelineMs(nextTimelineMs, totalDurationMs));
  }

  function commitTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(null);
    seekTimeline(nextTimelineMs);
  }

  function handleEnded() {
    if (!currentSegment) {
      return;
    }

    const nextIndex = currentIndex + 1;
    if (nextIndex >= playableSegments.length) {
      setCurrentTime((currentSegment.durationMs ?? 0) / 1000);
      playbackIntentRef.current = false;
      setIsPlaying(false);
      setPlaybackEndedAtEnd(true);
      pauseMediaElements();
      return;
    }
    const shouldResumeAfterGap = playbackIntentRef.current || isPlaying;
    const advanceToNextSegment = () =>
      advanceToSegment(nextIndex, shouldResumeAfterGap);
    const pauseGapMs = Math.max(0, currentSegment.pauseAfterMs);
    if (pauseGapMs === 0) {
      advanceToNextSegment();
      return;
    }

    setResumeProgress(0);
    const startedAt = window.performance.now();
    clearResumeTimer();
    resumeTimerRef.current = window.setInterval(() => {
      const progress = Math.min(
        1,
        (window.performance.now() - startedAt) / pauseGapMs,
      );
      setResumeProgress(progress);
      if (progress >= 1) {
        clearResumeTimer();
        advanceToNextSegment();
      }
    }, 50);
  }

  function playNowAfterPause() {
    const nextIndex = currentIndex + 1;
    if (nextIndex < playableSegments.length) {
      advanceToSegment(nextIndex, playbackIntentRef.current || isPlaying);
    }
  }

  playbackCommandsRef.current = {
    commitTimelineInput,
    handleTimelineInput,
    seekTimeline,
  };

  return (
    <div className="flex h-full flex-col bg-black text-white">
      <div className="relative min-h-0 flex-1">
        <video
          key={`${currentPlaybackSegment.segmentId}-${streamStartMs}-${streamRevision}-video`}
          ref={videoRef}
          className="h-full w-full object-contain"
          preload="metadata"
          src={streamUrl(
            currentPlaybackSegment.videoUrl,
            streamStartMs,
            streamRevision,
          )}
          onPause={() => setIsPlaying(false)}
          onPlay={() => {
            playbackIntentRef.current = true;
            setIsPlaying(true);
            setPendingAutoplay(false);
            setPlaybackEndedAtEnd(false);
          }}
          onTimeUpdate={(event) => {
            setCurrentTime(
              streamStartSeconds + event.currentTarget.currentTime,
            );
            syncAudioElements(event.currentTarget.currentTime);
          }}
          onCanPlay={() => {
            if (!pendingAutoplay || resumeProgress !== null) {
              return;
            }
            setPendingAutoplay(false);
            void playMediaElements();
          }}
          onEnded={handleEnded}
          onError={() => {
            playbackIntentRef.current = false;
            setIsPlaying(false);
            setPendingAutoplay(false);
            setPlaybackError('Playback failed: video stream is not supported.');
          }}
        />
        {currentPlaybackSegment.audio.map((audio) => (
          <audio
            key={`${currentPlaybackSegment.segmentId}-${audio.trackId}-${streamStartMs}-${streamRevision}`}
            ref={(element) => {
              if (element) {
                audioRefs.current.set(audio.trackId, element);
              } else {
                audioRefs.current.delete(audio.trackId);
              }
            }}
            preload="metadata"
            src={streamUrl(audio.url, streamStartMs, streamRevision)}
          />
        ))}
        {resumeProgress !== null ? (
          <div className="absolute inset-0 flex items-center justify-center bg-black/70">
            <div className="w-80 max-w-[80%] text-center">
              <div className="text-sm font-medium">
                Recording paused. Resuming at{' '}
                {formatTimestamp(
                  playableSegments[currentIndex + 1]?.startedAtMs ?? null,
                )}
              </div>
              <div className="mt-3 h-1 overflow-hidden rounded-full bg-white/20">
                <div
                  className="h-full bg-white"
                  style={{ width: `${resumeProgress * 100}%` }}
                />
              </div>
              <Button
                className="mt-4 gap-2"
                size="sm"
                variant="secondary"
                onClick={playNowAfterPause}
              >
                <SkipForward className="size-3.5" />
                Play now
              </Button>
            </div>
          </div>
        ) : null}
        {playbackError ? (
          <div className="absolute inset-0 flex items-center justify-center bg-black/80 p-6 text-center">
            <div>
              <div className="text-sm font-medium">Playback failed</div>
              <p className="mt-1 max-w-md text-xs text-white/70">
                {playbackError}
              </p>
            </div>
          </div>
        ) : null}
      </div>

      <div className="flex shrink-0 flex-col gap-2 border-t border-white/10 bg-black/90 p-3">
        <SegmentScrubber
          currentTimelineMs={Math.min(displayedTimelineMs, totalDurationMs)}
          playbackAssets={playbackAssets}
          segments={playableSegments}
          totalDurationMs={Math.max(totalDurationMs, 1)}
          onCommit={commitTimelineInput}
          onScrubUpdate={handleTimelineInput}
        />
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <Button
              aria-label="Skip back"
              size="icon"
              variant="ghost"
              onClick={() =>
                seekTimeline(Math.max(0, displayedTimelineMs - 10_000))
              }
            >
              <SkipBack className="size-4" />
            </Button>
            <Button
              aria-label={isPlaying ? 'Pause' : 'Play'}
              size="icon"
              variant="secondary"
              onClick={() => void togglePlayback()}
            >
              {isPlaying ? (
                <Pause className="size-4" />
              ) : (
                <Play className="size-4" />
              )}
            </Button>
            <Button
              aria-label="Skip forward"
              size="icon"
              variant="ghost"
              onClick={() =>
                seekTimeline(
                  Math.min(totalDurationMs, displayedTimelineMs + 10_000),
                )
              }
            >
              <SkipForward className="size-4" />
            </Button>
            <span className="ml-2 text-xs tabular-nums">
              {formatDuration(displayedTimelineMs)} /{' '}
              {formatDuration(totalDurationMs)}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <Button
              aria-label={isMuted ? 'Unmute' : 'Mute'}
              size="icon"
              variant="ghost"
              onClick={() => setIsMuted((muted) => !muted)}
            >
              {isMuted ? (
                <VolumeX className="size-4" />
              ) : (
                <Volume2 className="size-4" />
              )}
            </Button>
            <input
              aria-label="Volume"
              className="w-24 accent-white"
              max={1}
              min={0}
              step={0.05}
              type="range"
              value={volume}
              onChange={(event) => setVolume(Number(event.currentTarget.value))}
            />
            <Button
              aria-label="Fullscreen"
              size="icon"
              variant="ghost"
              onClick={() => void videoRef.current?.requestFullscreen?.()}
            >
              <Maximize className="size-4" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function streamUrl(url: string, startMs: number, revision: number): string {
  const separator = url.includes('?') ? '&' : '?';

  return `${url}${separator}startMs=${Math.max(
    0,
    Math.floor(startMs),
  )}&request=${Math.max(0, Math.floor(revision))}`;
}

type TimelineRegion =
  | {
      endMs: number;
      kind: 'pause';
      segment: LibraryRecordingSegment;
      startMs: number;
    }
  | {
      endMs: number;
      kind: 'segment';
      segment: LibraryRecordingSegment;
      startMs: number;
    };

interface HoverPreview {
  label: string;
  leftPercent: number;
  thumbnailUrl: null | string;
  title: string;
}

function SegmentScrubber({
  currentTimelineMs,
  onCommit,
  onScrubUpdate,
  playbackAssets,
  segments,
  totalDurationMs,
}: {
  currentTimelineMs: number;
  onCommit: (timelineMs: number) => void;
  onScrubUpdate: (timelineMs: number) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  segments: LibraryRecordingSegment[];
  totalDurationMs: number;
}) {
  const scrubberRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const [hoverPreview, setHoverPreview] = useState<HoverPreview | null>(null);
  const regions = useMemo(() => buildTimelineRegions(segments), [segments]);
  const progressPercent = percentOf(currentTimelineMs, totalDurationMs);

  function timelineMsFromPointer(event: PointerEvent<HTMLDivElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    const width = rect.width || 1;
    const position = Math.max(0, Math.min(width, event.clientX - rect.left));

    return (position / width) * totalDurationMs;
  }

  function updateHoverPreview(timelineMs: number) {
    setHoverPreview(
      previewForTimeline(timelineMs, totalDurationMs, regions, playbackAssets),
    );
  }

  function previewTimeline(timelineMs: number) {
    const resolvedTimelineMs = resolvePlayableTimelineMs(segments, timelineMs);
    onScrubUpdate(resolvedTimelineMs);
    updateHoverPreview(timelineMs);
  }

  function commitTimeline(timelineMs: number) {
    const resolvedTimelineMs = resolvePlayableTimelineMs(segments, timelineMs);
    setDragging(false);
    setHoverPreview(null);
    onCommit(resolvedTimelineMs);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.shiftKey ? 30_000 : 5_000;
    let nextTimelineMs: number | null = null;
    if (event.key === 'ArrowLeft') {
      nextTimelineMs = currentTimelineMs - step;
    } else if (event.key === 'ArrowRight') {
      nextTimelineMs = currentTimelineMs + step;
    } else if (event.key === 'Home') {
      nextTimelineMs = 0;
    } else if (event.key === 'End') {
      nextTimelineMs = totalDurationMs;
    }
    if (nextTimelineMs === null) {
      return;
    }

    event.preventDefault();
    commitTimeline(clampTimelineMs(nextTimelineMs, totalDurationMs));
  }

  return (
    <div className="relative py-1">
      {hoverPreview ? (
        <div
          aria-live="polite"
          className="pointer-events-none absolute bottom-full z-10 mb-3 w-44 -translate-x-1/2 overflow-hidden rounded-md border border-white/15 bg-neutral-950 text-white shadow-xl"
          style={{ left: `${hoverPreview.leftPercent}%` }}
        >
          {hoverPreview.thumbnailUrl ? (
            <img
              alt=""
              className="aspect-video w-full object-cover"
              src={hoverPreview.thumbnailUrl}
            />
          ) : (
            <div className="flex aspect-video w-full items-center justify-center bg-white/10 text-xs">
              {hoverPreview.title}
            </div>
          )}
          <div className="px-2 py-1 text-xs">
            <div className="font-medium">{hoverPreview.title}</div>
            <div className="text-white/70">{hoverPreview.label}</div>
          </div>
        </div>
      ) : null}

      <div
        aria-label="Playback position"
        aria-valuemax={Math.round(totalDurationMs)}
        aria-valuemin={0}
        aria-valuenow={Math.round(currentTimelineMs)}
        aria-valuetext={`${formatDuration(currentTimelineMs)} of ${formatDuration(
          totalDurationMs,
        )}`}
        className="group relative h-7 cursor-pointer outline-none"
        ref={scrubberRef}
        role="slider"
        tabIndex={0}
        onBlur={() => {
          setDragging(false);
          setHoverPreview(null);
        }}
        onKeyDown={handleKeyDown}
        onPointerCancel={(event) =>
          commitTimeline(timelineMsFromPointer(event))
        }
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture?.(event.pointerId);
          setDragging(true);
          previewTimeline(timelineMsFromPointer(event));
        }}
        onPointerEnter={(event) =>
          updateHoverPreview(timelineMsFromPointer(event))
        }
        onPointerLeave={() => {
          if (!dragging) {
            setHoverPreview(null);
          }
        }}
        onPointerMove={(event) => {
          const nextTimelineMs = timelineMsFromPointer(event);
          if (dragging) {
            previewTimeline(nextTimelineMs);
          } else {
            updateHoverPreview(nextTimelineMs);
          }
        }}
        onPointerUp={(event) => commitTimeline(timelineMsFromPointer(event))}
      >
        <div className="absolute top-1/2 right-0 left-0 h-2 -translate-y-1/2 overflow-hidden rounded-full bg-white/15 transition-[height] group-hover:h-3">
          {regions.map((region) => (
            <div
              aria-label={timelineRegionLabel(region)}
              className={cn(
                'absolute top-0 h-full',
                region.kind === 'segment'
                  ? 'bg-white/30'
                  : 'bg-white/10 bg-[repeating-linear-gradient(135deg,rgba(255,255,255,0.18)_0,rgba(255,255,255,0.18)_3px,transparent_3px,transparent_7px)]',
              )}
              data-playback-region={region.kind}
              key={`${region.kind}-${region.segment.id}-${region.startMs}`}
              style={{
                left: `${percentOf(region.startMs, totalDurationMs)}%`,
                width: `${Math.max(
                  0,
                  percentOf(region.endMs - region.startMs, totalDurationMs),
                )}%`,
              }}
            />
          ))}
          <div
            className="absolute top-0 left-0 h-full bg-white"
            style={{ width: `${progressPercent}%` }}
          />
        </div>
        {segments.map((segment) => {
          const startPercent = percentOf(
            segment.playbackOffsetMs,
            totalDurationMs,
          );
          const endPercent = percentOf(
            segment.playbackOffsetMs + (segment.durationMs ?? 0),
            totalDurationMs,
          );

          return (
            <div key={segment.id}>
              <div
                aria-label={`Segment ${segment.segmentIndex} starts`}
                className="absolute top-1/2 h-4 w-px -translate-y-1/2 bg-white/80"
                data-segment-boundary="start"
                style={{ left: `${startPercent}%` }}
              />
              <div
                aria-label={`Segment ${segment.segmentIndex} ends`}
                className="absolute top-1/2 h-4 w-px -translate-y-1/2 bg-white/45"
                data-segment-boundary="end"
                style={{ left: `${endPercent}%` }}
              />
            </div>
          );
        })}
        <div
          className="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-black/30 bg-white shadow transition-transform group-hover:scale-110"
          style={{ left: `${progressPercent}%` }}
        />
      </div>
    </div>
  );
}

function buildTimelineRegions(
  segments: LibraryRecordingSegment[],
): TimelineRegion[] {
  return segments.flatMap((segment) => {
    const durationMs = segment.durationMs ?? 0;
    const segmentStartMs = segment.playbackOffsetMs;
    const segmentEndMs = segmentStartMs + durationMs;
    const regions: TimelineRegion[] = [
      {
        endMs: segmentEndMs,
        kind: 'segment',
        segment,
        startMs: segmentStartMs,
      },
    ];
    const pauseEndMs = segmentEndMs + Math.max(0, segment.pauseAfterMs);
    if (pauseEndMs > segmentEndMs) {
      regions.push({
        endMs: pauseEndMs,
        kind: 'pause',
        segment,
        startMs: segmentEndMs,
      });
    }

    return regions;
  });
}

function previewForTimeline(
  timelineMs: number,
  totalDurationMs: number,
  regions: TimelineRegion[],
  playbackAssets: LibraryPlaybackAssets | null,
): HoverPreview | null {
  const clampedTimelineMs = clampTimelineMs(timelineMs, totalDurationMs);
  const region = regionForTimeline(regions, clampedTimelineMs);
  if (!region) {
    return null;
  }
  const leftPercent = percentOf(clampedTimelineMs, totalDurationMs);
  if (region.kind === 'pause') {
    return {
      label: `${formatDuration(region.endMs - region.startMs)} gap, resumes at ${formatTimestamp(
        nextSegmentStart(region.segment, regions),
      )}`,
      leftPercent,
      thumbnailUrl: null,
      title: 'Paused',
    };
  }

  const offsetMs = clampedTimelineMs - region.segment.playbackOffsetMs;
  const playbackSegment = playbackAssets?.segments.find(
    (segment) => segment.segmentId === region.segment.id,
  );
  const thumbnail = nearestThumbnail(playbackSegment, offsetMs);

  return {
    label: `Segment ${region.segment.segmentIndex} - ${formatDuration(offsetMs)}`,
    leftPercent,
    thumbnailUrl: thumbnail?.url ?? null,
    title: formatDuration(clampedTimelineMs),
  };
}

function regionForTimeline(
  regions: TimelineRegion[],
  timelineMs: number,
): TimelineRegion | null {
  return (
    regions.find(
      (region) => timelineMs >= region.startMs && timelineMs <= region.endMs,
    ) ?? null
  );
}

function resolvePlayableTimelineMs(
  segments: LibraryRecordingSegment[],
  timelineMs: number,
): number {
  const target = segmentForTimeline(segments, timelineMs);
  if (!target) {
    return timelineMs;
  }
  const segment = segments[target.index];
  if (!segment) {
    return timelineMs;
  }

  return segment.playbackOffsetMs + target.offsetMs;
}

function nearestThumbnail(
  playbackSegment: LibraryPlaybackSegment | undefined,
  offsetMs: number,
) {
  return playbackSegment?.thumbnails.reduce<
    LibraryPlaybackSegment['thumbnails'][number] | undefined
  >((nearest, thumbnail) => {
    if (!nearest) {
      return thumbnail;
    }

    return Math.abs(thumbnail.offsetMs - offsetMs) <
      Math.abs(nearest.offsetMs - offsetMs)
      ? thumbnail
      : nearest;
  }, undefined);
}

function nextSegmentStart(
  segment: LibraryRecordingSegment,
  regions: TimelineRegion[],
): number | null {
  const nextRegion = regions.find(
    (region) =>
      region.kind === 'segment' &&
      region.segment.segmentIndex > segment.segmentIndex,
  );

  return nextRegion?.segment.startedAtMs ?? null;
}

function timelineRegionLabel(region: TimelineRegion): string {
  if (region.kind === 'pause') {
    return `Paused after segment ${region.segment.segmentIndex}`;
  }

  return `Segment ${region.segment.segmentIndex}`;
}

function clampTimelineMs(timelineMs: number, totalDurationMs: number): number {
  return Math.max(0, Math.min(totalDurationMs, timelineMs));
}

function percentOf(value: number, total: number): number {
  if (total <= 0) {
    return 0;
  }

  return Math.max(0, Math.min(100, (value / total) * 100));
}

interface RulerTick {
  leftPx: number;
  major: boolean;
  timeMs: number;
}

function minVisibleWindowMs(totalDurationMs: number): number {
  return Math.min(Math.max(1, totalDurationMs), MIN_TRACK_WINDOW_MS);
}

function clampVisibleWindowMs(
  visibleWindowMs: number,
  totalDurationMs: number,
): number {
  const minimum = minVisibleWindowMs(totalDurationMs);
  return Math.min(
    Math.max(minimum, visibleWindowMs),
    Math.max(minimum, totalDurationMs),
  );
}

function clampScrollLeft(scrollLeft: number, viewport: HTMLDivElement): number {
  return Math.max(
    0,
    Math.min(
      scrollLeft,
      Math.max(0, viewport.scrollWidth - viewport.clientWidth),
    ),
  );
}

function buildRulerTicks({
  scrollLeft,
  timelineViewportWidth,
  timelineWidthPx,
  totalDurationMs,
  visibleWindowMs,
}: {
  scrollLeft: number;
  timelineViewportWidth: number;
  timelineWidthPx: number;
  totalDurationMs: number;
  visibleWindowMs: number;
}): RulerTick[] {
  const majorIntervalMs = chooseRulerIntervalMs(
    (visibleWindowMs / timelineViewportWidth) * RULER_TARGET_TICK_PX,
  );
  const minorIntervalMs = Math.max(1, majorIntervalMs / 4);
  const visibleStartMs = clampTimelineMs(
    (scrollLeft / timelineWidthPx) * totalDurationMs,
    totalDurationMs,
  );
  const visibleEndMs = clampTimelineMs(
    ((scrollLeft + timelineViewportWidth) / timelineWidthPx) * totalDurationMs,
    totalDurationMs,
  );
  const firstTickMs =
    Math.floor(
      Math.max(0, visibleStartMs - majorIntervalMs) / minorIntervalMs,
    ) * minorIntervalMs;
  const lastTickMs = Math.min(totalDurationMs, visibleEndMs + majorIntervalMs);
  const ticks: RulerTick[] = [];

  for (
    let timeMs = firstTickMs;
    timeMs <= lastTickMs;
    timeMs += minorIntervalMs
  ) {
    const major = Math.round(timeMs / minorIntervalMs) % 4 === 0;
    ticks.push({
      leftPx: (timeMs / totalDurationMs) * timelineWidthPx,
      major,
      timeMs,
    });
  }

  return ticks;
}

function chooseRulerIntervalMs(targetIntervalMs: number): number {
  const intervals = [
    1_000, 2_000, 5_000, 10_000, 15_000, 30_000, 60_000, 120_000, 300_000,
    600_000, 900_000, 1_800_000, 3_600_000,
  ];

  return (
    intervals.find((interval) => interval >= targetIntervalMs) ??
    intervals[intervals.length - 1]!
  );
}

function TrackTimeline({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playbackCommandsRef,
  playbackSnapshot,
  playbackAssets,
  thumbnailStatus,
  waveforms,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playbackCommandsRef: { current: TrackPlaybackCommands | null };
  playbackSnapshot: TrackPlaybackSnapshot;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  waveforms: LibraryWaveforms | null;
}) {
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{ active: boolean; pointerId: number | null }>({
    active: false,
    pointerId: null,
  });
  const programmaticScrollRef = useRef<number | null>(null);
  const [followPlayhead, setFollowPlayhead] = useState(true);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [viewportWidth, setViewportWidth] = useState(
    DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  );
  const [visibleWindowMs, setVisibleWindowMs] = useState(() =>
    clampVisibleWindowMs(
      Math.min(INITIAL_TRACK_WINDOW_MS, totalDurationMs),
      totalDurationMs,
    ),
  );
  const timelineViewportWidth = Math.max(
    1,
    viewportWidth - TRACK_LABEL_WIDTH_PX,
  );
  const timelineWidthPx = Math.max(
    timelineViewportWidth,
    (totalDurationMs / visibleWindowMs) * timelineViewportWidth,
  );
  const playheadLeftPx =
    TRACK_LABEL_WIDTH_PX +
    (clampTimelineMs(playbackSnapshot.displayedTimelineMs, totalDurationMs) /
      totalDurationMs) *
      timelineWidthPx;
  const zoomPercent = Math.max(
    100,
    Math.round((totalDurationMs / visibleWindowMs) * 100),
  );
  const canZoomIn = visibleWindowMs > minVisibleWindowMs(totalDurationMs);
  const canZoomOut = visibleWindowMs < totalDurationMs;
  const rulerTicks = useMemo(
    () =>
      buildRulerTicks({
        scrollLeft,
        timelineViewportWidth,
        timelineWidthPx,
        totalDurationMs,
        visibleWindowMs,
      }),
    [
      scrollLeft,
      timelineViewportWidth,
      timelineWidthPx,
      totalDurationMs,
      visibleWindowMs,
    ],
  );

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    function updateViewportWidth() {
      setViewportWidth(
        Math.max(1, viewport?.clientWidth ?? DEFAULT_TRACK_VIEWPORT_WIDTH_PX),
      );
    }

    updateViewportWidth();
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', updateViewportWidth);
      return () => window.removeEventListener('resize', updateViewportWidth);
    }

    const observer = new ResizeObserver(updateViewportWidth);
    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    const scrollViewport = viewport;
    function handleScroll() {
      setScrollLeft(scrollViewport.scrollLeft);
      if (programmaticScrollRef.current !== null) {
        window.clearTimeout(programmaticScrollRef.current);
        programmaticScrollRef.current = window.setTimeout(() => {
          programmaticScrollRef.current = null;
        }, 80);
        return;
      }
      if (playbackSnapshot.isPlaying) {
        setFollowPlayhead(false);
      }
    }

    scrollViewport.addEventListener('scroll', handleScroll, { passive: true });
    return () => scrollViewport.removeEventListener('scroll', handleScroll);
  }, [playbackSnapshot.isPlaying]);

  useEffect(() => {
    if (!followPlayhead || !playbackSnapshot.isPlaying) {
      return;
    }
    keepPlayheadVisible(playheadLeftPx);
    // keepPlayheadVisible reads viewport refs and does not affect React data flow.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [followPlayhead, playbackSnapshot.isPlaying, playheadLeftPx]);

  useEffect(() => {
    const frameId = window.requestAnimationFrame(() => setFollowPlayhead(true));
    return () => window.cancelAnimationFrame(frameId);
  }, [playbackSnapshot.seekRevision]);

  function markProgrammaticScroll() {
    if (programmaticScrollRef.current !== null) {
      window.clearTimeout(programmaticScrollRef.current);
    }
    programmaticScrollRef.current = window.setTimeout(() => {
      programmaticScrollRef.current = null;
    }, 80);
  }

  function keepPlayheadVisible(nextPlayheadLeftPx: number) {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    const visibleStartPx = viewport.scrollLeft + TRACK_LABEL_WIDTH_PX;
    const visibleEndPx = viewport.scrollLeft + viewport.clientWidth;
    let nextScrollLeft = viewport.scrollLeft;
    if (nextPlayheadLeftPx > visibleEndPx - TRACK_FOLLOW_EDGE_PX) {
      nextScrollLeft =
        nextPlayheadLeftPx - viewport.clientWidth + TRACK_FOLLOW_EDGE_PX;
    } else if (nextPlayheadLeftPx < visibleStartPx + TRACK_FOLLOW_EDGE_PX) {
      nextScrollLeft =
        nextPlayheadLeftPx - TRACK_LABEL_WIDTH_PX - TRACK_FOLLOW_EDGE_PX;
    } else {
      return;
    }

    markProgrammaticScroll();
    viewport.scrollLeft = clampScrollLeft(nextScrollLeft, viewport);
    setScrollLeft(viewport.scrollLeft);
  }

  function timelineMsFromClientX(clientX: number): number | null {
    const content = contentRef.current;
    if (!content) {
      return null;
    }
    const x = clientX - content.getBoundingClientRect().left;
    const timelineX = x - TRACK_LABEL_WIDTH_PX;
    if (timelineX < 0) {
      return null;
    }

    return clampTimelineMs(
      (timelineX / timelineWidthPx) * totalDurationMs,
      totalDurationMs,
    );
  }

  function commitPointerTimelineMs(clientX: number) {
    const nextTimelineMs = timelineMsFromClientX(clientX);
    if (nextTimelineMs === null) {
      return;
    }
    playbackCommandsRef.current?.commitTimelineInput(nextTimelineMs);
    setFollowPlayhead(true);
  }

  function handleTimelinePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (
      event.button !== 0 ||
      (event.target instanceof Element &&
        event.target.closest('[data-library-action]'))
    ) {
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs === null) {
      return;
    }

    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStateRef.current = { active: true, pointerId: event.pointerId };
    playbackCommandsRef.current?.handleTimelineInput(nextTimelineMs);
    setFollowPlayhead(true);
  }

  function handleTimelinePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (
      !dragStateRef.current.active ||
      dragStateRef.current.pointerId !== event.pointerId
    ) {
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs !== null) {
      playbackCommandsRef.current?.handleTimelineInput(nextTimelineMs);
    }
  }

  function handleTimelinePointerEnd(event: PointerEvent<HTMLDivElement>) {
    if (
      !dragStateRef.current.active ||
      dragStateRef.current.pointerId !== event.pointerId
    ) {
      return;
    }

    dragStateRef.current = { active: false, pointerId: null };
    event.currentTarget.releasePointerCapture(event.pointerId);
    commitPointerTimelineMs(event.clientX);
  }

  function handleTimelineWheel(event: WheelEvent<HTMLDivElement>) {
    if (!event.ctrlKey) {
      return;
    }

    event.preventDefault();
    const zoomIn = event.deltaY < 0;
    zoomVisibleWindow(
      zoomIn
        ? visibleWindowMs / TRACK_ZOOM_STEP
        : visibleWindowMs * TRACK_ZOOM_STEP,
      event.clientX,
    );
  }

  function zoomVisibleWindow(nextWindowMs: number, anchorClientX?: number) {
    const viewport = viewportRef.current;
    const nextVisibleWindowMs = clampVisibleWindowMs(
      nextWindowMs,
      totalDurationMs,
    );
    if (nextVisibleWindowMs === visibleWindowMs) {
      return;
    }

    const viewportRect = viewport?.getBoundingClientRect();
    const anchorX =
      anchorClientX && viewportRect
        ? anchorClientX - viewportRect.left
        : TRACK_LABEL_WIDTH_PX + timelineViewportWidth / 2;
    const anchorTimelineMs =
      timelineMsFromClientX(
        anchorClientX ??
          (viewportRect?.left ?? 0) +
            TRACK_LABEL_WIDTH_PX +
            timelineViewportWidth / 2,
      ) ??
      clampTimelineMs(playbackSnapshot.displayedTimelineMs, totalDurationMs);
    const nextTimelineWidthPx = Math.max(
      timelineViewportWidth,
      (totalDurationMs / nextVisibleWindowMs) * timelineViewportWidth,
    );

    setVisibleWindowMs(nextVisibleWindowMs);
    window.requestAnimationFrame(() => {
      if (!viewport) {
        return;
      }
      const nextScrollLeft =
        TRACK_LABEL_WIDTH_PX +
        (anchorTimelineMs / totalDurationMs) * nextTimelineWidthPx -
        anchorX;
      markProgrammaticScroll();
      viewport.scrollLeft = clampScrollLeft(nextScrollLeft, viewport);
      setScrollLeft(viewport.scrollLeft);
    });
  }

  return (
    <div className="bg-background flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between gap-3 border-b px-3">
        <div className="text-sm font-medium">Tracks</div>
        <div className="flex min-w-0 items-center gap-2">
          <div className="text-muted-foreground truncate text-xs">
            {waveforms?.error ?? 'Video and audio timeline'}
          </div>
          <div
            aria-label="Track zoom"
            className="flex shrink-0 items-center gap-1"
          >
            <Button
              aria-label="Zoom out tracks"
              disabled={!canZoomOut}
              size="icon-xs"
              variant="ghost"
              onClick={() =>
                zoomVisibleWindow(visibleWindowMs * TRACK_ZOOM_STEP)
              }
            >
              <Minus className="size-3" />
            </Button>
            <div
              aria-live="polite"
              className="text-muted-foreground w-12 text-center text-xs tabular-nums"
            >
              {zoomPercent}%
            </div>
            <Button
              aria-label="Zoom in tracks"
              disabled={!canZoomIn}
              size="icon-xs"
              variant="ghost"
              onClick={() =>
                zoomVisibleWindow(visibleWindowMs / TRACK_ZOOM_STEP)
              }
            >
              <Plus className="size-3" />
            </Button>
          </div>
        </div>
      </div>
      <ScrollArea
        className="min-h-0 flex-1"
        scrollbars="both"
        viewportRef={viewportRef}
      >
        <div
          ref={contentRef}
          aria-label="Track timeline"
          className="relative cursor-crosshair"
          role="group"
          style={{
            width: TRACK_LABEL_WIDTH_PX + timelineWidthPx,
          }}
          onPointerCancel={handleTimelinePointerEnd}
          onPointerDown={handleTimelinePointerDown}
          onPointerMove={handleTimelinePointerMove}
          onPointerUp={handleTimelinePointerEnd}
          onWheel={handleTimelineWheel}
        >
          <div
            aria-hidden="true"
            className="bg-primary pointer-events-none absolute top-0 bottom-0 z-30 w-px"
            style={{ left: playheadLeftPx }}
          />
          <div
            className="grid"
            style={{
              gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${timelineWidthPx}px`,
            }}
          >
            <div
              className="bg-background sticky left-0 z-40 border-r border-b px-3"
              style={{ height: TRACK_RULER_HEIGHT_PX }}
            />
            <TimeRuler ticks={rulerTicks} timelineWidthPx={timelineWidthPx} />
            {detail.tracks.map((track) => (
              <TrackLane
                detail={detail}
                key={track.id}
                mutedTrackIds={mutedTrackIds}
                onToggleTrackMuted={onToggleTrackMuted}
                playbackAssets={playbackAssets}
                thumbnailStatus={thumbnailStatus}
                timelineWidthPx={timelineWidthPx}
                totalDurationMs={totalDurationMs}
                track={track}
                waveforms={waveforms}
              />
            ))}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

function TimeRuler({
  ticks,
  timelineWidthPx,
}: {
  ticks: RulerTick[];
  timelineWidthPx: number;
}) {
  return (
    <div
      aria-label="Time ruler"
      className="bg-background relative sticky top-0 z-20 border-b"
      style={{ height: TRACK_RULER_HEIGHT_PX, width: timelineWidthPx }}
    >
      {ticks.map((tick) => (
        <div
          className="border-muted-foreground/40 absolute bottom-0 border-l"
          key={`${tick.timeMs}-${tick.major ? 'major' : 'minor'}`}
          style={{
            height: tick.major ? 22 : 10,
            left: tick.leftPx,
          }}
        >
          {tick.major ? (
            <div className="text-muted-foreground mt-0.5 ml-1 text-[0.625rem] tabular-nums">
              {formatDuration(tick.timeMs)}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}

function TrackLane({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playbackAssets,
  thumbnailStatus,
  timelineWidthPx,
  totalDurationMs,
  track,
  waveforms,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  timelineWidthPx: number;
  totalDurationMs: number;
  track: LibraryRecordingTrack;
  waveforms: LibraryWaveforms | null;
}) {
  const waveformTrack = waveforms?.tracks.find(
    (waveform) => waveform.trackId === track.kind,
  );
  const audioTrack = track.kind !== 'video';
  const muted = mutedTrackIds.has(track.id);

  return (
    <>
      <div
        className="bg-background sticky left-0 z-10 flex min-w-0 items-start gap-2 border-r border-b p-2"
        style={{ height: TRACK_LANE_HEIGHT_PX }}
      >
        {audioTrack ? (
          <Button
            aria-label={muted ? `Unmute ${track.label}` : `Mute ${track.label}`}
            data-library-action
            size="icon"
            variant={muted ? 'outline' : 'ghost'}
            onClick={() => onToggleTrackMuted(track.id)}
          >
            {muted ? (
              <VolumeX className="size-3.5" />
            ) : (
              <Volume2 className="size-3.5" />
            )}
          </Button>
        ) : null}
        <div className="min-w-0">
          <div className="truncate text-xs font-medium">{track.label}</div>
          <div className="text-muted-foreground truncate text-xs">
            {track.codec ?? track.description}
          </div>
        </div>
      </div>
      <div
        className="bg-muted relative overflow-hidden border-b"
        style={{ height: TRACK_LANE_HEIGHT_PX, width: timelineWidthPx }}
      >
        {detail.segments.map((segment) => (
          <div
            className={cn(
              'absolute top-1 bottom-1 overflow-hidden border',
              segment.fileExists &&
                segment.state === 'finalized' &&
                track.kind !== 'video'
                ? 'bg-primary/10 border-primary/30'
                : 'bg-muted-foreground/10 border-muted-foreground/20',
              segment.fileExists &&
                segment.state === 'finalized' &&
                track.kind === 'video' &&
                'bg-background',
            )}
            key={`${track.id}-${segment.id}`}
            style={{
              left:
                (segment.playbackOffsetMs / totalDurationMs) * timelineWidthPx,
              width:
                ((segment.durationMs ?? 0) / totalDurationMs) * timelineWidthPx,
            }}
            title={`Segment ${segment.segmentIndex}`}
          >
            {track.kind === 'video' ? (
              <VideoThumbnailStrip
                isLoading={thumbnailStatus === 'loading'}
                thumbnails={
                  playbackAssets?.segments.find(
                    (playbackSegment) =>
                      playbackSegment.segmentId === segment.id,
                  )?.thumbnails ?? []
                }
              />
            ) : waveformTrack ? (
              <WaveformShape
                samples={
                  waveformTrack.segments.find(
                    (waveform) => waveform.segmentId === segment.id,
                  )?.samples ?? []
                }
              />
            ) : null}
          </div>
        ))}
      </div>
    </>
  );
}

function VideoThumbnailStrip({
  isLoading,
  thumbnails,
}: {
  isLoading: boolean;
  thumbnails: Array<{ offsetMs: number; url: string }>;
}) {
  if (thumbnails.length === 0) {
    return (
      <div className="text-muted-foreground flex h-full items-center justify-center px-2 text-[0.625rem]">
        {isLoading ? 'Thumbnails loading...' : 'Thumbnails unavailable'}
      </div>
    );
  }

  return (
    <div className="flex h-full w-full">
      {thumbnails.map((thumbnail) => (
        <img
          alt=""
          aria-label="Video timeline thumbnail"
          className="h-full min-w-0 flex-1 border-r object-cover last:border-r-0"
          key={`${thumbnail.url}-${thumbnail.offsetMs}`}
          src={thumbnail.url}
        />
      ))}
    </div>
  );
}

function WaveformShape({ samples }: { samples: number[] }) {
  if (samples.length === 0) {
    return null;
  }
  const points = samples.map((sample, index) => {
    const x = samples.length === 1 ? 0 : (index / (samples.length - 1)) * 100;
    const y = 50 - Math.max(0.03, Math.min(1, sample)) * 45;

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });
  const reflected = [...samples].reverse().map((sample, index) => {
    const x =
      samples.length === 1
        ? 0
        : ((samples.length - 1 - index) / (samples.length - 1)) * 100;
    const y = 50 + Math.max(0.03, Math.min(1, sample)) * 45;

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });

  return (
    <svg
      aria-label="Audio waveform"
      className="text-primary/75 h-full w-full px-1"
      preserveAspectRatio="none"
      viewBox="0 0 100 100"
    >
      <polygon
        className="fill-current"
        points={`${points.join(' ')} ${reflected.join(' ')}`}
      />
    </svg>
  );
}

function StudioSidebar({ detail }: { detail: LibraryRecordingDetail }) {
  return (
    <aside className="bg-sidebar text-sidebar-foreground flex h-full flex-col">
      <div className="border-b p-3">
        <div className="text-sm font-medium">Details</div>
        <div className="text-muted-foreground mt-1 text-xs">
          Annotation and metadata workspace
        </div>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-3 p-3 text-xs">
          {detail.availability !== 'available' ? (
            <Alert
              variant={
                detail.availability === 'locked' ? 'default' : 'destructive'
              }
            >
              {detail.availability === 'locked' ? <Lock /> : <AlertTriangle />}
              <AlertTitle>
                {detail.availability === 'locked'
                  ? 'Recording in progress'
                  : 'Missing video files'}
              </AlertTitle>
              <AlertDescription>
                {detail.missingReasons.join(' ') ||
                  'Recording files are unavailable.'}
              </AlertDescription>
            </Alert>
          ) : null}
          <MetadataRow label="Recording" value={detail.id} />
          <MetadataRow
            label="Started"
            value={formatTimestamp(detail.startedAtMs)}
          />
          <MetadataRow
            label="Duration"
            value={formatDuration(detail.durationMs)}
          />
          <Separator />
          <div className="text-muted-foreground">
            Annotations, extracted metadata, and source-linked notes will appear
            here.
          </div>
        </div>
      </ScrollArea>
    </aside>
  );
}

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[5rem_1fr] gap-2">
      <div className="text-muted-foreground">{label}</div>
      <div className="truncate">{value}</div>
    </div>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecordingDeletable(detail: LibraryRecordingDetail): boolean {
  return detail.status === 'completed' || detail.status === 'failed';
}
