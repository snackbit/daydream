import type { AppSettingsState, RecorderScene } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useSettingsStore } from '@/features/settings/settings-store';
import { SettingsScreen } from './SettingsScreen';

const settingsClient = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  resetAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
}));
const recorderClient = vi.hoisted(() => ({
  getRecorderScene: vi.fn(),
}));

vi.mock('@/features/settings/settings-client', () => settingsClient);
vi.mock('@/features/recorder/recorder-client', () => recorderClient);

const defaultSettingsState = {
  defaults: {
    record: {
      fps: 0.2,
      videoBitrateKbps: 1_500,
    },
    storage: {
      retentionDays: 7,
      sizeLimitBytes: 50 * 1024 * 1024 * 1024,
    },
  },
  limits: {
    record: {
      fpsMax: 5,
      fpsMin: 1 / 60,
      videoBitrateKbpsMax: 10_000,
      videoBitrateKbpsMin: 250,
    },
    storage: {
      retentionDaysMax: 365,
      retentionDaysMin: 1,
      sizeLimitBytesMax: null,
      sizeLimitBytesMin: 1024 * 1024 * 1024,
    },
  },
  settings: {
    record: {
      fps: 0.2,
      videoBitrateKbps: 1_500,
    },
    storage: {
      retentionDays: 7,
      sizeLimitBytes: 50 * 1024 * 1024 * 1024,
    },
  },
} satisfies AppSettingsState;

const scene = {
  canvasHeight: 1440,
  canvasWidth: 2560,
  desktopAudioSourceId: 'desktop',
  id: 'scene-1',
  layoutMode: 'horizontal',
  microphoneAudioSourceId: 'mic',
  name: 'Scene',
  sources: [],
} satisfies RecorderScene;

beforeEach(() => {
  vi.resetAllMocks();
  settingsClient.getAppSettings.mockResolvedValue(defaultSettingsState);
  settingsClient.saveAppSettings.mockImplementation(async ({ settings }) => ({
    ...defaultSettingsState,
    settings,
  }));
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 0,
    sizeBytes: 0,
  });
  settingsClient.resetAppSettings.mockResolvedValue(defaultSettingsState);
  recorderClient.getRecorderScene.mockResolvedValue(scene);
  useSettingsStore.setState({
    draft: null,
    error: null,
    isDirty: false,
    loadStatus: 'idle',
    mutationStatus: 'idle',
    savedScene: null,
    settingsState: null,
    successMessage: null,
  });
});

afterEach(() => {
  cleanup();
});

test('SettingsScreen edits record and storage settings then resets them', async () => {
  render(<SettingsScreen />);

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
  expect(screen.getByText('2560 x 1440')).toBeTruthy();
  expect(screen.getByText('712.39 MiB')).toBeTruthy();

  fireEvent.change(screen.getByLabelText('Seconds per frame'), {
    target: { value: '10' },
  });
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  const storageTab = screen.getByRole('tab', { name: 'Storage/Library' });
  fireEvent.mouseDown(storageTab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(storageTab);
  fireEvent.click(storageTab);
  await screen.findByLabelText('Recording retention');
  fireEvent.change(screen.getByLabelText('Recording retention'), {
    target: { value: '30' },
  });
  fireEvent.change(screen.getByLabelText('Storage size limit'), {
    target: { value: '60' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(
      settingsClient.previewAppSettingsStorageCleanup,
    ).toHaveBeenCalledWith({
      settings: {
        record: {
          fps: 0.1,
          videoBitrateKbps: 2_000,
        },
        storage: {
          retentionDays: 30,
          sizeLimitBytes: 60 * 1024 * 1024 * 1024,
        },
      },
    }),
  );
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        record: {
          fps: 0.1,
          videoBitrateKbps: 2_000,
        },
        storage: {
          retentionDays: 30,
          sizeLimitBytes: 60 * 1024 * 1024 * 1024,
        },
      },
    }),
  );
  expect(await screen.findByText('Settings saved.')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^reset$/i }));
  fireEvent.click(screen.getByRole('button', { name: /^reset settings$/i }));

  await waitFor(() =>
    expect(
      settingsClient.previewAppSettingsStorageCleanup,
    ).toHaveBeenCalledWith({
      settings: defaultSettingsState.defaults,
    }),
  );
  await waitFor(() =>
    expect(settingsClient.resetAppSettings).toHaveBeenCalled(),
  );
  expect(await screen.findByText('Settings reset.')).toBeTruthy();
});

test('SettingsScreen confirms cleanup before saving deletion-causing storage settings', async () => {
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 3 * 1024 * 1024 * 1024,
  });
  render(<SettingsScreen />);

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  expect(await screen.findByText('Delete old recordings?')).toBeTruthy();
  expect(screen.getByText(/delete 2 sessions/i)).toBeTruthy();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole('button', { name: /^delete and save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalled(),
  );
});
