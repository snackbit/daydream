import { useEffect, useMemo, useState } from 'react';
import { useBlocker } from '@tanstack/react-router';
import type {
  AppSettings,
  AppSettingsStorageCleanupPreview,
} from '@daydream/shared';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import type { CadenceMode } from '@/features/settings/components/record-settings-tab';
import {
  collectSupersededAiSecretRefs,
  settingsForCleanupPreview,
} from '@/features/settings/settings-ai-secret-utils';
import { validateDraftSettings } from '@/features/settings/settings-screen-utils';
import { useSettingsStore } from '@/features/settings/settings-store';
import { useSettingsAiController } from '@/features/settings/use-settings-ai-controller';

type CleanupConfirmationAction = 'reset' | 'save';

export interface CleanupConfirmation {
  action: CleanupConfirmationAction;
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
  const reset = useSettingsStore((state) => state.reset);
  const save = useSettingsStore((state) => state.save);
  const savedScene = useSettingsStore((state) => state.savedScene);
  const setDraft = useSettingsStore((state) => state.setDraft);
  const settingsState = useSettingsStore((state) => state.settingsState);
  const successMessage = useSettingsStore((state) => state.successMessage);
  const loadNotifications = useNotificationsStore((state) => state.load);
  const [cadenceMode, setCadenceMode] = useState<CadenceMode | null>(null);
  const [cleanupConfirmation, setCleanupConfirmation] =
    useState<CleanupConfirmation | null>(null);
  const [cleanupPreviewing, setCleanupPreviewing] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const selectedAiModelId = draft?.ai.transcription.localModelId ?? null;
  const ai = useSettingsAiController({
    loadNotifications,
    selectedAiModelId,
  });

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  const validationError = useMemo(
    () =>
      draft && settingsState
        ? validateDraftSettings(draft, settingsState.limits)
        : null,
    [draft, settingsState],
  );
  const saving = mutationStatus === 'loading';
  const busy = saving || cleanupPreviewing;
  const hasUnsavedSettings = isDirty || ai.hasAiSecretDraft;
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

    setDraft(updater(draft));
  }

  async function requestSave() {
    if (!draft) {
      return;
    }

    await runWithCleanupConfirmation('save', settingsForCleanupPreview(draft), {
      proceedAfterSuccess: false,
    });
  }

  async function requestReset() {
    if (!settingsState) {
      return;
    }

    ai.setPendingSecretRefsForDeletion(
      collectSupersededAiSecretRefs(
        settingsState.settings,
        settingsState.defaults,
      ),
    );
    await runWithCleanupConfirmation('reset', settingsState.defaults, {
      proceedAfterSuccess: false,
    });
  }

  async function runWithCleanupConfirmation(
    action: CleanupConfirmationAction,
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
        if (action === 'reset') {
          setResetOpen(false);
        }
        setCleanupConfirmation({
          action,
          preview,
          proceedAfterSuccess,
          settings,
        });
        return false;
      }

      if (action === 'save') {
        return await savePreparedSettings(settings);
      }

      return await confirmReset();
    } finally {
      setCleanupPreviewing(false);
    }
  }

  async function confirmReset(): Promise<boolean> {
    await reset();
    await ai.deletePendingAiSecrets();
    await ai.refreshAiScaffolds();
    setCadenceMode(null);
    setResetOpen(false);
    return true;
  }

  async function confirmCleanup() {
    const confirmation = cleanupConfirmation;
    if (!confirmation) {
      return;
    }

    setCleanupConfirmation(null);
    if (confirmation.action === 'save') {
      const saved = await savePreparedSettings(confirmation.settings);
      if (saved && confirmation.proceedAfterSuccess) {
        blocker.proceed?.();
      }
    } else {
      await confirmReset();
    }
  }

  async function savePreparedSettings(settings: AppSettings): Promise<boolean> {
    const preparation = await ai.prepareAiSettingsForSave(
      settings,
      settingsState?.settings ?? null,
    );
    if (!preparation) {
      return false;
    }

    try {
      await save(preparation.settings);
    } catch {
      ai.setPendingSecretRefsForDeletion(preparation.createdSecretRefs);
      await ai.deletePendingAiSecrets();
      return false;
    }

    ai.setPendingSecretRefsForDeletion(preparation.secretRefsToDeleteAfterSave);
    await ai.deletePendingAiSecrets();
    ai.clearSecretDrafts();
    await ai.refreshAiScaffolds();
    return true;
  }

  async function saveAndLeaveSettings() {
    if (!draft) {
      return;
    }

    const saved = await runWithCleanupConfirmation(
      'save',
      settingsForCleanupPreview(draft),
      { proceedAfterSuccess: true },
    );
    if (saved) {
      blocker.proceed?.();
    }
  }

  function discardAndLeaveSettings() {
    if (settingsState) {
      setDraft(settingsState.settings);
    }
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
    requestReset,
    requestSave,
    resetOpen,
    runAiModelAction: ai.runAiModelAction,
    savedScene,
    saving,
    setCadenceMode,
    setResetOpen,
    setTranscriptionKeyDraft: ai.setTranscriptionKeyDraft,
    setVisionKeyDraft: ai.setVisionKeyDraft,
    settingsState,
    successMessage,
    transcriptionKeyDraft: ai.transcriptionKeyDraft,
    updateDraft,
    validationError,
    visionKeyDraft: ai.visionKeyDraft,
    saveAndLeaveSettings,
  };
}
