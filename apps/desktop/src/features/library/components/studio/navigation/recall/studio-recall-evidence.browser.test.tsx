import type * as TauriCore from '@tauri-apps/api/core';
import type { Event } from '@tauri-apps/api/event';
import type {
  LibraryRecordingDetail,
  LibraryPlaybackAssets,
  RecallEvidence,
} from '@daydream/shared';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { RecallEvidenceInspector } from '../../inspector/details/recall-evidence-inspector';
import { RecordingStudio } from '../../recording-studio';
import { useStudioTimelineStore } from '../../timeline/state/studio-timeline-store';
import { useStudioRecallEvidence } from './use-studio-recall-evidence';

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
const unsubscribe = vi.fn();
let receive: (event: Event<unknown>) => void;
const evidence: RecallEvidence = {
  evidenceId: 'evidence',
  recordingId: 'recording',
  segmentId: 'segment-2',
  sourceKind: 'microphone',
  sourceTrackId: 'microphone-2',
  sourceId: 'audio-source',
  sourceFrameRef: null,
  documentId: null,
  startOffsetMs: 500,
  endOffsetMs: 1000,
  absoluteStartMs: 1500,
  absoluteEndMs: 2000,
  contentRevision: 'revision',
  text: 'Synthetic private source',
};
const detail: LibraryRecordingDetail = {
  id: 'recording',
  availability: 'available',
  durationMs: 4000,
  endedAtMs: 5000,
  missingReasons: [],
  playableSegmentCount: 2,
  redactions: [],
  sizeBytes: 0,
  startedAtMs: 1000,
  status: 'completed',
  storageDir: '/tmp/synthetic',
  thumbnailPath: null,
  title: null,
  tracks: [
    {
      id: 'video-0',
      kind: 'video',
      label: 'Screen',
      description: '',
      codec: 'h264',
      streamIndex: 0,
    },
    ...[2, 3].map((index) => ({
      id: `microphone-${index}`,
      kind: 'microphone' as const,
      label: `Microphone ${index}`,
      description: '',
      codec: 'opus',
      streamIndex: index,
    })),
  ],
  segments: [2, 3].map((index) => ({
    id: `segment-${index}`,
    desktopAudioStreamIndex: null,
    microphoneAudioStreamIndex: index,
    durationMs: 2000,
    endedAtMs: 3000 + (index - 2) * 2000,
    fileExists: true,
    filePath: '/tmp/synthetic.mkv',
    pauseAfterMs: 0,
    playbackOffsetMs: (index - 2) * 2000,
    segmentIndex: index - 2,
    startedAtMs: 1000 + (index - 2) * 2000,
    state: 'finalized' as const,
    videoStreamIndex: 0,
  })),
};
const assets: LibraryPlaybackAssets = {
  error: null,
  recordingId: detail.id,
  sessionId: 'session',
  segments: detail.segments.map((segment) => ({
    segmentId: segment.id,
    audio: [],
    thumbnails: [],
    videoUrl: 'http://127.0.0.1/synthetic-video',
  })),
};
const noAction = () => undefined;
const noMutation = async () => undefined;

test('single-fragment inspector callers keep the anchor-track hide contract', async () => {
  const mask = vi.fn(noMutation);
  render(
    <RecallEvidenceInspector
      evidence={evidence}
      onSeek={noAction}
      onClear={noAction}
      onCreateRedactions={mask}
    />,
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Hide source interval' }),
  );
  expect(mask).toHaveBeenCalledWith([
    { startOffsetMs: 500, endOffsetMs: 1000, trackIds: ['microphone-2'] },
  ]);
});

