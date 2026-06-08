import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  LibraryWaveforms,
} from '@daydream/shared';
import {
  AlertTriangle,
  Lock,
  Maximize,
  Pause,
  Play,
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
  deleteLibraryRecording,
  getLibraryRecording,
  getRecordingWaveforms,
  localAssetUrl,
  prepareLibraryPlaybackAssets,
  prepareLibraryPlaybackThumbnails,
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

  const load = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    const isCurrentLoad = () => loadRunRef.current === loadRunId;

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
        prepareLibraryPlaybackAssets({ recordingId })
          .then((assets) => {
            if (!isCurrentLoad() || assets.recordingId !== recordingId) {
              return;
            }
            setPlaybackAssets(assets);
            if (assets.error !== null) {
              return;
            }
            setThumbnailStatus('loading');
            void prepareLibraryPlaybackThumbnails({ recordingId })
              .then((thumbnailAssets) => {
                if (
                  !isCurrentLoad() ||
                  thumbnailAssets.recordingId !== recordingId
                ) {
                  return;
                }
                setPlaybackAssets((current) =>
                  mergePlaybackThumbnails(current, thumbnailAssets),
                );
                setThumbnailStatus(
                  thumbnailAssets.error === null ? 'ready' : 'error',
                );
              })
              .catch(() => {
                if (!isCurrentLoad()) {
                  return;
                }
                // Timeline thumbnails are preview-only; failed previews must not block playback.
                setThumbnailStatus('error');
              });
          })
          .catch((playbackError: unknown) => {
            if (!isCurrentLoad()) {
              return;
            }
            setPlaybackAssets({
              error: errorMessage(playbackError),
              recordingId,
              segments: [],
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

function RecordingStudio({
  detail,
  playbackAssets,
  thumbnailStatus,
  waveforms,
}: RecordingStudioProps) {
  const [mutedTrackIds, setMutedTrackIds] = useState<Set<string>>(
    () => new Set(),
  );

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
              playbackAssets={playbackAssets}
            />
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize={32} minSize={18}>
            <TrackTimeline
              detail={detail}
              mutedTrackIds={mutedTrackIds}
              onToggleTrackMuted={toggleTrackMuted}
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

function mergePlaybackThumbnails(
  current: LibraryPlaybackAssets | null,
  thumbnails: {
    error: string | null;
    recordingId: string;
    segments: Array<{
      segmentId: string;
      thumbnails: Array<{ offsetMs: number; path: string }>;
    }>;
  },
): LibraryPlaybackAssets | null {
  if (!current || current.recordingId !== thumbnails.recordingId) {
    return current;
  }

  return {
    ...current,
    segments: current.segments.map((segment) => ({
      ...segment,
      thumbnails:
        thumbnails.segments.find(
          (thumbnailSegment) =>
            thumbnailSegment.segmentId === segment.segmentId,
        )?.thumbnails ?? segment.thumbnails,
    })),
  };
}

function StudioPlayer({
  detail,
  mutedTrackIds,
  playbackAssets,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  playbackAssets: LibraryPlaybackAssets | null;
}) {
  const audioRefs = useRef(new Map<string, HTMLAudioElement>());
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const pendingSeekRef = useRef<number | null>(null);
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
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
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

  useEffect(() => {
    const video = videoRef.current;
    if (!video) {
      return;
    }
    video.volume = volume;
    video.muted = isMuted;
  }, [isMuted, volume]);

  useEffect(() => {
    for (const [trackId, audio] of audioRefs.current) {
      audio.volume = volume;
      audio.muted = isMuted || mutedTrackIds.has(trackId);
    }
  }, [isMuted, mutedTrackIds, volume]);

  useEffect(() => {
    if (!isPlaying || resumeProgress !== null) {
      return;
    }
    void playMediaElements();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentIndex, isPlaying, resumeProgress]);

  if (detail.availability !== 'available' || !currentSegment) {
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
    if (!video || resumeProgress !== null) {
      return;
    }
    if (video.paused) {
      await playMediaElements();
      setIsPlaying(true);
    } else {
      pauseMediaElements();
      setIsPlaying(false);
    }
  }

  async function playMediaElements() {
    const video = videoRef.current;
    if (!video) {
      return;
    }
    syncAudioElements(video.currentTime, true);
    await video.play();
    await Promise.allSettled(
      [...audioRefs.current.values()].map((audio) => audio.play()),
    );
  }

  function pauseMediaElements() {
    videoRef.current?.pause();
    for (const audio of audioRefs.current.values()) {
      audio.pause();
    }
  }

  function syncAudioElements(currentTimeSeconds: number, force = false) {
    for (const [trackId, audio] of audioRefs.current) {
      audio.volume = volume;
      audio.muted = isMuted || mutedTrackIds.has(trackId);
      if (force || Math.abs(audio.currentTime - currentTimeSeconds) > 0.25) {
        audio.currentTime = currentTimeSeconds;
      }
    }
  }

  function seekTimeline(nextTimelineMs: number) {
    const target = segmentForTimeline(playableSegments, nextTimelineMs);
    if (!target) {
      return;
    }
    setCurrentIndex(target.index);
    pendingSeekRef.current = target.offsetMs / 1000;
    const video = videoRef.current;
    if (video && target.index === currentIndex) {
      video.currentTime = target.offsetMs / 1000;
      syncAudioElements(target.offsetMs / 1000, true);
    }
    setCurrentTime(target.offsetMs / 1000);
  }

  function handleEnded() {
    if (!currentSegment) {
      return;
    }

    const nextIndex = currentIndex + 1;
    if (nextIndex >= playableSegments.length) {
      setIsPlaying(false);
      pauseMediaElements();
      return;
    }
    const wasPlaying = isPlaying;
    const advanceToNextSegment = () => {
      setCurrentIndex(nextIndex);
      setCurrentTime(0);
      pendingSeekRef.current = 0;
      setResumeProgress(null);
      setIsPlaying(wasPlaying);
    };
    const pauseGapMs = Math.max(0, currentSegment.pauseAfterMs);
    if (pauseGapMs === 0) {
      advanceToNextSegment();
      return;
    }

    setResumeProgress(0);
    const startedAt = window.performance.now();
    const intervalId = window.setInterval(() => {
      const progress = Math.min(
        1,
        (window.performance.now() - startedAt) / pauseGapMs,
      );
      setResumeProgress(progress);
      if (progress >= 1) {
        window.clearInterval(intervalId);
        advanceToNextSegment();
      }
    }, 50);
  }

  return (
    <div className="flex h-full flex-col bg-black text-white">
      <div className="relative min-h-0 flex-1">
        <video
          ref={videoRef}
          className="h-full w-full object-contain"
          preload="metadata"
          src={localAssetUrl(currentPlaybackSegment.videoPath)}
          onLoadedMetadata={(event) => {
            const pendingSeek = pendingSeekRef.current;
            if (pendingSeek !== null) {
              event.currentTarget.currentTime = pendingSeek;
              pendingSeekRef.current = null;
            }
          }}
          onPause={() => setIsPlaying(false)}
          onPlay={() => setIsPlaying(true)}
          onTimeUpdate={(event) => {
            setCurrentTime(event.currentTarget.currentTime);
            syncAudioElements(event.currentTarget.currentTime);
          }}
          onEnded={handleEnded}
        />
        {currentPlaybackSegment.audio.map((audio) => (
          <audio
            key={`${currentPlaybackSegment.segmentId}-${audio.trackId}`}
            ref={(element) => {
              if (element) {
                audioRefs.current.set(audio.trackId, element);
              } else {
                audioRefs.current.delete(audio.trackId);
              }
            }}
            preload="metadata"
            src={localAssetUrl(audio.path)}
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
            </div>
          </div>
        ) : null}
      </div>

      <div className="flex shrink-0 flex-col gap-2 border-t border-white/10 bg-black/90 p-3">
        <input
          aria-label="Playback position"
          className="w-full accent-white"
          max={Math.max(totalDurationMs, 1)}
          min={0}
          type="range"
          value={Math.min(currentTimelineMs, totalDurationMs)}
          onChange={(event) => seekTimeline(Number(event.currentTarget.value))}
        />
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <Button
              aria-label="Skip back"
              size="icon"
              variant="ghost"
              onClick={() =>
                seekTimeline(Math.max(0, currentTimelineMs - 10_000))
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
                  Math.min(totalDurationMs, currentTimelineMs + 10_000),
                )
              }
            >
              <SkipForward className="size-4" />
            </Button>
            <span className="ml-2 text-xs tabular-nums">
              {formatDuration(currentTimelineMs)} /{' '}
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

function TrackTimeline({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playbackAssets,
  thumbnailStatus,
  waveforms,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  waveforms: LibraryWaveforms | null;
}) {
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));

  return (
    <div className="bg-background flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between border-b px-3">
        <div className="text-sm font-medium">Tracks</div>
        <div className="text-muted-foreground text-xs">
          {waveforms?.error ?? 'Video and audio timeline'}
        </div>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-2 p-3">
          {detail.tracks.map((track) => (
            <TrackLane
              detail={detail}
              key={track.id}
              mutedTrackIds={mutedTrackIds}
              onToggleTrackMuted={onToggleTrackMuted}
              playbackAssets={playbackAssets}
              thumbnailStatus={thumbnailStatus}
              totalDurationMs={totalDurationMs}
              track={track}
              waveforms={waveforms}
            />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}

function TrackLane({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playbackAssets,
  thumbnailStatus,
  totalDurationMs,
  track,
  waveforms,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
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
    <div className="grid grid-cols-[10rem_1fr] gap-3">
      <div className="flex min-w-0 items-start gap-2">
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
      <div className="bg-muted relative h-12 overflow-hidden border">
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
              left: `${(segment.playbackOffsetMs / totalDurationMs) * 100}%`,
              width: `${((segment.durationMs ?? 0) / totalDurationMs) * 100}%`,
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
    </div>
  );
}

function VideoThumbnailStrip({
  isLoading,
  thumbnails,
}: {
  isLoading: boolean;
  thumbnails: Array<{ offsetMs: number; path: string }>;
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
          key={`${thumbnail.path}-${thumbnail.offsetMs}`}
          src={localAssetUrl(thumbnail.path)}
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
