import { useCallback, useState } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type { GetOcrDocumentInput } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import '@/index.css';
import type { StudioInspectorSelection } from '../../navigation/studio-inspector-selection';
import type { StudioVisualFrame } from '../../observations/studio-visual-observations';
import { workflowDetail } from '../../testing/studio-workflow-fixture';
import { useStudioTimelineStore } from '../../timeline/state/studio-timeline-store';
import { StudioSidebar, type StudioSidebarModel } from '../studio-sidebar';

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
  vi.resetAllMocks();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
});
const noop = () => undefined;
const mutation = async () => undefined;
const transcript: StudioInspectorSelection = {
  kind: 'transcript-segment',
  chunkResult: 'text',
  confidence: null,
  endOffsetMs: 900,
  model: 'synthetic',
  provider: 'local-whisper-cpp',
  segmentId: 'segment',
  sourceRole: 'microphone',
  startOffsetMs: 100,
  text: 'Synthetic selected speech',
  timelineOffsetMs: 100,
  trackId: 'microphone',
  trackLabel: 'Microphone',
};
function SidebarWorkflow({
  model,
  onSeek = noop,
}: {
  model: StudioSidebarModel;
  onSeek?: (time: number) => void;
}) {
  const [selection, setSelection] = useState<StudioInspectorSelection | null>(
    model.selection,
  );
  const inspectTranscript = useCallback(() => setSelection(transcript), []);
  const clearTranscript = useCallback(() => setSelection(null), []);
  const inspectClip = useCallback(() => {
    const clip = Object.keys(useStudioTimelineStore.getState().clipsById)[0];
    if (!clip) throw new Error('Synthetic clip missing');
    useStudioTimelineStore.getState().selectClip(clip);
  }, []);
  const inspectScreen = useCallback(
    () => useStudioTimelineStore.getState().selectLane('video-0'),
    [],
  );
  const inspectExpiredClip = useCallback(() => {
    const store = useStudioTimelineStore.getState();
    const clip = Object.keys(store.clipsById)[0];
    if (!clip) throw new Error('Synthetic clip missing');
    store.initializeFromDetail({ ...model.detail, segments: [] });
    store.selectClip(clip);
  }, [model.detail]);
  const inspectFrame = useCallback(
    () => useStudioTimelineStore.getState().selectVisualFrame('legacy-frame'),
    [],
  );
  return (
    <>
      <button onClick={inspectTranscript}>Inspect transcript</button>
      <button onClick={inspectClip}>Inspect clip</button>
      <button onClick={inspectExpiredClip}>
        Inspect clip after source refresh
      </button>
      <button onClick={inspectScreen}>Inspect screen</button>
      <button onClick={inspectFrame}>Inspect legacy frame</button>
      <StudioSidebar
        model={{ ...model, selection }}
        actions={{
          onSeek,
          onClearActivitySelection: clearTranscript,
          onCreateRedactions: mutation,
          onDeleteRedaction: mutation,
          onUnredactRanges: mutation,
        }}
      />
    </>
  );
}
test('locked recordings with unknown duration show the blocking state and keep Review unavailable', () => {
  const detail = {
    ...workflowDetail,
    availability: 'locked' as const,
    durationMs: null,
    missingReasons: [],
  };
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  render(
    <SidebarWorkflow
      model={{
        detail,
        aiProcessingState: null,
        visualFrames: [],
        selection: null,
      }}
    />,
  );
  expect(screen.getByText('Recording in progress')).toBeTruthy();
  expect(screen.getByText('Recording files are unavailable.')).toBeTruthy();
  expect(screen.getByRole('tab', { name: 'Review' })).toBeDisabled();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});

test('a late selection event for a removed clip cannot reopen its editing fields', async () => {
  useStudioTimelineStore.getState().initializeFromDetail(workflowDetail);
  render(
    <SidebarWorkflow
      model={{
        detail: workflowDetail,
        aiProcessingState: null,
        visualFrames: [],
        selection: null,
      }}
    />,
  );
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Inspect clip after source refresh' }),
  );
  expect(screen.queryByRole('textbox', { name: 'Clip title' })).toBeNull();
  expect(screen.getByText('Recording details')).toBeTruthy();
  expect(useStudioTimelineStore.getState().clipsById).toEqual({});
});
test('source inspection selects Details, preserves transcript provenance, and clip edits update the owning store', async () => {
  useStudioTimelineStore.getState().initializeFromDetail(workflowDetail);
  const seek = vi.fn();
  render(
    <SidebarWorkflow
      onSeek={seek}
      model={{
        detail: workflowDetail,
        aiProcessingState: null,
        visualFrames: [],
        selection: null,
      }}
    />,
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Inspect transcript' }),
  );
  expect(screen.getByText(transcript.text)).toBeTruthy();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Jump to source' }));
  expect(seek).toHaveBeenLastCalledWith(100);
  await userEvent.click(
    screen.getByRole('button', { name: 'Clear selected source' }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Inspect clip' }));
  const title = await screen.findByRole('textbox', { name: 'Clip title' });
  await userEvent.clear(title);
  await userEvent.type(title, 'Renamed synthetic clip');
  expect(
    Object.values(useStudioTimelineStore.getState().clipsById).some(
      (clip) => clip.title === 'Renamed synthetic clip',
    ),
  ).toBe(true);
});
test.each([true, false])(
  'legacy frames retain an honest source fallback and source-local navigation (video source=%s)',
  async (hasVideo) => {
    const detail = {
      ...workflowDetail,
      tracks: hasVideo ? workflowDetail.tracks : [],
    };
    const frame: StudioVisualFrame = {
      id: 'legacy-frame',
      frameTimestampMs: 500,
      endTimestampMs: 501,
      sourceFrameRef: 'synthetic-frame',
      sourceFrameRefs: ['synthetic-frame'],
      ocrRegions: [],
      vlmObservations: [],
    };
    const seek = vi.fn();
    native.invoke.mockImplementation(
      async (command: string, args: { input: GetOcrDocumentInput }) => {
        if (command === 'log_frontend_failure') return;
        if (command === 'getOcrDocument')
          return {
            recordingId: args.input.recordingId,
            sourceTrackId: args.input.sourceTrackId,
            job: null,
            document: null,
          };
        throw new Error(`Unexpected native boundary: ${command}`);
      },
    );
    useStudioTimelineStore.getState().initializeFromDetail(detail);
    const model: StudioSidebarModel = {
      detail,
      aiProcessingState: null,
      visualFrames: [frame],
      selection: null,
    };
    const view = render(<SidebarWorkflow model={model} onSeek={seek} />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Inspect legacy frame' }),
    );
    await screen.findByText('Selected frame');
    await userEvent.click(
      screen.getByRole('button', { name: 'Jump to frame' }),
    );
    expect(seek).toHaveBeenLastCalledWith(500);
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith('getOcrDocument', {
        input: {
          recordingId: detail.id,
          sourceTrackId: hasVideo ? 'video-0' : '',
          timestampMs: 500,
          documentId: null,
        },
      }),
    );
    view.rerender(
      <SidebarWorkflow
        onSeek={seek}
        model={{ ...model, resolveSource: () => undefined }}
      />,
    );
    await screen.findByText('No recorded source at this time.');
    expect(screen.queryByRole('button', { name: 'Jump to frame' })).toBeNull();
    if (hasVideo) {
      await userEvent.click(
        screen.getByRole('button', { name: 'Inspect screen' }),
      );
      expect(screen.getByText('No recorded source at this time.')).toBeTruthy();
    }
  },
);