test('a cross-fragment Recall link shows the full utterance and hides all covered audio aliases', async () => {
  const spanning: RecallEvidence = {
    ...evidence,
    startOffsetMs: 1500,
    endOffsetMs: 2500,
    text: 'Synthetic cross-fragment utterance',
  };
  const mask = vi.fn(noMutation);
  render(
    <RecordingStudio
      detail={detail}
      initialEvidence={spanning}
      playbackAssets={assets}
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      thumbnailStatus="ready"
      onRefreshPlayback={noAction}
      onCreateRedactions={mask}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  expect(await screen.findByText(spanning.text)).toBeTruthy();
  expect(screen.queryByText(/This recall source is unavailable/)).toBeNull();
  await userEvent.click(
    screen.getByRole('button', { name: 'Hide source interval' }),
  );
  expect(mask).toHaveBeenCalledWith([
    {
      startOffsetMs: 1500,
      endOffsetMs: 2500,
      trackIds: ['microphone-2', 'microphone-3'],
    },
  ]);
});

function EvidenceStudio() {
  const read = useStudioRecallEvidence(
    {
      evidenceId: evidence.evidenceId,
      source: 'microphone',
      seek: evidence.startOffsetMs,
    },
    detail.id,
    'privacy',
  );
  return (
    <>
      <p role="status">{read.state?.status}</p>
      <RecordingStudio
        detail={detail}
        initialEvidence={read.state?.evidence ?? null}
        playbackAssets={assets}
        activity={null}
        aiProcessingState={null}
        visualObservations={null}
        thumbnailStatus="ready"
        onRefreshPlayback={noAction}
        onCreateRedactions={noMutation}
        onDeleteRedaction={noMutation}
        onUnredactRanges={noMutation}
      />
    </>
  );
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((accept, deny) => {
    resolve = accept;
    reject = deny;
  });
  return { promise, resolve, reject };
}
function dispatch(payload: unknown) {
  receive({ event: 'recall-state-changed', id: 1, payload });
}
function resolveCalls() {
  return native.invoke.mock.calls.filter(
    ([command]) => command === 'resolveRecallEvidence',
  );
}
function position() {
  return screen
    .getByRole('slider', { name: 'Playback position' })
    .getAttribute('aria-valuenow');
}
beforeEach(() => {
  vi.resetAllMocks();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  vi.spyOn(window.HTMLMediaElement.prototype, 'play').mockResolvedValue(
    undefined,
  );
  vi.spyOn(window.HTMLMediaElement.prototype, 'pause').mockImplementation(
    noAction,
  );
  receive = () => {
    throw new Error('Missing synthetic subscription');
  };
  native.listen.mockImplementation(
    async (name: string, callback: typeof receive) => {
      if (name !== 'recall-state-changed') return noAction;
      receive = callback;
      return unsubscribe;
    },
  );
  native.invoke.mockImplementation(async (command: string) =>
    command === 'resolveRecallEvidence'
      ? evidence
      : { recordingId: detail.id, binMs: 10, error: null, tracks: [] },
  );
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test('screen recall sources retain seeking and hiding in Debug without exposing recognized text', async () => {
  const seek = vi.fn();
  const hide = vi.fn().mockResolvedValue(undefined);
  render(
    <RecallEvidenceInspector
      evidence={{
        ...evidence,
        sourceKind: 'ocr',
        sourceTrackId: 'video-0',
        text: 'Synthetic hidden OCR',
      }}
      debugEnabled
      onSeek={seek}
      onClear={vi.fn()}
      onCreateRedactions={hide}
    />,
  );
  expect(screen.queryByText('Synthetic hidden OCR')).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Jump to source' }));
  expect(seek).toHaveBeenCalledWith(evidence.startOffsetMs);
  await userEvent.click(
    screen.getByRole('button', { name: 'Hide source interval' }),
  );
  await waitFor(() =>
    expect(hide).toHaveBeenCalledWith([
      {
        startOffsetMs: evidence.startOffsetMs,
        endOffsetMs: evidence.endOffsetMs,
        trackIds: ['video-0'],
      },
    ]),
  );
});

test('Studio waits for a validated native subscription before reading, showing or seeking private evidence', async () => {
  const subscription = deferred<() => void>();
  native.listen.mockImplementation((name: string, callback: typeof receive) => {
    if (name !== 'recall-state-changed') return Promise.resolve(noAction);
    receive = callback;
    return subscription.promise;
  });
  render(<EvidenceStudio />);
  await screen.findByText('loading');
  expect(resolveCalls()).toHaveLength(0);
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(position()).toBe('0');
  await act(async () => subscription.resolve(unsubscribe));
  await screen.findByText(evidence.text);
  await waitFor(() => expect(position()).toBe('500'));
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('Studio withholds evidence when native invalidation subscription fails even if its read would succeed', async () => {
  native.listen.mockRejectedValue(new Error('Synthetic listener failure'));
  render(<EvidenceStudio />);
  await screen.findByText('unavailable');
  expect(resolveCalls()).toHaveLength(0);
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(position()).toBe('0');
});

test('privacy events during subscription reconnection discard the old read before its result can show or seek', async () => {
  const oldRead = deferred<RecallEvidence | null>();
  const gap = deferred<() => void>();
  native.invoke.mockImplementation((command: string) =>
    command === 'resolveRecallEvidence'
      ? resolveCalls().length === 1
        ? oldRead.promise
        : Promise.resolve(null)
      : Promise.resolve({
          recordingId: detail.id,
          binMs: 10,
          error: null,
          tracks: [],
        }),
  );
  native.listen.mockImplementation((name: string, callback: typeof receive) => {
    if (name !== 'recall-state-changed') return Promise.resolve(noAction);
    receive = callback;
    return native.listen.mock.calls.filter(
      ([event]) => event === 'recall-state-changed',
    ).length === 1
      ? Promise.resolve(unsubscribe)
      : gap.promise;
  });
  render(<EvidenceStudio />);
  await waitFor(() => expect(resolveCalls()).toHaveLength(1));
  act(() => dispatch({ dayId: null, privacyRevision: 1 }));
  await waitFor(() =>
    expect(
      native.listen.mock.calls.filter(
        ([event]) => event === 'recall-state-changed',
      ),
    ).toHaveLength(2),
  );
  act(() => dispatch({ dayId: null, privacyRevision: 2 }));
  await act(async () => {
    oldRead.resolve(evidence);
    gap.resolve(unsubscribe);
  });
  await screen.findByText('unavailable');
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(position()).toBe('0');
});

test('malformed native metadata immediately withholds ready private evidence and closes its subscription', async () => {
  render(<EvidenceStudio />);
  await screen.findByText(evidence.text);
  const priorPosition = position();
  act(() => dispatch({ dayId: null, privacyRevision: 'invalid' }));
  await screen.findByText('unavailable');
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(unsubscribe).toHaveBeenCalledTimes(1);
  expect(position()).toBe(priorPosition);
});

test('a late private read cannot show or seek after malformed native metadata invalidates its epoch', async () => {
  const pending = deferred<RecallEvidence | null>();
  native.invoke.mockImplementation((command: string) =>
    command === 'resolveRecallEvidence'
      ? pending.promise
      : Promise.resolve({
          recordingId: detail.id,
          binMs: 10,
          error: null,
          tracks: [],
        }),
  );
  render(<EvidenceStudio />);
  await waitFor(() => expect(resolveCalls()).toHaveLength(1));
  act(() => dispatch({ privateText: 'Synthetic untrusted metadata' }));
  await screen.findByText('unavailable');
  await act(async () => pending.resolve(evidence));
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(position()).toBe('0');
});

test('Studio refuses a globally existing microphone alias that belongs to a different segment', async () => {
  native.invoke.mockImplementation(async (command: string) =>
    command === 'resolveRecallEvidence'
      ? { ...evidence, sourceTrackId: 'microphone-3' }
      : { recordingId: detail.id, binMs: 10, error: null, tracks: [] },
  );
  render(<EvidenceStudio />);
  await screen.findByText(/This recall source is unavailable/);
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(position()).toBe('0');
});
