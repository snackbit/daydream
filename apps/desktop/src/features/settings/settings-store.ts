import type {
  AppSettings,
  AppSettingsState,
  AppSettingsStorageCleanupPreview,
  RecorderScene,
  SettingsCredentialDrafts,
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
  invalidatePreferences: () => void;
  preferenceRevision: number;
  loadStatus: AsyncStatus;
  mutationStatus: MutationStatus;
  previewStorageCleanup: (
    settings: AppSettings,
  ) => Promise<AppSettingsStorageCleanupPreview | null>;
  reset: () => Promise<void>;
  savedScene: RecorderScene | null;
  save: (
    settings?: AppSettings,
    credentials?: SettingsCredentialDrafts,
  ) => Promise<void>;
  setDraft: (draft: AppSettings) => void;
  settingsState: AppSettingsState | null;
  successMessage: string | null;
}

export function createSettingsStore() {
  let generation = 0;
  return create<SettingsState>((set, get) => ({
    preferenceRevision: 0,
    invalidatePreferences() {
      generation++;
      set({
        draft: null,
        error: null,
        isDirty: false,
        loadStatus: 'loading',
        mutationStatus: 'idle',
        savedScene: null,
        settingsState: null,
        successMessage: null,
        preferenceRevision: get().preferenceRevision + 1,
      });
    },
    draft: null,
    error: null,
    isDirty: false,
    async load() {
      const ownedGeneration = generation;
      await runAsyncStoreAction({
        before: () =>
          set({
            error: null,
            loadStatus: 'loading',
            successMessage: null,
          }),
        failure: (error) => {
          if (ownedGeneration !== generation) return false;
          set({ error: settingsErrorMessage(error), loadStatus: 'error' });
          return undefined;
        },
        run: async () => {
          const [settingsState, savedScene] = await Promise.all([
            getAppSettings(),
            getRecorderScene(),
          ]);

          return { savedScene, settingsState };
        },
        success: ({ savedScene, settingsState }) => {
          if (ownedGeneration === generation)
            set({
              draft: settingsState.settings,
              error: null,
              isDirty: false,
              loadStatus: 'ready',
              savedScene,
              settingsState,
            });
        },
      });
    },
    loadStatus: 'idle',
    mutationStatus: 'idle',
    async previewStorageCleanup(settings) {
      const ownedGeneration = generation;
      set({
        error: null,
        successMessage: null,
      });
      try {
        const preview = await previewAppSettingsStorageCleanup({ settings });
        return ownedGeneration === generation ? preview : null;
      } catch (error) {
        if (ownedGeneration !== generation) return null;
        set({
          error: settingsErrorMessage(error),
          mutationStatus: 'error',
        });
        return null;
      }
    },
    async reset() {
      const ownedGeneration = generation;
      if (get().mutationStatus === 'loading')
        throw new Error('A settings request is already running.');
      await runAsyncStoreAction({
        before: () =>
          set({
            error: null,
            mutationStatus: 'loading',
            successMessage: null,
          }),
        failure: (error) => {
          if (ownedGeneration !== generation) return false;
          set({ error: settingsErrorMessage(error), mutationStatus: 'error' });
          return undefined;
        },
        run: resetAppSettings,
        success: (settingsState) => {
          if (ownedGeneration === generation)
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
            });
        },
      });
    },
    savedScene: null,
    async save(settings, credentials) {
      const ownedGeneration = generation;
      if (get().mutationStatus === 'loading')
        throw new Error('A settings request is already running.');
      const draftAtSave = get().draft;
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
        failure: (error) => {
          if (ownedGeneration !== generation) return false;
          set({ error: settingsErrorMessage(error), mutationStatus: 'error' });
          return undefined;
        },
        run: () =>
          saveAppSettings({
            settings: settingsToSave,
            ...(credentials && Object.keys(credentials).length
              ? { credentials }
              : {}),
          }),
        success: (settingsState) => {
          if (ownedGeneration !== generation) return;
          const draft =
            get().draft === draftAtSave ? settingsState.settings : get().draft;
          set({
            draft,
            error: null,
            isDirty:
              draft !== null && !settingsEqual(draft, settingsState.settings),
            mutationStatus: 'success',
            settingsState,
            successMessage: settingsMutationMessage(
              'Settings saved.',
              settingsState.storageCleanup,
            ),
          });
        },
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
        mutationStatus: get().mutationStatus === 'loading' ? 'loading' : 'idle',
        successMessage: null,
      });
    },
    settingsState: null,
    successMessage: null,
  }));
}
export const useSettingsStore = createSettingsStore();

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
