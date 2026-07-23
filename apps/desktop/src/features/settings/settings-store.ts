import type {
  AppSettings,
  AppSettingsState,
  AppSettingsStorageCleanupPreview,
  RecorderScene,
} from '@daydream/shared';
import { create } from 'zustand';
import { getRecorderScene } from '@/features/recorder/recorder-client';
import {
  runAsyncStoreAction,
  unknownErrorMessage,
} from '@/lib/async-store-action';
import {
  getAppSettings,
  previewAppSettingsStorageCleanup,
  resetAppSettings,
  saveAppSettings,
} from './settings-client';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';
type MutationStatus = 'error' | 'idle' | 'loading' | 'success';

interface SettingsState {
  draft: AppSettings | null;
  error: string | null;
  isDirty: boolean;
  load: () => Promise<void>;
  loadStatus: AsyncStatus;
  mutationStatus: MutationStatus;
  previewStorageCleanup: (
    settings: AppSettings,
  ) => Promise<AppSettingsStorageCleanupPreview | null>;
  reset: () => Promise<void>;
  savedScene: RecorderScene | null;
  save: (settings?: AppSettings) => Promise<void>;
  setDraft: (draft: AppSettings) => void;
  settingsState: AppSettingsState | null;
  successMessage: string | null;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  draft: null,
  error: null,
  isDirty: false,
  async load() {
    await runAsyncStoreAction({
      before: () =>
        set({
          error: null,
          loadStatus: 'loading',
          successMessage: null,
        }),
      failure: (error) =>
        set({
          error: settingsErrorMessage(error),
          loadStatus: 'error',
        }),
      run: async () => {
        const [settingsState, savedScene] = await Promise.all([
          getAppSettings(),
          getRecorderScene(),
        ]);

        return { savedScene, settingsState };
      },
      success: ({ savedScene, settingsState }) =>
        set({
          draft: settingsState.settings,
          error: null,
          isDirty: false,
          loadStatus: 'ready',
          savedScene,
          settingsState,
        }),
    });
  },
  loadStatus: 'idle',
  mutationStatus: 'idle',
  async previewStorageCleanup(settings) {
    set({
      error: null,
      successMessage: null,
    });
    try {
      return await previewAppSettingsStorageCleanup({ settings });
    } catch (error) {
      set({
        error: settingsErrorMessage(error),
        mutationStatus: 'error',
      });
      return null;
    }
  },
  async reset() {
    await runAsyncStoreAction({
      before: () =>
        set({
          error: null,
          mutationStatus: 'loading',
          successMessage: null,
        }),
      failure: (error) =>
        set({
          error: settingsErrorMessage(error),
          mutationStatus: 'error',
        }),
      run: resetAppSettings,
      success: (settingsState) =>
        set({
          draft: settingsState.settings,
          error: null,
          isDirty: false,
          mutationStatus: 'success',
          settingsState,
          successMessage: settingsMutationMessage(
            'Settings reset.',
            settingsState.storageCleanup,
          ),
        }),
    });
  },
  savedScene: null,
  async save(settings) {
    const settingsToSave = settings ?? get().draft;
    if (!settingsToSave) {
      set({ error: 'No settings are loaded.' });
      return;
    }

    await runAsyncStoreAction({
      before: () =>
        set({
          error: null,
          mutationStatus: 'loading',
          successMessage: null,
        }),
      failure: (error) =>
        set({
          error: settingsErrorMessage(error),
          mutationStatus: 'error',
        }),
      run: () => saveAppSettings({ settings: settingsToSave }),
      success: (settingsState) =>
        set({
          draft: settingsState.settings,
          error: null,
          isDirty: false,
          mutationStatus: 'success',
          settingsState,
          successMessage: settingsMutationMessage(
            'Settings saved.',
            settingsState.storageCleanup,
          ),
        }),
    });
  },
  setDraft(draft) {
    const settingsState = get().settingsState;
    set({
      draft,
      error: null,
      isDirty: settingsState
        ? !settingsEqual(draft, settingsState.settings)
        : true,
      mutationStatus: 'idle',
      successMessage: null,
    });
  },
  settingsState: null,
  successMessage: null,
}));

function settingsEqual(left: AppSettings, right: AppSettings): boolean {
  return (
    left.record.fps === right.record.fps &&
    left.record.videoBitrateKbps === right.record.videoBitrateKbps &&
    JSON.stringify(left.record.windowProtection) ===
      JSON.stringify(right.record.windowProtection) &&
    (left.debug?.enabled ?? false) === (right.debug?.enabled ?? false) &&
    (left.expert?.enabled ?? false) === (right.expert?.enabled ?? false) &&
    left.storage.retentionDays === right.storage.retentionDays &&
    left.storage.sizeLimitBytes === right.storage.sizeLimitBytes &&
    JSON.stringify(left.ai) === JSON.stringify(right.ai)
  );
}

function settingsMutationMessage(
  baseMessage: string,
  cleanup: AppSettingsState['storageCleanup'],
): string {
  if (!cleanup) {
    return baseMessage;
  }

  const deletedMessage =
    cleanup.deletedRecordingCount > 0
      ? ` Deleted ${cleanup.deletedRecordingCount} session${
          cleanup.deletedRecordingCount === 1 ? '' : 's'
        }.`
      : '';
  const failureMessage =
    cleanup.failedRecordingCount > 0
      ? ` ${cleanup.failedRecordingCount} cleanup deletion${
          cleanup.failedRecordingCount === 1 ? '' : 's'
        } failed.`
      : '';

  return `${baseMessage}${deletedMessage}${failureMessage}`;
}

function settingsErrorMessage(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return error.message;
  }

  return unknownErrorMessage(error);
}
