import { useCallback } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type { LibraryRecordingDetail } from '@daydream/shared';
import type { DragEndEvent } from '@dnd-kit/core';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/index.css';
import { PlaybackMediaGroup } from '../playback/media/playback-media-group';
import { useRecordingPlaybackController } from '../playback/use-recording-playback-controller';
import {
  workflowAssets,
  workflowDetail,
  workflowSegment,
} from '../testing/studio-workflow-fixture';
import { useStudioTimelineStore } from './state/studio-timeline-store';
import { TrackTimeline } from './track-timeline';
import { useTrackTimelineController } from './use-track-timeline-controller';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
});
const noop = () => undefined;
const mutation = async () => undefined;
const muted = new Set<string>();
const detail = {
  ...workflowDetail,
  durationMs: 30000,
  segments: [workflowSegment({ durationMs: 30000 })],
};
const assets = workflowAssets(detail);
const asset = assets.segments[0];
const errorDetail: LibraryRecordingDetail = {
  ...detail,
  tracks: [
    ...detail.tracks,
    {
      id: 'microphone',
      kind: 'microphone',
      label: 'Microphone',
      description: 'Synthetic audio',
      codec: 'opus',
      streamIndex: 1,
    },
  ],
  segments: detail.segments.map((segment) => ({
    ...segment,
    microphoneAudioStreamIndex: 1,
  })),
};
function TimelineWorkflow({ failed = false }: { failed?: boolean }) {
  if (!asset) throw new Error('Synthetic playback asset missing');
  const recording = failed ? errorDetail : detail;
  const playback = useRecordingPlaybackController({
    detail: recording,
    playbackAssets: assets,
    mutedTrackIds: muted,
  });
  const play = useCallback(() => {
    void playback.actions.togglePlayback();
  }, [playback.actions]);
  return (
    <TooltipProvider>
      <PlaybackMediaGroup
        active
        asset={asset}
        detail={recording}
        playback={playback}
        startMs={0}
        revision={playback.state.streamRevision}
        timelineOffsetMs={0}
        streamOffsetMs={0}
      />
      <button onClick={play}>Start playback</button>
      <output>{playback.state.isPlaying ? 'Playing' : 'Paused'}</output>
      <div style={{ width: 800, height: 400 }}>
        <TrackTimeline
          detail={recording}
          transcriptReadStatus={failed ? 'error' : 'ready'}
          playback={playback}
          playbackAssets={assets}
          mutedTrackIds={muted}
          activity={null}
          aiProcessingState={null}
          thumbnailStatus="idle"
          visualFrames={[]}
          onCreateRedactions={mutation}
          onUnredactRanges={mutation}
          onRefreshRedactions={noop}
          onToggleTrackMuted={noop}
          onClearInspectorSelection={noop}
          onInspectActivity={noop}
          onInspectTranscript={noop}
        />
      </div>
    </TooltipProvider>
  );
}
test('waveform read failure remains visible while selected ranges can still be zoomed without a reprocessing action', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    if (command === 'getRecordingWaveformTiles')
      throw new Error('Synthetic tile boundary failure');
    throw new Error(`Unexpected native boundary: ${command}`);
  });
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  render(<TimelineWorkflow />);
  await screen.findByText('Waveforms unavailable. Retry in Processing.');
  useStudioTimelineStore.getState().setTimeRangeSelection({
    laneIds: ['video-0'],
    startMs: 1000,
    endMs: 4000,
  });
  await userEvent.click(await screen.findByRole('button', { name: 'Zoom In' }));
  expect(screen.getByRole('button', { name: 'Redact' })).toBeTruthy();
  expect(
    screen.queryByRole('button', { name: 'Selection actions' }),
  ).toBeNull();
  expect(useStudioTimelineStore.getState().selection.ranges).toHaveLength(1);
});

function TimelineActionBoundary() {
  const playback = useRecordingPlaybackController({
    detail,
    playbackAssets: assets,
    mutedTrackIds: muted,
  });
  const controller = useTrackTimelineController({
    detail,
    playback,
    onCreateRedactions: mutation,
    onUnredactRanges: mutation,
    onClearInspectorSelection: noop,
    onInspectActivity: noop,
    onInspectTranscript: noop,
  });
  const dropOutside = useCallback(() => {
    const event: DragEndEvent = {
      activatorEvent: new PointerEvent('pointerup'),
      active: {
        id: 'video-0',
        data: { current: {} },
        rect: { current: { initial: null, translated: null } },
      },
      collisions: null,
      delta: { x: 0, y: 0 },
      over: null,
    };
    controller.handleDragEnd(event);
  }, [controller]);
  return (
    <>
      <button onClick={controller.zoomSelection}>Zoom absent selection</button>
      <button onClick={dropOutside}>Release outside a drop target</button>
      <output>Zoom {controller.viewModel.zoomPercent}%</output>
    </>
  );
}
test('empty selection and a dropped gesture without a target leave zoom and lane order intact', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    if (command === 'getRecordingWaveformTiles')
      return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
    throw new Error(`Unexpected native boundary: ${command}`);
  });
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  const order = useStudioTimelineStore.getState().laneOrder;
  render(<TimelineActionBoundary />);
  const zoom = screen.getByText(/^Zoom \d+%$/).textContent;
  await userEvent.click(
    screen.getByRole('button', { name: 'Zoom absent selection' }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Release outside a drop target' }),
  );
  expect(screen.getByText(/^Zoom \d+%$/)).toHaveTextContent(zoom ?? '');
  expect(useStudioTimelineStore.getState().laneOrder).toEqual(order);
  expect(useStudioTimelineStore.getState().selection.ranges).toHaveLength(0);
});

test('failed audio detail reads remain visible through the sortable lane and clear after a successful refresh', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    if (command === 'getRecordingWaveformTiles')
      return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
    throw new Error(`Unexpected native boundary: ${command}`);
  });
  useStudioTimelineStore.getState().initializeFromDetail(errorDetail);
  const view = render(<TimelineWorkflow failed />);
  await screen.findByText('Details could not load. Refresh to retry.');
  view.rerender(<TimelineWorkflow />);
  await waitFor(() =>
    expect(
      screen.queryByText('Details could not load. Refresh to retry.'),
    ).toBeNull(),
  );
  expect(screen.getByRole('group', { name: 'Track timeline' })).toBeTruthy();
});
test('manual horizontal scrolling during playback suspends follow until the user restores it', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    if (command === 'getRecordingWaveformTiles')
      return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
    throw new Error(`Unexpected native boundary: ${command}`);
  });
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(noop);
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  const view = render(<TimelineWorkflow />);
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic media element missing');
  Object.defineProperty(video, 'readyState', {
    configurable: true,
    get: () => 4,
  });
  await userEvent.click(screen.getByRole('button', { name: 'Start playback' }));
  await screen.findByText('Playing');
  await userEvent.click(screen.getByRole('button', { name: 'Zoom in tracks' }));
  const timeline = screen.getByRole('group', { name: 'Track timeline' });
  const viewport = timeline.closest('[data-slot="scroll-area-viewport"]');
  if (!(viewport instanceof HTMLElement))
    throw new Error('Timeline viewport missing');
  fireEvent.scroll(viewport, { target: { scrollLeft: 200 } });
  await userEvent.click(
    await screen.findByRole('button', { name: 'Follow playback' }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole('button', { name: 'Follow playback' }),
    ).toBeNull(),
  );
  expect(screen.getByText('Playing')).toBeTruthy();
});
