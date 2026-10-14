import type * as TauriCore from '@tauri-apps/api/core';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import '@/index.css';
import { studioStoryRecording } from '../../../recording-studio-route/stories/studio-route-fixture';
import type { StudioVisualFrame } from '../observations/studio-visual-observations';
import { useStudioTimelineStore } from '../timeline/state/studio-timeline-store';
import { StudioSidebar, type StudioSidebarModel } from './studio-sidebar';

vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: vi.fn(),
}));
const actions = {
  onClearActivitySelection: vi.fn(),
  onCreateRedactions: vi.fn(),
  onDeleteRedaction: vi.fn(),
  onSeek: vi.fn(),
  onUnredactRanges: vi.fn(),
};
afterEach(cleanup);

test('a chosen review tab survives detail reconciliation and selection clearing', async () => {
  const detail = {
    ...studioStoryRecording,
    availability: 'available' as const,
    tracks: [
      {
        id: 'screen',
        kind: 'video' as const,
        label: 'Screen',
        description: '',
        codec: 'h264',
        streamIndex: 0,
      },
    ],
  };
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  const model: StudioSidebarModel = {
    detail,
    aiProcessingState: null,
    visualFrames: [],
    selection: null,
  };
  const view = render(<StudioSidebar actions={actions} model={model} />);
  await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
  useStudioTimelineStore
    .getState()
    .initializeFromDetail(structuredClone(detail));
  view.rerender(
    <StudioSidebar
      actions={actions}
      model={{ ...model, detail: structuredClone(detail), resetRevision: 1 }}
    />,
  );
  expect(screen.getByRole('tab', { name: 'Review' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  useStudioTimelineStore.getState().selectLane(detail.tracks[0]!.id);
  await screen.findByRole('tab', { name: 'Details', selected: true });
  await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
  useStudioTimelineStore.getState().selectLane(detail.tracks[0]!.id);
  await screen.findByRole('tab', { name: 'Details', selected: true });
  useStudioTimelineStore.getState().clearSelection();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
  view.rerender(
    <StudioSidebar actions={actions} model={{ ...model, resetRevision: 2 }} />,
  );
  expect(screen.getByRole('tab', { name: 'Review' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});

test('selected screen details survive a viewport page replacement and are withheld immediately after privacy changes', async () => {
  const detail = {
    ...studioStoryRecording,
    availability: 'available' as const,
    tracks: [
      {
        id: 'screen',
        kind: 'video' as const,
        label: 'Screen',
        description: '',
        codec: 'h264',
        streamIndex: 0,
      },
    ],
  };
  const frame: StudioVisualFrame = {
    id: 'selected-frame',
    sourceTrackId: 'screen',
    frameTimestampMs: 5000,
    endTimestampMs: 5001,
    sourceFrameRef: 'synthetic',
    sourceFrameRefs: ['synthetic'],
    ocrRegions: [],
    vlmObservations: [
      {
        id: 'observation',
        chunkId: 'chunk',
        jobId: 'job',
        recordingId: detail.id,
        frameTimestampMs: 5000,
        sourceFrameRef: 'synthetic',
        region: { x: 0, y: 0, width: 100, height: 100 },
        text: 'Synthetic selected screen change',
        confidence: null,
        provider: 'openai-compatible',
        model: null,
        promptVersion: 'screen-changes-v1',
        observationKind: 'screen-observation',
      },
    ],
  };
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  const model: StudioSidebarModel = {
    detail,
    aiProcessingState: null,
    visualFrames: [frame],
    selection: null,
  };
  const view = render(<StudioSidebar actions={actions} model={model} />);
  useStudioTimelineStore.getState().selectVisualFrame(frame.id);
  await screen.findByText('Synthetic selected screen change');
  view.rerender(
    <StudioSidebar actions={actions} model={{ ...model, visualFrames: [] }} />,
  );
  expect(screen.getByText('Synthetic selected screen change')).toBeTruthy();
  view.rerender(
    <StudioSidebar
      actions={actions}
      model={{
        ...model,
        visualFrames: [],
        detail: {
          ...detail,
          redactions: [
            {
              id: 'cut',
              recordingId: detail.id,
              createdAtMs: 1,
              startOffsetMs: 4000,
              endOffsetMs: 6000,
              trackIds: ['screen'],
              reason: null,
            },
          ],
        },
      }}
    />,
  );
  expect(screen.queryByText('Synthetic selected screen change')).toBeNull();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});
