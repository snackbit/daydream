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
import { createSettingsSaveQueue } from './autosave/save-queue';
import { applySettingsEdit } from './autosave/settings-edit';
import {
  getAppSettings,
  previewAppSettingsStorageCleanup,
  resetAppSettings,
  saveAppSettings,
} from './settings-client';
import { validateDraftSettings } from './settings-screen-utils';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';
type MutationStatus = 'error' | 'idle' | 'loading' | 'success';

interface SettingsState {
  commitEditor: (
    update: (current: AppSettings) => AppSettings,
    credentials?: SettingsCredentialDrafts,
  ) => Promise<boolean>;
  commit: (
    update: (current: AppSettings) => AppSettings,
    credentials?: SettingsCredentialDrafts,
    applyStorageCleanup?: boolean,
  ) => Promise<boolean>;
  retryAutosave: () => Promise<boolean>;
  revertAutosave: () => void;
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
    applyStorageCleanup?: boolean,
  ) => Promise<void>;
  setDraft: (draft: AppSettings) => void;
  settingsState: AppSettingsState | null;
  successMessage: string | null;
}

export function createSettingsStore() {
  let generation = 0;
  let activeSave: Promise<void> | null = null;
  return create<SettingsState>((set, get) => {
    const queue = createSettingsSaveQueue({
      read: () => ({ draft: get().draft, generation, activeSave }),
      save: (settings, credentials, cleanup) =>
        get().save(settings, credentials, cleanup),
    });
    return {
      async commitEditor(update, credentials = {}) {
        const ownedGeneration = generation;
        try {
          if (activeSave) await activeSave;
          if (!(await queue.retry()) || ownedGeneration !== generation)
            return false;
          const state = get();
          if (!state.draft || !state.settingsState) return false;
          const next = update(state.draft);
          const error = validateDraftSettings(next, state.settingsState.limits);
          if (error) {
            set({ error, mutationStatus: 'error' });
            return false;
          }
          // Editor drafts and keys stay in their owner until the whole save succeeds.
          await get().save(next, credentials, false);
          return ownedGeneration === generation;
        } catch {
          return false;
        }
      },
      async commit(update, credentials = {}, applyStorageCleanup = false) {
        const state = get();
        if (!state.draft || !state.settingsState) return false;
        const next = update(state.draft);
        const error = validateDraftSettings(next, state.settingsState.limits);
        if (error) {
          set({ error, mutationStatus: 'error' });
          return false;
        }
        if (
          settingsEqual(next, state.draft) &&
          !Object.keys(credentials).length &&
          !applyStorageCleanup
        )
          return true;
        get().setDraft(next);
        return queue.enqueue(credentials, applyStorageCleanup);
      },
      retryAutosave: queue.retry,
      revertAutosave() {
        if (activeSave) return;
        queue.reset();
        const saved = get().settingsState;
        if (saved)
          set({
            draft: saved.settings,
            isDirty: false,
            error: null,
            mutationStatus: 'idle',
          });
      },
      preferenceRevision: 0,
      invalidatePreferences() {
        generation++;
        queue.reset();
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
        queue.reset();
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
            set({
              error: settingsErrorMessage(error),
              mutationStatus: 'error',
            });
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
      async save(settings, credentials, applyStorageCleanup) {
        const ownedGeneration = generation;
        if (get().mutationStatus === 'loading')
          throw new Error('A settings request is already running.');
        const draftAtSave = get().draft;
        const settingsToSave = settings ?? get().draft;
        if (!settingsToSave) {
          set({ error: 'No settings are loaded.' });
          return;
        }

        const operation = runAsyncStoreAction({
          before: () =>
            set({
              error: null,
              mutationStatus: 'loading',
              successMessage: null,
            }),
          failure: (error) => {
            if (ownedGeneration !== generation) return false;
            set({
              error: settingsErrorMessage(error),
              mutationStatus: 'error',
            });
            return undefined;
          },
          run: () =>
            saveAppSettings({
              settings: settingsToSave,
              ...(applyStorageCleanup !== undefined
                ? { applyStorageCleanup }
                : {}),
              ...(credentials && Object.keys(credentials).length
                ? { credentials }
                : {}),
            }),
          success: (settingsState) => {
            if (ownedGeneration !== generation) return;
            const currentDraft = get().draft;
            const draft =
              currentDraft === draftAtSave
                ? settingsState.settings
                : draftAtSave && currentDraft
                  ? applySettingsEdit(
                      draftAtSave,
                      currentDraft,
                      settingsState.settings,
                    )
                  : currentDraft;
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
        const pending = operation.then(() => undefined);
        activeSave = pending;
        try {
          await pending;
        } finally {
          if (activeSave === pending) activeSave = null;
        }
      },
      setDraft(draft) {
        const settingsState = get().settingsState;
        set({
          draft,
          error: get().mutationStatus === 'error' ? get().error : null,
          isDirty: settingsState
            ? !settingsEqual(draft, settingsState.settings)
            : true,
          mutationStatus:
            get().mutationStatus === 'loading'
              ? 'loading'
              : get().mutationStatus === 'error'
                ? 'error'
                : 'idle',
          successMessage: null,
        });
      },
      settingsState: null,
      successMessage: null,
    };
  });
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
