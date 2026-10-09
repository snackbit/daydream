import type { RecorderStatus } from '@daydream/shared';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { RecorderSessionControls } from '@/features/recorder/components/recorder-session-controls';
import { useRecorderSession } from '@/features/recorder/use-recorder-session';
import '@/index.css';
import { CaptureScreen } from './CaptureScreen';
import {
  cleanupCaptureFixture,
  client,
  deferred,
  readyStatus,
  resetCaptureFixture,
  savedScene,
} from './capture-browser-fixture';

vi.mock('@/features/recorder/recorder-client');
vi.mock('@/features/recorder/source-capture-client');
beforeEach(resetCaptureFixture);
afterEach(cleanupCaptureFixture);

function GlobalRecorderSessionFixture() {
  const session = useRecorderSession();
  return <RecorderSessionControls {...session} />;
}

test('CAP-LIFECYCLE-01 start, pause, failed resume, retry and stop preserve session ownership', async () => {
  const recording: RecorderStatus = {
    ...readyStatus,
    state: 'recording',
    activeRecordingId: 'synthetic-recording',
    activeSceneId: savedScene.id,
    activeSegmentId: 'synthetic-segment',
    updatedAtMs: 3,
  };
  const paused: RecorderStatus = {
    ...recording,
    state: 'paused',
    updatedAtMs: 4,
  };
  const starting = deferred<RecorderStatus>();
  const stopping = deferred<RecorderStatus>();
  client.startRecording.mockReturnValueOnce(starting.promise);
  client.pauseRecording.mockResolvedValue(paused);
  client.resumeRecording
    .mockRejectedValueOnce({
      category: 'dependency',
      code: 'recorder_storage_error',
      message: 'Synthetic resume unavailable',
      details: null,
    })
    .mockResolvedValueOnce({ ...recording, updatedAtMs: 5 });
  client.stopRecording.mockReturnValueOnce(stopping.promise);
  render(<CaptureScreen />);
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled(),
  );
  await userEvent.dblClick(screen.getByRole('button', { name: 'Start' }));
  await waitFor(() => expect(client.startRecording).toHaveBeenCalledTimes(1));
  expect(screen.getByRole('button', { name: 'Starting' })).toBeDisabled();
  expect(
    screen.getByRole('button', { name: 'Add monitor sources' }),
  ).toBeDisabled();
  expect(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
  ).toBeDisabled();
  starting.resolve(recording);
  await screen.findByRole('button', { name: 'Pause' });
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  await screen.findByRole('button', { name: 'Resume' });
  expect(screen.getByRole('combobox', { name: 'Output scale' })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Resume' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('button', { name: 'Resume' })).toBeEnabled();
  expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
  expect(screen.getByText('Status: paused')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Resume' }));
  await screen.findByRole('button', { name: 'Pause' });
  expect(screen.queryByRole('alert')).toBeNull();
  await userEvent.dblClick(screen.getByRole('button', { name: 'Stop' }));
  expect(client.stopRecording).toHaveBeenCalledTimes(1);
  expect(screen.getByRole('button', { name: 'Stopping' })).toBeDisabled();
  stopping.resolve({ ...readyStatus, updatedAtMs: 6 });
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled(),
  );
  expect(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
  ).toBeEnabled();
  expect(screen.getByText('Source setup saved')).toBeTruthy();
  expect(client.pauseRecording).toHaveBeenCalledWith({
    recordingId: 'synthetic-recording',
  });
  expect(client.resumeRecording).toHaveBeenNthCalledWith(2, {
    recordingId: 'synthetic-recording',
  });
  expect(client.stopRecording).toHaveBeenCalledWith({
    recordingId: 'synthetic-recording',
  });
});

