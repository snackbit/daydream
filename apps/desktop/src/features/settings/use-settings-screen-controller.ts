import { useEffect, useMemo, useState } from 'react';
import { useBlocker } from '@tanstack/react-router';
import type {
  AppSettings,
  AppSettingsStorageCleanupPreview,
} from '@daydream/shared';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import type { CadenceMode } from '@/features/settings/components/record-settings-tab';
import { validateDraftSettings } from '@/features/settings/settings-screen-utils';
import { useSettingsStore } from '@/features/settings/settings-store';
import { useSettingsAiController } from '@/features/settings/use-settings-ai-controller';
import { applySettingsEdit } from './autosave/settings-edit';

export interface CleanupConfirmation {
  proceedAfterSuccess: boolean;
  preview: AppSettingsStorageCleanupPreview;
  settings: AppSettings;
}

export function useSettingsScreenController() {
  const draft = useSettingsStore((state) => state.draft);
  const error = useSettingsStore((state) => state.error);
  const isDirty = useSettingsStore((state) => state.isDirty);
  const load = useSettingsStore((state) => state.load);
  const loadStatus = useSettingsStore((state) => state.loadStatus);
  const mutationStatus = useSettingsStore((state) => state.mutationStatus);
  const previewStorageCleanup = useSettingsStore(
    (state) => state.previewStorageCleanup,
  );
  const commit = useSettingsStore((state) => state.commit);
  const retryAutosave = useSettingsStore((state) => state.retryAutosave);
  const savedScene = useSettingsStore((state) => state.savedScene);
  const settingsState = useSettingsStore((state) => state.settingsState);
  const preferenceRevision = useSettingsStore(
    (state) => state.preferenceRevision,
  );
  const successMessage = useSettingsStore((state) => state.successMessage);
  const loadNotifications = useNotificationsStore((state) => state.load);
  const [cadenceMode, setCadenceMode] = useState<CadenceMode | null>(null);
  const [cleanupConfirmation, setCleanupConfirmation] =
    useState<CleanupConfirmation | null>(null);
  const [cleanupPreviewing, setCleanupPreviewing] = useState(false);
  const [preparingSave, setPreparingSave] = useState(false);
  const selectedAiModelId = draft?.ai.transcription.localModelId ?? null;
  const ai = useSettingsAiController({
    loadNotifications,
    selectedAiModelId,
  });
  const { clearSecretDrafts, clearPendingSecretRefsForDeletion } = ai;
  useEffect(() => {
    clearSecretDrafts();
    clearPendingSecretRefsForDeletion();
    setCadenceMode(null);
  }, [
    preferenceRevision,
    clearSecretDrafts,
    clearPendingSecretRefsForDeletion,
  ]);

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load().catch(() => undefined);
    }
  }, [load, loadStatus]);

  const validationError = useMemo(
    () =>
      draft && settingsState
        ? validateDraftSettings(draft, settingsState.limits)
        : null,
    [draft, settingsState],
  );
  const saving = mutationStatus === 'loading' || preparingSave;
  const busy = saving || cleanupPreviewing;
  const hasUnsavedSettings = isDirty && mutationStatus === 'error';
  const canSave = Boolean(
    draft && hasUnsavedSettings && !validationError && !busy,
  );

  const blocker = useBlocker({
    enableBeforeUnload: hasUnsavedSettings,
    shouldBlockFn: ({ current, next }) =>
      hasUnsavedSettings &&
      current.pathname === '/settings' &&
      next.pathname !== '/settings',
    withResolver: true,
  });

  function updateDraft(updater: (draft: AppSettings) => AppSettings) {
    if (!draft) {
      return;
    }

    void commit(updater);
  }

  function retryLoadSettings() {
    void load().catch(() => undefined);
  }

  async function requestSave() {
    if (!draft) {
      return;
    }

    await retryAutosave();
  }

  async function runWithCleanupConfirmation(
    settings: AppSettings,
    { proceedAfterSuccess }: { proceedAfterSuccess: boolean },
  ): Promise<boolean> {
    setCleanupPreviewing(true);
    try {
      const preview = await previewStorageCleanup(settings);
      if (!preview) {
        return false;
      }
      if (preview.recordingCount > 0) {
        setCleanupConfirmation({
          preview,
          proceedAfterSuccess,
          settings,
        });
        return false;
      }

      return await commit(
        (current) => ({ ...current, storage: settings.storage }),
        undefined,
        true,
      );
    } finally {
      setCleanupPreviewing(false);
    }
  }

  async function confirmCleanup() {
    const confirmation = cleanupConfirmation;
    if (!confirmation) {
      return;
    }

    setCleanupConfirmation(null);
    const saved = await commit(
      (current) => ({ ...current, storage: confirmation.settings.storage }),
      undefined,
      true,
    );
    if (saved && confirmation.proceedAfterSuccess) {
      blocker.proceed?.();
    }
  }

  async function savePreparedSettings(settings: AppSettings): Promise<boolean> {
    const base = useSettingsStore.getState().draft;
    if (!base) return false;
    setPreparingSave(true);
    try {
      const preparation = await ai.prepareAiSettingsForSave(settings);
      if (!preparation) {
        return false;
      }

      try {
        const saved = await useSettingsStore
          .getState()
          .commitEditor(
            (latest) => applySettingsEdit(base, preparation.settings, latest),
            preparation.credentials,
          );
        if (!saved) return false;
      } catch {
        return false;
      }

      ai.clearSavedSecretDrafts(preparation.credentials);
      await ai.refreshAiScaffolds();
      return true;
    } finally {
      setPreparingSave(false);
    }
  }

  async function saveAndLeaveSettings() {
    if (!draft) {
      return;
    }

    const saved = await retryAutosave();
    if (saved) {
      blocker.proceed?.();
    }
  }

  function discardAndLeaveSettings() {
    useSettingsStore.getState().revertAutosave();
    ai.clearSecretDrafts();
    ai.clearPendingSecretRefsForDeletion();
    blocker.proceed?.();
  }

  function dismissCleanupConfirmation() {
    if (!saving) {
      setCleanupConfirmation(null);
    }
  }

  return {
    modelKeyDrafts: ai.modelKeyDrafts,
    preferenceRevision,
    changeModelKey: ai.changeModelKey,
    aiActionError: ai.aiActionError,
    aiModels: ai.aiModels,
    aiProviderStatus: ai.aiProviderStatus,
    blocker,
    busy,
    cadenceMode,
    canSave,
    cleanupConfirmation,
    confirmCleanup,
    discardAndLeaveSettings,
    dismissCleanupConfirmation,
    draft,
    error,
    loadStatus,
    retryLoadSettings,
    inputActivityRepairMessage: ai.inputActivityRepairMessage,
    inputActivityRepairing: ai.inputActivityRepairing,
    inputActivityStatus: ai.inputActivityStatus,
    requestSave,
    savePreparedSettings,
    updateStorage: (updater: (draft: AppSettings) => AppSettings) => {
      if (draft)
        void runWithCleanupConfirmation(updater(draft), {
          proceedAfterSuccess: false,
        });
    },
    clearSecretDrafts: ai.clearSecretDrafts,
    refreshInputActivityStatus: ai.refreshInputActivityStatus,
    repairInputActivity: ai.repairInputActivity,
    runAiModelAction: ai.runAiModelAction,
    runModelTransferAction: ai.runModelTransferAction,
    modelActionErrors: ai.modelActionErrors,
    savedScene,
    saving,
    setCadenceMode,
    setTranscriptionKeyDraft: ai.setTranscriptionKeyDraft,
    setVisionKeyDraft: ai.setVisionKeyDraft,
    setRecallEmbeddingKeyDraft: ai.setRecallEmbeddingKeyDraft,
    setRecallGenerationKeyDraft: ai.setRecallGenerationKeyDraft,
    recallEmbeddingKeyDraft: ai.recallEmbeddingKeyDraft,
    recallGenerationKeyDraft: ai.recallGenerationKeyDraft,
    settingsState,
    successMessage,
    transcriptionKeyDraft: ai.transcriptionKeyDraft,
    updateDraft,
    validationError,
    visionKeyDraft: ai.visionKeyDraft,
    saveAndLeaveSettings,
  };
}