test('CAP-LIFECYCLE-02 failed start retains saved setup and retries without picking monitors again', async () => {
  client.startRecording
    .mockRejectedValueOnce({
      category: 'permission',
      code: 'recorder_portal_permission_denied',
      message: 'Synthetic capture permission expired',
      details: null,
    })
    .mockResolvedValueOnce({
      ...readyStatus,
      state: 'recording',
      activeRecordingId: 'synthetic-recording',
      activeSceneId: savedScene.id,
      updatedAtMs: 3,
    });
  render(<CaptureScreen />);
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled(),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Start' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled();
  expect(screen.getByText('Saved video: 1440 × 900')).toBeTruthy();
  expect(screen.getByText('Source setup saved')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Start' }));
  await screen.findByRole('button', { name: 'Pause' });
  expect(client.startRecording).toHaveBeenCalledTimes(2);
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
});

test('CAP-LIFECYCLE-03 pause and stop failures keep recording controlled until stop retry succeeds', async () => {
  const recording: RecorderStatus = {
    ...readyStatus,
    state: 'recording',
    activeRecordingId: 'synthetic-recording',
    activeSceneId: savedScene.id,
    updatedAtMs: 3,
  };
  client.getRecorderStatus.mockResolvedValue(recording);
  client.pauseRecording.mockRejectedValueOnce({
    category: 'lifecycle',
    code: 'recorder_already_active',
    message: 'Synthetic pause failed',
    details: null,
  });
  client.stopRecording
    .mockRejectedValueOnce({
      category: 'lifecycle',
      code: 'recorder_already_active',
      message: 'Synthetic stop failed',
      details: null,
    })
    .mockResolvedValueOnce({ ...readyStatus, updatedAtMs: 4 });
  render(<CaptureScreen />);
  await userEvent.click(await screen.findByRole('button', { name: 'Pause' }));
  await screen.findByRole('alert');
  expect(screen.getByText('Status: recording')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
  expect(screen.getByRole('combobox', { name: 'Output scale' })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Stop' }));
  await screen.findByText('Synthetic stop failed');
  expect(screen.getByText('Status: recording')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
  await userEvent.click(screen.getByRole('button', { name: 'Stop' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled(),
  );
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByRole('combobox', { name: 'Output scale' })).toBeEnabled();
  expect(client.stopRecording).toHaveBeenCalledTimes(2);
});

test('CAP-LIFECYCLE-04 pause finalization keeps Resume disabled until the native completion event', async () => {
  const recording: RecorderStatus = {
    ...readyStatus,
    state: 'recording',
    activeRecordingId: 'synthetic-recording',
    activeSceneId: savedScene.id,
    activeSegmentId: 'synthetic-segment',
    updatedAtMs: 3,
  };
  const finalizingPause: RecorderStatus = {
    ...recording,
    state: 'paused',
    finalization: {
      action: 'pause',
      phase: 'stopping-capture',
      recordingId: 'synthetic-recording',
      segmentId: 'synthetic-segment',
      startedAtMs: 4,
    },
    updatedAtMs: 4,
  };
  let publishStatus: ((status: RecorderStatus) => void) | undefined;
  client.listenToRecorderStatusChanged.mockImplementation(async (handler) => {
    publishStatus = handler;
    return vi.fn<() => void>();
  });
  client.getRecorderStatus.mockResolvedValue(recording);
  client.pauseRecording.mockResolvedValue(finalizingPause);
  client.resumeRecording.mockResolvedValue({ ...recording, updatedAtMs: 6 });
  render(
    <>
      <GlobalRecorderSessionFixture />
      <CaptureScreen />
    </>,
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Pause' }));
  await screen.findByText('Saving segment');
  expect(screen.getByRole('button', { name: 'Resume' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Stop' })).toBeDisabled();
  expect(
    screen.getByRole('button', { name: 'Resume recording' }),
  ).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Stop recording' })).toBeDisabled();
  expect(screen.getByRole('combobox', { name: 'Output scale' })).toBeDisabled();
  if (!publishStatus)
    throw new Error('Expected native recorder status listener');
  publishStatus({
    ...recording,
    state: 'paused',
    activeSegmentId: null,
    updatedAtMs: 5,
  });
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Resume' })).toBeEnabled(),
  );
  expect(
    screen.getByRole('button', { name: 'Resume recording' }),
  ).toBeEnabled();
  expect(screen.queryByText('Saving segment')).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Resume' }));
  await screen.findByRole('button', { name: 'Pause' });
  expect(client.resumeRecording).toHaveBeenCalledExactlyOnceWith({
    recordingId: 'synthetic-recording',
  });
});
